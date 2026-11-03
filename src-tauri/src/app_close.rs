//! Close handshake for the main window, and the exit request that follows it.
//!
//! Every close request of the main window is held open. The shell saves what it is editing and then
//! answers. It acknowledges first (`app_close_ack`), which shows it is alive and working on the request.
//! It then ends the request with `app_close_confirm` (close the window now) or `app_close_cancel` (keep it
//! open). Until the shell acknowledges, the request sits on a fallback timer of `CLOSE_CONFIRM_TIMEOUT`,
//! so a shell that is hung or never loaded still lets the window close. An acknowledged request has no
//! timer: it ends in confirm or cancel.
//!
//! A request is only pending until the shell answers, so project saves keep working while the shell
//! flushes. `CLOSE_IN_PROGRESS` is raised when the close starts (confirmed or timed out), and from then on
//! project commands are refused, which is what stops a save from racing the project teardown.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use tauri::{ExitRequestApi, Manager};

use crate::ai::SharedAiRuntime;
use crate::image_generation::SharedImageGeneration;
use crate::project_commands::close_project_for_app;
use crate::watcher::SharedProjectWatcher;
use crate::{
    wait_for_project_commands, CloseWorkerGuard, SharedCore, SharedPhysicalJobs, SharedPluginHost,
    CLOSE_IN_PROGRESS, ON_CLOSE_THREAD,
};

/// How long a held close request waits for the shell to acknowledge it before the window closes anyway.
pub(crate) const CLOSE_CONFIRM_TIMEOUT: Duration = Duration::from_secs(8);

/// Set just before a confirmed close destroys the main window. The exit that follows (the last window going
/// away) was asked for by this module, so it must not be routed back into a close.
static EXIT_FROM_CONFIRMED_CLOSE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    Pending,
    InProgress,
}

/// The handshake as plain state, with no clock and no Tauri handles.
///
/// `request` returns the generation that the fallback timer must carry back to `timed_out`. Each request
/// gets a new generation, so a timer left over from a cancelled request cannot close a later one.
#[derive(Debug)]
pub(crate) struct CloseHandshake {
    stage: Stage,
    generation: u64,
    acknowledged: bool,
}

impl CloseHandshake {
    pub(crate) const fn new() -> Self {
        Self {
            stage: Stage::Idle,
            generation: 0,
            acknowledged: false,
        }
    }

    /// Records a close request. Returns the generation for the fallback timer, or `None` when a close is
    /// already pending or in progress, in which case the caller only keeps the window open.
    pub(crate) fn request(&mut self) -> Option<u64> {
        if self.stage != Stage::Idle {
            return None;
        }
        self.generation += 1;
        self.stage = Stage::Pending;
        self.acknowledged = false;
        Some(self.generation)
    }

    /// The shell is alive and working on the request for `generation`. The fallback timer is disarmed, so
    /// the request now ends only in confirm or cancel. Returns false for a stale generation or when no
    /// request is pending.
    pub(crate) fn acknowledge(&mut self, generation: u64) -> bool {
        if self.stage == Stage::Pending && self.generation == generation {
            self.acknowledged = true;
            return true;
        }
        false
    }

    /// The generation of the request that is pending, if any.
    pub(crate) fn pending_generation(&self) -> Option<u64> {
        (self.stage == Stage::Pending).then_some(self.generation)
    }

    /// The shell saved and allows the close. Returns true only for the call that starts it.
    pub(crate) fn confirm(&mut self) -> bool {
        self.start_close(self.generation)
    }

    /// The shell keeps the window open. Has no effect once the close has started.
    pub(crate) fn cancel(&mut self) {
        if self.stage == Stage::Pending {
            self.stage = Stage::Idle;
        }
    }

    /// The fallback timer for `generation` fired. Returns true only when that request is still pending and
    /// the shell has not acknowledged it.
    pub(crate) fn timed_out(&mut self, generation: u64) -> bool {
        if self.acknowledged {
            return false;
        }
        self.start_close(generation)
    }

    /// The close started but the window could not be destroyed, so requests and saves work again.
    pub(crate) fn abandon(&mut self) {
        if self.stage == Stage::InProgress {
            self.stage = Stage::Idle;
        }
    }

    fn start_close(&mut self, generation: u64) -> bool {
        if self.stage == Stage::Pending && self.generation == generation {
            self.stage = Stage::InProgress;
            return true;
        }
        false
    }

    #[cfg(test)]
    pub(crate) fn is_pending(&self) -> bool {
        self.stage == Stage::Pending
    }

    #[cfg(test)]
    pub(crate) fn is_in_progress(&self) -> bool {
        self.stage == Stage::InProgress
    }

    #[cfg(test)]
    pub(crate) fn is_acknowledged(&self) -> bool {
        self.acknowledged
    }
}

/// Process-wide handshake. Every transition that starts or ends a close goes through the helpers below,
/// so `CLOSE_IN_PROGRESS` always agrees with the stage.
static APP_CLOSE: Mutex<CloseHandshake> = Mutex::new(CloseHandshake::new());

fn handshake() -> MutexGuard<'static, CloseHandshake> {
    APP_CLOSE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn request_close() -> Option<u64> {
    handshake().request()
}

/// Acknowledges the request that is pending now. Returns false when nothing is pending.
pub(crate) fn acknowledge_close() -> bool {
    let mut state = handshake();
    match state.pending_generation() {
        Some(generation) => state.acknowledge(generation),
        None => false,
    }
}

