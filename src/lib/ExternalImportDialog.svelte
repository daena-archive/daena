<script lang="ts">
import { nativeErrorText } from "$lib/i18n/native-errors.ts";
import { manifestText } from "$lib/i18n/manifest-copy";
import { ui } from "$lib/i18n/ui";
import { onMount, tick } from "svelte";
import { listen } from "@tauri-apps/api/event";
import { FileText, FolderOpen, X, AlertTriangle, CircleCheck, LoaderCircle } from "@lucide/svelte";
import {
  EXTERNAL_IMPORT_PROGRESS_EVENT,
  project,
  type ExternalImporterDescriptor,
  type ExternalImportAnalysisStatus,
  type ExternalImportCommitReport,
  type ExternalImportPage,
  type ExternalImportPageItem,
  type ExternalImportValidationSummary,
  type Entity,
  type ImportCandidatePlan,
  type ImportMappingDecision,
  type ImportMappingOverrides,
  type ImportObjectDecision,
  type ProjectModuleManifest,
  type StagedObject,
} from "$lib/project/client";
import { buildExternalImportMappingCatalog, importFolderFor, type ImportFieldChoice } from "$lib/externalImport";
import {
  displayError,
  pageItemSubtitle,
  pageItemTitle,
  previewValue,
  relationshipSources,
  stagedDocumentFormat,
} from "$lib/externalImportDisplay";

let {
  modules,
  entities,
  onCommitted,
  onClose,
}: {
  modules: ProjectModuleManifest[];
  entities: Entity[];
  onCommitted: (report: ExternalImportCommitReport) => Promise<void>;
  onClose: () => void;
} = $props();

const pageSize = 50;
let dialogElement = $state<HTMLDivElement | null>(null);
let importers = $state<ExternalImporterDescriptor[]>([]);
let importerKey = $state("");
let sourceKind = $state<"file" | "folder">("folder");
let sourceName = $state("");
let status = $state<ExternalImportAnalysisStatus | null>(null);
let page = $state<ExternalImportPage | null>(null);
let pageOffset = $state(0);
let selectedItemIndex = $state(0);
let inspectedItem = $derived.by(() => page?.items[selectedItemIndex] ?? null);
let plan = $state<ImportCandidatePlan | null>(null);
let mappings = $state<ImportMappingOverrides>({ global: {}, categories: {}, folders: {}, items: {} });
let decisions = $state<Record<string, ImportObjectDecision>>({});
let validation = $state<ExternalImportValidationSummary | null>(null);
let report = $state<ExternalImportCommitReport | null>(null);
let acknowledgeWarnings = $state(false);
let confirmCommit = $state(false);
let commitRequestId = $state("");
let validating = $state(false);
let committing = $state(false);
let mappingScope = $state<"global" | "category" | "folder" | "item">("global");
let mappingCategory = $state("");
let busy = $state(false);
let previewLoading = $state(false);
let planLoading = $state(false);
let error = $state("");
let planRequest = 0;
let planTimer: number | null = null;
let pollTimer: number | null = null;
let unlistenProgress: (() => void) | null = null;
let lastFocused: Element | null = null;

const catalog = () => buildExternalImportMappingCatalog(modules);
const importerKeyOf = (importer: ExternalImporterDescriptor) => `${importer.pluginId ?? ""}:${importer.id}`;
const activeImporter = () => importers.find((importer) => importerKeyOf(importer) === importerKey) ?? null;
const selectedImporter = $derived(activeImporter());
const importerSupports = (kind: "file" | "folder") => activeImporter()?.sourceKinds.includes(kind) ?? false;
const selectedPageItem = () => inspectedItem;
const selectedObject = (): StagedObject | null => {
  const item = selectedPageItem();
  return item?.kind === "object" ? item.value : null;
};
const selectedFolder = () => (selectedObject() ? importFolderFor(selectedObject()!.source_path) : "");
const selectedCategories = () => selectedObject()?.tags ?? [];

function pageItemKey(item: ExternalImportPageItem, index: number): string {
  if (item.kind === "object" || item.kind === "asset") return item.value.id;
  return `${item.kind}:${pageOffset + index}`;
}

function normalizeSourceKind() {
  const importer = activeImporter();
  if (!importer || importer.sourceKinds.includes(sourceKind)) return;
  sourceKind = importer.sourceKinds.includes("folder") ? "folder" : "file";
}

function selectPageItem(index: number) {
  selectedItemIndex = index;
  const item = page?.items[index];
  if (mappingScope === "folder" && (item?.kind !== "object" || importFolderFor(item.value.source_path) === "")) {
    mappingScope = "global";
  }
  if (item?.kind === "object") {
    if (!item.value.tags?.includes(mappingCategory)) mappingCategory = item.value.tags?.[0] ?? "";
    if (mappingScope === "category" && !mappingCategory) mappingScope = "global";
  } else if (mappingScope === "category") {
    mappingScope = "global";
  }
}

function currentDecision(): ImportMappingDecision {
  const object = selectedObject();
  if (mappingScope === "item" && object) return mappings.items?.[object.id] ?? {};
  if (mappingScope === "category" && mappingCategory) return mappings.categories?.[mappingCategory] ?? {};
  const folder = selectedFolder();
  if (mappingScope === "folder" && folder) return mappings.folders?.[folder] ?? {};
  return mappings.global ?? {};
}

function invalidateValidation() {
  validation = null;
  acknowledgeWarnings = false;
  confirmCommit = false;
  commitRequestId = "";
}

function objectDecision(object: StagedObject): ImportObjectDecision {
  return decisions[object.id] ?? { kind: "create" };
}

function existingTargetId(object: StagedObject): string {
  const decision = objectDecision(object);
  return decision.kind === "map_to_existing" ? decision.entity_id : "";
}

function setObjectAction(object: StagedObject, kind: ImportObjectDecision["kind"]) {
  let decision: ImportObjectDecision;
  if (kind === "map_to_existing") {
    const target = entities[0];
    if (!target) return;
    decision = { kind, entity_id: target.id, expected_revision: target.revision };
  } else {
    decision = { kind };
  }
  decisions = { ...decisions, [object.id]: decision };
  invalidateValidation();
}

