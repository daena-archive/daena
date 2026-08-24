import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const shell = await readFile(new URL("../src/routes/+page.svelte", import.meta.url), "utf8");
const controls = await readFile(new URL("../src/lib/shell/controls.css", import.meta.url), "utf8");
const sidebar = await readFile(new URL("../src/lib/shell/AppSidebar.svelte", import.meta.url), "utf8");
const sidebarStyles = await readFile(new URL("../src/lib/shell/AppSidebar.css", import.meta.url), "utf8");
const projectCenter = await readFile(new URL("../src/lib/ProjectCenter.svelte", import.meta.url), "utf8");
const projectSwitcher = await readFile(new URL("../src/lib/shell/ProjectSwitcher.svelte", import.meta.url), "utf8");
const statusCenter = await readFile(new URL("../src/lib/shell/StatusCenter.svelte", import.meta.url), "utf8");
const workspaceNav = await readFile(new URL("../src/lib/shell/WorkspaceViewNav.svelte", import.meta.url), "utf8");
const quickOpen = await readFile(new URL("../src/lib/shell/QuickOpen.svelte", import.meta.url), "utf8");
const toolbar = await readFile(new URL("../src/lib/shell/GlobalToolbar.svelte", import.meta.url), "utf8");

function cssRule(source, selector) {
  const start = source.indexOf(`${selector} {`);
  assert.notEqual(start, -1, `missing CSS rule ${selector}`);
  const end = source.indexOf("}", start);
  assert.notEqual(end, -1, `unterminated CSS rule ${selector}`);
  return source.slice(start, end + 1);
}

function cssToken(rule, name) {
  const match = rule.match(new RegExp(`--${name}:\\s*(#[0-9a-fA-F]{6})`));
  assert.ok(match, `missing hex token --${name}`);
  return match[1];
}

function relativeLuminance(hex) {
  const channels = hex
    .slice(1)
    .match(/../g)
    .map((channel) => Number.parseInt(channel, 16) / 255)
    .map((channel) => (channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4));
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(foreground, background) {
  const foregroundLuminance = relativeLuminance(foreground);
  const backgroundLuminance = relativeLuminance(background);
  return (
    (Math.max(foregroundLuminance, backgroundLuminance) + 0.05) /
    (Math.min(foregroundLuminance, backgroundLuminance) + 0.05)
  );
}

function assertContrast(rule, foreground, background, minimum, label) {
  assert.ok(
    contrast(cssToken(rule, foreground), cssToken(rule, background)) >= minimum,
    `${label} meets ${minimum}:1 contrast`,
  );
}

const lightTheme = cssRule(shell, ":global(:root)");
const darkTheme = cssRule(shell, ':global(:root[data-theme="dark"])');
for (const token of [
  "theme-surface-bg",
  "theme-neutral-text",
  "theme-danger-text",
  "theme-success-text",
  "theme-warning-text",
  "theme-info-text",
  "focus-ring",
  "control-min-height",
  "touch-target-min",
  "rail-bg",
]) {
  assert.match(lightTheme, new RegExp(`--${token}:`), `${token} is available in the default light theme`);
}

assertContrast(lightTheme, "ink", "surface", 7, "light primary text");
assertContrast(lightTheme, "ink-soft", "surface", 4.5, "light secondary text");
assertContrast(lightTheme, "on-accent", "accent-dark", 4.5, "light primary action");
assertContrast(darkTheme, "ink", "surface", 7, "dark primary text");
assertContrast(darkTheme, "ink-soft", "surface", 4.5, "dark secondary text");
assertContrast(darkTheme, "on-accent", "accent-dark", 4.5, "dark primary action");
assertContrast(lightTheme, "rail-text-soft", "rail-bg", 4.5, "sidebar text");
assertContrast(lightTheme, "brass-ink", "rail-accent", 4.5, "sidebar primary action");

assert.match(controls, /:focus-visible/, "shared controls expose a visible keyboard focus state");
assert.match(controls, /@media \(pointer: coarse\)/, "coarse pointers receive larger shared targets");
assert.match(controls, /var\(--touch-target-min\)/, "touch sizing uses a shared semantic token");
assert.match(controls, /@media \(prefers-reduced-motion: reduce\)/, "motion can be reduced across the shell");
assert.match(controls, /@media \(forced-colors: active\)/, "focus remains visible in forced-colors mode");

assert.doesNotMatch(sidebar, />Snapshots<\/span>/, "Snapshots is not a standalone rail destination");
assert.match(sidebar, />Project<\/span>/, "Project center remains reachable from the sidebar rail");
assert.match(sidebar, />Settings<\/span>/, "application settings remain reachable from the sidebar rail");
assert.doesNotMatch(projectCenter, /Developer fixtures|Add example world/, "developer fixtures stay out of beta UI");
assert.doesNotMatch(
  shell,
  /<strong>Plugins<\/strong>|Install package|No plugins installed/,
  "extension UI uses author-facing copy",
);
assert.match(
  shell,
  /<summary>Capabilities, namespaces, services &amp; migrations<\/summary>[\s\S]*Host API/,
  "technical extension metadata is collapsed",
);

assert.match(projectSwitcher, /event\.key !== "Escape"/, "the project menu closes with Escape");
assert.match(projectSwitcher, /projectButton\?\.focus\(\)/, "the project menu restores trigger focus");
assert.match(statusCenter, /event\.key !== "Escape"/, "the status popover closes with Escape");
assert.match(statusCenter, /setOpen\(false, true\)/, "the status popover restores trigger focus");
for (const key of ["ArrowRight", "ArrowLeft", "Home", "End"])
  assert.match(workspaceNav, new RegExp(`event\\.key === "${key}"`), `workspace tabs support ${key}`);
assert.match(workspaceNav, /tabindex=\{activeView === view\.id \? 0 : -1\}/, "workspace tabs use roving focus");
assert.match(quickOpen, /role="combobox"/, "Quick Open exposes its search/result relationship");
assert.match(quickOpen, /aria-autocomplete="list"/, "Quick Open declares list autocomplete");

assert.match(
  cssRule(toolbar, ".history-actions button"),
  /width: 36px[\s\S]*height: 36px/,
  "history targets are usable",
);
assert.match(cssRule(quickOpen, ".clear-query"), /width: 36px[\s\S]*height: 36px/, "Quick Open clear target is usable");
assert.match(
  cssRule(quickOpen, ".quick-open-item"),
  /min-height: 44px/,
  "Quick Open results are comfortably targetable",
);
assert.match(cssRule(workspaceNav, ".workspace-view-nav button"), /min-height: var\(--control-min-height\)/);
assert.match(cssRule(sidebarStyles, ".recent-project-remove"), /width: 36px[\s\S]*height: 36px/);

for (const source of [sidebarStyles, projectCenter, statusCenter, workspaceNav, quickOpen, toolbar])
  assert.match(source, /@media \(max-width:/, "core beta surfaces include responsive behavior");

console.log("beta polish and accessibility contracts passed");