pub(crate) fn confirm_close() -> bool {
    let mut state = handshake();
    let started = state.confirm();
    if started {
        CLOSE_IN_PROGRESS.store(true, Ordering::Release);
    }
    started
}

pub(crate) fn cancel_close() {
    handshake().cancel();
}

fn timed_out_close(generation: u64) -> bool {
    let mut state = handshake();
    let started = state.timed_out(generation);
    if started {
        CLOSE_IN_PROGRESS.store(true, Ordering::Release);
    }
    started
}

/// Ends a close that did not destroy the window. Also used when the close worker exits without
/// scheduling the destroy.
pub(crate) fn abandon_close() {
    let mut state = handshake();
    state.abandon();
    CLOSE_IN_PROGRESS.store(false, Ordering::Release);
    EXIT_FROM_CONFIRMED_CLOSE.store(false, Ordering::Release);
}

/// Called from the window event handler after `prevent_close`. Holds the request open and arms the
/// fallback unless the request is already pending or closing.
pub(crate) fn on_close_requested(window: &tauri::Window) {
    let Some(generation) = request_close() else {
        return;
    };
    let app = window.app_handle().clone();
    let window = window.clone();
    thread::spawn(move || {
        thread::sleep(CLOSE_CONFIRM_TIMEOUT);
        if timed_out_close(generation) {
            run_confirmed_close(&app, window);
        }
    });
}

/// Runs the project teardown and then destroys the window, off the main thread.
fn run_confirmed_close(app: &tauri::AppHandle, window: tauri::Window) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ON_CLOSE_THREAD.with(|flag| flag.set(true));
        let mut close_worker = CloseWorkerGuard {
            destroy_scheduled: false,
        };
        wait_for_project_commands();
        let core = app.state::<SharedCore>();
        let jobs = app.state::<SharedPhysicalJobs>();
        let plugins = app.state::<SharedPluginHost>();
        let ai_runtime = app.state::<SharedAiRuntime>();
        let watcher = app.state::<SharedProjectWatcher>();
        let image_jobs = app.state::<SharedImageGeneration>();
        if let Err(error) = close_project_for_app(
            &app,
            core.inner(),
            jobs.inner(),
            plugins.inner(),
            ai_runtime.inner(),
            watcher.inner(),
            image_jobs.inner(),
        ) {
            eprintln!("project cleanup during window close failed: {error}");
        }
        close_worker.destroy_scheduled = app
            .run_on_main_thread(move || {
                // Marked before the destroy, so the exit that the destroy causes is allowed through.
                EXIT_FROM_CONFIRMED_CLOSE.store(true, Ordering::Release);
                if window.destroy().is_err() {
                    abandon_close();
                }
            })
            .is_ok();
    });
}

/// Whether an exit request should be turned into a close of the main window. Every stage qualifies: while a
/// close is pending the close is just held, and while one is in progress the exit must wait for the teardown
/// to finish. Only an exit that this module started, or one with no main window left, is allowed through.
pub(crate) fn should_intercept_exit(initiated_by_us: bool, main_window_exists: bool) -> bool {
    !initiated_by_us && main_window_exists
}

/// Routes `RunEvent::ExitRequested` through the close handshake.
///
/// Tauri 2.11.6 raises `ExitRequested` from two places only: when the last window is destroyed
/// (`tauri-runtime-wry`, `TaoWindowEvent::Destroyed` with an empty window store), and when the app asks to
/// exit (`RequestExit`, from `AppHandle::exit` / `restart`). The app does neither today, so this is a guard
/// for those paths. The macOS Quit menu item does not reach it. The predefined Quit item (`muda`) sends
/// `terminate:` to `NSApp`, and tao 0.35.3 implements no `applicationShouldTerminate:`, so Cmd+Q raises
/// neither `ExitRequested` nor `CloseRequested`.
pub(crate) fn on_exit_requested(app: &tauri::AppHandle, api: &ExitRequestApi) {
    let main = app.get_webview_window("main");
    let initiated_by_us = EXIT_FROM_CONFIRMED_CLOSE.load(Ordering::Acquire);
    if !should_intercept_exit(initiated_by_us, main.is_some()) {
        return;
    }
    api.prevent_exit();
    if let Some(window) = main {
        if let Err(error) = window.close() {
            eprintln!("could not route the exit request through the window close: {error}");
        }
    }
}

fn require_main_window(window: &tauri::Window) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("only the main window answers its close request".into())
    }
}

/// The shell has started on the held close request, so the request no longer needs the fallback timer.
/// The shell cannot name the request before its first call, so this acknowledges the request that is
/// pending now. That is the request the handler was started for: a new request can only become pending
/// once the previous one has been cancelled or has ended, and the shell acknowledges first in its handler.
#[tauri::command]
pub(crate) fn app_close_ack(window: tauri::Window) -> Result<(), String> {
    require_main_window(&window)?;
    acknowledge_close();
    Ok(())
}

/// The shell has saved and its leave guards agree, so the held close may proceed.
#[tauri::command]
pub(crate) fn app_close_confirm(
    window: tauri::Window,
    app: tauri::AppHandle,
) -> Result<(), String> {
    require_main_window(&window)?;
    if confirm_close() {
        run_confirmed_close(&app, window);
    }
    Ok(())
}

/// The shell keeps the window open, either because a save failed or because a leave guard refused.
#[tauri::command]
pub(crate) fn app_close_cancel(window: tauri::Window) -> Result<(), String> {
    require_main_window(&window)?;
    cancel_close();
    Ok(())
}
