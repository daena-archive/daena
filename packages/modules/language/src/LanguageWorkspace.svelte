<script lang="ts">
import { onMount } from "svelte";
import type { EntitySummary, ModuleContext } from "../../../module-api/src/index";
import ConfirmModal from "./ConfirmModal.svelte";
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
  ["forms", "Forms"],
  ["samples", "Samples"],
];

let { context }: { context: ModuleContext } = $props();

let cancelled = false;
let selectedLanguage: EntitySummary | null = $state(null);
let pane: Pane = $state("overview");
let pendingLexemeId: string | null = $state(null);
let languageQuery = $state("");
let languageSummaries: EntitySummary[] = $state([]);
let languageLoading = $state(false);
let languageLoadError = $state("");
let creatingLanguage = $state(false);
let languageCreateName = $state("");
let languageCreateError = $state("");
let createBusy = $state(false);
let languageRequest = $state(0);

let paneListEl: HTMLDivElement | undefined = $state();
let createNameInput: HTMLInputElement | undefined = $state();

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
    if (guard && !await guard()) return false;
  }
  return true;
}

let mutationCounter = 0;

function setMutationActive(active: boolean) {
  mutationCounter = Math.max(0, mutationCounter + (active ? 1 : -1));
}

function isMutating() {
  return mutationCounter > 0;
}

const visibleLanguages = $derived(viewLanguageList(languageQuery, languageSummaries));

function viewLanguageList(query: string, summaries: EntitySummary[]) {
  const needle = query.trim().toLocaleLowerCase();
  return needle ? summaries.filter((language) => language.name.toLocaleLowerCase().includes(needle)) : summaries;
}

$effect(() => {
  if (creatingLanguage && createNameInput) createNameInput.focus();
});

onMount(() => {
  languageLoading = true;
  void loadLanguages();
  return () => {
    cancelled = true;
  };
});

async function loadLanguages() {
  const token = ++languageRequest;
  try {
    const languages = await context.entities.list({ type: "language", limit: 500 });
    if (cancelled || token !== languageRequest) return;
    languageSummaries = languages;
    languageLoading = false;
    languageLoadError = "";
    if (!selectedLanguage && languages.length) {
      selectedLanguage = languages.find((language) => language.id === context.focusEntityId) ?? languages[0];
    }
  } catch (cause) {
    if (cancelled || token !== languageRequest) return;
    languageLoading = false;
    languageLoadError = cause instanceof Error ? cause.message : String(cause);
  }
}

async function openLanguage(language: EntitySummary) {
  if (language.id === selectedLanguage?.id) return;
  if (isMutating() || !await canLeave()) return;
  selectedLanguage = language;
  pendingLexemeId = null;
}

async function switchPane(id: Pane) {
  if (pane === id) return;
  if (!await canLeave()) return;
  pane = id;
}

async function openLinkedLexeme(lexemeId: string) {
  pendingLexemeId = lexemeId;
  if (pane === "lexicon") return;
  if (!await canLeave()) {
    pendingLexemeId = null;
    return;
  }
  pane = "lexicon";
}

function clearPendingLexeme() {
  pendingLexemeId = null;
}

function onLanguageChanged(language: EntitySummary) {
  languageSummaries = languageSummaries.map((item) => (item.id === language.id ? language : item));
  if (selectedLanguage?.id === language.id) selectedLanguage = language;
}

