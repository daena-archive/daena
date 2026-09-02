// Background checkpoint export worker.
use super::{Generation, ProjectStore};
use crate::error::CoreError;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub(super) const RUNTIME_STORAGE_ROLE: &str = "daena.runtime";
pub(super) const RUNTIME_SCHEMA_VERSION: i64 = 1;
pub(super) const EXPORTER_CONTRACT_VERSION: &str = "1";
pub(super) const BACKGROUND_EXPORT_IDLE_DELAY: Duration = Duration::from_secs(2);
pub(super) const BACKGROUND_EXPORT_MAX_DELAY: Duration = Duration::from_secs(30);

pub(super) type CheckpointExportStatusListener = Arc<dyn Fn() + Send + Sync + 'static>;

static CHECKPOINT_EXPORT_STATUS_LISTENER: OnceLock<Mutex<Option<CheckpointExportStatusListener>>> =
    OnceLock::new();

pub(super) fn checkpoint_export_status_listener_slot(
) -> &'static Mutex<Option<CheckpointExportStatusListener>> {
    CHECKPOINT_EXPORT_STATUS_LISTENER.get_or_init(|| Mutex::new(None))
}

/// Register a listener invoked after the background export worker finishes an
/// export attempt (success, skip-when-clean, or failure). Used by the shell to
/// refresh checkpoint status without polling.
pub fn set_checkpoint_export_status_listener(listener: Option<CheckpointExportStatusListener>) {
    if let Ok(mut slot) = checkpoint_export_status_listener_slot().lock() {
        *slot = listener;
    }
}

pub(super) fn notify_checkpoint_export_status() {
    let listener = {
        let Ok(slot) = checkpoint_export_status_listener_slot().lock() else {
            return;
        };
        slot.clone()
    };
    if let Some(listener) = listener {
        listener();
    }
}

pub(super) fn reset_required_error() -> CoreError {
    CoreError::ResetRequired(
        "unsupported Daena runtime storage; close Daena and remove .daena/ before reopening this project".into(),
    )
}

pub struct CheckpointHandle {
    pub(super) root: PathBuf,
    pub(super) database: PathBuf,
    pub(super) export_sender: Option<mpsc::Sender<ExportWorkerCommand>>,
}

pub(super) enum ExportWorkerCommand {
    Wake,
    Flush(String, mpsc::Sender<Result<Generation, CoreError>>),
    StopWithoutDrain(mpsc::Sender<Result<(), CoreError>>),
    Stop(mpsc::Sender<Result<(), CoreError>>),
}

pub(super) struct ExportWorker {
    pub(super) sender: mpsc::Sender<ExportWorkerCommand>,
    join: Option<JoinHandle<()>>,
}

