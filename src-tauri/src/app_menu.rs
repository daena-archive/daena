//! macOS menu. Quit and Close Window go through the close handshake, and the tab items tell the shell
//! which tab shortcut to run.
//!
//! With no menu set, tauri 2.11.6 installs `Menu::default` on macOS. Its Quit item is the predefined `quit`,
//! which muda maps to `terminate:`. AppKit then exits without raising `CloseRequested` or `ExitRequested`, so
//! the handshake in app_close.rs never sees the quit. The predefined close-window item also has a fixed Cmd+W
//! accelerator, which this app uses for Close Tab. This module builds the same menu otherwise, with these
//! differences:
//!
//! - Quit is a custom item (`CmdOrCtrl+Q`). It closes the main window so the handshake runs, and it exits
//!   only when that window is already gone.
//! - Close Window is a custom item (`CmdOrCtrl+Shift+W`) in the File and Window menus. It closes the main
//!   window the same way and does nothing when that window is already gone. It does not exit the app.
//! - File also has New Tab (`CmdOrCtrl+T`), Reopen Closed Tab (`CmdOrCtrl+Shift+T`) and Close Tab
//!   (`CmdOrCtrl+W`).
//! - Window also has Show Next Tab and Show Previous Tab. Their accelerators are
//!   `CmdOrCtrl+Shift+BracketRight` and `CmdOrCtrl+Shift+BracketLeft` (⇧⌘] and ⇧⌘[).
//!
//! Those five tab items emit `shell-shortcut` to the main webview. The keydown listeners do not handle
//! ⌘T, ⌘W or ⇧⌘T, so the menu is the only path for them. Other platforms keep the menu they had before,
//! which is none.
#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use tauri::menu::{
    AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, HELP_SUBMENU_ID,
    WINDOW_SUBMENU_ID,
};
use tauri::{AppHandle, Emitter, Manager, Wry};

/// Id of the custom Quit item. `on_menu_event` recognises the click by this id.
pub(crate) const QUIT_MENU_ID: &str = "app-quit";

/// Accelerator of the custom Quit item. `CmdOrCtrl` is Cmd on macOS.
pub(crate) const QUIT_ACCELERATOR: &str = "CmdOrCtrl+Q";

/// Id of both Close Window items. `on_menu_event` recognises the click by this id.
pub(crate) const WINDOW_CLOSE_MENU_ID: &str = "window-close";

/// Close Window. Cmd+W is Close Tab, so the window moves to Shift+Cmd+W.
pub(crate) const WINDOW_CLOSE_ACCELERATOR: &str = "CmdOrCtrl+Shift+W";

pub(crate) const TAB_NEW_ACCELERATOR: &str = "CmdOrCtrl+T";
pub(crate) const TAB_REOPEN_ACCELERATOR: &str = "CmdOrCtrl+Shift+T";
pub(crate) const TAB_CLOSE_ACCELERATOR: &str = "CmdOrCtrl+W";

/// muda 0.19.3 parses both `BracketRight` and `]` as `Code::BracketRight` (`parse_code` in
/// accelerator.rs). The named code is the `keyboard-types` code and does not depend on a one-character token.
pub(crate) const TAB_NEXT_ACCELERATOR: &str = "CmdOrCtrl+Shift+BracketRight";
/// Same parser: `BracketLeft` and `[` are `Code::BracketLeft`.
pub(crate) const TAB_PREVIOUS_ACCELERATOR: &str = "CmdOrCtrl+Shift+BracketLeft";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowCloseAction {
    CloseMainWindow,
    /// No main window. Unlike Quit, Close Window must not exit the app.
    Ignore,
}

/// Close Window closes the main window when it exists, so the same handshake runs. With no main window it
/// does nothing: only Quit exits the app.
pub(crate) fn window_close_action(main_exists: bool) -> WindowCloseAction {
    if main_exists {
        WindowCloseAction::CloseMainWindow
    } else {
        WindowCloseAction::Ignore
    }
}

/// Menu ids that are tab shortcuts. Quit and Close Window are not in this map.
pub(crate) fn menu_shortcut(id: &str) -> Option<&'static str> {
    match id {
        "tab-new" => Some("tab-new"),
        "tab-close" => Some("tab-close"),
        "tab-reopen" => Some("tab-reopen"),
        "tab-next" => Some("tab-next"),
        "tab-previous" => Some("tab-previous"),
        _ => None,
    }
}

/// Builds the macOS menu: the app menu (About, Services, Hide, Hide Others, Quit), File, Edit, View, Window
/// and Help. The Window and Help submenus carry tauri's ids, so tauri registers them with AppKit as the
/// Window and Help menus.
pub(crate) fn build_macos_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    localized_menu(app, None)
}

