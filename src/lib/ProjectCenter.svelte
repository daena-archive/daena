<script lang="ts">
import { onMount, type Snippet } from "svelte";
import {
  AlertTriangle,
  Archive,
  Bot,
  Boxes,
  ChevronLeft,
  CircleCheck,
  Cpu,
  DatabaseBackup,
  DatabaseZap,
  Download,
  FileCog,
  FolderOpen,
  GitBranch,
  Import as ImportIcon,
  Puzzle,
  RefreshCw,
  SlidersHorizontal,
  Sparkles,
  Wrench,
  X,
} from "@lucide/svelte";
import { project } from "$lib/project/client";
import type { ProjectSection } from "$lib/modules/workspace";
import { confirmDialog } from "$lib/dialogs.svelte";
import { setSchemaEditorDiscardPrompt } from "$lib/schemaEditorGuard";
import ArchivedDocumentsPanel from "$lib/ArchivedDocumentsPanel.svelte";

type ProjectSummary = {
  name: string;
  root: string;
  indexStatus: string;
  sync: { state: string; dirty_count: number; export_error: string | null };
  aiEnabled: boolean;
};

let {
  section = $bindable("overview" as ProjectSection),
  summary,
  diagnostics = [],
  snapshotChangeCount = 0,
  snapshotRepository = false,
  snapshotBranch = null,
  archivedCount = 0,
  aiIndexStatus,
  aiIndexBusy = false,
  aiIndexMessage = "",
  remoteProvider = false,
  extensions,
  fields,
  snapshots,
  onClose,
  onBeforeNavigate,
  onImportExternal,
  onExportMarkdown,
  onPortableBackup,
  onRecoveryBackup,
  onRestoreRecoveryBackup,
  onImportCheckpoint,
  onRebuildIndex,
  onSeedExample,
  onToggleAi,
  onAiRemoteConsent,
  onAiIndexRefresh,
  onAiIndexRebuild,
  onAiIndexCancel,
  typeLabel,
  onArchiveChanged,
  onArchiveToast,
}: {
  section?: ProjectSection;
  summary: ProjectSummary;
  diagnostics?: string[];
  snapshotChangeCount?: number;
  snapshotRepository?: boolean;
  snapshotBranch?: string | null;
  archivedCount?: number;
  aiIndexStatus: { state: string | null; message: string | null } | null;
  aiIndexBusy?: boolean;
  aiIndexMessage?: string;
  remoteProvider?: boolean;
  extensions: Snippet;
  fields: Snippet;
  snapshots: Snippet;
  onClose: () => void;
  onBeforeNavigate?: (next: ProjectSection | null) => boolean | Promise<boolean>;
  onImportExternal: () => void;
  onExportMarkdown: () => Promise<void>;
  onPortableBackup: () => Promise<string>;
  onRecoveryBackup: () => Promise<string>;
  onRestoreRecoveryBackup: (path: string) => Promise<void>;
  onImportCheckpoint: () => Promise<void>;
  onRebuildIndex: () => Promise<void>;
  onSeedExample: () => Promise<void>;
  onToggleAi: (enabled: boolean) => void;
  onAiRemoteConsent: (allowed: boolean) => void;
  onAiIndexRefresh: () => void;
  onAiIndexRebuild: () => void;
  onAiIndexCancel: () => void;
  typeLabel: (entityType: string | null) => string;
  onArchiveChanged?: () => void;
  onArchiveToast?: (message: string) => void;
} = $props();

let actionBusy = $state(false);
let actionMessage = $state("");
let actionError = $state("");
let recoveryPath = $state("");

onMount(() => {
  setSchemaEditorDiscardPrompt(() =>
    confirmDialog({
      title: "Discard unsaved Fields & Types changes?",
      message: "Your edits to types, fields, and templates will be lost.",
      confirmLabel: "Discard",
      danger: true,
    }),
  );
  return () => setSchemaEditorDiscardPrompt(null);
});

const checkpointCurrent = $derived(summary.sync.state === "clean" && !summary.sync.export_error);

async function goToSection(next: ProjectSection) {
  if (next === section) return;
  if (onBeforeNavigate && !(await onBeforeNavigate(next))) return;
  section = next;
}

