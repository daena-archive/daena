<script lang="ts">
import { X, Settings2, FolderOpen, ChevronLeft, Sun, Moon, Monitor, Download } from "@lucide/svelte";
import { checkAppUpdate, formatUpdateMessage, openDownloadPage, type UpdateChannelPreference } from "$lib/appUpdate";
import type { ThemePreference } from "$lib/theme";

type SettingsSection = "general";
type RecentProject = { name: string; root: string };

let {
  section = $bindable("general" as SettingsSection),
  version,
  recentProjects,
  themePreference,
  onThemeChange,
  updateChannelPreference,
  onUpdateChannelChange,
  onRemoveRecent,
  onClose,
  onBeforeNavigate,
}: {
  section?: SettingsSection;
  version: string;
  recentProjects: RecentProject[];
  themePreference: ThemePreference;
  onThemeChange: (preference: ThemePreference) => void;
  updateChannelPreference: UpdateChannelPreference;
  onUpdateChannelChange: (preference: UpdateChannelPreference) => void;
  onRemoveRecent: (root: string) => void;
  onClose: () => void;
  onBeforeNavigate?: (next: SettingsSection | null) => boolean | Promise<boolean>;
} = $props();

let updateBusy = $state(false);
let updateMessage = $state("");

async function checkForUpdate() {
  if (updateBusy) return;
  updateBusy = true;
  updateMessage = "";
  try {
    const result = await checkAppUpdate(updateChannelPreference);
    updateMessage = formatUpdateMessage(result);
  } catch (cause) {
    updateMessage = cause instanceof Error ? cause.message : String(cause);
  } finally {
    updateBusy = false;
  }
}

async function openUpdatesPage() {
  try {
    await openDownloadPage();
  } catch (cause) {
    updateMessage = cause instanceof Error ? cause.message : String(cause);
  }
}

async function goToSection(next: SettingsSection) {
  if (next === section) return;
  if (onBeforeNavigate && !(await onBeforeNavigate(next))) return;
  section = next;
}

async function handleClose() {
  if (onBeforeNavigate && !(await onBeforeNavigate(null))) return;
  onClose();
}
</script>