function onLanguageArchived(languageId: string) {
  languageSummaries = languageSummaries.filter((language) => language.id !== languageId);
  if (selectedLanguage?.id === languageId) {
    selectedLanguage = languageSummaries[0] ?? null;
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

function openCreateForm() {
  creatingLanguage = true;
  languageCreateName = "";
  languageCreateError = "";
}

function cancelCreateLanguage() {
  creatingLanguage = false;
  languageCreateName = "";
  languageCreateError = "";
}

async function submitCreateLanguage(event: SubmitEvent) {
  event.preventDefault();
  languageCreateName = languageCreateName.trim();
  if (!languageCreateName) {
    languageCreateError = "Language name is required.";
    createNameInput?.focus();
    return;
  }
  createBusy = true;
  try {
    const created = await context.entities.create({ name: languageCreateName, type: "language" });
    languageSummaries = [created, ...languageSummaries.filter((language) => language.id !== created.id)];
    languageLoading = false;
    selectedLanguage = created;
    creatingLanguage = false;
    languageCreateName = "";
    languageCreateError = "";
    createBusy = false;
    pendingLexemeId = null;
  } catch (cause) {
    languageCreateError = cause instanceof Error ? cause.message : String(cause);
    createBusy = false;
    createNameInput?.focus();
  }
}
</script>

<section class="language-workspace" class:language-workspace-embedded={context.embedded}>
  {#if !context.embedded}
    <aside class="language-panel language-sidebar" aria-busy={languageLoading}>
      <div class="language-sidebar-head">
        <div>
          <p class="language-sidebar-kicker">Language studio</p>
          <h2>Languages</h2>
        </div>
        <button type="button" class="language-button secondary" onclick={openCreateForm}>Create language</button>
      </div>
      <p class="language-sidebar-intro">Choose a language to shape its words, sounds, writing, and grammar.</p>
      <label class="language-field">
        <span>Filter languages</span>
        <input name="languageQuery" type="search" bind:value={languageQuery} />
      </label>
      {#if creatingLanguage}
        <form class="language-create" onsubmit={submitCreateLanguage}>
          <label class="language-field">
            <span>Language name</span>
            <input
              name="languageCreateName"
              autocomplete="off"
              bind:this={createNameInput}
              bind:value={languageCreateName}
              oninput={() => (languageCreateError = "")} />
          </label>
          {#if languageCreateError}
            <p class="language-status error" role="alert">{languageCreateError}</p>
          {/if}
          <div class="language-create-actions">
            <button type="button" class="language-button secondary" onclick={cancelCreateLanguage}>Cancel</button>
            <button type="submit" class="language-button" disabled={createBusy}
              >{createBusy ? "Creating…" : "Create"}</button>
          </div>
        </form>
      {/if}
      <ul class="language-list">
        {#if languageLoading}
          <li><p class="language-empty" role="status">Loading languages…</p></li>
        {:else if languageLoadError}
          <li><p class="language-status error" role="alert">{languageLoadError}</p></li>
        {:else if languageSummaries.length === 0}
          <li><p class="language-empty" role="status">No languages yet. Create one to start.</p></li>
        {:else if visibleLanguages.length === 0}
          <li><p class="language-empty" role="status">No languages match that filter.</p></li>
        {:else}
          {#each visibleLanguages as language (language.id)}
            <li>
              <button
                type="button"
                aria-current={selectedLanguage?.id === language.id ? "page" : undefined}
                onclick={() => openLanguage(language)}>
                <span class="language-list-name">{language.name}</span>
                <span class="language-list-meta"
                  >{selectedLanguage?.id === language.id ? "Selected language" : "Open language"}</span>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </aside>
  {/if}
  <div id="language-pane" class="language-panel language-main" role="tabpanel" aria-labelledby={`language-tab-${pane}`}>
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
        setMutationActive={setMutationActive} />
    </div>
    <div class="language-pane" hidden={pane !== "writing"}>
      <Writing
        {context}
        {selectedLanguage}
        active={pane === "writing"}
        registerLeaveGuard={registerWritingGuard}
        setMutationActive={setMutationActive} />
    </div>
    <div class="language-pane" hidden={pane !== "grammar"}>
      <Grammar {context} {selectedLanguage} active={pane === "grammar"} registerLeaveGuard={registerGrammarGuard} />
    </div>
    <div class="language-pane" hidden={pane !== "forms"}>
      <Forms
        {context}
        {selectedLanguage}
        active={pane === "forms"}
        registerLeaveGuard={registerFormsGuard}
        setMutationActive={setMutationActive} />
    </div>
    <div class="language-pane" hidden={pane !== "samples"}>
      <Samples
        {context}
        {selectedLanguage}
        active={pane === "samples"}
        openLexeme={openLinkedLexeme}
        registerLeaveGuard={registerSamplesGuard}
        setMutationActive={setMutationActive} />
    </div>
    <div class="language-pane" hidden={pane !== "lexicon"}>
      <Lexicon
        {context}
        {selectedLanguage}
        active={pane === "lexicon"}
        pendingLexemeId={pendingLexemeId}
        onPendingLexemeHandled={clearPendingLexeme}
        registerLeaveGuard={registerLexiconGuard}
        setMutationActive={setMutationActive} />
    </div>
  </div>
</section>

<ConfirmModal />

<style>
.language-workspace {
  display: grid;
  grid-template-columns: minmax(220px, 260px) minmax(0, 1fr);
  gap: 18px;
  height: 100%;
  min-height: 0;
  color: var(--ink);
}
.language-workspace-embedded {
  grid-template-columns: minmax(0, 1fr);
  height: auto;
}
.language-panel {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 16px;
  background: var(--surface);
  padding: 22px 20px 24px;
  box-shadow: var(--shadow-sm, 0 2px 8px rgba(38, 42, 33, 0.05));
}
.language-sidebar {
  gap: 14px;
}
.language-sidebar-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.language-sidebar-kicker {
  margin: 0 0 5px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}
.language-sidebar-intro {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
}
.language-sidebar-intro {
  margin-top: -5px;
}
.language-field {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
}
@media (max-width: 760px) {
  .language-workspace {
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .language-sidebar {
    max-height: none;
  }
  .language-main {
    min-height: 34rem;
  }
  .language-tabs {
    flex-wrap: nowrap;
    overflow-x: auto;
    overscroll-behavior-inline: contain;
    padding-bottom: 10px;
    scrollbar-width: thin;
  }
  .language-tabs button {
    flex: 0 0 auto;
  }
}
.language-panel h2 {
  margin: 0;
  font-family: var(--font-display);
  font-weight: 500;
  font-size: 24px;
  line-height: 1.15;
}
.language-list {
  display: grid;
  gap: 8px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}
.language-list button {
  display: grid;
  gap: 3px;
  width: 100%;
  padding: 11px 12px;
  border: 1px solid #ebe7de;
  border-radius: 10px;
  background: var(--surface);
  color: inherit;
  text-align: left;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(38, 42, 33, 0.03);
}
.language-list button:hover {
  border-color: #e5d8c6;
  background: var(--surface-muted);
}
.language-list button[aria-current="page"] {
  border-color: #d8c3a5;
  background: var(--surface-muted);
  box-shadow:
    inset 3px 0 var(--accent),
    0 1px 2px rgba(38, 42, 33, 0.03);
  color: var(--ink);
}
.language-list-name {
  font-weight: 600;
}
.language-list-meta {
  color: var(--ink-faint);
  font-size: 11px;
}
.language-create {
  display: grid;
  gap: 10px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-muted);
}
.language-create-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}
.language-field input {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}
.language-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 0 0 8px;
  padding: 0 0 12px;
  background: var(--surface);
}
.language-tabs button {
  padding: 7px 12px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: transparent;
  color: var(--ink-soft);
  cursor: pointer;
}
.language-tabs button:hover {
  border-color: #d8c3a5;
  color: var(--ink);
  background: var(--surface-muted);
}
.language-tabs button[aria-selected="true"] {
  border-color: var(--accent-dark);
  background: var(--surface-muted);
  color: var(--accent-dark);
}
.language-button {
  padding: 8px 12px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
  cursor: pointer;
}
.language-button:hover {
  filter: brightness(1.06);
}
.language-button.secondary {
  background: transparent;
  color: var(--accent-dark);
}
.language-button.secondary:hover {
  background: var(--surface-muted);
}
.language-button:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  filter: none;
}
.language-button:focus-visible,
.language-tabs button:focus-visible,
.language-list button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.language-empty,
.language-status {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.6;
}
.language-status.error {
  color: #a14f42;
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
  padding: 8px 12px;
  border: 1px solid var(--accent-dark);
  border-radius: 8px;
  background: var(--accent-dark);
  color: #fff;
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
  border-color: #a14f42 !important;
  color: #a14f42 !important;
  background: transparent;
}
:global(.language-group) {
  display: grid;
  gap: 10px;
  min-width: 0;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface-muted);
}
:global(.language-group .language-group) {
  background: var(--surface);
}
:global(.language-field) {
  display: grid;
  gap: 6px;
  min-width: 0;
  color: var(--ink-soft);
  font-size: 11px;
  letter-spacing: 0.01em;
}
:global(.language-field input),
:global(.language-field textarea) {
  box-sizing: border-box;
  width: 100%;
  min-width: 0;
  padding: 9px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--surface);
  color: var(--ink);
  font: inherit;
}
:global(.language-field textarea) {
  min-height: 4.5em;
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
  color: #a14f42;
}
@media (max-width: 760px) {
  :global(.language-inline) {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