function setExistingTarget(object: StagedObject, entityId: string) {
  const target = entities.find((entity) => entity.id === entityId);
  if (!target) return;
  decisions = {
    ...decisions,
    [object.id]: {
      kind: "map_to_existing",
      entity_id: target.id,
      expected_revision: target.revision,
    },
  };
  invalidateValidation();
}

function replaceCurrentDecision(decision: ImportMappingDecision) {
  const object = selectedObject();
  const folder = selectedFolder();
  if (mappingScope === "item" && object) {
    mappings = { ...mappings, items: { ...(mappings.items ?? {}), [object.id]: decision } };
  } else if (mappingScope === "category" && mappingCategory) {
    mappings = {
      ...mappings,
      categories: { ...(mappings.categories ?? {}), [mappingCategory]: decision },
    };
  } else if (mappingScope === "folder" && folder) {
    mappings = { ...mappings, folders: { ...(mappings.folders ?? {}), [folder]: decision } };
  } else {
    mappings = { ...mappings, global: decision };
  }
  invalidateValidation();
  schedulePlanRefresh();
}

function setEntityType(entityType: string) {
  replaceCurrentDecision({ ...currentDecision(), entityType: entityType || null });
}

function setFieldMapping(sourceKey: string, target: string, relationship = false) {
  const decision = currentDecision();
  const property = relationship ? "relationshipMappings" : "fieldMappings";
  const next = { ...(decision[property] ?? {}) };
  if (target) next[sourceKey] = target;
  else delete next[sourceKey];
  replaceCurrentDecision({ ...decision, [property]: next });
}

function effectiveEntityType(object: StagedObject | null): string {
  if (!object) return "";
  return plan?.objects.find((candidate) => candidate.stagedObjectId === object.id)?.mapping.entityType ?? "";
}

function fieldsForEntity(entityType: string): ImportFieldChoice[] {
  return catalog().fields.filter((choice) => {
    const scopes = choice.definition.entityTypes ?? [];
    return scopes.length === 0 || scopes.includes(entityType);
  });
}

function schedulePlanRefresh() {
  if (planTimer !== null) window.clearTimeout(planTimer);
  planTimer = window.setTimeout(() => void refreshPlan(), 180);
}

async function refreshPlan() {
  if (status?.state !== "ready") return;
  const request = ++planRequest;
  planLoading = true;
  try {
    const next = await project.externalImportCandidatePlan(status.sessionId, catalog().fingerprint, mappings);
    if (request === planRequest) plan = next;
  } catch (cause) {
    if (request === planRequest) error = displayError(cause);
  } finally {
    if (request === planRequest) planLoading = false;
  }
}

async function validatePlan() {
  if (status?.state !== "ready") return;
  validating = true;
  error = "";
  confirmCommit = false;
  try {
    validation = await project.externalImportValidate(status.sessionId, mappings, decisions);
    commitRequestId = validation.validationId ? crypto.randomUUID() : "";
  } catch (cause) {
    validation = null;
    error = displayError(cause);
  } finally {
    validating = false;
  }
}

async function commitPlan() {
  if (!status || !validation?.validationId) return;
  committing = true;
  error = "";
  try {
    report = await project.externalImportCommit(
      status.sessionId,
      validation.validationId,
      acknowledgeWarnings,
      commitRequestId,
    );
    await onCommitted(report);
    confirmCommit = false;
  } catch (cause) {
    error = displayError(cause);
  } finally {
    committing = false;
  }
}

async function loadPage(offset: number) {
  if (status?.state !== "ready") return;
  previewLoading = true;
  error = "";
  try {
    page = await project.externalImportAnalysisPage(status.sessionId, offset, pageSize);
    pageOffset = offset;
    selectedItemIndex = 0;
    mappingScope = "global";
  } catch (cause) {
    error = displayError(cause);
  } finally {
    previewLoading = false;
  }
}

async function preparePreview() {
  if (previewLoading || page) return;
  await loadPage(0);
  await refreshPlan();
}

function applyStatus(next: ExternalImportAnalysisStatus) {
  if (status && next.sessionId !== status.sessionId) return;
  status = next;
  if (next.state === "ready") void preparePreview();
  if (next.state === "failed") error = next.error ? displayError(next.error) : $ui("Analysis failed.");
}

async function pollStatus() {
  if (!status || !["queued", "analyzing"].includes(status.state)) return;
  try {
    applyStatus(await project.externalImportAnalysisStatus(status.sessionId));
  } catch (cause) {
    error = displayError(cause);
  }
}

async function chooseAndAnalyze() {
  const importer = activeImporter();
  if (!importer || !importer.sourceKinds.includes(sourceKind)) return;
  busy = true;
  error = "";
  page = null;
  plan = null;
  mappings = { global: {}, categories: {}, folders: {}, items: {} };
  decisions = {};
  validation = null;
  report = null;
  try {
    const source = await project.externalImportSelectSource(
      sourceKind,
      sourceKind === "file" ? importer.extensions : undefined,
    );
    if (!source) return;
    sourceName = source.displayName;
    applyStatus(
      await project.externalImportAnalyzeBegin(source.sourceHandle, importer.id, undefined, importer.pluginId),
    );
  } catch (cause) {
    error = displayError(cause);
  } finally {
    busy = false;
  }
}

async function cancelAnalysis() {
  if (!status) return;
  busy = true;
  try {
    applyStatus(await project.externalImportAnalysisCancel(status.sessionId));
    page = null;
    plan = null;
    validation = null;
  } catch (cause) {
    error = displayError(cause);
  } finally {
    busy = false;
  }
}

async function startOver() {
  if (status && status.state !== "cancelled") await cancelAnalysis();
  status = null;
  sourceName = "";
  page = null;
  plan = null;
  decisions = {};
  validation = null;
  report = null;
  error = "";
}

async function closeDialog() {
  if (status && !["cancelled", "failed"].includes(status.state)) {
    try {
      await project.externalImportAnalysisCancel(status.sessionId);
    } catch {}
  }
  onClose();
}

function focusableElements(): HTMLElement[] {
  return Array.from(
    dialogElement?.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])',
    ) ?? [],
  );
}