<section class="settings-view" aria-label="Settings">
  <header class="settings-header">
    <div class="header-left">
      <div class="header-icon">
        <Settings2 size={18} strokeWidth={1.8} aria-hidden="true" />
      </div>
      <div>
        <span class="panel-kicker">APPLICATION</span>
        <h1>Settings</h1>
        <p>Appearance and recent projects that follow you across projects.</p>
      </div>
    </div>
    <button type="button" class="quiet-button header-back" onclick={() => void handleClose()}
      ><ChevronLeft size={14} strokeWidth={1.9} aria-hidden="true" /> Back</button>
  </header>

  <div class="settings-layout">
    <nav class="settings-nav" aria-label="Settings sections">
      <button
        type="button"
        class:active={section === "general"}
        class="settings-nav-button"
        onclick={() => void goToSection("general")}
        ><FolderOpen size={14} strokeWidth={1.8} aria-hidden="true" /> General</button>
    </nav>

    <div class="settings-panel">
      {#if section === "general"}
        <div class="panel-hero">
          <div class="hero-icon">
            <FolderOpen size={18} strokeWidth={1.8} aria-hidden="true" />
          </div>
          <div class="hero-copy">
            <span class="kicker">APPLICATION</span>
            <strong>General</strong>
            <p>Appearance and recent projects. These preferences follow you across projects.</p>
          </div>
          <div class="hero-stats">
            <span class="stat-pill"
              ><FolderOpen size={12} strokeWidth={1.8} aria-hidden="true" /> {recentProjects.length} recent</span>
          </div>
        </div>

        <div class="block elevated appearance-block">
          <div class="block-heading">
            <div class="heading-left">
              <span class="heading-icon accent"><Sun size={14} strokeWidth={1.8} aria-hidden="true" /></span>
              <h4>Appearance</h4>
            </div>
            <span class="block-hint">Follows your preference across projects</span>
          </div>
          <div class="theme-options" role="group" aria-label="Color theme">
            <button
              type="button"
              class:active={themePreference === "light"}
              aria-pressed={themePreference === "light"}
              onclick={() => onThemeChange("light")}>
              <Sun size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Light</strong><small>Warm paper</small></span>
            </button>
            <button
              type="button"
              class:active={themePreference === "dark"}
              aria-pressed={themePreference === "dark"}
              onclick={() => onThemeChange("dark")}>
              <Moon size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Dark</strong><small>Forest night</small></span>
            </button>
            <button
              type="button"
              class:active={themePreference === "system"}
              aria-pressed={themePreference === "system"}
              onclick={() => onThemeChange("system")}>
              <Monitor size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>System</strong><small>Match this computer</small></span>
            </button>
          </div>
        </div>

        <div class="block elevated">
          <div class="block-heading">
            <div class="heading-left">
              <span class="heading-icon"><Download size={14} strokeWidth={1.8} aria-hidden="true" /></span>
              <h4>About</h4>
            </div>
            <span class="block-hint">v{version}</span>
          </div>
          <div class="theme-options update-channel-options" role="group" aria-label="Update channel">
            <button
              type="button"
              class:active={updateChannelPreference === "auto"}
              aria-pressed={updateChannelPreference === "auto"}
              onclick={() => onUpdateChannelChange("auto")}>
              <Monitor size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Auto</strong><small>Match this build</small></span>
            </button>
            <button
              type="button"
              class:active={updateChannelPreference === "stable"}
              aria-pressed={updateChannelPreference === "stable"}
              onclick={() => onUpdateChannelChange("stable")}>
              <Download size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Stable</strong><small>Production releases</small></span>
            </button>
            <button
              type="button"
              class:active={updateChannelPreference === "beta"}
              aria-pressed={updateChannelPreference === "beta"}
              onclick={() => onUpdateChannelChange("beta")}>
              <Download size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Beta</strong><small>Beta and stable</small></span>
            </button>
            <button
              type="button"
              class:active={updateChannelPreference === "alpha"}
              aria-pressed={updateChannelPreference === "alpha"}
              onclick={() => onUpdateChannelChange("alpha")}>
              <Download size={16} strokeWidth={1.8} aria-hidden="true" />
              <span><strong>Alpha</strong><small>Earliest previews</small></span>
            </button>
          </div>
          <div class="update-actions">
            <button type="button" class="primary" onclick={() => void checkForUpdate()} disabled={updateBusy}
              >{updateBusy ? "Checking…" : "Check for update"}</button>
            <button type="button" class="quiet" onclick={() => void openUpdatesPage()}>Open download page</button>
            {#if updateMessage}<span class="update-status">{updateMessage}</span>{/if}
          </div>
        </div>

        <div class="block elevated">
          <div class="block-heading">
            <div class="heading-left">
              <span class="heading-icon"><FolderOpen size={14} strokeWidth={1.8} aria-hidden="true" /></span>
              <h4>Recent projects</h4>
              <span class="count-badge">{recentProjects.length}</span>
            </div>
            <span class="block-hint">Stored in your application profile</span>
          </div>
          {#if recentProjects.length === 0}
            <div class="empty-inline">
              <FolderOpen size={16} strokeWidth={1.7} aria-hidden="true" />
              <div>
                <strong>No recent projects yet</strong>
                <span>Open a project to begin. Recent projects appear here for quick access.</span>
              </div>
            </div>
          {:else}
            <ul class="settings-recent-list">
              {#each recentProjects as recent}
                <li>
                  <div class="recent-copy">
                    <strong>{recent.name}</strong>
                    <small>{recent.root}</small>
                  </div>
                  <button
                    type="button"
                    class="quiet icon"
                    aria-label="Remove {recent.name}"
                    onclick={() => onRemoveRecent(recent.root)}
                    ><X size={14} strokeWidth={1.8} aria-hidden="true" /></button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</section>

<style>
.settings-view {
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 58px);
  padding: 28px 32px 40px;
  background: var(--canvas);
}
.settings-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 22px;
  padding: 16px 16px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.header-left {
  display: flex;
  gap: 14px;
  align-items: flex-start;
}
.header-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
  flex: 0 0 40px;
}
.settings-header h1 {
  margin: 2px 0 6px;
  font: 600 22px/1.1 var(--font-display, Georgia, serif);
  color: var(--ink);
  letter-spacing: -0.01em;
}
.settings-header p {
  margin: 0;
  max-width: 520px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.panel-kicker {
  color: var(--accent);
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.header-back {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border-radius: 999px;
}
.settings-layout {
  display: grid;
  grid-template-columns: 200px minmax(0, 1fr);
  gap: 22px;
  align-items: start;
}
.settings-nav {
  display: grid;
  gap: 6px;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
  position: sticky;
  top: 16px;
}
.settings-nav-button {
  width: 100%;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
  font:
    600 13px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  transition: all 0.14s ease;
}
.settings-nav-button:hover {
  background: var(--theme-warning-bg, #efe8d9);
  color: var(--ink);
}
.settings-nav-button.active {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: var(--on-accent);
  box-shadow: 0 1px 0 rgba(48, 44, 38, 0.12);
}
.settings-panel {
  min-width: 0;
  display: grid;
  gap: 18px;
  padding: 22px 24px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow:
    0 1px 0 rgba(40 40 20 / 4%),
    0 8px 24px rgba(48, 44, 38, 0.04);
}
.panel-hero {
  display: grid;
  grid-template-columns: 40px 1fr;
  gap: 14px;
  padding: 16px 16px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.panel-hero .hero-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.hero-copy .kicker {
  color: var(--accent);
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.hero-copy strong {
  display: block;
  margin-top: 3px;
  color: var(--ink);
  font: 600 16px/1.15 var(--font-display, Georgia, serif);
}
.hero-copy p {
  margin: 6px 0 0;
  max-width: 640px;
  color: var(--ink-soft);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.hero-stats {
  grid-column: 2;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 2px;
}
.stat-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.block {
  display: grid;
  gap: 14px;
  padding: 18px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.block.elevated {
  box-shadow:
    0 1px 0 rgba(48, 44, 38, 0.03),
    0 8px 24px rgba(48, 44, 38, 0.04);
}
.block-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--theme-warning-border, #f0e8d9);
}
.heading-left {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.heading-icon {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
}
.heading-icon.accent {
  background: var(--accent-dark);
  border-color: var(--accent-dark);
  color: var(--on-accent);
}
.heading-left h4 {
  margin: 0;
  font:
    600 13px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  color: var(--ink);
  letter-spacing: -0.01em;
}
.count-badge {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  padding: 0 7px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    700 11px Inter,
    sans-serif;
}

.block-hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--ink-faint);
  font:
    500 11.5px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.theme-options {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}
.update-channel-options {
  grid-template-columns: repeat(4, minmax(0, 1fr));
  margin-bottom: 12px;
}
.theme-options button {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface-subtle);
  color: var(--ink-soft);
  text-align: left;
  cursor: pointer;
  transition:
    border-color 0.14s ease,
    background 0.14s ease,
    color 0.14s ease,
    box-shadow 0.14s ease;
}
.theme-options button:hover {
  border-color: var(--accent-soft);
  background: var(--surface-warm);
  color: var(--ink);
}
.theme-options button.active {
  border-color: var(--accent);
  background: var(--accent-bg);
  color: var(--ink);
  box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--accent) 24%, transparent);
}
.theme-options button span,
.theme-options button strong,
.theme-options button small {
  display: block;
}
.theme-options button strong {
  color: inherit;
  font-size: 12px;
}
.theme-options button small {
  margin-top: 2px;
  color: var(--ink-faint);
  font-size: 10.5px;
}
.empty-inline {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 14px 14px;
  border: 1px dashed var(--line-strong);
  border-radius: 11px;
  background: var(--surface-quiet);
  color: var(--ink-muted);
}
.empty-inline strong {
  display: block;
  color: var(--ink);
  font:
    600 13px Inter,
    sans-serif;
  margin-bottom: 3px;
}
.empty-inline span {
  font:
    400 12px/1.5 Inter,
    sans-serif;
}

.settings-recent-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.settings-recent-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--theme-warning-border, #ebe3d6);
  border-radius: 12px;
  background: var(--surface-quiet);
  transition:
    border-color 0.14s ease,
    box-shadow 0.14s ease;
}
.settings-recent-list li:hover {
  border-color: var(--theme-warning-border, #e0d6c4);
  box-shadow: 0 4px 14px rgba(48, 44, 38, 0.05);
}
.recent-copy strong,
.recent-copy small {
  display: block;
}
.recent-copy small {
  margin-top: 3px;
  color: var(--ink-soft);
  font:
    500 11px ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
}
.update-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.primary {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 14px;
  border: 1px solid var(--accent-dark);
  border-radius: 9px;
  background: var(--accent-dark);
  color: var(--on-accent);
  font:
    700 12px Inter,
    sans-serif;
  cursor: pointer;
  box-shadow: 0 1px 0 rgba(48, 44, 38, 0.12);
  transition: all 0.14s ease;
}
.primary:hover {
  background: #4a6b57;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(48, 44, 38, 0.12);
}
.primary:active {
  transform: translateY(0);
  box-shadow: none;
}
.primary:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}
.quiet {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink-muted);
  font:
    600 11.5px Inter,
    sans-serif;
  cursor: pointer;
  transition: all 0.14s ease;
}
.quiet:hover {
  border-color: var(--theme-warning-border, #b7a88f);
  background: var(--surface-warm);
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(48, 44, 38, 0.06);
}
.quiet:active {
  transform: translateY(0);
  box-shadow: none;
}
.quiet.icon {
  width: 32px;
  height: 32px;
  padding: 0;
  display: grid;
  place-items: center;
  border-radius: 9px;
}
.quiet:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}
.quiet-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--line-strong);
  border-radius: 9px;
  background: var(--surface);
  color: var(--ink-muted);
  font:
    600 11.5px Inter,
    sans-serif;
  cursor: pointer;
  transition: all 0.14s ease;
}
.quiet-button:hover {
  background: var(--surface-warm);
  border-color: var(--theme-warning-border, #b7a88f);
}
.update-status {
  color: var(--theme-danger-text, #9d5b42);
  font-size: 11px;
  line-height: 1.4;
}
@media (max-width: 760px) {
  .settings-view {
    padding: 18px 16px 28px;
  }
  .settings-layout {
    grid-template-columns: 1fr;
  }
  .settings-nav {
    flex-direction: row;
    flex-wrap: wrap;
    position: static;
  }
  .settings-panel {
    padding: 16px;
  }
  .theme-options,
  .update-channel-options {
    grid-template-columns: 1fr;
  }
  .panel-hero {
    grid-template-columns: 1fr;
  }
  .hero-stats {
    grid-column: 1;
  }
}
</style>
