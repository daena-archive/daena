import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

// The main window's close is held open natively (src-tauri/src/app_close.rs) until the shell answers.
// The close handler must answer on every branch:
// - the default is prevented before anything else, so the JS wrapper never destroys the window itself;
// - no project open: confirm straight away;
// - a failed save, a refused leave guard, or an exception: cancel, which keeps the window open;
// - every check passed: confirm, which hands the teardown to Rust.

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const between = (source, start, end) => {
  const from = source.indexOf(start);
  assert.notEqual(from, -1, `source contains ${start}`);
  const to = source.indexOf(end, from + start.length);
  assert.notEqual(to, -1, `source contains ${end} after ${start}`);
  return source.slice(from, to);
};

const page = await read("src/routes/+page.svelte");
const handler = between(page, "onCloseRequested(async (event) => {", ".then((cleanup) => {");

const preventIndex = handler.indexOf("event.preventDefault();");
assert.notEqual(preventIndex, -1, "the handler prevents the default");
assert.ok(preventIndex < handler.indexOf("await"), "the default is prevented before the first await");
assert.equal(
  handler.search(/\bawait\b/),
  handler.indexOf("await project.appCloseAck();"),
  "the acknowledgement is the handler's first await, before any save or guard",
);
assert.doesNotMatch(handler, /destroy\(/, "the shell never destroys the window itself");
assert.match(handler, /if \(closeDecisionPending\) return;/, "a second request during a decision is not decided again");
assert.match(handler, /finally \{\s*closeDecisionPending = false;/, "the decision flag is cleared on every path");
assert.match(
  handler,
  /const keepWindowOpen = \(\) => project\.appCloseCancel\(\)/,
  "keeping the window open goes through appCloseCancel",
);

const branches = [
  ["no project open", /if \(!ready\) \{\s*await project\.appCloseConfirm\(\);\s*return;\s*\}/],
  ["draft could not be saved", /could not be saved\.";\s*await keepWindowOpen\(\);\s*return;/],
  [
    "language guard refuses",
    /if \(!\(await canLeaveLanguageSection\(\)\)\) \{\s*await keepWindowOpen\(\);\s*return;\s*\}/,
  ],
  [
    "plugin view guard refuses",
    /if \(!\(await leavePluginView\(\{ closeWebviews: true \}\)\)\) \{\s*await keepWindowOpen\(\);\s*return;\s*\}/,
  ],
  ["every check passed", /await project\.appCloseConfirm\(\);\s*\} catch \(cause\)/],
  ["exception", /\} catch \(cause\) \{[\s\S]*?await keepWindowOpen\(\);\s*\} finally/],
];
for (const [name, pattern] of branches) {
  assert.match(handler, pattern, `the ${name} branch answers the shell`);
}

const client = await read("src/lib/project/client.ts");
assert.match(
  client,
  /appCloseConfirm: \(\) => invoke<void>\("app_close_confirm"\)/,
  "client confirms via app_close_confirm",
);
assert.match(
  client,
  /appCloseCancel: \(\) => invoke<void>\("app_close_cancel"\)/,
  "client cancels via app_close_cancel",
);

assert.match(client, /appCloseAck: \(\) => invoke<void>\("app_close_ack"\)/, "client acknowledges via app_close_ack");

const appClose = await read("src-tauri/src/app_close.rs");
const lib = await read("src-tauri/src/lib.rs");
for (const command of ["app_close_ack", "app_close_confirm", "app_close_cancel"]) {
  assert.match(
    appClose,
    new RegExp(`#\\[tauri::command\\]\\s*pub\\(crate\\) fn ${command}\\(`),
    `${command} is a Tauri command`,
  );
  assert.match(lib, new RegExp(`app_close::${command},`), `${command} is registered with the invoke handler`);
}
assert.match(
  lib,
  /api\.prevent_close\(\);\s*app_close::on_close_requested\(window\);/,
  "the native close request is held open before the shell decides",
);
assert.doesNotMatch(
  between(appClose, "pub(crate) fn request_close", "pub(crate) fn confirm_close"),
  /CLOSE_IN_PROGRESS/,
  "a pending request leaves project saves allowed",
);

assert.doesNotMatch(
  between(appClose, "pub(crate) fn acknowledge_close", "pub(crate) fn confirm_close"),
  /CLOSE_IN_PROGRESS/,
  "an acknowledgement leaves project saves allowed",
);
assert.match(
  lib,
  /if let tauri::RunEvent::ExitRequested \{ api, \.\. \} = event \{\s*app_close::on_exit_requested\(app_handle, &api\);/,
  "exit requests are routed through the close handshake",
);

// Quit on macOS: tauri's default menu sends terminate: (no close, no exit request), so the menu is ours.
const appMenu = await read("src-tauri/src/app_menu.rs");
assert.doesNotMatch(appMenu, /PredefinedMenuItem::quit/, "the predefined Quit item is no longer used");
assert.doesNotMatch(lib, /Menu::default/, "the app does not fall back to tauri's default menu");
assert.match(
  appMenu,
  /pub\(crate\) const QUIT_ACCELERATOR: &str = "CmdOrCtrl\+Q";/,
  "the custom Quit item uses the CmdOrCtrl+Q accelerator",
);
assert.match(
  appMenu,
  /MenuItem::with_id\(\s*app,\s*QUIT_MENU_ID,[\s\S]*?Some\(QUIT_ACCELERATOR\)/,
  "the custom Quit item carries that accelerator",
);
assert.match(appMenu, /if event\.id == QUIT_MENU_ID/, "the Quit click is recognised by its id");
assert.match(appMenu, /event\.id == WINDOW_CLOSE_MENU_ID/, "Close Window is recognised by its id");
assert.match(appMenu, /window_close_action\(main\.is_some\(\)\)/, "Close Window consults window_close_action");
assert.match(appMenu, /menu_shortcut\(event\.id\.as_ref\(\)\)/, "tab items are mapped by menu_shortcut");
assert.match(
  appMenu,
  /emit_to\(\s*"main",\s*"shell-shortcut",\s*shortcut\s*\)/,
  "a tab menu item emits the shortcut name to the main webview",
);
assert.match(appMenu, /pub\(crate\) const WINDOW_CLOSE_ACCELERATOR: &str = "CmdOrCtrl\+Shift\+W";/);
assert.match(appMenu, /pub\(crate\) const TAB_CLOSE_ACCELERATOR: &str = "CmdOrCtrl\+W";/);
assert.match(appMenu, /pub\(crate\) const TAB_NEXT_ACCELERATOR: &str = "CmdOrCtrl\+Shift\+BracketRight";/);
assert.match(appMenu, /pub\(crate\) const TAB_PREVIOUS_ACCELERATOR: &str = "CmdOrCtrl\+Shift\+BracketLeft";/);
assert.doesNotMatch(appMenu, /PredefinedMenuItem::close_window/, "Cmd+W is no longer the predefined close-window item");
assert.match(appMenu, /window\.close\(\)/, "the Quit handler closes the main window");
assert.match(
  lib,
  /#\[cfg\(target_os = "macos"\)\]\s*let builder = builder\s*\.menu\(app_menu::build_macos_menu\)\s*\.on_menu_event\(app_menu::on_menu_event\);/,
  "the custom menu is installed on macOS only",
);
for (const item of [
  "about",
  "services",
  "hide",
  "hide_others",
  "undo",
  "redo",
  "cut",
  "copy",
  "paste",
  "select_all",
  "fullscreen",
  "minimize",
  "maximize",
]) {
  assert.match(appMenu, new RegExp(`PredefinedMenuItem::${item}\\(`), `the menu keeps the predefined ${item} item`);
}

console.log("window close handshake: ok");
