<script lang="ts">
import { onMount, tick } from "svelte";
import type { EntitySummary, ModuleContext, UUID } from "../../../module-api/src/index";
import ConfirmModal from "./ConfirmModal.svelte";
import WorkspaceGuide from "../../../../src/lib/guides/WorkspaceGuide.svelte";
import { dismissGuide, isGuideDismissed } from "../../../../src/lib/guides/persist.ts";
import type { GuideMode } from "../../../../src/lib/guides/types.ts";
import { LANGUAGE_GUIDE_ID, languageGuideSteps } from "./guide.ts";
import Overview from "./panes/Overview.svelte";
import Lexicon from "./panes/Lexicon.svelte";
import Sounds from "./panes/Sounds.svelte";
import Writing from "./panes/Writing.svelte";
import Grammar from "./panes/Grammar.svelte";
import Forms from "./panes/Forms.svelte";
import Samples from "./panes/Samples.svelte";

type Pane = "overview" | "lexicon" | "sounds" | "writing" | "grammar" | "forms" | "samples";

const PANES: [Pane, string][] = [
  ["overview", "Overview"],
  ["lexicon", "Lexicon"],
  ["sounds", "Sounds"],
  ["writing", "Writing"],
  ["grammar", "Grammar"],
  ["forms", "Morphology"],
  ["samples", "Samples"],
];

let { context }: { context: ModuleContext } = $props();

let cancelled = false;
let selectedLanguage: EntitySummary | null = $state(null);
let incompatibleFocus = $state(false);
let pane: Pane = $state("overview");
let pendingLexemeId: string | null = $state(null);
// svelte-ignore state_referenced_locally
let languageLoading = $state(Boolean(context.focusEntityId));
let languageRequest = $state(0);
let guideOpen = $state(false);
let guideMode: GuideMode = $state("tour");
let guideIndex = $state(0);
let tourPaused = $state(false);
let guideBooted = $state(false);
const guideSteps = $derived(languageGuideSteps({ hasLanguage: Boolean(selectedLanguage), pane, mode: guideMode }));

let paneListEl: HTMLDivElement | undefined = $state();

type BreadcrumbItem = { label: string; onclick?: () => void };
let breadcrumbExtra: BreadcrumbItem[] = $state([]);

function setBreadcrumbExtra(items: BreadcrumbItem[]) {
  breadcrumbExtra = items;
}

$effect(() => {
  const externalPane = (context.moduleState?.pane as Pane) ?? "overview";
  if (externalPane !== pane) {
    // History restoration: update local pane without recording new history.
    // Respect leave guards by attempting to leave; if blocked, revert shell state.
    void (async () => {
      if (!(await canLeave())) {
        // Revert shell to local pane
        context.onModuleStateChange?.({ pane });
        return;
      }
      pane = externalPane;
    })();
  }
});

$effect(() => {
  void pane;
  breadcrumbExtra = [];
});

const leaveGuards: Partial<Record<Pane, (() => Promise<boolean> | boolean) | null>> = {};

function registerLeaveGuard(paneId: Pane) {
  return (guard: (() => Promise<boolean> | boolean) | null) => {
    leaveGuards[paneId] = guard;
  };
}

const registerOverviewGuard = registerLeaveGuard("overview");
const registerLexiconGuard = registerLeaveGuard("lexicon");
const registerSoundsGuard = registerLeaveGuard("sounds");
const registerWritingGuard = registerLeaveGuard("writing");
const registerGrammarGuard = registerLeaveGuard("grammar");
const registerFormsGuard = registerLeaveGuard("forms");
const registerSamplesGuard = registerLeaveGuard("samples");

async function canLeave() {
  for (const paneId of PANES.map(([id]) => id)) {
    const guard = leaveGuards[paneId];
    if (guard && !(await guard())) return false;
  }
  return true;
}

$effect(() => {
  // Expose guard to shell for history navigation (Back/Forward leaving language)
  (window as unknown as Record<string, unknown>).__daena_canLeaveLanguage = canLeave;
  return () => {
    delete (window as unknown as Record<string, unknown>).__daena_canLeaveLanguage;
  };
});

let mutationCounter = 0;

function setMutationActive(active: boolean) {
  mutationCounter = Math.max(0, mutationCounter + (active ? 1 : -1));
}

function isMutating() {
  return mutationCounter > 0;
}

