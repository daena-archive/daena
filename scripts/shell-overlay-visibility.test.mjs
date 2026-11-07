import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { compile } from "svelte/compiler";

import { anyRectOverlaps, rectsOverlap } from "../src/lib/plugins/popover-overlap.ts";

// Plugin webviews are native children drawn above the main webview.
// - A modal overlay hides every embedded plugin webview.
// - An anchored popover (project switcher, status center) hides them only when its rectangle overlaps a placeholder.

const rect = (left, top, width, height) => ({ left, top, right: left + width, bottom: top + height });

assert.equal(rectsOverlap(rect(0, 0, 100, 100), rect(50, 50, 100, 100)), true, "partly overlapping rectangles overlap");
assert.equal(rectsOverlap(rect(0, 0, 100, 100), rect(10, 10, 20, 20)), true, "a contained rectangle overlaps");
assert.equal(rectsOverlap(rect(0, 0, 100, 100), rect(-20, 40, 40, 40)), true, "a sliver across an edge overlaps");
assert.equal(rectsOverlap(rect(0, 0, 100, 100), rect(100, 0, 50, 100)), false, "rectangles touching on an edge do not");
assert.equal(rectsOverlap(rect(0, 0, 100, 100), rect(200, 0, 50, 50)), false, "disjoint rectangles do not overlap");
assert.equal(
  rectsOverlap(rect(0, 0, 100, 100), rect(50, 50, 0, 0)),
  false,
  "an unlaid-out placeholder overlaps nothing",
);
assert.equal(rectsOverlap(rect(50, 50, 0, 40), rect(0, 0, 100, 100)), false, "a zero-width popover overlaps nothing");

// The expanded sidebar menu sits inside the 220px rail; a flyout in the 56px collapsed rail extends into the content.
const expandedMenu = rect(0, 80, 220, 200);
const collapsedFlyout = rect(62, 80, 200, 160);
assert.equal(
  anyRectOverlaps([expandedMenu], [rect(220, 0, 900, 800)]),
  false,
  "an inline sidebar menu leaves the pane alone",
);
assert.equal(
  anyRectOverlaps([collapsedFlyout], [rect(262, 0, 800, 800)]),
  false,
  "a flyout clear of the pane leaves it alone",
);
assert.equal(anyRectOverlaps([collapsedFlyout], [rect(240, 0, 800, 800)]), true, "a flyout over the pane covers it");
assert.equal(anyRectOverlaps([], [rect(0, 0, 100, 100)]), false, "no open popover covers nothing");
assert.equal(anyRectOverlaps([rect(0, 0, 10, 10)], []), false, "no placeholder is covered");
assert.equal(
  anyRectOverlaps([rect(500, 0, 10, 10), rect(40, 40, 10, 10)], [rect(0, 0, 100, 100)]),
  true,
  "one overlapping popover among several is enough",
);

const read = (path) => readFile(resolve(import.meta.dirname, "..", path), "utf8");
const between = (source, start, end) => {
  const from = source.indexOf(start);
  assert.notEqual(from, -1, `source contains ${start}`);
  const to = source.indexOf(end, from + start.length);
  assert.notEqual(to, -1, `source contains ${end} after ${start}`);
  return source.slice(from, to);
};

const shell = await read("src/routes/+page.svelte");
compile(shell, { filename: resolve(import.meta.dirname, "../src/routes/+page.svelte"), css: "injected" });

const projectSwitcher = await read("src/lib/shell/ProjectSwitcher.svelte");
const statusCenter = await read("src/lib/shell/StatusCenter.svelte");
const sandboxView = await read("src/lib/plugins/SandboxView.svelte");
compile(projectSwitcher, { filename: "ProjectSwitcher.svelte", css: "injected" });
compile(statusCenter, { filename: "StatusCenter.svelte", css: "injected" });

assert.doesNotMatch(shell, /shellOverlayOpen/, "the single unconditional overlay flag is gone");
const modal = between(shell, "const modalOverlayOpen = $derived(", "\n);");
assert.doesNotMatch(modal, /showProjectMenu|statusCenterOpen/, "the modal set leaves out the anchored popovers");
assert.match(modal, /dialogState\.open/, "dialogs remain in the modal set");
assert.match(modal, /showDiscardPrompt/, "the modal set keeps the discard prompt");
assert.match(
  shell,
  /const anchoredPopoverOpen = \$derived\(showProjectMenu \|\| statusCenterOpen \|\| tabStripPopoverOpen \|\| railMenuOpen\);/,
  "the anchored popovers are the project switcher, the status center, the tab strip, and the rail menu",
);

const measuring = between(shell, "async function measureAnchoredPopovers()", "\n}\n");
assert.match(measuring, /await tick\(\)/, "measurement waits for the DOM to render the popover");
assert.match(measuring, /\[data-shell-popover\]/, "popovers are found by their stable hook");
assert.match(measuring, /\.sandbox-view-container/, "placeholders are the sandbox view containers");
assert.match(measuring, /anyRectOverlaps\(/, "the overlap decision is the tested helper");
assert.match(measuring, /anchoredPopoverOpen && /, "a closed popover never hides a webview");
assert.match(
  shell,
  /if \(!anchoredPopoverOpen\) return;\s*const onResize = \(\) => void measureAnchoredPopovers\(\);\s*window\.addEventListener\("resize", onResize\);/,
  "an open popover is measured again on window resize",
);
assert.match(
  shell,
  /const visible = !modalOverlayOpen && !popoverOverlapsWebview;/,
  "a webview is hidden by a modal, or by a popover that overlaps it",
);

assert.match(sandboxView, /class="sandbox-view-container"/, "the placeholder selector matches SandboxView's container");
assert.match(projectSwitcher, /class="project-menu" role="menu" data-shell-popover="project-menu"/);
assert.match(statusCenter, /class="status-popover"\s+data-shell-popover="status-center"/);

// The overlap check treats every .sandbox-view-container as covering every embedded webview. That is only sound
// while the Language pane, the Writing pane and the sandbox views are exclusive branches; a new mount must re-check
// this assumption. The Writing pane requires `!sandboxView` and sits in the same `{#if}` chain as the Language pane.
const sandboxMounts = shell.match(/<SandboxView[\s>]/g) ?? [];
assert.equal(sandboxMounts.length, 4, "four SandboxView mounts; re-check measureAnchoredPopovers if this changes");

console.log("shell overlay visibility: ok");