onMount(() => {
  let disposed = false;
  lastFocused = document.activeElement;
  void tick().then(() => focusableElements()[0]?.focus());
  void project
    .externalImporters()
    .then((available) => {
      if (disposed) return;
      importers = available;
      importerKey = available[0] ? `${available[0].pluginId ?? ""}:${available[0].id}` : "";
      normalizeSourceKind();
    })
    .catch((cause) => (error = displayError(cause)));
  void listen<ExternalImportAnalysisStatus>(EXTERNAL_IMPORT_PROGRESS_EVENT, (event) => {
    if (!disposed && status && event.payload.sessionId === status.sessionId) {
      applyStatus(event.payload);
    }
  }).then((unlisten) => {
    if (disposed) unlisten();
    else unlistenProgress = unlisten;
  });
  pollTimer = window.setInterval(() => void pollStatus(), 800);
  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.preventDefault();
      void closeDialog();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = focusableElements();
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  window.addEventListener("keydown", handleKeydown, true);
  return () => {
    disposed = true;
    unlistenProgress?.();
    if (pollTimer !== null) window.clearInterval(pollTimer);
    if (planTimer !== null) window.clearTimeout(planTimer);
    window.removeEventListener("keydown", handleKeydown, true);
    if (lastFocused instanceof HTMLElement && lastFocused.isConnected) lastFocused.focus();
  };
});
</script>