onMount(() => {
  // Load language entity if focusEntityId is provided
  if (context.focusEntityId) {
    void loadLanguage(context.focusEntityId);
  }

  return () => {
    cancelled = true;
  };
});

async function loadLanguage(entityId: string) {
  const token = ++languageRequest;
  languageLoading = true;
  try {
    const entity = await context.entities.get(entityId as UUID);
    if (cancelled || token !== languageRequest) return;
    if (entity?.type === "daena.language:language") {
      selectedLanguage = entity;
      incompatibleFocus = false;
    } else {
      selectedLanguage = null;
      incompatibleFocus = entity !== null;
    }
    languageLoading = false;
  } catch (cause) {
    if (cancelled || token !== languageRequest) return;
    languageLoading = false;
    incompatibleFocus = false;
  }
}

async function switchPane(id: Pane) {
  if (pane === id) return;
  if (!(await canLeave())) return;
  pane = id;
  context.onModuleStateChange?.({ pane: id });
}

async function openLinkedLexeme(lexemeId: string) {
  pendingLexemeId = lexemeId;
  if (pane === "lexicon") return;
  if (!(await canLeave())) {
    pendingLexemeId = null;
    return;
  }
  if (context.onModuleStateChange) {
    context.onModuleStateChange({ pane: "lexicon" });
  } else {
    pane = "lexicon";
  }
}

function clearPendingLexeme() {
  pendingLexemeId = null;
}

function onLanguageChanged(language: EntitySummary) {
  if (selectedLanguage?.id === language.id) selectedLanguage = language;
}

function onLanguageArchived(languageId: string) {
  if (selectedLanguage?.id === languageId) {
    selectedLanguage = null;
    pendingLexemeId = null;
  }
}

function roveTabs(event: KeyboardEvent, index: number) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight" && event.key !== "Home" && event.key !== "End") {
    return;
  }
  event.preventDefault();
  const tabs = paneListEl?.querySelectorAll<HTMLButtonElement>("button") ?? [];
  const next =
    event.key === "Home"
      ? 0
      : event.key === "End"
        ? tabs.length - 1
        : (index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
  tabs[next]?.focus();
  tabs[next]?.click();
}

function openGuide() {
  tourPaused = false;
  guideMode = "hint";
  guideIndex = 0;
  guideOpen = true;
}

function finishGuide() {
  guideOpen = false;
  tourPaused = false;
  dismissGuide(LANGUAGE_GUIDE_ID);
}

async function handleGuidePrimary(step: { id: string; action?: string }) {
  if (step.action === "pause") {
    guideOpen = false;
    tourPaused = !isGuideDismissed(LANGUAGE_GUIDE_ID);
    return;
  }
  if (step.action === "lexicon") await switchPane("lexicon");
  if (step.action === "grammar") await switchPane("grammar");
  if (step.action === "sounds") await switchPane("sounds");
  await tick();
}

$effect(() => {
  if (guideBooted || languageLoading) return;
  guideBooted = true;
  if (!isGuideDismissed(LANGUAGE_GUIDE_ID)) {
    guideMode = "tour";
    guideIndex = 0;
    guideOpen = true;
  }
});

$effect(() => {
  if (!selectedLanguage || !tourPaused || isGuideDismissed(LANGUAGE_GUIDE_ID)) return;
  tourPaused = false;
  guideMode = "tour";
  guideIndex = 0;
  guideOpen = true;
});
</script>