async function handleClose() {
  if (onBeforeNavigate && !(await onBeforeNavigate(null))) return;
  onClose();
}

async function runAction(action: () => Promise<void>, success: string) {
  if (actionBusy) return;
  actionBusy = true;
  actionMessage = "";
  actionError = "";
  try {
    await action();
    if (success) actionMessage = success;
  } catch (cause) {
    actionError = cause instanceof Error ? cause.message : String(cause);
  } finally {
    actionBusy = false;
  }
}

async function createPortableBackup() {
  await runAction(async () => {
    const path = await onPortableBackup();
    actionMessage = `Portable backup created at ${path}`;
  }, "");
}

async function createRecoveryBackup() {
  await runAction(async () => {
    recoveryPath = await onRecoveryBackup();
    actionMessage = `Recovery backup created at ${recoveryPath}`;
  }, "");
}

async function pickRestorePath() {
  try {
    const selection = await project.pickDirectory();
    const path = typeof selection === "string" ? selection : Array.isArray(selection) ? selection[0] : null;
    if (path) recoveryPath = path;
  } catch {
    // Picker cancellation is not a project failure.
  }
}

async function restoreRecoveryBackup() {
  if (!recoveryPath.trim()) return;
  const confirmed = await confirmDialog({
    title: "Restore recovery backup?",
    message: "This replaces the current runtime state with the selected recovery backup.",
    confirmLabel: "Restore",
    danger: true,
  });
  if (!confirmed) return;
  await runAction(async () => onRestoreRecoveryBackup(recoveryPath.trim()), "Recovery backup restored.");
}

async function seedExampleProject() {
  const confirmed = await confirmDialog({
    title: "Add example world?",
    message: "This adds Daena's example entries to the current project.",
    confirmLabel: "Add example world",
  });
  if (confirmed) await runAction(onSeedExample, "Example world added.");
}
</script>