impl ExportWorker {
    pub(super) fn start(root: &Path, database: &Path) -> Result<Self, CoreError> {
        let root = root.to_path_buf();
        let database = database.to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let join = thread::Builder::new()
            .name("daena-export-worker".into())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    let export = |reason: &str, force: bool| {
                        let result = (|| {
                            let worker_store = ProjectStore::open_database(
                                &database,
                                Some(root.clone()),
                                None,
                                false,
                                false,
                            )?;
                            if force {
                                worker_store.flush_checkpoint(reason)
                            } else {
                                worker_store.flush_checkpoint_if_dirty(reason)
                            }
                        })();
                        notify_checkpoint_export_status();
                        result
                    };
                    match command {
                        ExportWorkerCommand::Wake => {
                            let started_at = Instant::now();
                            let max_deadline = started_at + BACKGROUND_EXPORT_MAX_DELAY;
                            let mut idle_deadline = started_at + BACKGROUND_EXPORT_IDLE_DELAY;
                            loop {
                                let deadline = idle_deadline.min(max_deadline);
                                let remaining = deadline.saturating_duration_since(Instant::now());
                                match receiver.recv_timeout(remaining) {
                                    Ok(ExportWorkerCommand::Wake) => {
                                        idle_deadline = (Instant::now()
                                            + BACKGROUND_EXPORT_IDLE_DELAY)
                                            .min(max_deadline);
                                    }
                                    Ok(ExportWorkerCommand::Flush(reason, reply)) => {
                                        let _ = reply.send(export(&reason, true));
                                        break;
                                    }
                                    Ok(ExportWorkerCommand::Stop(reply)) => {
                                        let _ = reply.send(
                                            export("project close checkpoint", false).map(|_| ()),
                                        );
                                        return;
                                    }
                                    Ok(ExportWorkerCommand::StopWithoutDrain(reply)) => {
                                        let _ = reply.send(Ok(()));
                                        return;
                                    }
                                    Err(mpsc::RecvTimeoutError::Timeout) => {
                                        let _ = export("background checkpoint export", false);
                                        break;
                                    }
                                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                                }
                            }
                        }
                        ExportWorkerCommand::Flush(reason, reply) => {
                            let _ = reply.send(export(&reason, true));
                        }
                        ExportWorkerCommand::Stop(reply) => {
                            let _ =
                                reply.send(export("project close checkpoint", false).map(|_| ()));
                            break;
                        }
                        ExportWorkerCommand::StopWithoutDrain(reply) => {
                            let _ = reply.send(Ok(()));
                            break;
                        }
                    }
                }
            })
            .map_err(|source| CoreError::Io {
                operation: "start export worker",
                source,
            })?;
        Ok(Self {
            sender,
            join: Some(join),
        })
    }

    pub(super) fn wake(&self) {
        let _ = self.sender.send(ExportWorkerCommand::Wake);
    }

    pub(super) fn flush(&self, reason: String) -> Result<Generation, CoreError> {
        let (sender, receiver) = mpsc::channel();
        self.sender
            .send(ExportWorkerCommand::Flush(reason, sender))
            .map_err(|_| CoreError::Conflict("export worker is not running".into()))?;
        receiver
            .recv()
            .map_err(|_| CoreError::Conflict("export worker stopped before flush".into()))?
    }

    pub(super) fn stop(mut self) -> Result<(), CoreError> {
        let (sender, receiver) = mpsc::channel();
        let send_result = self.sender.send(ExportWorkerCommand::Stop(sender));
        let result = if send_result.is_ok() {
            receiver
                .recv()
                .map_err(|_| CoreError::Conflict("export worker stopped before drain".into()))?
        } else {
            Ok(())
        };
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| CoreError::Conflict("export worker panicked".into()))?;
        }
        result
    }

    pub(crate) fn stop_without_drain(mut self) -> Result<(), CoreError> {
        let (sender, receiver) = mpsc::channel();
        let send_result = self
            .sender
            .send(ExportWorkerCommand::StopWithoutDrain(sender));
        let result = if send_result.is_ok() {
            receiver
                .recv()
                .map_err(|_| CoreError::Conflict("export worker stopped before pause".into()))?
        } else {
            Ok(())
        };
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| CoreError::Conflict("export worker panicked".into()))?;
        }
        result
    }
}

impl CheckpointHandle {
    pub fn flush_checkpoint(&self, reason: impl Into<String>) -> Result<Generation, CoreError> {
        let reason = reason.into();
        if let Some(sender) = &self.export_sender {
            return flush_export_worker(sender, reason);
        }
        let store = ProjectStore::open_checkpoint_writer(&self.database, self.root.clone())?;
        store.flush_checkpoint(reason)
    }
}

pub(super) fn flush_export_worker(
    sender: &mpsc::Sender<ExportWorkerCommand>,
    reason: String,
) -> Result<Generation, CoreError> {
    let (reply_sender, reply_receiver) = mpsc::channel();
    sender
        .send(ExportWorkerCommand::Flush(reason, reply_sender))
        .map_err(|_| CoreError::Conflict("export worker is not running".into()))?;
    reply_receiver
        .recv()
        .map_err(|_| CoreError::Conflict("export worker stopped before checkpoint flush".into()))?
}