#[cfg(target_os = "macos")]
pub(crate) fn sync_language(
    app: &AppHandle,
    catalog: Option<&std::collections::HashMap<String, String>>,
) -> tauri::Result<()> {
    app.set_menu(localized_menu(app, catalog)?)?;
    Ok(())
}

fn localized_menu(app: &AppHandle, catalog: Option<&std::collections::HashMap<String, String>>) -> tauri::Result<Menu<Wry>> {
    let label = |key: &'static str| catalog.and_then(|items| items.get(key)).map(String::as_str).unwrap_or(key);
    let translated = |key: &str| catalog.and_then(|items| items.get(key)).map(|text| text.replace("{0}", &app.package_info().name));
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
        label("Quit {0}").replace("{0}", &pkg_info.name),
        true,
        Some(QUIT_ACCELERATOR),
    )?;
    let show_next_tab = MenuItem::with_id(
        app,
        "tab-next",
        label("Show Next Tab"),
        true,
        Some(TAB_NEXT_ACCELERATOR),
    )?;
    let show_previous_tab = MenuItem::with_id(
        app,
        "tab-previous",
        label("Show Previous Tab"),
        true,
        Some(TAB_PREVIOUS_ACCELERATOR),
    )?;
    let window_close = MenuItem::with_id(
        app,
        WINDOW_CLOSE_MENU_ID,
        label("Close Window"),
        true,
        Some(WINDOW_CLOSE_ACCELERATOR),
    )?;
    let window_menu = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        label("Window"),
        true,
        &[
            &PredefinedMenuItem::minimize(app, translated("Minimize").as_deref())?,
            &PredefinedMenuItem::maximize(app, translated("Zoom").as_deref())?,
            &PredefinedMenuItem::separator(app)?,
            &show_next_tab,
            &show_previous_tab,
            &PredefinedMenuItem::separator(app)?,
            &window_close,
        ],
    )?;
    let help_menu = Submenu::with_id_and_items(app, HELP_SUBMENU_ID, label("Help"), true, &[])?;
    Menu::with_items(
        app,
        &[
            &Submenu::with_items(
                app,
                pkg_info.name.clone(),
                true,
                &[
                    &PredefinedMenuItem::about(app, translated("About {0}").as_deref(), Some(about_metadata))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, translated("Services").as_deref())?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, translated("Hide {0}").as_deref())?,
                    &PredefinedMenuItem::hide_others(app, translated("Hide Others").as_deref())?,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?,
            &Submenu::with_items(
                app,
                label("File"),
                true,
                &[
                    &MenuItem::with_id(app, "tab-new", label("New Tab"), true, Some(TAB_NEW_ACCELERATOR))?,
                    &MenuItem::with_id(
                        app,
                        "tab-reopen",
                        label("Reopen Closed Tab"),
                        true,
                        Some(TAB_REOPEN_ACCELERATOR),
                    )?,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(
                        app,
                        "tab-close",
                        label("Close Tab"),
                        true,
                        Some(TAB_CLOSE_ACCELERATOR),
                    )?,
                    &MenuItem::with_id(
                        app,
                        WINDOW_CLOSE_MENU_ID,
                        label("Close Window"),
                        true,
                        Some(WINDOW_CLOSE_ACCELERATOR),
                    )?,
                ],
            )?,
            &Submenu::with_items(
                app,
                label("Edit"),
                true,
                &[
                    &PredefinedMenuItem::undo(app, translated("Undo").as_deref())?,
                    &PredefinedMenuItem::redo(app, translated("Redo").as_deref())?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, translated("Cut").as_deref())?,
                    &PredefinedMenuItem::copy(app, translated("Copy").as_deref())?,
                    &PredefinedMenuItem::paste(app, translated("Paste").as_deref())?,
                    &PredefinedMenuItem::select_all(app, translated("Select All").as_deref())?,
                ],
            )?,
            &Submenu::with_items(
                app,
                label("View"),
                true,
                &[&PredefinedMenuItem::fullscreen(app, translated("Toggle Full Screen").as_deref())?],
            )?,
            &window_menu,
            &help_menu,
        ],
    )
}

/// Runs a custom menu item. Quit and Close Window close the main window. Tab items emit the shortcut name
/// to the main webview, which runs it through the same path as a keydown.
pub(crate) fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id == QUIT_MENU_ID {
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
        return;
    }
    if event.id == WINDOW_CLOSE_MENU_ID {
        let main = app.get_webview_window("main");
        if let WindowCloseAction::CloseMainWindow = window_close_action(main.is_some()) {
            if let Some(window) = main {
                if let Err(error) = window.close() {
                    eprintln!("could not close the main window: {error}");
                }
            }
        }
        return;
    }
    if let Some(shortcut) = menu_shortcut(event.id.as_ref()) {
        if let Err(error) = app.emit_to("main", "shell-shortcut", shortcut) {
            eprintln!("could not send shell shortcut {shortcut}: {error}");
        }
    }
}