<section class="project-center" aria-label="Project">
  <header class="project-header">
    <div class="header-left">
      <div class="header-icon"><Boxes size={18} strokeWidth={1.8} aria-hidden="true" /></div>
      <div>
        <span class="panel-kicker">PROJECT</span>
        <h1>{summary.name}</h1>
        <p>Manage this project's structure, portability, history, and extensions.</p>
      </div>
    </div>
    <button type="button" class="quiet-button header-back" onclick={() => void handleClose()}>
      <ChevronLeft size={14} strokeWidth={1.9} aria-hidden="true" /> Back
    </button>
  </header>

  <div class="project-layout">
    <nav class="project-nav" aria-label="Project sections">
      <button class:active={section === "overview"} onclick={() => void goToSection("overview")}>
        <Boxes size={14} strokeWidth={1.8} aria-hidden="true" /> Overview
      </button>
      <button class:active={section === "data"} onclick={() => void goToSection("data")}>
        <DatabaseBackup size={14} strokeWidth={1.8} aria-hidden="true" /> Data &amp; recovery
      </button>
      <button class:active={section === "extensions"} onclick={() => void goToSection("extensions")}>
        <Puzzle size={14} strokeWidth={1.8} aria-hidden="true" /> Extensions
      </button>
      <button class:active={section === "fields"} onclick={() => void goToSection("fields")}>
        <SlidersHorizontal size={14} strokeWidth={1.8} aria-hidden="true" /> Fields &amp; Types
      </button>
      <button class:active={section === "snapshots"} onclick={() => void goToSection("snapshots")}>
        <GitBranch size={14} strokeWidth={1.8} aria-hidden="true" /> Snapshots
        {#if snapshotChangeCount > 0}<span class="nav-count">{snapshotChangeCount}</span>{/if}
      </button>
      <button class:active={section === "archive"} onclick={() => void goToSection("archive")}>
        <Archive size={14} strokeWidth={1.8} aria-hidden="true" /> Archive
        {#if archivedCount > 0}<span class="nav-count">{archivedCount}</span>{/if}
      </button>
      <div class="nav-separator"></div>
      <button class:active={section === "advanced"} onclick={() => void goToSection("advanced")}>
        <Wrench size={14} strokeWidth={1.8} aria-hidden="true" /> Advanced
      </button>
    </nav>

    <div class="project-panel">
      {#if section === "overview"}
        <div class="panel-hero">
          <div class="hero-icon"><Boxes size={18} strokeWidth={1.8} aria-hidden="true" /></div>
          <div class="hero-copy">
            <span class="kicker">PROJECT CENTER</span>
            <strong>One home for this project</strong>
            <p>Import, recovery, Fields &amp; Types, extensions, Snapshots, and diagnostics live here.</p>
          </div>
        </div>
        <div class="status-grid" aria-label="Project status summary">
          <button type="button" onclick={() => void goToSection("data")}>
            <span class:ok={checkpointCurrent} class:error={Boolean(summary.sync.export_error)} class="status-icon">
              {#if summary.sync.export_error}<AlertTriangle size={17} />{:else}<CircleCheck size={17} />{/if}
            </span>
            <span
              ><strong>{checkpointCurrent ? "Checkpoint current" : "Checkpoint needs attention"}</strong><small>
                {summary.sync.export_error ??
                  (summary.sync.dirty_count > 0
                    ? `${summary.sync.dirty_count} portable change${summary.sync.dirty_count === 1 ? "" : "s"} pending`
                    : "Portable project files are up to date")}
              </small></span>
          </button>
          <button type="button" onclick={() => void goToSection("snapshots")}>
            <span class="status-icon"><GitBranch size={17} /></span>
            <span
              ><strong>{snapshotChangeCount} snapshot-ready changes</strong><small>
                {snapshotRepository ? snapshotBranch || "Detached history" : "Set up Snapshots for this project"}
              </small></span>
          </button>
          <button type="button" onclick={() => void goToSection("extensions")}>
            <span class="status-icon"><Puzzle size={17} /></span>
            <span><strong>Extensions</strong><small>Install, enable, update, or review capabilities</small></span>
          </button>
          <button type="button" onclick={() => void goToSection("fields")}>
            <span class="status-icon"><SlidersHorizontal size={17} /></span>
            <span
              ><strong>Fields &amp; Types</strong><small
                >Shape project-specific authoring without editing packages</small
              ></span>
          </button>
          {#if archivedCount > 0}
            <button type="button" onclick={() => void goToSection("archive")}>
              <span class="status-icon"><Archive size={17} /></span>
              <span
                ><strong>{archivedCount} archived entr{archivedCount === 1 ? "y" : "ies"}</strong><small
                  >Restore or permanently delete archived work</small
                ></span>
            </button>
          {/if}
        </div>
        <div class="project-identity">
          <div><span>Project folder</span><strong title={summary.root}>{summary.root}</strong></div>
          <div><span>Search index</span><strong>{summary.indexStatus || "Unknown"}</strong></div>
          <div><span>AI features</span><strong>{summary.aiEnabled ? "Enabled" : "Off"}</strong></div>
        </div>
        {#if diagnostics.length > 0}
          <div class="diagnostic-callout" role="alert">
            <AlertTriangle size={17} strokeWidth={1.8} aria-hidden="true" />
            <div>
              <strong>Project needs attention</strong>
              <p>{diagnostics[0]}</p>
            </div>
            <button type="button" class="quiet-button" onclick={() => void goToSection("advanced")}>Review</button>
          </div>
        {/if}
      {:else if section === "data"}
        <div class="section-heading">
          <span class="heading-icon"><DatabaseBackup size={17} /></span>
          <div>
            <span class="kicker">PORTABILITY</span>
            <h2>Data &amp; recovery</h2>
            <p>Bring material in, take readable work out, and create explicit recovery copies.</p>
          </div>
        </div>
        <section class="operation-card">
          <div>
            <strong>Import &amp; export</strong>
            <p>Import external material through a reviewed mapping, or export the whole project as Markdown.</p>
          </div>
          <div class="action-row">
            <button type="button" class="primary-button" onclick={onImportExternal}
              ><ImportIcon size={14} /> Import material</button>
            <button
              type="button"
              class="quiet-button"
              disabled={actionBusy}
              onclick={() => void runAction(onExportMarkdown, "")}><Download size={14} /> Export Markdown</button>
          </div>
        </section>
        <section class="operation-card">
          <div>
            <strong>Backups</strong>
            <p>A portable backup travels between Daena installations. A recovery backup is a local undo point.</p>
          </div>
          <div class="action-row">
            <button
              type="button"
              class="primary-button"
              disabled={actionBusy}
              onclick={() => void createPortableBackup()}><DatabaseZap size={14} /> Create portable backup</button>
            <button type="button" class="quiet-button" disabled={actionBusy} onclick={() => void createRecoveryBackup()}
              ><Wrench size={14} /> Create recovery backup</button>
          </div>
        </section>
        <section class="operation-card">
          <div>
            <strong>Restore a recovery backup</strong>
            <p>
              Choose the recovery folder to replace the current runtime state. Daena asks for confirmation before
              restoring.
            </p>
          </div>
          <label class="path-field"
            ><span>Recovery folder</span>
            <div>
              <input bind:value={recoveryPath} placeholder="Choose or paste a recovery backup folder" /><button
                type="button"
                class="quiet-button"
                onclick={() => void pickRestorePath()}><FolderOpen size={14} /> Browse</button>
            </div></label>
          <button
            type="button"
            class="danger-button"
            disabled={actionBusy || !recoveryPath.trim()}
            onclick={() => void restoreRecoveryBackup()}>Restore recovery backup</button>
        </section>
      {:else if section === "extensions"}
        {@render extensions()}
      {:else if section === "fields"}
        {@render fields()}
      {:else if section === "snapshots"}
        {@render snapshots()}
      {:else if section === "archive"}
        <ArchivedDocumentsPanel
          {typeLabel}
          onChanged={onArchiveChanged}
          onToast={onArchiveToast} />
      {:else}
        <div class="section-heading">
          <span class="heading-icon"><Wrench size={17} /></span>
          <div>
            <span class="kicker">ADVANCED</span>
            <h2>Diagnostics &amp; maintenance</h2>
            <p>Technical controls are kept here so everyday project work stays focused.</p>
          </div>
        </div>
        <section class="operation-card">
          <div>
            <strong>Project diagnostics</strong>
            <p>Runtime index: {summary.indexStatus || "unknown"}. Portable checkpoint: {summary.sync.state}.</p>
          </div>
          {#if diagnostics.length > 0}<ul class="diagnostic-list">
              {#each diagnostics as diagnostic}<li>{diagnostic}</li>{/each}
            </ul>{:else}<p class="healthy-copy"><CircleCheck size={14} /> No unresolved project diagnostics.</p>{/if}
          {#if summary.sync.export_error}<p class="error-copy">{summary.sync.export_error}</p>{/if}
          <div class="action-row">
            <button
              type="button"
              class="quiet-button"
              disabled={actionBusy}
              onclick={() => void runAction(onImportCheckpoint, "")}
              ><RefreshCw size={14} /> Import portable checkpoint</button>
            <button
              type="button"
              class="quiet-button"
              disabled={actionBusy}
              onclick={() => void runAction(onRebuildIndex, "Search index rebuilt.")}
              ><DatabaseZap size={14} /> Rebuild search index</button>
          </div>
        </section>
        <section class="operation-card">
          <div class="split-heading">
            <div>
              <strong>AI access for this project</strong>
              <p>The provider is configured in application Settings; access and indexing are project-scoped.</p>
            </div>
            <span class:ok={summary.aiEnabled} class="state-pill">{summary.aiEnabled ? "Enabled" : "Off"}</span>
          </div>
          <div class="action-row">
            <button
              type="button"
              class={summary.aiEnabled ? "quiet-button" : "primary-button"}
              onclick={() => onToggleAi(!summary.aiEnabled)}
              ><Bot size={14} /> {summary.aiEnabled ? "Disable AI" : "Enable AI"}</button>
            {#if remoteProvider && summary.aiEnabled}<button
                type="button"
                class="quiet-button"
                onclick={() => onAiRemoteConsent(true)}>Allow remote provider</button
              ><button type="button" class="quiet-button" onclick={() => onAiRemoteConsent(false)}
                >Revoke remote access</button
              >{/if}
          </div>
          {#if summary.aiEnabled}<div class="sub-operation">
              <div>
                <strong>Semantic retrieval</strong>
                <p>
                  {aiIndexStatus?.message ?? aiIndexStatus?.state ?? "Status not checked"}{aiIndexMessage
                    ? ` · ${aiIndexMessage}`
                    : ""}
                </p>
              </div>
              <div class="action-row">
                <button type="button" class="quiet-button" onclick={onAiIndexRefresh}>Refresh</button><button
                  type="button"
                  class="primary-button"
                  disabled={aiIndexBusy}
                  onclick={onAiIndexRebuild}>{aiIndexBusy ? "Building index…" : "Build semantic index"}</button
                >{#if aiIndexBusy}<button type="button" class="quiet-button" onclick={onAiIndexCancel}>Cancel</button
                  >{/if}
              </div>
            </div>{/if}
        </section>
        <details class="raw-controls">
          <summary><FileCog size={15} /> Developer fixtures</summary>
          <div>
            <p>Add Daena's example content to this project. This is intended for testing authoring flows.</p>
            <button type="button" class="quiet-button" disabled={actionBusy} onclick={() => void seedExampleProject()}
              ><Sparkles size={14} /> Add example world</button>
          </div>
        </details>
      {/if}

      {#if actionError}<div class="action-feedback error" role="alert">
          <AlertTriangle size={15} /> <span>{actionError}</span><button
            aria-label="Dismiss"
            onclick={() => (actionError = "")}><X size={14} /></button>
        </div>{/if}
      {#if actionMessage}<div class="action-feedback success" role="status">
          <CircleCheck size={15} /> <span>{actionMessage}</span><button
            aria-label="Dismiss"
            onclick={() => (actionMessage = "")}><X size={14} /></button>
        </div>{/if}
    </div>
  </div>
</section>

<style>
.project-center {
  display: flex;
  flex-direction: column;
  min-height: calc(100vh - 58px);
  padding: 28px 32px 40px;
  background: var(--canvas);
}
.project-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 22px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.header-left,
.action-row,
.split-heading,
.healthy-copy,
.action-feedback,
.path-field div,
.raw-controls summary {
  display: flex;
  align-items: center;
}
.header-left {
  gap: 14px;
  align-items: flex-start;
}
.header-icon,
.hero-icon {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  flex: 0 0 40px;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.panel-kicker,
.kicker {
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.1em;
}
.project-header h1 {
  margin: 2px 0 6px;
  color: var(--ink);
  font: 600 22px/1.1 var(--font-display);
}
.project-header p,
.hero-copy p,
.section-heading p,
.operation-card p,
.raw-controls p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12.5px;
  line-height: 1.5;
}
.header-back {
  border-radius: 999px;
}
.project-layout {
  display: grid;
  grid-template-columns: 210px minmax(0, 1fr);
  gap: 22px;
  align-items: start;
}
.project-nav {
  position: sticky;
  top: 16px;
  display: grid;
  gap: 5px;
  padding: 6px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
}
.project-nav button {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid transparent;
  border-radius: 9px;
  background: transparent;
  color: var(--ink-soft);
  font-size: 13px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}
.project-nav button:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.project-nav button.active {
  border-color: var(--accent-dark);
  background: var(--accent-dark);
  color: var(--on-accent);
}
.nav-count {
  margin-left: auto;
  padding: 1px 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--surface) 20%, transparent);
  font-size: 10px;
}
.nav-separator {
  height: 1px;
  margin: 3px 6px;
  background: var(--line);
}
.project-panel {
  min-width: 0;
  display: grid;
  gap: 18px;
  padding: 22px 24px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.panel-hero,
.section-heading {
  display: flex;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.hero-copy strong {
  display: block;
  margin: 4px 0 7px;
  color: var(--ink);
  font: 600 18px var(--font-display);
}
.section-heading .heading-icon {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  flex: 0 0 38px;
  border-radius: 10px;
  background: var(--surface-warm);
  color: var(--accent-dark);
}
.section-heading h2 {
  margin: 4px 0 6px;
  color: var(--ink);
  font: 600 19px var(--font-display);
}
.status-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}
.status-grid button {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
}
.status-grid button:hover {
  border-color: var(--accent-soft);
  background: var(--surface-muted);
}
.status-grid button > span:last-child {
  min-width: 0;
  display: grid;
  gap: 3px;
}
.status-grid small {
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.4;
}
.status-icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  flex: 0 0 34px;
  border-radius: 9px;
  background: var(--surface-warm);
  color: var(--ink-muted);
}
.status-icon.ok {
  background: var(--theme-success-bg, var(--accent-bg));
  color: var(--success);
}
.status-icon.error {
  background: var(--danger-bg);
  color: var(--danger);
}
.project-identity {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr;
  border: 1px solid var(--line);
  border-radius: 11px;
  overflow: hidden;
}
.project-identity > div {
  min-width: 0;
  display: grid;
  gap: 4px;
  padding: 13px 15px;
}
.project-identity > div + div {
  border-left: 1px solid var(--line);
}
.project-identity span {
  color: var(--ink-faint);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.project-identity strong {
  overflow: hidden;
  color: var(--ink-soft);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.diagnostic-callout {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 13px;
  border: 1px solid var(--danger-line);
  border-radius: 10px;
  background: var(--danger-bg);
  color: var(--danger);
}
.diagnostic-callout div {
  flex: 1;
}
.diagnostic-callout p {
  margin: 3px 0 0;
  font-size: 12px;
}
.operation-card {
  display: grid;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-subtle);
}
.operation-card > div > strong {
  display: block;
  margin-bottom: 4px;
  color: var(--ink);
  font-size: 14px;
}
.action-row {
  flex-wrap: wrap;
  gap: 8px;
}
.primary-button,
.quiet-button,
.danger-button {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 9px 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.primary-button {
  border: 1px solid var(--accent-dark);
  background: var(--accent-dark);
  color: var(--on-accent);
}
.quiet-button {
  border: 1px solid var(--line);
  background: var(--surface);
  color: var(--ink-soft);
}
.danger-button {
  justify-self: start;
  border: 1px solid var(--danger-line);
  background: var(--danger-bg);
  color: var(--danger);
}
button:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}
button:focus-visible,
summary:focus-visible {
  outline: 3px solid var(--focus-ring);
  outline-offset: 2px;
}
.path-field {
  display: grid;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 11px;
}
.path-field div {
  gap: 8px;
}
.path-field input {
  min-width: 0;
  flex: 1;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
}
.diagnostic-list {
  margin: 0;
  padding-left: 19px;
  color: var(--danger);
  font-size: 12px;
  line-height: 1.5;
}
.healthy-copy {
  gap: 6px;
  color: var(--success) !important;
}
.error-copy {
  color: var(--danger) !important;
}
.split-heading {
  justify-content: space-between;
  gap: 16px;
  align-items: flex-start;
}
.state-pill {
  padding: 4px 8px;
  border-radius: 999px;
  background: var(--surface-warm);
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
}
.state-pill.ok {
  background: var(--theme-success-bg, var(--accent-bg));
  color: var(--success);
}
.sub-operation {
  display: grid;
  gap: 10px;
  padding-top: 13px;
  border-top: 1px solid var(--line);
}
.raw-controls {
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--surface-subtle);
}
.raw-controls summary {
  gap: 8px;
  padding: 13px 15px;
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 700;
  cursor: pointer;
}
.raw-controls > div {
  display: grid;
  gap: 10px;
  padding: 0 15px 15px;
}
.action-feedback {
  position: sticky;
  bottom: 14px;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--surface);
  box-shadow: var(--shadow-md);
  font-size: 12px;
}
.action-feedback span {
  flex: 1;
}
.action-feedback button {
  display: grid;
  place-items: center;
  border: 0;
  background: transparent;
  color: currentColor;
  cursor: pointer;
}
.action-feedback.error {
  border-color: var(--danger-line);
  color: var(--danger);
}
.action-feedback.success {
  color: var(--success);
}
@media (max-width: 760px) {
  .project-center {
    padding: 18px 16px 28px;
  }
  .project-layout {
    grid-template-columns: 1fr;
  }
  .project-nav {
    position: static;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .nav-separator {
    display: none;
  }
  .project-panel {
    padding: 16px;
  }
  .status-grid,
  .project-identity {
    grid-template-columns: 1fr;
  }
  .project-identity > div + div {
    border-top: 1px solid var(--line);
    border-left: 0;
  }
  .project-header {
    align-items: flex-start;
  }
  .project-header p {
    display: none;
  }
}
</style>
