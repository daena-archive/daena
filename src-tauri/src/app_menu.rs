//! macOS menu, so that Cmd+Q and the Quit item go through the close handshake.
//!
//! With no menu set, tauri 2.11.6 installs `Menu::default` on macOS. Its Quit item is the predefined `quit`,
//! which muda maps to `terminate:`. AppKit then exits without raising `CloseRequested` or `ExitRequested`, so
//! the handshake in app_close.rs never sees the quit. This module builds the same menu, mirroring
//! `Menu::default` (macOS branch) item for item, with one difference: Quit is a custom item with the
//! `CmdOrCtrl+Q` accelerator. Its handler closes the main window, so the handshake runs before the app exits.
//! Other platforms keep the menu they had before, which is none.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use tauri::menu::{
    AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, HELP_SUBMENU_ID,
    WINDOW_SUBMENU_ID,
};
use tauri::{AppHandle, Manager, Wry};

/// Id of the custom Quit item. `on_menu_event` recognises the click by this id.
pub(crate) const QUIT_MENU_ID: &str = "app-quit";

/// Accelerator of the custom Quit item. `CmdOrCtrl` is Cmd on macOS.
pub(crate) const QUIT_ACCELERATOR: &str = "CmdOrCtrl+Q";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuitAction {
    CloseMainWindow,
    ExitApp,
}

/// Quit closes the main window when it exists, so its close handshake (app_close.rs) runs and the app exits
/// only after the shell has saved. With no main window left there is nothing to save, so the app exits.
pub(crate) fn quit_action(main_window_exists: bool) -> QuitAction {
    if main_window_exists {
        QuitAction::CloseMainWindow
    } else {
        QuitAction::ExitApp
    }
}

/// Builds the macOS menu: the app menu (About, Services, Hide, Hide Others, Quit), File, Edit, View, Window
/// and Help. The Window and Help submenus carry tauri's ids, so tauri registers them with AppKit as the
/// Window and Help menus.
pub(crate) fn build_macos_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let pkg_info = app.package_info();
    let config = app.config();
    let about_metadata = AboutMetadata {
        name: Some(pkg_info.name.clone()),
        version: Some(pkg_info.version.to_string()),
        copyright: config.bundle.copyright.clone(),
        authors: config
            .bundle
            .publisher
            .clone()
            .map(|publisher| vec![publisher]),
        ..Default::default()
    };
    let quit = MenuItem::with_id(
        app,
        QUIT_MENU_ID,
        format!("Quit {}", pkg_info.name),
        true,
        Some(QUIT_ACCELERATOR),
    )?;
    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let help_menu = Submenu::with_id_and_items(app, HELP_SUBMENU_ID, "Help", true, &[])?;
    Menu::with_items(
        app,
        &[
            &Submenu::with_items(
                app,
                pkg_info.name.clone(),
                true,
                &[
                    &PredefinedMenuItem::about(app, None, Some(about_metadata))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?,
            &Submenu::with_items(
                app,
                "File",
                true,
                &[&PredefinedMenuItem::close_window(app, None)?],
            )?,
            &Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "View",
                true,
                &[&PredefinedMenuItem::fullscreen(app, None)?],
            )?,
            &window_menu,
            &help_menu,
        ],
    )
}

/// Runs the quit action for a click on the custom Quit item.
pub(crate) fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id != QUIT_MENU_ID {
        return;
    }
    let main = app.get_webview_window("main");
    match quit_action(main.is_some()) {
        QuitAction::CloseMainWindow => {
            if let Some(window) = main {
                if let Err(error) = window.close() {
                    eprintln!("could not close the main window for quit: {error}");
                }
            }
        }
        QuitAction::ExitApp => app.exit(0),
    }
}
