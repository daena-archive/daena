/**
 * Tab shortcut names shared by the shell keydown listener, the macOS menu, and the
 * plugin webviews. The Writing and Language packages keep their own copies of this
 * list and of `tabShortcutForKey` (they cannot import `src/`).
 */

export const TAB_SHORTCUTS = [
  "tab-new",
  "tab-close",
  "tab-reopen",
  "tab-next",
  "tab-previous",
  "tab-1",
  "tab-2",
  "tab-3",
  "tab-4",
  "tab-5",
  "tab-6",
  "tab-7",
  "tab-8",
  "tab-last",
] as const;

export type TabShortcut = (typeof TAB_SHORTCUTS)[number];

export function isTabShortcut(value: unknown): value is TabShortcut {
  return typeof value === "string" && (TAB_SHORTCUTS as readonly string[]).includes(value);
}

/** A keydown as the mapping reads it. A KeyboardEvent has every field. */
export interface TabKeyPress {
  key: string;
  code?: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  repeat: boolean;
}

const DIGITS: Readonly<Record<string, TabShortcut>> = {
  "1": "tab-1",
  "2": "tab-2",
  "3": "tab-3",
  "4": "tab-4",
  "5": "tab-5",
  "6": "tab-6",
  "7": "tab-7",
  "8": "tab-8",
  "9": "tab-last",
};

/**
 * The tab shortcut a keydown stands for on this platform, or null.
 * This is the keydown column only. On macOS, ⌘T, ⌘W and ⇧⌘T belong to the menu, so they are null here.
 * Next and previous allow key repeat. New, close, reopen and the digit jumps do not. Alt never matches.
 */
export function tabShortcutForKey(press: TabKeyPress, isMac: boolean): TabShortcut | null {
  if (press.altKey) return null;

  // Ctrl+Tab and Ctrl+Shift+Tab on both platforms. The other modifier must not be held.
  if (press.ctrlKey && !press.metaKey && press.key === "Tab") {
    return press.shiftKey ? "tab-previous" : "tab-next";
  }

  // Windows and Linux also cycle with Ctrl+PageDown and Ctrl+PageUp.
  if (!isMac && press.ctrlKey && !press.metaKey && !press.shiftKey) {
    if (press.key === "PageDown") return "tab-next";
    if (press.key === "PageUp") return "tab-previous";
  }

  const platform = isMac ? press.metaKey && !press.ctrlKey : press.ctrlKey && !press.metaKey;
  if (!platform) return null;

  // A shifted digit is not a tab shortcut. ⇧⌘T is the menu's reopen on macOS.
  if (press.shiftKey) {
    if (!isMac && !press.repeat && press.key.toLowerCase() === "t") return "tab-reopen";
    return null;
  }

  if (press.repeat) return null;

  const digit = DIGITS[press.key];
  if (digit) return digit;

  // ⌘T and ⌘W are menu items on macOS, so the keydown path leaves them alone.
  if (isMac) return null;
  const key = press.key.toLowerCase();
  if (key === "t") return "tab-new";
  if (key === "w") return "tab-close";
  return null;
}