<section class="language-workspace">
  <div id="language-pane" class="language-panel language-main" role="tabpanel" aria-labelledby={`language-tab-${pane}`}>
    <div class="language-main-header">
      <nav class="language-breadcrumb" aria-label="Breadcrumb">
        <ol>
          <li>
            <button type="button" class="language-breadcrumb-link" onclick={() => switchPane("overview")}
              >Languages</button>
          </li>
          {#if selectedLanguage}
            <li>
              <span class="language-breadcrumb-sep" aria-hidden="true">›</span>
              <button type="button" class="language-breadcrumb-link" onclick={() => switchPane("overview")}
                >{selectedLanguage.name}</button>
            </li>
            <li>
              <span class="language-breadcrumb-sep" aria-hidden="true">›</span>
              {#if breadcrumbExtra.length > 0}
                <button type="button" class="language-breadcrumb-link" onclick={() => switchPane(pane)}
                  >{PANES.find(([id]) => id === pane)?.[1] ?? ""}</button>
              {:else}
                <span class="language-breadcrumb-current">{PANES.find(([id]) => id === pane)?.[1] ?? ""}</span>
              {/if}
            </li>
            {#each breadcrumbExtra as item, i (item.label)}
              <li>
                <span class="language-breadcrumb-sep" aria-hidden="true">›</span>
                {#if i < breadcrumbExtra.length - 1 && item.onclick}
                  <button type="button" class="language-breadcrumb-link" onclick={item.onclick}>{item.label}</button>
                {:else}
                  <span class="language-breadcrumb-current">{item.label}</span>
                {/if}
              </li>
            {/each}
          {/if}
        </ol>
      </nav>
      <button type="button" class="language-help-button" onclick={openGuide} aria-label="Show language guide">?</button>
    </div>
    {#if languageLoading && !selectedLanguage}
      <p class="language-empty language-loading" role="status" aria-live="polite">Loading language…</p>
    {:else if !selectedLanguage}
      <div class="language-empty-screen" role="status">
        <div class="language-empty-mark" aria-hidden="true"></div>
        {#if incompatibleFocus}
          <h3>The selected item is not a language.</h3>
          <p>Select a Language entity to work with its words, sounds, writing, and grammar.</p>
        {:else}
          <h3>Your language workshop is waiting.</h3>
          <p>
            Select a language from the list, or create your first language to begin building words, sounds, writing and
            grammar.
          </p>
        {/if}
      </div>
    {:else}
      <div bind:this={paneListEl} class="language-tabs" role="tablist" aria-label="Language workspace">
        {#each PANES as [id, label], index (id)}
          <button
            type="button"
            role="tab"
            id={`language-tab-${id}`}
            aria-controls="language-pane"
            aria-selected={pane === id}
            tabindex={pane === id ? 0 : -1}
            onclick={() => switchPane(id)}
            onkeydown={(event) => roveTabs(event, index)}>{label}</button>
        {/each}
      </div>
      <div class="language-pane" hidden={pane !== "overview"}>
        <Overview
          {context}
          {selectedLanguage}
          active={pane === "overview"}
          registerLeaveGuard={registerOverviewGuard}
          {onLanguageChanged}
          {onLanguageArchived} />
      </div>
      <div class="language-pane" hidden={pane !== "sounds"}>
        <Sounds
          {context}
          {selectedLanguage}
          active={pane === "sounds"}
          registerLeaveGuard={registerSoundsGuard}
          {setMutationActive} />
      </div>
      <div class="language-pane" hidden={pane !== "writing"}>
        <Writing
          {context}
          {selectedLanguage}
          active={pane === "writing"}
          registerLeaveGuard={registerWritingGuard}
          {setMutationActive} />
      </div>
      <div class="language-pane" hidden={pane !== "grammar"}>
        <Grammar
          {context}
          {selectedLanguage}
          active={pane === "grammar"}
          registerLeaveGuard={registerGrammarGuard}
          {setMutationActive}
          {setBreadcrumbExtra} />
      </div>
      <div class="language-pane" hidden={pane !== "forms"}>
        <Forms
          {context}
          {selectedLanguage}
          active={pane === "forms"}
          registerLeaveGuard={registerFormsGuard}
          {setMutationActive} />
      </div>
      <div class="language-pane" hidden={pane !== "samples"}>
        <Samples
          {context}
          {selectedLanguage}
          active={pane === "samples"}
          openLexeme={openLinkedLexeme}
          registerLeaveGuard={registerSamplesGuard}
          {setMutationActive} />
      </div>
      <div class="language-pane" hidden={pane !== "lexicon"}>
        <Lexicon
          {context}
          {selectedLanguage}
          active={pane === "lexicon"}
          {pendingLexemeId}
          onPendingLexemeHandled={clearPendingLexeme}
          registerLeaveGuard={registerLexiconGuard}
          {setMutationActive} />
      </div>
    {/if}
  </div>
</section>

<ConfirmModal />

{#if guideOpen}
  <WorkspaceGuide
    open={guideOpen}
    steps={guideSteps}
    stepIndex={guideIndex}
    onStepIndex={(index) => (guideIndex = index)}
    onDismiss={finishGuide}
    onComplete={finishGuide}
    onPrimary={handleGuidePrimary} />
{/if}

<style>
.language-workspace {
  display: block;
  height: 100%;
  min-height: 0;
  color: var(--ink);
}
.language-panel {
  --language-control-height: 34px;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 650px;
  height: 100%;
  flex: 1;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  padding: 22px 20px 24px;
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
  box-sizing: border-box;
}

@media (max-width: 760px) {
  .language-workspace {
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .language-main {
    min-height: 34rem;
  }
  .language-panel {
    min-height: auto;
  }
  .language-empty-screen {
    min-height: 420px;
  }
  .language-tabs {
    overflow-x: auto;
    overscroll-behavior-inline: contain;
    padding-inline: 12px;
    scrollbar-width: thin;
  }
  .language-tabs button {
    flex: 0 0 auto;
  }
}
.language-help-button {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 14px;
  font-weight: 700;
  cursor: pointer;
  flex-shrink: 0;
}
.language-help-button:hover,
.language-help-button:focus-visible {
  background: var(--theme-warning-bg, #ebe6dd);
  color: var(--ink);
  border-color: var(--theme-neutral-border-strong, var(--line-strong));
  outline: 0;
}
.language-main-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.language-breadcrumb {
  margin: 0;
  padding: 0;
}
.language-breadcrumb ol {
  display: flex;
  align-items: center;
  gap: 2px;
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: 13px;
}
.language-breadcrumb li {
  display: flex;
  align-items: center;
  gap: 2px;
}
.language-breadcrumb-sep {
  color: var(--ink-faint);
  font-size: 11px;
  margin: 0 2px;
  user-select: none;
}
.language-breadcrumb-link {
  display: inline-flex;
  align-items: center;
  padding: 2px 6px;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--ink-soft);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
  text-decoration: none;
  transition:
    background 0.15s,
    color 0.15s;
}
.language-breadcrumb-link:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.language-breadcrumb-link:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.language-breadcrumb-current {
  padding: 2px 6px;
  color: var(--ink);
  font-weight: 600;
  font-size: 13px;
}
.language-tabs {
  position: sticky;
  top: 0;
  z-index: 3;
  display: flex;
  gap: 4px;
  min-height: 46px;
  margin: 0 -20px 16px;
  padding: 7px 20px;
  border-bottom: 1px solid var(--line);
  background: color-mix(in srgb, var(--surface) 92%, transparent);
  backdrop-filter: blur(14px);
}
.language-tabs button {
  min-height: var(--control-min-height, 34px);
  padding: 7px 11px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--ink-soft);
  font: inherit;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
}
.language-tabs button:hover {
  background: var(--surface-muted);
  color: var(--ink);
}
.language-tabs button[aria-selected="true"] {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--accent-dark);
  color: var(--on-accent, #fff);
}
.language-empty-screen {
  display: grid;
  place-items: center;
  place-content: center;
  flex: 1;
  min-height: 520px;
  padding: 48px 24px;
  text-align: center;
}
.language-empty-mark {
  width: 44px;
  height: 44px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-empty-screen h3 {
  margin: 18px 0 6px;
  font: 500 23px var(--font-display);
  color: var(--ink);
}
.language-empty-screen p {
  max-width: 320px;
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.6;
}
.language-empty.language-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 36px 0;
  color: var(--ink-soft);
  font-size: 12px;
}
.language-empty.language-loading::before {
  content: "";
  width: 11px;
  height: 11px;
  flex: 0 0 11px;
  border: 2px solid var(--line);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: language-spin 0.75s linear infinite;
}
@keyframes language-spin {
  to {
    transform: rotate(360deg);
  }
}
@media (prefers-reduced-motion: reduce) {
  .language-empty.language-loading::before {
    animation: none;
  }
}
:global(.language-panel h2),
:global(.language-panel h3) {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 500;
}
:global(.language-panel h2) {
  font-size: 24px;
  line-height: 1.15;
}
:global(.language-panel h3) {
  font-size: 16px;
  line-height: 1.3;
}
:global(.language-button) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 34px;
  padding: 0 12px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
:global(.language-button:hover) {
  filter: brightness(1.06);
}
:global(.language-button.secondary) {
  background: transparent;
  color: var(--accent-dark);
}
:global(.language-button.secondary:hover) {
  background: var(--surface-muted);
}
:global(.language-button:disabled) {
  opacity: 0.45;
  cursor: not-allowed;
  filter: none;
}
:global(.language-button:focus-visible) {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
:global(.language-danger) {
  border-color: var(--danger) !important;
  color: var(--danger) !important;
  background: transparent;
}
:global(.language-group),
:global(.language-form-section) {
  display: grid;
  gap: 12px;
  min-width: 0;
  padding: 16px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 12px;
  background: var(--surface);
}
:global(.language-group .language-group) {
  padding: 12px;
  background: var(--surface-muted);
  border-radius: 10px;
}
:global(.language-group-head) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-width: 0;
}
:global(.language-group > h3),
:global(.language-group-head h3),
:global(.language-form-section h3) {
  margin: 0;
  color: var(--ink);
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.02em;
}
:global(.language-link-row) {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 34px;
  padding: 0 10px 0 12px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 8px;
  background: var(--theme-surface-bg, var(--surface));
}
:global(.language-link-kind) {
  flex: none;
  color: var(--ink-muted);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
:global(.language-link-label) {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--ink);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
:global(.language-field) {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
}
:global(.language-field > span) {
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
:global(.language-field-prompt > span) {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.01em;
  text-transform: none;
}
:global(.language-field input),
:global(.language-field textarea),
:global(.language-field select) {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 0 10px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 8px;
  background: var(--theme-surface-bg, var(--surface));
  color: var(--ink);
  font: 12px/1.35 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
:global(.language-panel input:not([type="checkbox"]):not([type="radio"]):not([type="file"])),
:global(.language-panel select) {
  box-sizing: border-box;
  height: var(--language-control-height);
  min-height: var(--language-control-height);
  line-height: 1.2;
}
:global(.language-panel select) {
  appearance: none;
  padding-right: 34px !important;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='14' height='14' viewBox='0 0 24 24' fill='none' stroke='%23676d63' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E") !important;
  background-repeat: no-repeat !important;
  background-position: right 11px center !important;
  background-size: 14px !important;
  cursor: pointer;
}
:global(.language-panel input:not([type="checkbox"]):not([type="radio"]):not([type="file"]):hover),
:global(.language-panel textarea:hover),
:global(.language-panel select:hover) {
  border-color: var(--theme-warning-border, #d8c3a5);
}
:global(.language-panel input:not([type="checkbox"]):not([type="radio"]):not([type="file"]):focus-visible),
:global(.language-panel textarea:focus-visible),
:global(.language-panel select:focus-visible) {
  border-color: var(--accent) !important;
  outline: none !important;
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.16) !important;
}
:global(.language-panel input::placeholder),
:global(.language-panel textarea::placeholder) {
  color: var(--ink-faint);
  opacity: 0.78;
}
:global(.language-panel select:disabled) {
  cursor: not-allowed;
}
:global(.language-field textarea) {
  min-height: 4.5em;
  padding: 8px 10px;
  resize: vertical;
}
:global(.language-inline) {
  display: flex;
  align-items: end;
  gap: 8px;
  min-width: 0;
}
:global(.language-inline > .language-button) {
  flex: 0 0 auto;
}
:global(.language-empty),
:global(.language-status) {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.6;
}
:global(.language-status.error) {
  color: var(--danger);
}
:global(.language-section-grid) {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
:global(.language-field-wide) {
  grid-column: 1 / -1;
}
:global(.language-toolbar) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 34px;
  flex-wrap: wrap;
}
:global(.language-toolbar-title) {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
:global(.language-toolbar-title h2) {
  margin: 0;
  font-size: 22px;
}
:global(.language-toolbar-status) {
  color: var(--ink-muted);
  font-size: 12px;
}
:global(.language-toolbar-actions) {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
:global(.language-mode-toggle) {
  display: inline-flex;
  align-items: center;
  min-height: 34px;
  padding: 0 12px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 8px;
  background: var(--theme-surface-bg, var(--surface));
  color: var(--ink-soft);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}
:global(.language-mode-toggle[aria-pressed="true"]) {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--accent-dark);
  color: var(--on-accent, #fff);
}
:global(.language-actions) {
  position: sticky;
  bottom: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: 12px;
  padding: 12px 0 4px;
  border-top: 1px solid var(--line-soft, var(--line));
  background: var(--surface);
  flex-wrap: wrap;
}
:global(.language-actions > span) {
  display: flex;
  gap: 8px;
}
@media (max-width: 760px) {
  :global(.language-inline) {
    flex-direction: column;
    align-items: stretch;
  }
  :global(.language-section-grid) {
    grid-template-columns: 1fr;
  }
  :global(.language-actions) {
    flex-direction: column;
    align-items: stretch;
  }
  :global(.language-actions > span) {
    flex-direction: column;
  }
}
</style>