<div class="import-backdrop" role="presentation" onclick={() => void closeDialog()}>
  <div
    bind:this={dialogElement}
    class="import-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="external-import-title"
    tabindex="-1"
    onclick={(event) => event.stopPropagation()}
    onkeydown={(event) => event.stopPropagation()}>
    <header class="import-header">
      <div>
        <span class="kicker">{$ui("PROJECT IMPORT")}</span>
        <h2 id="external-import-title">{$ui("Import external material")}</h2>
        <p>{$ui("Analyze, inspect, and map source material before anything is added to this project.")}</p>
      </div>
      <button class="icon-button" type="button" aria-label={$ui("Close import")} onclick={() => void closeDialog()}>
        <X size={17} strokeWidth={1.8} />
      </button>
    </header>

    {#if error}<p class="message error-message" role="alert">{$nativeErrorText(error)}</p>{/if}

    {#if !status}
      <section class="source-step" aria-label={$ui("Choose import source")}>
        <label>
          <span>{$ui("Importer")}</span>
          <select bind:value={importerKey} disabled={busy || importers.length === 0} onchange={normalizeSourceKind}>
            {#each importers as importer (importerKeyOf(importer))}
              <option value={importerKeyOf(importer)}>{importer.pluginId ? $manifestText(importer.pluginId, "importer", importer.id, importer.name) : $ui(importer.name)}</option>
            {/each}
          </select>
          {#if selectedImporter}<small>{selectedImporter.pluginId ? $manifestText(selectedImporter.pluginId, "importerDescription", selectedImporter.id, selectedImporter.description) : $ui(selectedImporter.description)}</small>{/if}
        </label>
        <fieldset>
          <legend>{$ui("Source")}</legend>
          <div class="source-kinds">
            <label class:active={sourceKind === "folder" && importerSupports("folder")}>
              <input type="radio" bind:group={sourceKind} value="folder" disabled={!importerSupports("folder")} />
              <FolderOpen size={19} strokeWidth={1.7} /><span
                ><strong>{$ui("Folder")}</strong><small>{$ui("Analyze supported files recursively")}</small></span>
            </label>
            <label class:active={sourceKind === "file" && importerSupports("file")}>
              <input type="radio" bind:group={sourceKind} value="file" disabled={!importerSupports("file")} />
              <FileText size={19} strokeWidth={1.7} /><span
                ><strong>{$ui("Single file")}</strong><small>{$ui("Documents, ZIP archives, or MediaWiki XML")}</small></span>
            </label>
          </div>
        </fieldset>
        <button
          class="primary-button"
          type="button"
          disabled={busy || !activeImporter() || !importerSupports(sourceKind)}
          onclick={chooseAndAnalyze}>
          {busy ? $ui("Opening…") : $ui("Choose {0}", $ui(sourceKind))}
        </button>
      </section>
    {:else if status.state === "queued" || status.state === "analyzing"}
      <section class="analysis-step" aria-live="polite" aria-busy="true">
        <LoaderCircle class="spinner" size={28} strokeWidth={1.7} />
        <div>
          <span class="kicker">{$ui("ANALYZING")}</span>
          <h3>{sourceName || $ui("Selected source")}</h3>
        </div>
        <div class="progress-grid">
          <div><strong>{status.processedEntries}</strong><span>{$ui("Entries inspected")}</span></div>
          <div><strong>{status.stagedObjectCount}</strong><span>{$ui("Documents staged")}</span></div>
          <div><strong>{status.unsupportedCount}</strong><span>{$ui("Unsupported")}</span></div>
          <div><strong>{Math.round(status.sourceBytes / 1024).toLocaleString()}</strong><span>{$ui("KB read")}</span></div>
        </div>
        {#if status.currentSourcePath}<p class="current-path">{status.currentSourcePath}</p>{/if}
        <button class="secondary-button" type="button" disabled={busy} onclick={cancelAnalysis}>{$ui("Cancel analysis")}</button>
      </section>
    {:else if status.state === "cancelled" || status.state === "failed"}
      <section class="empty-step">
        <AlertTriangle size={28} strokeWidth={1.6} />
        <h3>{status.state === "cancelled" ? $ui("Analysis cancelled") : $ui("Could not analyze this source")}</h3>
        <p>{$ui("No project content was changed.")}</p>
        <button class="primary-button" type="button" onclick={startOver}>{$ui("Choose another source")}</button>
      </section>
    {:else if report}
      <section class="result-step" aria-live="polite">
        <CircleCheck size={32} strokeWidth={1.6} />
        <div>
          <span class="kicker">{$ui("IMPORT COMPLETE")}</span>
          <h3>{$ui("Project content was updated")}</h3>
          <p> {$ui("Created {0}, mapped {1}, and skipped {2} source {3}. Imported {4} {5} and {6} {7}. Converted {8} {9}; reported {10} unsupported {11} and {12} missing {13}.", report.created.length, report.mapped.length, report.skippedSourcePaths.length, report.skippedSourcePaths.length === 1 ? $ui("item") : $ui("items"), report.assets.length, report.assets.length === 1 ? $ui("attachment") : $ui("attachments"), report.relationships.length, report.relationships.length === 1 ? $ui("relationship") : $ui("relationships"), report.fields.length, report.fields.length === 1 ? $ui("field") : $ui("fields"), report.unsupported.length, report.unsupported.length === 1 ? $ui("item") : $ui("items"), report.missingReferences.length, report.missingReferences.length === 1 ? $ui("reference") : $ui("references"))} </p>
        </div>
        {#if report.decisions.length || report.assets.length || report.relationships.length || report.unsupported.length}
          <div class="result-list">
            {#each [...report.created, ...report.mapped] as item}
              <div>
                <strong>{item.sourcePath}</strong><span>{item.entityType ?? $ui("Existing entity")} · {item.entityId}</span>
              </div>
            {/each}
            {#each report.assets as asset}
              <div><strong>{asset.sourcePath}</strong><span>{$ui("Attachment · {0}", asset.filename)}</span></div>
            {/each}
            {#each report.relationships as relationship}
              <div>
                <strong>{relationship.relationshipType}</strong><span
                  >{$ui("Relationship · {0} → {1}", relationship.sourceEntityId, relationship.targetEntityId)}</span>
              </div>
            {/each}
            {#each report.fields as field}
              <div>
                <strong>{field.sourcePath} · {field.sourceKey}</strong><span
                  >{$ui("Field · {0}:{1}", field.namespace, field.key)}</span>
              </div>
            {/each}
            {#each report.decisions.filter((decision) => decision.decision === "skip") as decision}
              <div><strong>{decision.sourcePath}</strong><span>{$ui("Decision · skipped")}</span></div>
            {/each}
            {#each report.unsupported as item}
              <div><strong>{item.source_path}</strong><span>{$ui("Unsupported · {0}", $nativeErrorText(item.reason))}</span></div>
            {/each}
            {#each report.missingReferences as reference}
              <div>
                <strong>{reference.sourcePath}</strong><span>{$ui("Missing {0} · {1}", reference.kind, reference.target)}</span>
              </div>
            {/each}
            {#each report.diagnostics as diagnostic}
              <div><strong>{diagnostic.code}</strong><span>{$nativeErrorText(diagnostic.message)}</span></div>
            {/each}
          </div>
        {/if}
        {#if report.warnings.length}
          <div class="message warning-message">
            <AlertTriangle size={15} strokeWidth={1.8} />
            <span
              >{$ui("Imported with {0} acknowledged warning{1}.", report.warnings.length, report.warnings.length === 1
                ? ""
                : $ui("s"))}</span>
          </div>
        {/if}
      </section>
    {:else if status.result}
      <div class="preview-shell">
        <section class="summary-row" aria-label={$ui("Analysis summary")}>
          <div><strong>{status.result.summary.document_count}</strong><span>{$ui("Documents")}</span></div>
          <div><strong>{status.result.summary.folder_count}</strong><span>{$ui("Folders")}</span></div>
          <div><strong>{status.result.summary.asset_count}</strong><span>{$ui("Assets")}</span></div>
          <div><strong>{status.result.summary.unsupported_count}</strong><span>{$ui("Unsupported")}</span></div>
          <div>
            <strong>{status.result.summary.warning_count + status.result.summary.error_count}</strong><span
              >{$ui("Diagnostics")}</span>
          </div>
          <div class:needs-attention={(plan?.unresolvedDecisionCount ?? 0) > 0}>
            <strong>{plan?.unresolvedDecisionCount ?? "—"}</strong><span>{$ui("Unresolved mappings")}</span>
          </div>
        </section>

        {#if catalog().entityTypes.length === 0}
          <p class="message error-message"> {$ui("No enabled plugin currently contributes an entity type. Enable a plugin before mapping imported items.")} </p>
        {/if}
        {#if plan?.issues.length}<div class="message warning-message" role="status">
            <AlertTriangle size={15} strokeWidth={1.8} />
            <span>{plan.issues[0].message}</span>
          </div>{/if}
        {#if validation}
          <section class:invalid={validation.errorCount > 0} class="validation-panel" aria-live="polite">
            <div>
              <strong>{validation.errorCount > 0 ? $ui("Resolve validation errors") : $ui("Plan validated")}</strong>
              <span> {$ui("{0} create · {1} map · {2} skip · {3} assets · {4} relationships · {5} warnings", validation.createCount, validation.mapCount, validation.skipCount, validation.assetCount, validation.relationshipCount, validation.warningCount)} </span>
            </div>
            {#if validation.issues.length}
              <ul>
                {#each validation.issues as issue}
                  <li class={issue.severity}><strong>{issue.code}</strong> {$nativeErrorText(issue.message)}</li>
                {/each}
              </ul>
            {/if}
            {#if confirmCommit && validation.validationId}
              <div class="commit-confirmation">
                <strong>{$ui("Commit this validated plan?")}</strong>
                <span>{$ui("The changes are applied as one atomic project mutation.")}</span>
                {#if validation.warningCount > 0}
                  <label>
                    <input type="checkbox" bind:checked={acknowledgeWarnings} /> {$ui("I reviewed and accept the {0} warning{1}.", validation.warningCount, validation.warningCount === 1
                      ? ""
                      : $ui("s"))} </label>
                {/if}
              </div>
            {/if}
          </section>
        {/if}

        <div class="preview-grid">
          <section class="item-list" aria-label={$ui("Staged items")}>
            <div class="panel-heading">
              <div><span class="kicker">{$ui("STAGED ITEMS")}</span><strong>{$ui("{0} total", page?.totalItems ?? 0)}</strong></div>
              {#if previewLoading}<LoaderCircle class="spinner" size={16} />{/if}
            </div>
            <div class="item-scroll">
              {#each page?.items ?? [] as item, index (pageItemKey(item, index))}
                <button class:active={selectedItemIndex === index} type="button" onclick={() => selectPageItem(index)}>
                  <span class={`item-kind ${item.kind}`}>{item.kind.slice(0, 1).toUpperCase()}</span>
                  <span><strong>{pageItemTitle(item)}</strong><small>{pageItemSubtitle(item)}</small></span>
                </button>
              {/each}
            </div>
            <div class="pager">
              <button
                type="button"
                disabled={pageOffset === 0 || previewLoading}
                onclick={() => loadPage(Math.max(0, pageOffset - pageSize))}>{$ui("Previous")}</button>
              <span>{pageOffset + 1}–{Math.min(pageOffset + pageSize, page?.totalItems ?? 0)}</span>
              <button
                type="button"
                disabled={!page || pageOffset + pageSize >= page.totalItems || previewLoading}
                onclick={() => loadPage(pageOffset + pageSize)}>{$ui("Next")}</button>
            </div>
          </section>

          <section class="item-inspector" aria-label={$ui("Selected staged item")}>
            {#if inspectedItem?.kind === "object"}
              {@const object = selectedObject()!}
              <div class="inspector-heading">
                <div>
                  <span class="kicker">{$ui("DOCUMENT")}</span>
                  <h3>{object.title}</h3>
                  <small>{object.source_path}</small>
                </div>
                <span class="format-badge">{stagedDocumentFormat(object)}</span>
              </div>
              <div class="decision-card">
                <label>
                  <span>{$ui("Import action")}</span>
                  <select
                    value={decisions[object.id]?.kind ?? ""}
                    onchange={(event) => {
                      const kind = event.currentTarget.value as ImportObjectDecision["kind"];
                      if (kind) setObjectAction(object, kind);
                    }}>
                    <option value="">{$ui("Create (default)")}</option>
                    <option value="create">{$ui("Create new entity")}</option>
                    <option value="skip">{$ui("Skip this item")}</option>
                    <option value="map_to_existing" disabled={entities.length === 0}>{$ui("Map to existing entity")}</option>
                  </select>
                </label>
                {#if objectDecision(object).kind === "map_to_existing"}
                  <label>
                    <span>{$ui("Existing entity")}</span>
                    <select
                      value={existingTargetId(object)}
                      onchange={(event) => setExistingTarget(object, event.currentTarget.value)}>
                      {#each entities.filter((entity) => !entity.deleted) as entity}
                        <option value={entity.id}>{entity.name} · {entity.entity_type}</option>
                      {/each}
                    </select>
                  </label>
                {/if}
              </div>
              {#if objectDecision(object).kind === "create"}
                <div class="mapping-card">
                  <div class="mapping-heading">
                    <div>
                      <span class="kicker">{$ui("MAPPING")}</span><strong
                        >{planLoading
                          ? $ui("Updating candidate plan…")
                          : effectiveEntityType(object) || $ui("Needs a type")}</strong>
                    </div>
                    {#if !planLoading && effectiveEntityType(object)}<CircleCheck size={17} strokeWidth={1.8} />{/if}
                  </div>
                  <div class="scope-tabs" role="group" aria-label={$ui("Mapping override scope")}>
                    <button
                      class:active={mappingScope === "global"}
                      type="button"
                      onclick={() => (mappingScope = "global")}>{$ui("All items")}</button>
                    <button
                      class:active={mappingScope === "category"}
                      type="button"
                      disabled={selectedCategories().length === 0}
                      onclick={() => {
                        mappingCategory = mappingCategory || selectedCategories()[0] || "";
                        mappingScope = "category";
                      }}>{$ui("Category")}</button>
                    <button
                      class:active={mappingScope === "folder"}
                      type="button"
                      disabled={!selectedFolder()}
                      onclick={() => (mappingScope = "folder")}>{$ui("Folder")}</button>
                    <button class:active={mappingScope === "item"} type="button" onclick={() => (mappingScope = "item")}
                      >{$ui("This item")}</button>
                  </div>
                  {#if mappingScope === "category" && selectedCategories().length > 0}
                    <label>
                      <span>{$ui("Source category")}</span>
                      <select bind:value={mappingCategory}>
                        {#each selectedCategories() as category}<option value={category}>{category}</option>{/each}
                      </select>
                    </label>
                  {/if}
                  <label
                    ><span>{$ui("Entity type")}</span><select
                      value={currentDecision().entityType ?? ""}
                      onchange={(event) => setEntityType(event.currentTarget.value)}>
                      <option value=""
                        >{mappingScope === "global" ? $ui("Choose a type") : $ui("Inherit broader mapping")}</option>
                      {#each catalog().entityTypes as choice}<option value={choice.id}
                          >{$manifestText(choice.moduleId, "module", choice.moduleId, choice.moduleName)} · {$manifestText(choice.moduleId, "type", choice.id, choice.label)}</option
                        >{/each}
                    </select></label>
                  {#if Object.keys(object.fields ?? {}).length > 0}
                    <div class="mapping-fields">
                      <span>{$ui("Source fields")}</span>{#each Object.keys(object.fields ?? {}).sort() as sourceKey}<label
                          ><span class="source-field"
                            ><code>{sourceKey}</code><small>{previewValue(object.fields?.[sourceKey])}</small></span
                          ><select
                            value={currentDecision().fieldMappings?.[sourceKey] ?? ""}
                            onchange={(event) => setFieldMapping(sourceKey, event.currentTarget.value)}
                            ><option value=""
                              >{mappingScope === "global"
                                ? $ui("Preserve as unmapped metadata")
                                : $ui("Inherit broader mapping")}</option
                            >{#each fieldsForEntity(effectiveEntityType(object)) as choice}<option value={choice.id}
                                >{$manifestText(choice.moduleId, "module", choice.moduleId, choice.moduleName)} · {$manifestText(choice.moduleId, "field", choice.key, choice.label)}</option
                              >{/each}</select
                          ></label
                        >{/each}
                    </div>
                  {/if}
                  {#if relationshipSources(object).length > 0}
                    <div class="mapping-fields">
                      <span>{$ui("Resolved source links")}</span>{#each relationshipSources(object) as sourceKey}<label
                          ><code>{sourceKey}</code><select
                            value={currentDecision().relationshipMappings?.[sourceKey] ?? ""}
                            onchange={(event) => setFieldMapping(sourceKey, event.currentTarget.value, true)}
                            ><option value=""
                              >{mappingScope === "global" ? $ui("Leave unresolved") : $ui("Inherit broader mapping")}</option
                            >{#each catalog().relationships as choice}<option value={choice.id}
                                >{$manifestText(choice.moduleId, "module", choice.moduleId, choice.moduleName)} · {$manifestText(choice.moduleId, "field", choice.key, choice.label)}</option
                              >{/each}</select
                          ></label
                        >{/each}
                    </div>
                  {/if}
                  {#if catalog().relationships.length === 0 && relationshipSources(object).length > 0}
                    <small
                      >{$ui("No enabled relationship type without required metadata can accept resolved source links.")}</small>
                  {/if}
                  {#if mappingScope === "folder" && selectedFolder()}<small
                      >{$ui("Overrides items under")} <code>{selectedFolder()}</code>.</small
                    >{/if}
                  {#if mappingScope === "category" && mappingCategory}<small
                      >{$ui("Overrides items tagged with source category")} <code>{mappingCategory}</code>.</small
                    >{/if}
                </div>
              {/if}
              {#if (object.aliases?.length ?? 0) > 0 || (object.tags?.length ?? 0) > 0 || Object.keys(object.metadata ?? {}).length > 0}
                <div class="source-context">
                  <span class="kicker">{$ui("SOURCE CONTEXT")}</span>
                  {#if object.aliases?.length}<p><strong>{$ui("Aliases")}</strong>{object.aliases.join(", ")}</p>{/if}
                  {#if object.tags?.length}<p><strong>{$ui("Tags / categories")}</strong>{object.tags.join(", ")}</p>{/if}
                  {#each Object.entries(object.metadata ?? {}).sort( ([left], [right]) => left.localeCompare(right) ) as [key, value]}
                    <p><strong>{key}</strong>{previewValue(value)}</p>
                  {/each}
                </div>
              {/if}
              <div class="document-preview">
                <span class="kicker">{$ui("SOURCE PREVIEW")}</span>
                <pre>{object.body?.body ?? $ui("No document body.")}</pre>
              </div>
              {#if (object.links ?? []).length}
                <div class="diagnostics">
                  <span class="kicker">{$ui("DISCOVERED LINKS")}</span>
                  {#each object.links ?? [] as link}
                    <p class={link.resolution === "missing" ? "error" : ""}>
                      <strong>{link.kind}</strong>{link.target} · {link.resolution}
                      {#if link.resolved_object_id}<small>{$ui("target {0}", link.resolved_object_id.slice(0, 12))}</small>{/if}
                    </p>
                  {/each}
                </div>
              {/if}
              {#if object.diagnostics?.length}<div class="diagnostics">
                  <span class="kicker">{$ui("DIAGNOSTICS")}</span>{#each object.diagnostics as diagnostic}<p
                      class={diagnostic.severity}>
                      <strong>{diagnostic.code}</strong>{$nativeErrorText(diagnostic.message)}
                    </p>{/each}
                </div>{/if}
              {#if Object.keys(object.raw_source_data ?? {}).length > 0}<details>
                  <summary>{$ui("Raw source data")}</summary>
                  <pre>{JSON.stringify(object.raw_source_data, null, 2)}</pre>
                </details>{/if}
            {:else if inspectedItem?.kind === "unsupported"}
              <div class="unsupported-detail">
                <AlertTriangle size={24} strokeWidth={1.7} /><span class="kicker">{$ui("UNSUPPORTED SOURCE DATA")}</span>
                <h3>{inspectedItem.value.source_path}</h3>
                <p>{inspectedItem.value.reason}</p>
                <pre>{JSON.stringify(inspectedItem.value.raw_metadata ?? {}, null, 2)}</pre>
              </div>
            {:else if inspectedItem?.kind === "diagnostic"}
              <div class="unsupported-detail">
                <AlertTriangle size={24} strokeWidth={1.7} /><span class="kicker">{inspectedItem.value.severity}</span>
                <h3>{inspectedItem.value.code}</h3>
                <p>{inspectedItem.value.message}</p>
              </div>
            {:else if inspectedItem?.kind === "asset"}
              <div class="unsupported-detail">
                <FileText size={24} strokeWidth={1.7} /><span class="kicker">{$ui("ASSET")}</span>
                <h3>{inspectedItem.value.filename}</h3>
                <p> {$ui("{0} · {1} bytes · {2}", inspectedItem.value.source_path, inspectedItem.value.size.toLocaleString(), inspectedItem.value.mime_type ?? $ui("unknown media type"))} </p>
                <pre>{JSON.stringify(
                    {
                      contentHash: inspectedItem.value.content_hash,
                      ownerObjectId: inspectedItem.value.owner_object_id,
                      relationship: inspectedItem.value.relationship,
                    },
                    null,
                    2,
                  )}</pre>
              </div>
            {:else}<div class="empty-inspector">{$ui("Select an item to inspect it.")}</div>{/if}
          </section>
        </div>
      </div>
    {/if}

    <footer class="import-footer">
      <div>
        <strong>{report ? $ui("Import committed") : $ui("No project changes yet")}</strong><span
          >{report
            ? $ui("Request {0}", report.requestId.slice(0, 12))
            : plan
              ? $ui("Candidate {0}", plan.planId.slice(7, 19))
              : $ui("Analysis and mapping are preview-only.")}</span>
      </div>
      <div>
        {#if status?.state === "ready" && !report}
          <button class="secondary-button" type="button" disabled={validating || committing} onclick={startOver}
            >{$ui("Start over")}</button>
          <button class="secondary-button" type="button" disabled={validating || committing} onclick={validatePlan}
            >{validating ? $ui("Validating…") : $ui("Validate plan")}</button>
          {#if validation?.validationId && validation.errorCount === 0 && !confirmCommit}
            <button class="primary-button" type="button" onclick={() => (confirmCommit = true)}>{$ui("Review commit")}</button>
          {:else if validation?.validationId && validation.errorCount === 0 && confirmCommit}
            <button
              class="primary-button"
              type="button"
              disabled={committing || (validation.warningCount > 0 && !acknowledgeWarnings)}
              onclick={commitPlan}>{committing ? $ui("Committing…") : $ui("Commit import")}</button>
          {/if}
        {/if}
        <button class={report ? "primary-button" : "secondary-button"} type="button" onclick={() => void closeDialog()}
          >{$ui("Close")}</button>
      </div>
    </footer>
  </div>
</div>

<style>
.import-backdrop {
  position: fixed;
  z-index: 95;
  inset: 0;
  padding: 18px;
  background: rgba(37, 37, 31, 0.42);
  display: grid;
  place-items: center;
}
.import-dialog {
  width: min(1180px, 100%);
  height: min(820px, calc(100vh - 36px));
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 15px;
  background: var(--surface);
  box-shadow: 0 28px 80px rgba(30, 34, 27, 0.3);
  outline: none;
}
.import-header,
.import-footer {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  padding: 18px 22px;
  border-bottom: 1px solid var(--line);
}
.import-header h2 {
  margin: 3px 0 0;
  font: 700 22px/1.2 var(--font-display, Georgia, serif);
  color: var(--ink);
}
.import-header p {
  margin: 5px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
}
.kicker {
  display: block;
  color: var(--accent);
  font-size: 9px;
  font-weight: 750;
  letter-spacing: 0.09em;
  text-transform: uppercase;
}
.icon-button {
  width: 31px;
  height: 31px;
  display: grid;
  place-items: center;
  flex: none;
  border: 0;
  border-radius: 8px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  cursor: pointer;
}
.source-step,
.analysis-step,
.empty-step {
  width: min(620px, calc(100% - 40px));
  margin: auto;
  display: grid;
  gap: 20px;
  padding: 30px;
}
.source-step label,
.mapping-card label,
.decision-card label,
.commit-confirmation label {
  display: grid;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 650;
}
.source-step select,
.mapping-card select,
.decision-card select {
  min-height: 39px;
  width: 100%;
  padding: 7px 10px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
  color: var(--ink);
}
.source-step small,
.mapping-card small {
  color: var(--ink-faint);
  font-size: 11px;
}
.source-step fieldset {
  padding: 0;
  border: 0;
}
.source-step legend {
  margin-bottom: 8px;
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 650;
}
.source-kinds {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.source-kinds label {
  grid-template-columns: 20px 24px 1fr;
  align-items: center;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--canvas);
  cursor: pointer;
}
.source-kinds label.active {
  border-color: var(--theme-warning-border, #9e7550);
  box-shadow: 0 0 0 2px rgba(180, 119, 63, 0.1);
}
.source-kinds label span {
  display: grid;
  gap: 2px;
}
.source-kinds input {
  accent-color: var(--accent-dark);
}
.primary-button,
.secondary-button,
.pager button,
.scope-tabs button {
  min-height: 36px;
  padding: 8px 13px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.primary-button {
  border: 1px solid var(--accent-dark);
  background: var(--accent-dark);
  color: #fff;
}
.secondary-button,
.pager button,
.scope-tabs button {
  border: 1px solid var(--line);
  background: transparent;
  color: var(--ink-soft);
}
button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.analysis-step,
.empty-step {
  place-items: center;
  text-align: center;
}
.analysis-step h3,
.empty-step h3 {
  margin: 4px 0;
  color: var(--ink);
}
.analysis-step p,
.empty-step p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 12px;
}
.spinner {
  animation: spin 1s linear infinite;
}
.progress-grid,
.summary-row {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 8px;
  width: 100%;
}
.progress-grid {
  grid-template-columns: repeat(4, 1fr);
}
.progress-grid div,
.summary-row div {
  display: grid;
  gap: 3px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--canvas);
}
.progress-grid strong,
.summary-row strong {
  font: 700 18px/1 var(--font-display, Georgia, serif);
}
.progress-grid span,
.summary-row span {
  color: var(--ink-soft);
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.current-path {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.message {
  margin: 10px 22px 0;
  padding: 9px 11px;
  border-radius: 8px;
  font-size: 11px;
  line-height: 1.4;
}
.error-message {
  background: var(--theme-danger-bg, #fbeae5);
  color: var(--theme-danger-text, #913f2b);
}
.warning-message {
  display: flex;
  align-items: center;
  gap: 8px;
  background: var(--theme-warning-bg, #fff2d8);
  color: var(--theme-warning-text, #79571e);
}
.validation-panel {
  display: grid;
  gap: 8px;
  margin-bottom: 10px;
  padding: 10px 12px;
  border: 1px solid var(--theme-success-border, #a8c4b3);
  border-radius: 9px;
  background: var(--theme-success-bg, #eef6f1);
  color: var(--accent-dark);
  font-size: 11px;
}
.validation-panel.invalid {
  border-color: var(--theme-danger-border, #dfb3a6);
  background: var(--theme-danger-bg, #fbeae5);
  color: var(--theme-danger-text, #913f2b);
}
.validation-panel > div:first-child,
.commit-confirmation {
  display: grid;
  gap: 3px;
}
.validation-panel ul {
  max-height: 110px;
  margin: 0;
  padding-left: 18px;
  overflow: auto;
}
.validation-panel li.warning {
  color: var(--theme-warning-text, #79571e);
}
.commit-confirmation {
  padding-top: 8px;
  border-top: 1px solid currentColor;
}
.commit-confirmation label {
  display: flex;
  align-items: center;
  font-weight: 600;
}
.result-step {
  width: min(720px, calc(100% - 40px));
  min-height: 0;
  margin: auto;
  display: grid;
  justify-items: center;
  gap: 16px;
  padding: 30px;
  text-align: center;
  color: var(--accent-dark);
}
.result-step h3,
.result-step p {
  margin: 4px 0;
}
.result-step p,
.result-list span {
  color: var(--ink-soft);
  font-size: 11px;
}
.result-list {
  width: 100%;
  max-height: 300px;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 9px;
  text-align: left;
}
.result-list div {
  display: grid;
  gap: 2px;
  padding: 9px 11px;
  border-bottom: 1px solid var(--line);
}
.preview-shell {
  min-height: 0;
  display: flex;
  flex: 1;
  flex-direction: column;
  padding: 12px 16px 0;
}
.summary-row {
  margin-bottom: 10px;
}
.summary-row div {
  padding: 9px 11px;
}
.summary-row .needs-attention {
  border-color: var(--theme-warning-border, #d4a14f);
  background: var(--theme-warning-bg, #fff8e9);
}
.preview-grid {
  min-height: 0;
  display: grid;
  grid-template-columns: minmax(245px, 31%) 1fr;
  flex: 1;
  border: 1px solid var(--line);
  border-radius: 11px 11px 0 0;
  overflow: hidden;
}
.item-list {
  min-height: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--line);
  background: var(--canvas);
}
.panel-heading,
.inspector-heading,
.mapping-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.panel-heading {
  padding: 12px;
  border-bottom: 1px solid var(--line);
}
.panel-heading > div {
  display: grid;
  gap: 3px;
}
.item-scroll {
  min-height: 0;
  overflow: auto;
  flex: 1;
}
.item-scroll > button {
  width: 100%;
  display: grid;
  grid-template-columns: 25px 1fr;
  gap: 9px;
  padding: 10px 12px;
  border: 0;
  border-bottom: 1px solid var(--line);
  background: transparent;
  text-align: left;
  cursor: pointer;
}
.item-scroll > button.active {
  background: var(--theme-surface-bg, #fff);
  border-left: 3px solid var(--accent);
  padding-left: 9px;
}
.item-scroll > button > span:last-child {
  min-width: 0;
  display: grid;
  gap: 3px;
}
.item-scroll strong,
.item-scroll small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.item-scroll strong {
  color: var(--ink);
  font-size: 12px;
}
.item-scroll small {
  color: var(--ink-faint);
  font-size: 10px;
}
.item-kind {
  width: 23px;
  height: 23px;
  display: grid;
  place-items: center;
  border-radius: 6px;
  background: var(--theme-warning-bg, #e9e6de);
  color: var(--theme-neutral-text-soft, #68675f);
  font-size: 9px;
  font-weight: 800;
}
.item-kind.object {
  background: var(--theme-success-bg, #e4eee8);
  color: var(--accent-dark);
}
.item-kind.unsupported,
.item-kind.diagnostic {
  background: var(--theme-warning-bg, #f9e7d8);
  color: var(--theme-danger-text, #9b5738);
}
.pager {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 5px;
  padding: 8px;
  border-top: 1px solid var(--line);
}
.pager button {
  min-height: 30px;
  padding: 5px 8px;
}
.pager span {
  font-size: 10px;
  color: var(--ink-soft);
}
.item-inspector {
  min-width: 0;
  overflow: auto;
  padding: 16px 18px;
}
.inspector-heading h3 {
  margin: 3px 0;
  font: 700 19px/1.2 var(--font-display, Georgia, serif);
}
.inspector-heading small {
  color: var(--ink-faint);
  font-size: 10px;
}
.format-badge {
  padding: 5px 8px;
  border-radius: 6px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 10px;
}
.mapping-card {
  display: grid;
  gap: 12px;
  margin-top: 14px;
  padding: 13px;
  border: 1px solid var(--theme-warning-border, #ded8ca);
  border-radius: 10px;
  background: var(--theme-warning-bg, #faf8f2);
}
.decision-card {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  margin-top: 14px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--canvas);
}
.mapping-heading strong {
  font-size: 12px;
}
.scope-tabs {
  display: flex;
  gap: 5px;
}
.scope-tabs button {
  min-height: 29px;
  padding: 5px 9px;
}
.scope-tabs button.active {
  border-color: var(--accent-dark);
  background: var(--accent-dark);
  color: #fff;
}
.mapping-fields {
  display: grid;
  gap: 7px;
}
.mapping-fields > span {
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
}
.mapping-fields label {
  grid-template-columns: minmax(90px, 1fr) 2fr;
  align-items: center;
}
.mapping-fields code,
.mapping-card small code {
  font-size: 10px;
}
.source-field {
  min-width: 0;
  display: grid;
  gap: 2px;
}
.source-field small {
  overflow: hidden;
  color: var(--ink-faint);
  font-size: 9px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.document-preview,
.diagnostics,
.source-context {
  display: grid;
  gap: 7px;
  margin-top: 14px;
}
.source-context {
  padding: 11px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--canvas);
}
.source-context p {
  display: grid;
  grid-template-columns: minmax(110px, 0.7fr) 2fr;
  gap: 8px;
  margin: 0;
  color: var(--ink-soft);
  font-size: 10px;
}
.source-context strong {
  color: var(--ink);
}
.document-preview pre,
.unsupported-detail pre,
details pre {
  max-height: 280px;
  margin: 0;
  padding: 13px;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--canvas);
  color: var(--ink);
  font:
    12px/1.55 ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
  white-space: pre-wrap;
}
.diagnostics p {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 8px;
  margin: 0;
  padding: 8px;
  border-radius: 7px;
  background: var(--theme-warning-bg, #fff4e4);
  color: var(--theme-warning-text, #775421);
  font-size: 11px;
}
.diagnostics p small {
  grid-column: 2;
  color: inherit;
  opacity: 0.75;
}
.diagnostics p.error,
.diagnostics p.fatal {
  background: var(--theme-danger-bg, #fbeae5);
  color: var(--theme-danger-text, #913f2b);
}
.unsupported-detail,
.empty-inspector {
  min-height: 280px;
  display: grid;
  place-items: start;
  align-content: center;
  justify-items: center;
  text-align: center;
  color: var(--ink-soft);
}
.unsupported-detail h3 {
  margin: 4px 0;
  color: var(--ink);
}
.unsupported-detail p {
  margin: 3px 0 12px;
  font-size: 12px;
}
.unsupported-detail pre {
  width: 100%;
  text-align: left;
}
.import-footer {
  align-items: center;
  border-top: 1px solid var(--line);
  border-bottom: 0;
  padding: 11px 18px;
}
.import-footer > div {
  display: flex;
  align-items: center;
  gap: 8px;
}
.import-footer > div:first-child {
  display: grid;
  gap: 2px;
}
.import-footer strong {
  font-size: 11px;
  color: var(--ink);
}
.import-footer span {
  font-size: 10px;
  color: var(--ink-faint);
}
details {
  margin-top: 12px;
}
details summary {
  cursor: pointer;
  color: var(--ink-soft);
  font-size: 11px;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
@media (max-width: 760px) {
  .import-backdrop {
    padding: 8px;
  }
  .import-dialog {
    height: calc(100vh - 16px);
  }
  .preview-grid {
    grid-template-columns: 1fr;
  }
  .item-list {
    max-height: 240px;
    border-right: 0;
    border-bottom: 1px solid var(--line);
  }
  .summary-row {
    grid-template-columns: repeat(3, 1fr);
  }
  .source-kinds {
    grid-template-columns: 1fr;
  }
  .import-header p {
    display: none;
  }
}
</style>
