import type {
  DaenaModule,
  EntityRecord,
  EntitySummary,
  ModuleContext,
  ModuleManifest,
  ModuleRecord,
  ModuleRecordQuery,
} from "../../../module-api/src/index";
import manifestJson from "../manifest.json";
import {
  emptyLexeme,
  firstGloss,
  lexiconExport,
  normalizeLexeme,
  parseLexiconImport,
  PART_OF_SPEECH_SUGGESTIONS,
  serializeLexeme,
  STATUS_SUGGESTIONS,
  type LexemeValue,
} from "./lexeme";
import { emptyOrthography, normalizeOrthography, serializeOrthography, type OrthographyValue } from "./orthography";
import {
  BACKNESS_SUGGESTIONS,
  consonantChart,
  emptyPhoneme,
  emptyPhonologyNotes,
  HEIGHT_SUGGESTIONS,
  MANNER_SUGGESTIONS,
  normalizePhoneme,
  normalizePhonologyNotes,
  PHONEME_KINDS,
  PLACE_SUGGESTIONS,
  ROUNDING_SUGGESTIONS,
  serializePhoneme,
  serializePhonologyNotes,
  vowelChart,
  VOICING_SUGGESTIONS,
  type PhonemeValue,
  type PhonologyNotes,
} from "./phonology";
import { emptyGrammarUiState, type GrammarUiState } from "./grammar";
import { loadGrammarIndex } from "./grammar/repository";
import { renderGrammarPane, tryLeaveGrammar, type GrammarPaneContext } from "./grammar/pane";
import {
  clearOverride,
  emptyOperation,
  emptyParadigm,
  emptyRule,
  emptySlot,
  normalizeParadigm,
  OPERATION_KINDS,
  PARADIGM_KINDS,
  pinOverride,
  previewParadigm,
  serializeParadigm,
  type MorphOperationKind,
  type Paradigm,
  type ParadigmKind,
} from "./morphology";
import {
  alertMessage,
  button,
  emptyMessage,
  emptyState,
  field,
  groupHead,
  input,
  replaceEditor,
  row,
  textarea,
} from "./ui";
import {
  emptySample,
  emptyToken,
  groupSamples,
  normalizeSample,
  SAMPLE_KINDS,
  samplePreviewHtml,
  sampleTitle,
  serializeSample,
  tokenizeSample,
  type Sample,
  type SampleKind,
} from "./samples";

const manifest = manifestJson as unknown as ModuleManifest;

type Pane = "overview" | "lexicon" | "sounds" | "writing" | "grammar" | "forms" | "samples";

export const language: DaenaModule = {
  manifest,
  views: [
    {
      id: "lexicon",
      title: "Lexicon",
      mount(element: HTMLElement, context: ModuleContext) {
        let cancelled = false;
        let selectedLanguage: EntitySummary | null = null;
        let records: ModuleRecord<LexemeValue>[] = [];
        let editing: ModuleRecord<LexemeValue> | null = null;
        let editorOpen = false;
        let draft: LexemeValue = emptyLexeme();
        let search = "";
        let statusFilter = "";
        let tagFilter = "";
        let sort: ModuleRecordQuery["sort"] = "lemma";
        let homonymsOnly = false;
        let page = 0;
        let hasNextPage = false;
        let homonymCount = 0;
        let request = 0;
        let languageRequest = 0;
        let searchTimer: number | null = null;
        let pane: Pane = "overview";
        let phonemes: ModuleRecord<PhonemeValue>[] = [];
        let phonemeEditing: ModuleRecord<PhonemeValue> | null = null;
        let phonemeEditorOpen = false;
        let phonemeDraft: PhonemeValue = emptyPhoneme();
        let phonologyRecord: ModuleRecord<PhonologyNotes> | null = null;
        let phonologyDraft: PhonologyNotes = emptyPhonologyNotes();
        let phonologyNotesOpen = false;
        let orthographies: ModuleRecord<OrthographyValue>[] = [];
        let orthographyEditing: ModuleRecord<OrthographyValue> | null = null;
        let orthographyEditorOpen = false;
        let orthographyDraft: OrthographyValue = emptyOrthography();
        let grammarUi: GrammarUiState = emptyGrammarUiState();
        let pendingLexemeId: string | null = null;
        let paradigms: ModuleRecord<Paradigm>[] = [];
        let paradigmEditing: ModuleRecord<Paradigm> | null = null;
        let paradigmEditorOpen = false;
        let paradigmDraft: Paradigm = emptyParadigm();
        let previewStem = "";
        let previewLexemeId = "";
        let samples: ModuleRecord<Sample>[] = [];
        let sampleEditing: ModuleRecord<Sample> | null = null;
        let sampleEditorOpen = false;
        let sampleDraft: Sample = emptySample();
        let languageQuery = "";
        let languageSummaries: EntitySummary[] = [];
        let languageListLoaded = false;
        let languageLoading = false;
        let languageLoadError = "";
        let creatingLanguage = false;
        let languageCreateName = "";
        let languageCreateError = "";
        let overviewEntity: EntityRecord | null = null;
        let overviewName = "";
        let overviewFields: Record<string, unknown> = {};
        let overviewSavedFields: Record<string, unknown> = {};
        let overviewFieldRevisions: Record<string, string> = {};
        let overviewDocument = "";
        let overviewSavedDocument = "";
        let overviewDocumentRevision = "";
        let overviewLoading = false;
        let overviewSaving = false;
        let overviewSavingAutomatically = false;
        let overviewDeleting = false;
        let overviewDirty = false;
        let overviewError = "";
        let overviewRequest = 0;
        let overviewAutosaveTimer: number | null = null;
        let overviewAutosaveQueued = false;
        let paneLoading = false;
        let lexiconLoading = false;
        let lexiconSaving = false;
        let focusName = "";
        let focusOffset = 0;

        const root = document.createElement("section");
        root.className = "language-workspace";
        if (context.embedded) root.classList.add("language-workspace-embedded");
        const style = document.createElement("style");
        style.textContent = `
          .language-workspace{display:grid;grid-template-columns:minmax(220px,260px) minmax(0,1fr);gap:18px;height:100%;min-height:0;color:var(--ink)}
          .language-workspace-embedded{grid-template-columns:minmax(0,1fr);height:auto}
          .language-panel{display:flex;flex-direction:column;min-width:0;min-height:0;overflow:auto;border:1px solid var(--line);border-radius:16px;background:var(--surface);padding:22px 20px 24px;box-shadow:var(--shadow-sm,0 2px 8px rgba(38,42,33,.05))}
          .language-sidebar{gap:14px}
          .language-sidebar-head{display:flex;align-items:flex-start;justify-content:space-between;gap:12px}
          .language-sidebar-kicker,.language-toolbar-eyebrow{margin:0 0 5px;color:var(--accent);font-size:10px;font-weight:700;letter-spacing:.12em;text-transform:uppercase}
          .language-sidebar-intro,.language-toolbar-subtitle{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.55}
          .language-sidebar-intro{margin-top:-5px}
          .language-panel h2,.language-panel h3{margin:0;font-family:var(--font-display);font-weight:500}
          .language-panel h2{font-size:24px;line-height:1.15}.language-panel h3{font-size:16px;line-height:1.3}
          .language-list,.lexeme-list{display:grid;gap:8px;margin:4px 0 0;padding:0;list-style:none}
          .language-list button{display:grid;gap:3px;width:100%;padding:11px 12px;border:1px solid #ebe7de;border-radius:10px;background:var(--surface);color:inherit;text-align:left;cursor:pointer;box-shadow:0 1px 2px rgba(38,42,33,.03)}
          .language-list button:hover{border-color:#e5d8c6;background:var(--surface-muted)}
          .language-list button[aria-current=page]{border-color:#d8c3a5;background:var(--surface-muted);box-shadow:inset 3px 0 var(--accent),0 1px 2px rgba(38,42,33,.03);color:var(--ink)}
          .language-list-name{font-weight:600}
          .language-list-meta{color:var(--ink-faint);font-size:11px}
          .language-create{display:grid;gap:10px;padding:14px;border:1px solid var(--line);border-radius:12px;background:var(--surface-muted)}
          .language-create-actions{display:flex;justify-content:flex-end;gap:8px;flex-wrap:wrap}
          .language-toolbar{display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap}
          .language-toolbar-title{display:grid;gap:3px}
          .language-toolbar-title h2{margin:0}
          .language-toolbar-actions{display:flex;flex-wrap:wrap;gap:8px}
          .language-pane-section{display:grid;gap:10px;margin-top:16px;padding:16px;border:1px solid var(--line);border-radius:14px;background:var(--surface-muted)}
          .language-pane-section > p{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.55}
          .language-pane-section .lexeme-list{margin-top:2px}
          .language-sounds-notes{display:block;margin-top:8px;padding:0;border:0;border-top:1px solid var(--line);border-radius:0;background:transparent;overflow:visible}
          .language-sounds-notes summary{display:flex;align-items:center;justify-content:space-between;gap:12px;padding:11px 0;color:var(--ink);cursor:pointer;list-style:none}
          .language-sounds-notes summary::-webkit-details-marker{display:none}
          .language-sounds-notes summary::after{content:"⌄";color:var(--ink-faint);font-size:15px;line-height:1;transition:transform .16s ease}
          .language-sounds-notes[open] summary{border-bottom:1px solid var(--line)}
          .language-sounds-notes[open] summary::after{transform:rotate(180deg)}
          .language-sounds-notes-title{display:flex;align-items:baseline;gap:9px;min-width:0}
          .language-sounds-notes-title strong{font-family:var(--font-display);font-size:15px;font-weight:500}
          .language-sounds-notes-title span,.language-sounds-notes-meta{color:var(--ink-faint);font-size:11px}
          .language-sounds-notes-meta{white-space:nowrap}
          .language-sounds-notes-body{display:grid;gap:10px;padding:12px 0 14px}
          .language-sounds-notes-body > p{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.55}
          .language-sounds-notes-body .language-sounds-notes-content{display:grid;gap:10px;margin-top:0;padding:0}
          .language-sounds-notes-content > p{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.55}
          .language-group.language-sounds-chart{margin-top:4px;padding:14px 0 0;border:0;border-top:1px solid var(--line);border-radius:0;background:transparent}
          .language-sounds-chart-heading{display:flex;align-items:baseline;justify-content:space-between;gap:12px;flex-wrap:wrap}
          .language-sounds-chart-heading h3{font-family:var(--font-sans);font-size:11px;font-weight:700;letter-spacing:.1em;text-transform:uppercase;color:var(--ink-soft)}
          .language-sounds-chart .language-empty{margin:0;color:var(--ink-faint);font-size:12px}
          .language-pane-summary{margin:16px 0 0;color:var(--ink-faint);font-size:11px}
          .language-overview{display:flex;flex:1;flex-direction:column;gap:16px;margin-top:18px;min-width:0;min-height:0}
          .language-overview-identity{display:grid;grid-template-columns:minmax(0,1fr) minmax(180px,.55fr);gap:16px;padding:18px;border:1px solid var(--line);border-radius:14px;background:var(--surface-muted)}
          .language-overview-identity h3{font-size:20px}
          .language-overview-identity p{margin:5px 0 0;color:var(--ink-soft);font-size:12px;line-height:1.55}
          .language-overview-identity-meta{display:grid;align-content:center;justify-items:end;gap:4px;color:var(--ink-soft);font-size:12px;text-align:right}
          .language-overview-identity-meta strong{color:var(--accent-dark);font-size:13px}
          .language-overview-section{display:grid;gap:12px;padding:16px;border:1px solid var(--line);border-radius:14px;background:var(--surface)}
          .language-overview-section h3{font-size:17px}
          .language-overview-section > p{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.55}
          .language-overview-fields{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}
          .language-overview-document{min-height:16rem;resize:vertical;line-height:1.6}
          .language-overview-status{display:flex;align-items:center;gap:8px;min-height:32px;padding:8px 12px;border:1px solid var(--line);border-radius:999px;background:var(--surface-muted);color:var(--ink-soft);font-size:12px;font-weight:600;white-space:nowrap}
          .language-overview-status::before{content:"";width:7px;height:7px;flex:0 0 7px;border-radius:50%;background:currentColor}
          .language-overview-status[data-state=saved]{border-color:#c6d8cb;background:#eef3ef;color:var(--accent-dark)}
          .language-overview-status[data-state=saving]{border-color:#d8c3a5;color:var(--accent-dark)}
          .language-overview-status[data-state=saving]::before{animation:language-pulse 1.2s ease-in-out infinite}
          .language-overview-status[data-state=error]{border-color:#e2b7af;background:#fff5f2;color:#a14f42}
          .language-overview-actions{display:flex;align-items:center;justify-content:flex-end;gap:12px;flex-wrap:wrap;margin:auto -20px -24px;padding:12px 20px 24px;border-top:1px solid var(--line);background:var(--surface);box-shadow:0 -8px 16px -16px rgba(38,42,33,.4)}
          .language-search-row{display:grid;grid-template-columns:minmax(0,1fr);gap:10px;margin-top:16px}
          .language-filter-panel{margin-top:10px;border:1px solid var(--line);border-radius:12px;background:var(--surface-muted)}
          .language-filter-panel summary{padding:10px 12px;color:var(--accent-dark);font-size:12px;font-weight:600;cursor:pointer;list-style-position:inside}
          .language-filter-panel[open] summary{border-bottom:1px solid var(--line)}
          .language-filter-panel .language-filters{margin:0;padding:12px}
          .language-filters{display:grid;grid-template-columns:repeat(3,minmax(110px,1fr));gap:10px 12px;align-items:end}
          .language-filters .language-check{grid-column:1/-1;padding:2px 0 0}
          .language-filter-actions{display:flex;align-items:center;justify-content:space-between;gap:8px;grid-column:1/-1;flex-wrap:wrap}
          .language-search,.language-filters input,.language-filters select,.language-field input,.language-field textarea,.language-field select{box-sizing:border-box;width:100%;min-width:0;padding:9px 10px;border:1px solid var(--line);border-radius:8px;background:var(--surface);color:var(--ink);font:inherit}
          .language-field textarea{min-height:4.5em;resize:vertical}
          .language-check{display:flex;align-items:center;gap:8px;color:var(--ink-soft);font-size:12px}
          .language-tabs{display:flex;flex-wrap:wrap;gap:6px;margin:0 0 8px;padding:0 0 12px;background:var(--surface)}
          .language-tabs button{padding:7px 12px;border:1px solid var(--line);border-radius:999px;background:transparent;color:var(--ink-soft);cursor:pointer}
          .language-tabs button:hover{border-color:#d8c3a5;color:var(--ink);background:var(--surface-muted)}
          .language-tabs button[aria-selected=true]{border-color:var(--accent-dark);background:var(--surface-muted);color:var(--accent-dark)}
          .language-chart-wrap{overflow-x:auto;margin:8px 0 4px}
          .language-chart,.paradigm-preview{width:100%;border-collapse:collapse;font-size:12px}
          .language-chart th,.language-chart td,.paradigm-preview th,.paradigm-preview td{border:1px solid var(--line);padding:8px;text-align:center;min-width:52px}
          .paradigm-preview th,.paradigm-preview td{text-align:left}
          .language-chart th,.paradigm-preview th{background:var(--surface-muted);font-weight:600;color:var(--ink-soft)}
          .language-chart button{border:0;background:transparent;color:inherit;font:inherit;cursor:pointer}
          .language-chart .is-empty{color:var(--ink-faint)}
          .grammar-home{display:grid;gap:16px;margin-top:14px}
          .grammar-cards{display:grid;grid-template-columns:repeat(auto-fill,minmax(180px,1fr));gap:10px}
          .grammar-card,.grammar-system{display:grid;gap:4px;width:100%;padding:12px;border:1px solid #ebe7de;border-radius:10px;background:var(--surface);color:inherit;text-align:left;cursor:pointer}
          .grammar-card:hover,.grammar-system:hover{border-color:#e5d8c6;background:var(--surface-muted)}
          .grammar-card strong,.grammar-system strong{font-size:14px}
          .grammar-card span,.grammar-system span,.grammar-glance dd{color:var(--ink-soft);font-size:12px}
          .grammar-glance{display:grid;grid-template-columns:minmax(8rem,12rem) minmax(0,1fr);gap:6px 14px;margin:0;padding:14px;border:1px solid var(--line);border-radius:10px;background:var(--surface-muted)}
          .grammar-glance dt{margin:0;color:var(--ink-faint);font-size:11px}
          .grammar-glance dd{margin:0}
          .grammar-systems{display:grid;gap:8px}
          .grammar-status{display:flex;gap:14px;flex-wrap:wrap;border:0;margin:0;padding:0}
          .grammar-status legend{padding:0;color:var(--ink-soft);font-size:11px}
          .grammar-help{margin:8px 0 0;font-size:13px;line-height:1.55}
          .grammar-learn{margin:4px 0 8px}
          .grammar-choice-editor,.grammar-choice-stack{display:grid;gap:12px;min-width:0}
          .grammar-choices,.grammar-checks{display:grid;gap:8px;margin:0;padding:0;border:0}
          .grammar-choices{grid-template-columns:repeat(auto-fit,minmax(168px,1fr))}
          .grammar-choices legend,.grammar-checks legend,.grammar-status legend{padding:0;color:var(--ink-soft);font-size:11px}
          .grammar-choice{display:grid;gap:4px;align-content:start;padding:12px;border:1px solid var(--line);border-radius:10px;background:var(--surface);cursor:pointer}
          .grammar-choice.is-selected{border-color:var(--accent-dark);background:var(--surface-muted)}
          .grammar-choice input{margin:0}
          .grammar-choice span,.grammar-choice em{color:var(--ink-soft);font-size:12px;line-height:1.45}
          .grammar-choice em{font-style:italic}
          .grammar-checks{display:flex;flex-wrap:wrap;gap:8px 16px}
          .grammar-checks label{display:grid;gap:2px;align-content:start}
          .grammar-template-hint{color:var(--ink-faint);font-size:11px}
          .grammar-inventory,.grammar-inventory-item{display:grid;gap:10px;min-width:0}
          .grammar-inventory-item{padding:12px;border:1px solid var(--line);border-radius:10px;background:var(--surface)}
          .grammar-inventory-toolbar{display:flex;flex-wrap:wrap;align-items:center;gap:8px}
          .grammar-paradigm{overflow:auto;max-width:100%;max-height:min(70vh,36rem)}
          .grammar-paradigm-table{border-collapse:collapse;min-width:100%}
          .grammar-paradigm-table caption.visually-hidden,.visually-hidden{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0}
          .grammar-paradigm-table th,.grammar-paradigm-table td{border:1px solid var(--line);padding:8px;vertical-align:top;text-align:left;background:var(--surface)}
          .grammar-paradigm-table thead th{position:sticky;top:0;z-index:2;background:var(--surface-muted)}
          .grammar-paradigm-table th[scope="row"]{position:sticky;left:0;z-index:1}
          .grammar-paradigm-table thead th:first-child{z-index:3;left:0}
          .grammar-starter-list{margin:0;padding-left:1.2em}
          .grammar-paradigm-cell{display:grid;gap:6px;min-width:8rem}
          .grammar-diagnostic{display:grid;gap:8px;justify-items:start}
          .grammar-example{display:grid;gap:8px;padding:12px;border:1px solid var(--line);border-radius:10px;background:var(--surface-muted)}
          .sample-block{padding:14px;border:1px solid var(--line);border-radius:10px;background:var(--surface-muted);font-size:13px;line-height:1.55}
          .sample-block h3{margin:0 0 8px;font-family:var(--font-display);font-weight:500}
          .sample-ref{padding:0;border:0;border-bottom:1px dotted var(--accent-dark);background:transparent;color:var(--accent-dark);font:inherit;cursor:pointer}
          .paradigm-preview{margin:12px 0}
          .form-provenance{display:inline-block;padding:2px 7px;border-radius:999px;background:var(--surface);font-size:10px;letter-spacing:.04em;text-transform:uppercase;color:var(--ink-soft)}
          .form-provenance.is-authored{color:var(--accent-dark);background:#eef3ef}
          .form-provenance.is-missing{color:var(--ink-faint)}
          .sample-interlinear{display:flex;flex-wrap:wrap;gap:10px 18px;margin:10px 0}
          .sample-token{display:grid;gap:2px;justify-items:center;text-align:center;padding:6px 8px;border:1px solid var(--line);border-radius:8px;background:var(--surface)}
          .sample-token .surface,.sample-ref{font-weight:600}
          .sample-token .gloss,.sample-token .grammar,.sample-transliteration{color:var(--ink-soft);font-size:11px}
          .sample-translation{margin:8px 0 0;font-style:italic}
          .sample-source{margin:0 0 8px;white-space:pre-wrap}
          .language-item,.lexeme-row{display:grid;grid-template-columns:minmax(0,1.2fr) auto minmax(0,1.4fr);gap:8px 12px;align-items:baseline;width:100%;padding:10px 12px;border:1px solid #ebe7de;border-radius:10px;background:var(--surface);color:inherit;text-align:left;cursor:pointer;box-shadow:0 1px 2px rgba(38,42,33,.03)}
          .lexeme-row{grid-template-columns:minmax(0,1.05fr) minmax(0,.6fr) minmax(0,1.55fr) minmax(0,.7fr);padding:13px 14px}
          .language-item:hover,.lexeme-row:hover{border-color:#e5d8c6;background:var(--surface-muted)}
          .language-item strong,.lexeme-row strong{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
          .language-item small,.lexeme-row small{color:var(--ink-faint)}
          .language-item span,.lexeme-row span{min-width:0;color:var(--ink-soft);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
          .lexeme-meaning{font-size:13px}
          .lexeme-status{justify-self:end;padding:3px 7px;border-radius:999px;background:var(--surface-muted);font-size:10px;letter-spacing:.03em}
          .language-results{margin:14px 0 0;color:var(--ink-faint);font-size:11px}
          .language-button{padding:8px 12px;border:1px solid var(--accent-dark);border-radius:8px;background:var(--accent-dark);color:#fff;cursor:pointer}
          .language-button:hover{filter:brightness(1.06)}
          .language-button.secondary{background:transparent;color:var(--accent-dark)}
          .language-button.secondary:hover{background:var(--surface-muted)}
          .language-button:disabled{opacity:.45;cursor:not-allowed;filter:none}
          .language-button:focus-visible,.language-tabs button:focus-visible,.language-list button:focus-visible,.language-item:focus-visible,.lexeme-row:focus-visible,.grammar-card:focus-visible,.grammar-system:focus-visible,.sample-ref:focus-visible,.grammar-choice:focus-within,.grammar-status input:focus-visible,.grammar-checks input:focus-visible,.grammar-learn summary:focus-visible{outline:3px solid rgba(180,119,63,.24);outline-offset:2px}
          .language-empty,.language-status{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.6}
          .language-status.error{color:#a14f42}
          .language-loading{display:flex;align-items:center;gap:8px;color:var(--ink-soft)}
          .language-loading::before{content:"";width:11px;height:11px;flex:0 0 11px;border:2px solid var(--line);border-top-color:var(--accent);border-radius:50%;animation:language-spin .75s linear infinite}
          @keyframes language-spin{to{transform:rotate(360deg)}}
          @keyframes language-pulse{50%{opacity:.35}}
          @media(prefers-reduced-motion:reduce){.language-loading::before{animation:none}}
          .language-empty-card{display:grid;gap:12px;justify-items:start;margin:18px 0;padding:20px;border:1px dashed var(--line);border-radius:12px;background:var(--surface-muted)}
          .language-editor{display:grid;gap:16px;margin-top:16px;min-width:0}
          .language-editor-head{display:grid;gap:4px;padding-bottom:2px}
          .language-editor-head p{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.55}
          .language-form-section{display:grid;gap:10px;min-width:0;padding:14px;border:1px solid var(--line);border-radius:12px;background:var(--surface-muted)}
          .language-section-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:10px 12px}
          .language-field-wide{grid-column:1/-1}
          .language-field{display:grid;gap:6px;min-width:0;color:var(--ink-soft);font-size:11px;letter-spacing:.01em}
          .language-actions{display:flex;align-items:center;justify-content:space-between;gap:10px;flex-wrap:wrap;margin:0 -20px -24px;padding:12px 20px 24px;border-top:1px solid var(--line);background:var(--surface);box-shadow:0 -8px 16px -16px rgba(38,42,33,.4)}
          .language-actions span{display:flex;gap:8px;flex-wrap:wrap}
          .language-danger{border-color:#a14f42!important;color:#a14f42!important;background:transparent}
          .language-group{display:grid;gap:10px;min-width:0;padding:12px;border:1px solid var(--line);border-radius:10px;background:var(--surface-muted)}
          .language-group .language-group{background:var(--surface)}
          .language-group-head{display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap}
          .language-inline{display:flex;align-items:end;gap:8px;min-width:0}
          .language-inline-fields{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:8px;flex:1;min-width:0}
          .language-inline>.language-button{flex:0 0 auto}
          .file-input{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)}
          @media(max-width:760px){
            .language-workspace{display:flex;flex-direction:column;overflow:auto}
            .language-sidebar{max-height:none}
            .language-filters,.lexeme-row,.language-item,.language-section-grid{grid-template-columns:1fr}
            .language-overview-identity,.language-overview-fields{grid-template-columns:1fr}
            .language-overview-identity-meta{justify-items:start;text-align:left}
            .language-main{min-height:34rem}
            .language-item span,.lexeme-row span,.lexeme-row small{white-space:normal}
            .lexeme-status{justify-self:start}
            .language-inline{flex-direction:column;align-items:stretch}
            .language-tabs{flex-wrap:nowrap;overflow-x:auto;overscroll-behavior-inline:contain;padding-bottom:10px;scrollbar-width:thin}
            .language-tabs button{flex:0 0 auto}
          }
          `;

        function overviewFieldDefinitions() {
          return manifest.schemas.flatMap((schema) => schema.fields).filter((field) => !field.relationshipType);
        }

        function clearOverviewAutosave() {
          if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
          overviewAutosaveTimer = null;
          overviewAutosaveQueued = false;
        }

        function scheduleOverviewAutosave() {
          if (!overviewDirty || !selectedLanguage || !overviewEntity || overviewDeleting) {
            if (!overviewDirty) clearOverviewAutosave();
            return;
          }
          if (overviewSaving) {
            overviewAutosaveQueued = true;
            return;
          }
          if (overviewAutosaveTimer !== null) window.clearTimeout(overviewAutosaveTimer);
          overviewAutosaveTimer = window.setTimeout(() => {
            overviewAutosaveTimer = null;
            void saveOverview(true);
          }, 800);
        }

        function tryLeaveOverview(confirmLeave: (message: string) => boolean) {
          if (!overviewDirty) {
            clearOverviewAutosave();
            return true;
          }
          const allowed = confirmLeave("You have unsaved language details. Leave without saving?");
          if (allowed) {
            clearOverviewAutosave();
            overviewDirty = false;
            overviewError = "";
          }
          return allowed;
        }

        async function loadOverview() {
          clearOverviewAutosave();
          paneLoading = false;
          if (!selectedLanguage) {
            overviewEntity = null;
            overviewLoading = false;
            render();
            return;
          }
          const token = ++overviewRequest;
          overviewLoading = true;
          overviewError = "";
          render();
          try {
            const [entity, fieldRecords] = await Promise.all([
              context.entities.get(selectedLanguage.id),
              context.fields.listRecords(selectedLanguage.id),
            ]);
            if (cancelled || token !== overviewRequest) return;
            if (!entity) throw new Error("This language is no longer available.");
            const values = Object.fromEntries(fieldRecords.map((record) => [record.key, record.value]));
            for (const definition of overviewFieldDefinitions()) {
              if (!(definition.key in values)) values[definition.key] = "";
            }
            const document = entity.documents.find((item) => item.format === "markdown") ?? entity.documents[0];
            overviewEntity = entity;
            overviewName = entity.name;
            overviewFields = values;
            overviewSavedFields = { ...values };
            overviewFieldRevisions = Object.fromEntries(fieldRecords.map((record) => [record.key, record.revision]));
            overviewDocument = document?.body ?? "";
            overviewSavedDocument = overviewDocument;
            overviewDocumentRevision = document?.revision ?? "";
            overviewDirty = false;
            overviewLoading = false;
            overviewError = "";
            render();
          } catch (cause) {
            if (cancelled || token !== overviewRequest) return;
            overviewLoading = false;
            overviewError = cause instanceof Error ? cause.message : String(cause);
            render();
          }
        }

        async function saveOverview(automatic = false) {
          if (!selectedLanguage || !overviewEntity || overviewSaving || overviewDeleting) return;
          clearOverviewAutosave();
          const name = overviewName.trim();
          if (!name) {
            overviewError = "Language name is required.";
            render();
            return;
          }
          const entityId = overviewEntity.id;
          const draftFields = { ...overviewFields };
          const draftDocument = overviewDocument;
          overviewSaving = true;
          overviewSavingAutomatically = automatic;
          overviewError = "";
          render();
          try {
            if (name !== overviewEntity.name) {
              overviewEntity = await context.entities.update(
                overviewEntity.id,
                { name },
                { expectedRevision: overviewEntity.revision, requestId: crypto.randomUUID() },
              );
            }
            for (const definition of overviewFieldDefinitions()) {
              const value = draftFields[definition.key] ?? "";
              if (JSON.stringify(value) === JSON.stringify(overviewSavedFields[definition.key] ?? "")) continue;
              await context.fields.set(overviewEntity.id, definition.key, value, {
                expectedRevision: overviewFieldRevisions[definition.key] ?? "",
                requestId: crypto.randomUUID(),
              });
            }
            if (draftDocument !== overviewSavedDocument) {
              await context.documents.save(
                { entityId: overviewEntity.id, body: draftDocument, format: "markdown" },
                { expectedRevision: overviewDocumentRevision, requestId: crypto.randomUUID() },
              );
            }
            const currentDraftChanged =
              overviewName.trim() !== name ||
              overviewFieldDefinitions().some(
                (definition) =>
                  JSON.stringify(overviewFields[definition.key] ?? "") !==
                  JSON.stringify(draftFields[definition.key] ?? ""),
              ) ||
              overviewDocument !== draftDocument;
            const currentDraftName = overviewName;
            const currentDraftFields = { ...overviewFields };
            const currentDraftDocument = overviewDocument;
            const needsFollowUpSave = currentDraftChanged || overviewAutosaveQueued;
            overviewSaving = false;
            overviewSavingAutomatically = false;
            await loadOverview();
            if (selectedLanguage?.id !== entityId) return;
            if (needsFollowUpSave) {
              overviewName = currentDraftName;
              overviewFields = currentDraftFields;
              overviewDocument = currentDraftDocument;
              overviewDirty = true;
              render();
              scheduleOverviewAutosave();
              return;
            }
            languageSummaries = languageSummaries.map((language) =>
              language.id === selectedLanguage?.id
                ? { ...language, name, revision: overviewEntity?.revision ?? language.revision }
                : language,
            );
            selectedLanguage = selectedLanguage
              ? { ...selectedLanguage, name, revision: overviewEntity?.revision ?? selectedLanguage.revision }
              : selectedLanguage;
            render();
          } catch (cause) {
            overviewSaving = false;
            overviewSavingAutomatically = false;
            overviewDirty = true;
            overviewError = cause instanceof Error ? cause.message : String(cause);
            render();
            if (overviewAutosaveQueued) scheduleOverviewAutosave();
          }
        }

        async function archiveOverviewLanguage() {
          if (!selectedLanguage || !overviewEntity || overviewDeleting) return;
          const name = selectedLanguage.name;
          const message = overviewDirty
            ? `Archive “${name}”? Unsaved language details will be discarded.`
            : `Archive “${name}”? It will be removed from the active language list.`;
          if (!window.confirm(message)) return;
          clearOverviewAutosave();
          overviewDeleting = true;
          overviewError = "";
          render();
          try {
            await context.entities.delete(overviewEntity.id, {
              expectedRevision: overviewEntity.revision,
              requestId: crypto.randomUUID(),
            });
            languageSummaries = languageSummaries.filter((language) => language.id !== overviewEntity?.id);
            selectedLanguage = languageSummaries[0] ?? null;
            overviewEntity = null;
            overviewName = "";
            overviewFields = {};
            overviewSavedFields = {};
            overviewFieldRevisions = {};
            overviewDocument = "";
            overviewSavedDocument = "";
            overviewDocumentRevision = "";
            overviewDirty = false;
            overviewAutosaveQueued = false;
            overviewDeleting = false;
            overviewLoading = false;
            resetEditors();
            render();
            if (selectedLanguage) void loadPane();
          } catch (cause) {
            overviewDeleting = false;
            overviewError = cause instanceof Error ? cause.message : String(cause);
            render();
          }
        }

        function renderOverview(panel: HTMLElement, error = "") {
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const titleBlock = document.createElement("div");
          titleBlock.className = "language-toolbar-title";
          const eyebrow = document.createElement("p");
          eyebrow.className = "language-toolbar-eyebrow";
          eyebrow.textContent = "Unified language workspace";
          const title = document.createElement("h2");
          title.textContent = "Overview";
          const subtitle = document.createElement("p");
          subtitle.className = "language-toolbar-subtitle";
          subtitle.textContent = selectedLanguage
            ? `${selectedLanguage.name} · identity, properties, and canonical notes`
            : "Select a language to begin.";
          titleBlock.append(eyebrow, title, subtitle);
          const overviewStatus = document.createElement("span");
          overviewStatus.className = "language-overview-status";
          overviewStatus.setAttribute("role", "status");
          overviewStatus.setAttribute("aria-live", "polite");
          overviewStatus.textContent = selectedLanguage ? "Loading language details…" : "Select a language";
          toolbar.append(titleBlock, overviewStatus);
          panel.append(toolbar);
          if (!selectedLanguage) {
            panel.append(emptyState("Select a language, or create one from the list."));
            return;
          }
          if (overviewLoading || !overviewEntity) {
            panel.append(loadingMessage("Loading language details…"));
            return;
          }
          const form = document.createElement("form");
          form.className = "language-overview";
          let identityStatus: HTMLElement | null = null;
          const syncOverviewStatus = () => {
            const hasError = Boolean(overviewError || error);
            overviewStatus.dataset.state = hasError
              ? "error"
              : overviewDeleting || overviewSaving
                ? "saving"
                : overviewDirty
                  ? "dirty"
                  : "saved";
            overviewStatus.textContent = hasError
              ? "Changes need attention"
              : overviewDeleting
                ? "Archiving language…"
                : overviewSaving
                  ? overviewSavingAutomatically
                    ? "Saving automatically…"
                    : "Saving language details…"
                  : overviewDirty
                    ? "Changes save automatically"
                    : "All changes saved";
          };
          const syncOverviewDirty = () => {
            const nameDirty = overviewName.trim() !== overviewEntity?.name;
            const fieldsDirty = overviewFieldDefinitions().some(
              (definition) =>
                JSON.stringify(overviewFields[definition.key] ?? "") !==
                JSON.stringify(overviewSavedFields[definition.key] ?? ""),
            );
            overviewDirty = nameDirty || fieldsDirty || overviewDocument !== overviewSavedDocument;
            syncOverviewStatus();
            if (identityStatus) identityStatus.textContent = overviewDirty ? "Draft changes" : "Ready to build";
          };
          const identity = document.createElement("section");
          identity.className = "language-overview-identity";
          const identityCopy = document.createElement("div");
          const identityTitle = document.createElement("h3");
          identityTitle.textContent = "Language identity";
          const identityIntro = document.createElement("p");
          identityIntro.textContent = "Keep the name and short identity details close while you build the language.";
          const nameControl = input("overviewName", overviewName);
          nameControl.autocomplete = "off";
          nameControl.oninput = () => {
            overviewName = nameControl.value;
            syncOverviewDirty();
            scheduleOverviewAutosave();
          };
          identityCopy.append(identityTitle, identityIntro, field("Language name", nameControl));
          const identityMeta = document.createElement("div");
          identityMeta.className = "language-overview-identity-meta";
          const metaLabel = document.createElement("span");
          metaLabel.textContent = "Workspace status";
          const metaValue = document.createElement("strong");
          metaValue.textContent = overviewDirty ? "Draft changes" : "Ready to build";
          identityStatus = metaValue;
          identityMeta.append(metaLabel, metaValue);
          identity.append(identityCopy, identityMeta);
          form.append(identity);

          const properties = document.createElement("section");
          properties.className = "language-overview-section";
          const propertiesTitle = document.createElement("h3");
          propertiesTitle.textContent = "Properties";
          const propertiesIntro = document.createElement("p");
          propertiesIntro.textContent = "A few useful anchors for how this language belongs in the world.";
          const propertyFields = document.createElement("div");
          propertyFields.className = "language-overview-fields";
          for (const definition of overviewFieldDefinitions()) {
            const value = overviewFields[definition.key];
            const control = definition.multiple
              ? textarea(`overview-${definition.key}`, Array.isArray(value) ? value.join("\n") : String(value ?? ""), 2)
              : input(`overview-${definition.key}`, String(value ?? ""));
            control.oninput = () => {
              overviewFields = {
                ...overviewFields,
                [definition.key]: definition.multiple
                  ? control.value
                      .split(/[,\n]/)
                      .map((item) => item.trim())
                      .filter(Boolean)
                  : control.value,
              };
              syncOverviewDirty();
              scheduleOverviewAutosave();
            };
            propertyFields.append(field(definition.label, control));
          }
          properties.append(propertiesTitle, propertiesIntro, propertyFields);
          form.append(properties);

          const documentSection = document.createElement("section");
          documentSection.className = "language-overview-section";
          const documentTitle = document.createElement("h3");
          documentTitle.textContent = "Canonical notes";
          const documentIntro = document.createElement("p");
          documentIntro.textContent =
            "Describe what makes this language itself. These notes stay with the language as the projection grows.";
          const documentControl = textarea("overviewDocument", overviewDocument, 12);
          documentControl.className = "language-overview-document";
          documentControl.oninput = () => {
            overviewDocument = documentControl.value;
            syncOverviewDirty();
            scheduleOverviewAutosave();
          };
          documentSection.append(documentTitle, documentIntro, documentControl);
          form.append(documentSection);

          const message = overviewError || error;
          if (message) form.append(alertMessage(message));
          const actions = document.createElement("div");
          actions.className = "language-overview-actions";
          const archive = button(
            overviewDeleting ? "Archiving…" : "Archive language",
            "language-button secondary language-danger",
            () => void archiveOverviewLanguage(),
          );
          archive.disabled = overviewSaving || overviewDeleting;
          const archiveGroup = document.createElement("span");
          archiveGroup.className = "language-overview-danger";
          archiveGroup.append(archive);
          actions.append(archiveGroup);
          form.append(actions);
          syncOverviewDirty();
          form.onsubmit = (event) => {
            event.preventDefault();
          };
          panel.append(form);
        }

        async function loadLanguages() {
          const token = ++languageRequest;
          try {
            const languages = await context.entities.list({ type: "language", limit: 500 });
            if (cancelled || token !== languageRequest) return;
            languageSummaries = languages;
            languageListLoaded = true;
            languageLoading = false;
            languageLoadError = "";
            let shouldLoadPane = false;
            if (!selectedLanguage && languages.length) {
              selectedLanguage = languages.find((language) => language.id === context.focusEntityId) ?? languages[0];
              shouldLoadPane = true;
            }
            render();
            if (shouldLoadPane) void loadPane();
          } catch (cause) {
            if (cancelled || token !== languageRequest) return;
            languageLoading = false;
            languageLoadError = cause instanceof Error ? cause.message : String(cause);
            render();
          }
        }

        function paneToolbar(titleText: string, subtitleText: string, action?: HTMLElement) {
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const titleBlock = document.createElement("div");
          titleBlock.className = "language-toolbar-title";
          const eyebrow = document.createElement("p");
          eyebrow.className = "language-toolbar-eyebrow";
          eyebrow.textContent = "Focused projection";
          const title = document.createElement("h2");
          title.textContent = titleText;
          const subtitle = document.createElement("p");
          subtitle.className = "language-toolbar-subtitle";
          subtitle.textContent = subtitleText;
          titleBlock.append(eyebrow, title, subtitle);
          toolbar.append(titleBlock);
          if (action) {
            const actions = document.createElement("div");
            actions.className = "language-toolbar-actions";
            actions.append(action);
            toolbar.append(actions);
          }
          return toolbar;
        }

        function loadingMessage(message: string) {
          const loading = emptyMessage(message);
          loading.classList.add("language-loading");
          loading.setAttribute("aria-live", "polite");
          return loading;
        }

        async function loadRecords() {
          if (!selectedLanguage) {
            records = [];
            paradigms = [];
            lexiconLoading = false;
            render();
            return;
          }
          const token = ++request;
          lexiconLoading = true;
          render();
          try {
            const [result, paradigmList] = await Promise.all([
              context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
                query: search || undefined,
                status: statusFilter || undefined,
                tag: tagFilter || undefined,
                sort,
                homonymsOnly: homonymsOnly || undefined,
                limit: 51,
                offset: page * 50,
              }),
              context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
            ]);
            if (!cancelled && token === request) {
              lexiconLoading = false;
              hasNextPage = result.length > 50;
              records = result.slice(0, 50).map((record) => ({
                ...record,
                value: normalizeLexeme(record.value),
              }));
              paradigms = paradigmList.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
              if (editing) {
                const current = records.find((record) => record.id === editing?.id);
                if (current) editing = current;
              }
              if (pendingLexemeId) {
                const target =
                  records.find((record) => record.id === pendingLexemeId) ??
                  (editing?.id === pendingLexemeId ? editing : null) ??
                  (await findLexeme(pendingLexemeId, token));
                if (cancelled || token !== request) return;
                if (target) {
                  editing = target;
                  editorOpen = true;
                  draft = normalizeLexeme(target.value);
                  pendingLexemeId = null;
                } else {
                  pendingLexemeId = null;
                  render("Linked word was not found in this language.");
                  return;
                }
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) {
              lexiconLoading = false;
              render(cause instanceof Error ? cause.message : String(cause));
            }
          }
        }

        async function findLexeme(id: string, token: number) {
          if (!selectedLanguage) return null;
          for (let offset = 0; offset < 2000; offset += 100) {
            const batch = await context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
              limit: 100,
              offset,
              sort: "lemma",
            });
            if (cancelled || token !== request) return null;
            const found = batch.find((record) => record.id === id);
            if (found) return { ...found, value: normalizeLexeme(found.value) };
            if (batch.length < 100) break;
          }
          return null;
        }

        async function refreshHomonyms(lemma: string) {
          if (!selectedLanguage || !lemma) {
            homonymCount = 0;
            return;
          }
          const matches = await context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
            query: lemma,
            limit: 100,
          });
          homonymCount = matches.filter(
            (record) =>
              record.value.lemma.toLocaleLowerCase() === lemma.toLocaleLowerCase() && record.id !== editing?.id,
          ).length;
        }

        function scheduleLoad() {
          page = 0;
          if (searchTimer !== null) window.clearTimeout(searchTimer);
          searchTimer = window.setTimeout(() => void loadRecords(), 180);
        }

        function clearLexiconFilters() {
          search = "";
          statusFilter = "";
          tagFilter = "";
          sort = "lemma";
          homonymsOnly = false;
          scheduleLoad();
        }

        function formPreviewTable(
          paradigm: Paradigm,
          stem: string,
          forms: LexemeValue["forms"],
          paradigmId: string,
          actions?: {
            onPin?: (slot: Paradigm["slots"][number], form: string) => void;
            onClear?: (slot: Paradigm["slots"][number]) => void;
          },
        ) {
          const table = document.createElement("table");
          table.className = "paradigm-preview";
          const head = document.createElement("thead");
          const headRow = document.createElement("tr");
          for (const label of ["Slot", "Form", "Source", "Rule", "Override"]) {
            const cell = document.createElement("th");
            cell.textContent = label;
            headRow.append(cell);
          }
          head.append(headRow);
          const body = document.createElement("tbody");
          for (const cell of previewParadigm(paradigm, stem, forms, paradigmId)) {
            const rowEl = document.createElement("tr");
            const slot = document.createElement("th");
            slot.scope = "row";
            slot.textContent = cell.slot.features ? `${cell.slot.label} (${cell.slot.features})` : cell.slot.label;
            const formCell = document.createElement("td");
            formCell.textContent = cell.form || "—";
            if (cell.provenance === "authored" && cell.generated && cell.generated !== cell.form) {
              const generated = document.createElement("small");
              generated.textContent = ` rule: ${cell.generated}`;
              formCell.append(generated);
            }
            const source = document.createElement("td");
            const badge = document.createElement("span");
            badge.className = `form-provenance${cell.provenance === "authored" ? " is-authored" : cell.provenance === "missing" ? " is-missing" : ""}`;
            badge.textContent =
              cell.provenance === "authored" ? "authored" : cell.provenance === "generated" ? "generated" : "no rule";
            source.append(badge);
            const rule = document.createElement("td");
            rule.textContent = cell.ruleName || "—";
            const override = document.createElement("td");
            if (actions?.onPin && cell.form && cell.provenance === "generated") {
              override.append(
                button("Pin override", "language-button secondary", () => actions.onPin?.(cell.slot, cell.form)),
              );
            } else if (actions?.onClear && cell.provenance === "authored") {
              override.append(
                button("Clear override", "language-button secondary", () => actions.onClear?.(cell.slot)),
              );
            }
            rowEl.append(slot, formCell, source, rule, override);
            body.append(rowEl);
          }
          table.append(head, body);
          const scroller = document.createElement("div");
          scroller.className = "language-chart-wrap";
          scroller.append(table);
          return scroller;
        }

        function editForm(error = "") {
          const form = document.createElement("form");
          form.className = "language-editor";
          const lists = document.createElement("div");
          lists.className = "visually-hidden";
          const posList = document.createElement("datalist");
          posList.id = "language-pos";
          posList.append(...PART_OF_SPEECH_SUGGESTIONS.map((item) => new Option(item)));
          const statusList = document.createElement("datalist");
          statusList.id = "language-status";
          statusList.append(...STATUS_SUGGESTIONS.map((item) => new Option(item)));
          lists.append(posList, statusList);
          const editorHead = document.createElement("div");
          editorHead.className = "language-editor-head";
          const editorTitle = document.createElement("h3");
          editorTitle.textContent = editing ? "Edit word" : "New word";
          const editorIntro = document.createElement("p");
          editorIntro.textContent =
            "Capture the core meaning first; pronunciation, forms, and notes can grow with the entry.";
          editorHead.append(editorTitle, editorIntro);
          form.append(editorHead, lists);
          const core = document.createElement("section");
          core.className = "language-form-section";
          const coreTitle = document.createElement("h3");
          coreTitle.textContent = "Core details";
          const coreFields = document.createElement("div");
          coreFields.className = "language-section-grid";
          const lemmaField = field("Lemma", input("lemma", draft.lemma));
          lemmaField.classList.add("language-field-wide");
          coreFields.append(
            lemmaField,
            field("Part of speech (optional)", input("partOfSpeech", draft.partOfSpeech, "language-pos")),
            field("Status (optional)", input("status", draft.status, "language-status")),
            field("Tags — comma or line separated (optional)", textarea("tags", draft.tags.join("\n"), 2)),
          );
          core.append(coreTitle, coreFields);
          form.append(core);
          const paradigmSelect = document.createElement("select");
          paradigmSelect.name = "paradigmId";
          paradigmSelect.setAttribute("aria-label", "Paradigm");
          paradigmSelect.append(new Option("None", "", !draft.paradigmId, !draft.paradigmId));
          for (const record of paradigms) {
            paradigmSelect.append(
              new Option(
                record.value.name || "Untitled paradigm",
                record.id,
                record.id === draft.paradigmId,
                record.id === draft.paradigmId,
              ),
            );
          }
          paradigmSelect.onchange = () => {
            capture(form);
            replaceEditor(form, editForm(error));
          };
          form.append(field("Paradigm (optional)", paradigmSelect));
          if (homonymCount > 0) {
            const notice = document.createElement("p");
            notice.className = "language-status";
            notice.textContent = `${homonymCount} other ${homonymCount === 1 ? "entry shares" : "entries share"} this lemma. Duplicate lemmas are kept as distinct homonyms.`;
            form.append(notice);
          }
          const pronunciations = document.createElement("section");
          pronunciations.className = "language-group language-form-section";
          pronunciations.append(
            groupHead("Pronunciation variants", () => {
              capture(form);
              draft.pronunciations.push({ id: crypto.randomUUID(), value: "" });
              replaceEditor(form, editForm(error));
            }),
          );
          for (const [index, item] of draft.pronunciations.entries()) {
            pronunciations.append(
              row(
                [
                  field("Pronunciation", input(`pronunciation-${index}`, item.value)),
                  field("Note (optional)", input(`pronunciation-note-${index}`, item.note)),
                ],
                () => {
                  capture(form);
                  draft.pronunciations.splice(index, 1);
                  replaceEditor(form, editForm(error));
                },
              ),
            );
          }
          const forms = document.createElement("section");
          forms.className = "language-group language-form-section";
          forms.append(
            groupHead("Alternate forms", () => {
              capture(form);
              draft.forms.push({ id: crypto.randomUUID(), form: "" });
              replaceEditor(form, editForm(error));
            }),
          );
          for (const [index, item] of draft.forms.entries()) {
            forms.append(
              row(
                [
                  field("Form", input(`form-${index}`, item.form)),
                  field("Kind (optional)", input(`form-kind-${index}`, item.kind)),
                  field("Pronunciation (optional)", input(`form-pronunciation-${index}`, item.pronunciation)),
                ],
                () => {
                  capture(form);
                  draft.forms.splice(index, 1);
                  replaceEditor(form, editForm(error));
                },
              ),
            );
          }
          const senses = document.createElement("section");
          senses.className = "language-group language-form-section";
          senses.append(
            groupHead("Senses", () => {
              capture(form);
              draft.senses.push({ id: crypto.randomUUID(), examples: [] });
              replaceEditor(form, editForm(error));
            }),
          );
          for (const [index, sense] of draft.senses.entries()) {
            const block = document.createElement("div");
            block.className = "language-group";
            const head = document.createElement("div");
            head.className = "language-group-head";
            const title = document.createElement("h3");
            title.textContent = `Sense ${index + 1}`;
            head.append(
              title,
              button("Remove sense", "language-button secondary language-danger", () => {
                capture(form);
                draft.senses.splice(index, 1);
                if (draft.senses.length === 0) draft.senses.push({ id: crypto.randomUUID(), examples: [] });
                replaceEditor(form, editForm(error));
              }),
            );
            block.append(
              head,
              field("Gloss (optional)", input(`sense-gloss-${index}`, sense.gloss)),
              field("Definition (optional)", textarea(`sense-definition-${index}`, sense.definition, 2)),
              field("Usage notes (optional)", textarea(`sense-usage-${index}`, sense.usageNotes, 2)),
            );
            for (const [exampleIndex, example] of sense.examples.entries()) {
              block.append(
                row(
                  [
                    field("Example", textarea(`sense-${index}-example-${exampleIndex}`, example.text, 2)),
                    field(
                      "Translation (optional)",
                      textarea(`sense-${index}-translation-${exampleIndex}`, example.translation, 2),
                    ),
                  ],
                  () => {
                    capture(form);
                    draft.senses[index].examples.splice(exampleIndex, 1);
                    replaceEditor(form, editForm(error));
                  },
                ),
              );
            }
            block.append(
              button("Add example", "language-button secondary", () => {
                capture(form);
                draft.senses[index].examples.push({ id: crypto.randomUUID(), text: "" });
                replaceEditor(form, editForm(error));
              }),
            );
            senses.append(block);
          }
          form.append(
            pronunciations,
            forms,
            senses,
            field("Etymology (optional)", textarea("etymology", draft.etymology)),
            field("Source notes (optional)", textarea("sourceNotes", draft.sourceNotes)),
            field("Notes (optional)", textarea("notes", draft.notes)),
          );
          const attached = paradigms.find((record) => record.id === draft.paradigmId);
          if (attached) {
            const preview = document.createElement("section");
            preview.className = "language-group language-form-section";
            const heading = document.createElement("h3");
            heading.textContent = "Generated forms preview";
            preview.append(
              heading,
              emptyMessage(
                "Generated cells are a preview. Pinning stores an authored override on this word; changing a rule does not delete pinned or other authored forms.",
              ),
              formPreviewTable(attached.value, draft.lemma, draft.forms, attached.id, {
                onPin: (slot, formValue) => {
                  capture(form);
                  draft.forms = pinOverride(draft.forms, attached.id, slot, formValue);
                  replaceEditor(form, editForm(error));
                },
                onClear: (slot) => {
                  capture(form);
                  draft.forms = clearOverride(draft.forms, attached.id, slot);
                  replaceEditor(form, editForm(error));
                },
              }),
            );
            form.append(preview);
          }
          if (error) {
            const message = document.createElement("p");
            message.className = "language-status error";
            message.setAttribute("role", "alert");
            message.textContent = error;
            form.append(message);
          }
          if (lexiconSaving) form.append(emptyMessage("Saving word…"));
          const actions = document.createElement("div");
          actions.className = "language-actions";
          const left = document.createElement("span");
          if (editing) {
            left.append(
              button("Add homonym", "language-button secondary", () => {
                capture(form);
                const lemma = draft.lemma;
                editing = null;
                editorOpen = true;
                draft = { ...emptyLexeme(), lemma };
                void refreshHomonyms(lemma).then(() => render());
              }),
              button("Delete", "language-button secondary language-danger", async () => {
                if (!selectedLanguage || !editing || !window.confirm(`Delete “${editing.value.lemma}”?`)) return;
                try {
                  await context.records.delete("lexemes", editing.id, selectedLanguage.id, {
                    expectedRevision: editing.revision,
                    requestId: crypto.randomUUID(),
                  });
                  editing = null;
                  editorOpen = false;
                  draft = emptyLexeme();
                  await loadRecords();
                } catch (cause) {
                  render(cause instanceof Error ? cause.message : String(cause));
                }
              }),
            );
          }
          const right = document.createElement("span");
          right.append(
            button("Cancel", "language-button secondary", () => {
              editing = null;
              editorOpen = false;
              draft = emptyLexeme();
              render();
            }),
          );
          const save = document.createElement("button");
          save.type = "submit";
          save.className = "language-button";
          save.textContent = lexiconSaving ? "Saving…" : "Save word";
          save.disabled = lexiconSaving;
          right.append(save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage || lexiconSaving) return;
            capture(form);
            const value = normalizeLexeme(draft);
            if (!value.lemma) {
              form.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
              render("Lemma is required.");
              return;
            }
            draft = value;
            lexiconSaving = true;
            save.disabled = true;
            save.textContent = "Saving…";
            try {
              const payload = serializeLexeme(value);
              if (editing) {
                const updated = await context.records.update("lexemes", editing.id, selectedLanguage.id, payload, {
                  expectedRevision: editing.revision,
                  requestId: crypto.randomUUID(),
                });
                editing = { ...updated, value: normalizeLexeme(updated.value) };
              } else {
                const created = await context.records.create("lexemes", selectedLanguage.id, payload, {
                  requestId: crypto.randomUUID(),
                });
                editing = { ...created, value: normalizeLexeme(created.value) };
              }
              editorOpen = true;
              draft = editing.value;
              lexiconSaving = false;
              await loadRecords();
              await refreshHomonyms(draft.lemma);
              render();
            } catch (cause) {
              lexiconSaving = false;
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          return form;
        }

        function capture(form: HTMLFormElement) {
          const data = new FormData(form);
          draft.lemma = String(data.get("lemma") ?? "");
          draft.partOfSpeech = String(data.get("partOfSpeech") ?? "");
          draft.status = String(data.get("status") ?? "");
          draft.tags = String(data.get("tags") ?? "").split(/[\n,]/);
          draft.etymology = String(data.get("etymology") ?? "");
          draft.sourceNotes = String(data.get("sourceNotes") ?? "");
          draft.notes = String(data.get("notes") ?? "");
          draft.paradigmId = String(data.get("paradigmId") ?? "") || undefined;
          draft.pronunciations = draft.pronunciations.map((item, index) => ({
            ...item,
            value: String(data.get(`pronunciation-${index}`) ?? ""),
            note: String(data.get(`pronunciation-note-${index}`) ?? ""),
          }));
          draft.forms = draft.forms.map((item, index) => ({
            ...item,
            form: String(data.get(`form-${index}`) ?? ""),
            kind: String(data.get(`form-kind-${index}`) ?? ""),
            pronunciation: String(data.get(`form-pronunciation-${index}`) ?? ""),
          }));
          draft.senses = draft.senses.map((sense, index) => ({
            ...sense,
            gloss: String(data.get(`sense-gloss-${index}`) ?? ""),
            definition: String(data.get(`sense-definition-${index}`) ?? ""),
            usageNotes: String(data.get(`sense-usage-${index}`) ?? ""),
            examples: sense.examples.map((example, exampleIndex) => ({
              ...example,
              text: String(data.get(`sense-${index}-example-${exampleIndex}`) ?? ""),
              translation: String(data.get(`sense-${index}-translation-${exampleIndex}`) ?? ""),
            })),
          }));
        }

        async function exportLexicon() {
          if (!selectedLanguage) return;
          const values: LexemeValue[] = [];
          for (let offset = 0; ; offset += 100) {
            const batch = await context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
              limit: 100,
              offset,
              sort: "lemma",
            });
            values.push(...batch.map((record) => normalizeLexeme(record.value)));
            if (batch.length < 100) break;
          }
          const blob = new Blob([lexiconExport(selectedLanguage.name, values)], { type: "application/json" });
          const url = URL.createObjectURL(blob);
          const link = document.createElement("a");
          link.href = url;
          link.download = `${selectedLanguage.name.replace(/\s+/g, "-").toLowerCase()}-lexicon.json`;
          link.click();
          URL.revokeObjectURL(url);
        }

        async function importLexicon(file: File) {
          if (!selectedLanguage) return;
          try {
            const lexemes = parseLexiconImport(await file.text());
            for (const value of lexemes) {
              await context.records.create("lexemes", selectedLanguage.id, serializeLexeme(value), {
                requestId: crypto.randomUUID(),
              });
            }
            page = 0;
            await loadPane();
          } catch (cause) {
            render(cause instanceof Error ? cause.message : String(cause));
          }
        }

        function resetEditors() {
          editing = null;
          editorOpen = false;
          draft = emptyLexeme();
          phonemeEditing = null;
          phonemeEditorOpen = false;
          phonemeDraft = emptyPhoneme();
          phonologyNotesOpen = false;
          orthographyEditing = null;
          orthographyEditorOpen = false;
          orthographyDraft = emptyOrthography();
          grammarUi = emptyGrammarUiState();
          paradigmEditing = null;
          paradigmEditorOpen = false;
          paradigmDraft = emptyParadigm();
          previewStem = "";
          previewLexemeId = "";
          sampleEditing = null;
          sampleEditorOpen = false;
          sampleDraft = emptySample();
          lexiconSaving = false;
        }

        async function loadPane() {
          if (pane === "overview") return loadOverview();
          if (pane === "sounds") return loadSounds();
          if (pane === "writing") return loadWriting();
          if (pane === "grammar") return loadGrammar();
          if (pane === "forms") return loadForms();
          if (pane === "samples") return loadSamples();
          return loadRecords();
        }

        async function loadSounds() {
          if (!selectedLanguage) {
            phonemes = [];
            phonologyRecord = null;
            phonologyDraft = emptyPhonologyNotes();
            phonologyNotesOpen = false;
            paneLoading = false;
            render();
            return;
          }
          const token = ++request;
          paneLoading = true;
          render();
          try {
            const [inventory, notes] = await Promise.all([
              context.records.list<PhonemeValue>("phonemes", selectedLanguage.id, { limit: 100, sort: "symbol" }),
              context.records.list<PhonologyNotes>("phonology", selectedLanguage.id, { limit: 1 }),
            ]);
            if (!cancelled && token === request) {
              paneLoading = false;
              phonemes = inventory.map((record) => ({ ...record, value: normalizePhoneme(record.value) }));
              phonologyRecord = notes[0] ? { ...notes[0], value: normalizePhonologyNotes(notes[0].value) } : null;
              phonologyDraft = phonologyRecord?.value ?? emptyPhonologyNotes();
              if (phonemeEditing) {
                const current = phonemes.find((record) => record.id === phonemeEditing?.id);
                if (current) phonemeEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) {
              paneLoading = false;
              render(cause instanceof Error ? cause.message : String(cause));
            }
          }
        }

        async function loadWriting() {
          if (!selectedLanguage) {
            orthographies = [];
            paneLoading = false;
            render();
            return;
          }
          const token = ++request;
          paneLoading = true;
          render();
          try {
            const [systems, inventory] = await Promise.all([
              context.records.list<OrthographyValue>("orthographies", selectedLanguage.id, {
                limit: 100,
                sort: "name",
              }),
              context.records.list<PhonemeValue>("phonemes", selectedLanguage.id, { limit: 100, sort: "symbol" }),
            ]);
            if (!cancelled && token === request) {
              paneLoading = false;
              orthographies = systems.map((record) => ({ ...record, value: normalizeOrthography(record.value) }));
              phonemes = inventory.map((record) => ({ ...record, value: normalizePhoneme(record.value) }));
              if (orthographyEditing) {
                const current = orthographies.find((record) => record.id === orthographyEditing?.id);
                if (current) orthographyEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) {
              paneLoading = false;
              render(cause instanceof Error ? cause.message : String(cause));
            }
          }
        }

        async function loadGrammar() {
          if (!selectedLanguage) {
            grammarUi.index = emptyGrammarUiState().index;
            records = [];
            paneLoading = false;
            render();
            return;
          }
          const token = ++request;
          paneLoading = true;
          render();
          try {
            const loaded = await loadGrammarIndex(context.records, selectedLanguage.id);
            const [lexemes, sampleRecords, paradigmRecords] = await Promise.all([
              context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
              context.records.list<Sample>("samples", selectedLanguage.id, { limit: 100, sort: "title" }),
              context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
            ]);
            if (!cancelled && token === request) {
              paneLoading = false;
              grammarUi.index = loaded.index;
              records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
              samples = sampleRecords.map((record) => ({ ...record, value: normalizeSample(record.value) }));
              paradigms = paradigmRecords.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) {
              paneLoading = false;
              render(cause instanceof Error ? cause.message : String(cause));
            }
          }
        }

        async function loadForms() {
          if (!selectedLanguage) {
            paradigms = [];
            records = [];
            paneLoading = false;
            render();
            return;
          }
          const token = ++request;
          paneLoading = true;
          render();
          try {
            const [tables, lexemes] = await Promise.all([
              context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
              context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
            ]);
            if (!cancelled && token === request) {
              paneLoading = false;
              paradigms = tables.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
              records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
              if (paradigmEditing) {
                const current = paradigms.find((record) => record.id === paradigmEditing?.id);
                if (current) paradigmEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) {
              paneLoading = false;
              render(cause instanceof Error ? cause.message : String(cause));
            }
          }
        }

        async function loadSamples() {
          if (!selectedLanguage) {
            samples = [];
            records = [];
            paneLoading = false;
            render();
            return;
          }
          const token = ++request;
          paneLoading = true;
          render();
          try {
            const [items, lexemes] = await Promise.all([
              context.records.list<Sample>("samples", selectedLanguage.id, { limit: 100, sort: "title" }),
              context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
            ]);
            if (!cancelled && token === request) {
              paneLoading = false;
              samples = items.map((record) => ({ ...record, value: normalizeSample(record.value) }));
              records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
              if (sampleEditing) {
                const current = samples.find((record) => record.id === sampleEditing?.id);
                if (current) sampleEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) {
              paneLoading = false;
              render(cause instanceof Error ? cause.message : String(cause));
            }
          }
        }

        function datalist(id: string, values: string[]) {
          const list = document.createElement("datalist");
          list.id = id;
          list.append(...values.map((item) => new Option(item)));
          return list;
        }

        function capturePhoneme(form: HTMLFormElement) {
          const data = new FormData(form);
          phonemeDraft = normalizePhoneme({
            symbol: data.get("symbol"),
            ipa: data.get("ipa"),
            kind: data.get("kind"),
            place: data.get("place"),
            manner: data.get("manner"),
            voicing: data.get("voicing"),
            height: data.get("height"),
            backness: data.get("backness"),
            rounding: data.get("rounding"),
            notes: data.get("notes"),
            example: data.get("example"),
          });
        }

        function phonemeForm(error = "") {
          const form = document.createElement("form");
          form.className = "language-editor";
          const kindSelect = document.createElement("select");
          kindSelect.name = "kind";
          kindSelect.setAttribute("aria-label", "Sound kind");
          for (const item of PHONEME_KINDS) {
            kindSelect.append(new Option(item, item, item === phonemeDraft.kind, item === phonemeDraft.kind));
          }
          form.append(
            datalist("language-place", PLACE_SUGGESTIONS),
            datalist("language-manner", MANNER_SUGGESTIONS),
            datalist("language-voice", VOICING_SUGGESTIONS),
            datalist("language-height", HEIGHT_SUGGESTIONS),
            datalist("language-backness", BACKNESS_SUGGESTIONS),
            datalist("language-rounding", ROUNDING_SUGGESTIONS),
            field("Symbol", input("symbol", phonemeDraft.symbol)),
            field("IPA (optional)", input("ipa", phonemeDraft.ipa)),
            field("Kind", kindSelect),
            field("Place (optional)", input("place", phonemeDraft.place, "language-place")),
            field("Manner (optional)", input("manner", phonemeDraft.manner, "language-manner")),
            field("Voicing (optional)", input("voicing", phonemeDraft.voicing, "language-voice")),
            field("Height (optional)", input("height", phonemeDraft.height, "language-height")),
            field("Backness (optional)", input("backness", phonemeDraft.backness, "language-backness")),
            field("Rounding (optional)", input("rounding", phonemeDraft.rounding, "language-rounding")),
            field("Example (optional)", input("example", phonemeDraft.example)),
            field("Notes (optional)", textarea("notes", phonemeDraft.notes)),
          );
          if (error) form.append(alertMessage(error));
          const actions = document.createElement("div");
          actions.className = "language-actions";
          const left = document.createElement("span");
          if (phonemeEditing) {
            left.append(
              button("Delete", "language-button secondary language-danger", async () => {
                if (!selectedLanguage || !phonemeEditing || !window.confirm(`Delete “${phonemeEditing.value.symbol}”?`))
                  return;
                try {
                  await context.records.delete("phonemes", phonemeEditing.id, selectedLanguage.id, {
                    expectedRevision: phonemeEditing.revision,
                    requestId: crypto.randomUUID(),
                  });
                  phonemeEditing = null;
                  phonemeEditorOpen = false;
                  phonemeDraft = emptyPhoneme();
                  await loadSounds();
                } catch (cause) {
                  render(cause instanceof Error ? cause.message : String(cause));
                }
              }),
            );
          }
          const right = document.createElement("span");
          right.append(
            button("Cancel", "language-button secondary", () => {
              phonemeEditing = null;
              phonemeEditorOpen = false;
              phonemeDraft = emptyPhoneme();
              render();
            }),
          );
          const save = document.createElement("button");
          save.type = "submit";
          save.className = "language-button";
          save.textContent = "Save sound";
          right.append(save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage) return;
            capturePhoneme(form);
            if (!phonemeDraft.symbol) {
              form.querySelector<HTMLInputElement>("[name=symbol]")?.focus();
              render("Symbol is required. IPA is optional.");
              return;
            }
            try {
              const payload = serializePhoneme(phonemeDraft);
              if (phonemeEditing) {
                const updated = await context.records.update(
                  "phonemes",
                  phonemeEditing.id,
                  selectedLanguage.id,
                  payload,
                  { expectedRevision: phonemeEditing.revision, requestId: crypto.randomUUID() },
                );
                phonemeEditing = { ...updated, value: normalizePhoneme(updated.value) };
              } else {
                const created = await context.records.create("phonemes", selectedLanguage.id, payload, {
                  requestId: crypto.randomUUID(),
                });
                phonemeEditing = { ...created, value: normalizePhoneme(created.value) };
              }
              phonemeEditorOpen = true;
              phonemeDraft = phonemeEditing.value;
              await loadSounds();
            } catch (cause) {
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          return form;
        }

        function captureOrthography(form: HTMLFormElement) {
          const data = new FormData(form);
          orthographyDraft.name = String(data.get("name") ?? "");
          orthographyDraft.status = String(data.get("status") ?? "");
          orthographyDraft.notes = String(data.get("notes") ?? "");
          orthographyDraft.mappings = orthographyDraft.mappings.map((item, index) => ({
            ...item,
            grapheme: String(data.get(`grapheme-${index}`) ?? ""),
            sounds: String(data.get(`sounds-${index}`) ?? "").split(/[\s,]+/),
            environment: String(data.get(`environment-${index}`) ?? ""),
            notes: String(data.get(`mapping-notes-${index}`) ?? ""),
          }));
        }

        function orthographyForm(error = "") {
          const form = document.createElement("form");
          form.className = "language-editor";
          form.append(
            datalist("language-status", STATUS_SUGGESTIONS),
            datalist(
              "language-sounds",
              phonemes.map((item) => item.value.symbol),
            ),
            field("Name", input("name", orthographyDraft.name)),
            field("Status (optional)", input("status", orthographyDraft.status, "language-status")),
            field("Notes (optional)", textarea("notes", orthographyDraft.notes)),
          );
          const mappings = document.createElement("section");
          mappings.className = "language-group";
          mappings.append(
            groupHead("Grapheme to sound", () => {
              captureOrthography(form);
              orthographyDraft.mappings.push({ id: crypto.randomUUID(), grapheme: "", sounds: [] });
              replaceEditor(form, orthographyForm(error), "[name=name]");
            }),
          );
          for (const [index, item] of orthographyDraft.mappings.entries()) {
            mappings.append(
              row(
                [
                  field("Grapheme", input(`grapheme-${index}`, item.grapheme)),
                  field("Sounds", input(`sounds-${index}`, item.sounds.join(" "), "language-sounds")),
                  field("Environment (optional)", input(`environment-${index}`, item.environment)),
                  field("Notes (optional)", input(`mapping-notes-${index}`, item.notes)),
                ],
                () => {
                  captureOrthography(form);
                  orthographyDraft.mappings.splice(index, 1);
                  replaceEditor(form, orthographyForm(error), "[name=name]");
                },
              ),
            );
          }
          form.append(mappings);
          if (error) form.append(alertMessage(error));
          const actions = document.createElement("div");
          actions.className = "language-actions";
          const left = document.createElement("span");
          if (orthographyEditing) {
            left.append(
              button("Delete", "language-button secondary language-danger", async () => {
                if (
                  !selectedLanguage ||
                  !orthographyEditing ||
                  !window.confirm(`Delete “${orthographyEditing.value.name}”?`)
                )
                  return;
                try {
                  await context.records.delete("orthographies", orthographyEditing.id, selectedLanguage.id, {
                    expectedRevision: orthographyEditing.revision,
                    requestId: crypto.randomUUID(),
                  });
                  orthographyEditing = null;
                  orthographyEditorOpen = false;
                  orthographyDraft = emptyOrthography();
                  await loadWriting();
                } catch (cause) {
                  render(cause instanceof Error ? cause.message : String(cause));
                }
              }),
            );
          }
          const right = document.createElement("span");
          right.append(
            button("Cancel", "language-button secondary", () => {
              orthographyEditing = null;
              orthographyEditorOpen = false;
              orthographyDraft = emptyOrthography();
              render();
            }),
          );
          const save = document.createElement("button");
          save.type = "submit";
          save.className = "language-button";
          save.textContent = "Save writing system";
          right.append(save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage) return;
            captureOrthography(form);
            const value = normalizeOrthography(orthographyDraft);
            if (!value.name) {
              form.querySelector<HTMLInputElement>("[name=name]")?.focus();
              render("Writing system name is required.");
              return;
            }
            orthographyDraft = value;
            try {
              const payload = serializeOrthography(value);
              if (orthographyEditing) {
                const updated = await context.records.update(
                  "orthographies",
                  orthographyEditing.id,
                  selectedLanguage.id,
                  payload,
                  { expectedRevision: orthographyEditing.revision, requestId: crypto.randomUUID() },
                );
                orthographyEditing = { ...updated, value: normalizeOrthography(updated.value) };
              } else {
                const created = await context.records.create("orthographies", selectedLanguage.id, payload, {
                  requestId: crypto.randomUUID(),
                });
                orthographyEditing = { ...created, value: normalizeOrthography(created.value) };
              }
              orthographyEditorOpen = true;
              orthographyDraft = orthographyEditing.value;
              await loadWriting();
            } catch (cause) {
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          return form;
        }

        function chartTable(
          caption: string,
          chart: ReturnType<typeof consonantChart>,
          onSelect: (item: PhonemeValue) => void,
        ) {
          const wrap = document.createElement("section");
          wrap.className = "language-group language-sounds-chart";
          const headingRow = document.createElement("div");
          headingRow.className = "language-sounds-chart-heading";
          const heading = document.createElement("h3");
          heading.textContent = caption;
          headingRow.append(heading);
          wrap.append(headingRow);
          if (!chart.columns.length) {
            wrap.append(
              emptyMessage(
                caption === "Consonants"
                  ? "Add place and manner to position consonants here."
                  : "Add height and backness to position vowels here.",
              ),
            );
            return wrap;
          }
          const scroller = document.createElement("div");
          scroller.className = "language-chart-wrap";
          const table = document.createElement("table");
          table.className = "language-chart";
          const head = document.createElement("thead");
          const headRow = document.createElement("tr");
          headRow.append(document.createElement("th"));
          for (const column of chart.columns) {
            const cell = document.createElement("th");
            cell.scope = "col";
            cell.textContent = column;
            headRow.append(cell);
          }
          head.append(headRow);
          const body = document.createElement("tbody");
          for (const rowLabel of chart.rows) {
            const tableRow = document.createElement("tr");
            const rowHead = document.createElement("th");
            rowHead.scope = "row";
            rowHead.textContent = rowLabel;
            tableRow.append(rowHead);
            for (const column of chart.columns) {
              const cell = document.createElement("td");
              const items = chart.cells.find((entry) => entry.row === rowLabel && entry.column === column)?.items ?? [];
              if (!items.length) {
                cell.className = "is-empty";
                cell.textContent = "·";
              } else {
                for (const item of items) {
                  const symbol = button(item.symbol, "language-button secondary", () => onSelect(item));
                  symbol.title = item.ipa ? `${item.symbol} (${item.ipa})` : item.symbol;
                  cell.append(symbol);
                }
              }
              tableRow.append(cell);
            }
            body.append(tableRow);
          }
          table.append(head, body);
          scroller.append(table);
          wrap.append(scroller);
          if (chart.unplaced.length) {
            const leftover = document.createElement("p");
            leftover.className = "language-empty";
            leftover.textContent = `Unplaced: ${chart.unplaced.map((item) => item.symbol).join(", ")}`;
            wrap.append(leftover);
          }
          return wrap;
        }

        async function savePhonology(form: HTMLFormElement) {
          if (!selectedLanguage) return;
          const data = new FormData(form);
          phonologyDraft = normalizePhonologyNotes({
            syllableStructure: data.get("syllableStructure"),
            stress: data.get("stress"),
            tone: data.get("tone"),
            phonotactics: data.get("phonotactics"),
            notes: data.get("notes"),
          });
          const payload = serializePhonologyNotes(phonologyDraft);
          if (phonologyRecord) {
            const updated = await context.records.update(
              "phonology",
              phonologyRecord.id,
              selectedLanguage.id,
              payload,
              {
                expectedRevision: phonologyRecord.revision,
                requestId: crypto.randomUUID(),
              },
            );
            phonologyRecord = { ...updated, value: normalizePhonologyNotes(updated.value) };
          } else {
            const created = await context.records.create("phonology", selectedLanguage.id, payload, {
              requestId: crypto.randomUUID(),
            });
            phonologyRecord = { ...created, value: normalizePhonologyNotes(created.value) };
          }
          phonologyDraft = phonologyRecord.value;
        }

        function openPhoneme(record: ModuleRecord<PhonemeValue>) {
          phonemeEditing = record;
          phonemeEditorOpen = true;
          phonemeDraft = normalizePhoneme(record.value);
          render();
        }

        function renderSounds(panel: HTMLElement, error: string) {
          const addSound = () => {
            phonemeEditing = null;
            phonemeEditorOpen = true;
            phonemeDraft = emptyPhoneme();
            render();
          };
          const add = button("Add sound", "language-button", addSound);
          add.disabled = !selectedLanguage;
          panel.append(
            paneToolbar(
              "Sounds",
              selectedLanguage
                ? `${selectedLanguage.name} · phoneme inventory and phonology notes`
                : "Select a language to document its sound system.",
              add,
            ),
          );
          if (!selectedLanguage) {
            panel.append(emptyState("Select a language to document its sounds."));
            return;
          }
          if (paneLoading) {
            panel.append(loadingMessage("Loading sound inventory…"));
            return;
          }
          if (phonemeEditorOpen) {
            panel.append(phonemeForm(error));
            return;
          }
          const notes = document.createElement("form");
          notes.className = "language-editor language-pane-form language-sounds-notes-content";
          notes.append(
            field("Syllable structure (optional)", textarea("syllableStructure", phonologyDraft.syllableStructure, 2)),
            field("Stress (optional)", textarea("stress", phonologyDraft.stress, 2)),
            field("Tone (optional)", textarea("tone", phonologyDraft.tone, 2)),
            field("Phonotactics (optional)", textarea("phonotactics", phonologyDraft.phonotactics, 2)),
            field("Notes (optional)", textarea("notes", phonologyDraft.notes, 2)),
          );
          const saveNotes = document.createElement("button");
          saveNotes.type = "submit";
          saveNotes.className = "language-button";
          saveNotes.textContent = "Save sound notes";
          notes.append(saveNotes);
          notes.onsubmit = async (event) => {
            event.preventDefault();
            try {
              await savePhonology(notes);
              render();
            } catch (cause) {
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          const notesSection = document.createElement("details");
          notesSection.className = "language-sounds-notes";
          notesSection.open = phonologyNotesOpen;
          notesSection.ontoggle = () => {
            phonologyNotesOpen = notesSection.open;
          };
          const notesSummary = document.createElement("summary");
          const notesTitle = document.createElement("span");
          notesTitle.className = "language-sounds-notes-title";
          const notesTitleText = document.createElement("strong");
          notesTitleText.textContent = "Phonology notes";
          const notesTitleHint = document.createElement("span");
          notesTitleHint.textContent = "Optional sound-pattern notes";
          notesTitle.append(notesTitleText, notesTitleHint);
          const notesMeta = document.createElement("span");
          notesMeta.className = "language-sounds-notes-meta";
          notesMeta.textContent = phonologyRecord ? "Saved" : "Optional";
          notesSummary.append(notesTitle, notesMeta);
          const notesIntro = document.createElement("p");
          notesIntro.textContent = "Capture the sound patterns that sit behind the inventory and charts.";
          const notesBody = document.createElement("div");
          notesBody.className = "language-sounds-notes-body";
          notesBody.append(notesIntro, notes);
          notesSection.append(notesSummary, notesBody);
          panel.append(notesSection);
          const values = phonemes.map((record) => record.value);
          const openFromChart = (item: PhonemeValue) => {
            const record = phonemes.find(
              (entry) => entry.value.symbol === item.symbol && entry.value.kind === item.kind,
            );
            if (record) openPhoneme(record);
          };
          panel.append(chartTable("Consonants", consonantChart(values), openFromChart));
          panel.append(chartTable("Vowels", vowelChart(values), openFromChart));
          const other = phonemes.filter((record) => record.value.kind === "tone" || record.value.kind === "other");
          if (other.length) {
            const leftover = emptyMessage(`Other sounds: ${other.map((record) => record.value.symbol).join(", ")}`);
            panel.append(leftover);
          }
          if (error) panel.append(alertMessage(error));
          else if (phonemes.length === 0)
            panel.append(
              emptyState(
                "No sounds yet. Add consonants and vowels; charts stay empty until place, manner, height, or backness is filled in.",
                button("Add first sound", "language-button secondary", addSound),
              ),
            );
          else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of phonemes) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "language-item";
              rowButton.setAttribute("aria-label", `Edit sound ${record.value.symbol}`);
              const symbol = document.createElement("strong");
              symbol.textContent = record.value.symbol;
              const kind = document.createElement("small");
              kind.textContent = record.value.kind;
              const detail = document.createElement("span");
              detail.textContent =
                record.value.ipa ||
                [record.value.place, record.value.manner, record.value.height, record.value.backness]
                  .filter(Boolean)
                  .join(" · ") ||
                "No features yet";
              rowButton.append(symbol, kind, detail);
              rowButton.onclick = () => openPhoneme(record);
              item.append(rowButton);
              list.append(item);
            }
            const inventory = document.createElement("section");
            inventory.className = "language-pane-section";
            const inventoryTitle = document.createElement("h3");
            inventoryTitle.textContent = "Sound inventory";
            const inventorySummary = document.createElement("p");
            inventorySummary.textContent = `${phonemes.length} sound${phonemes.length === 1 ? "" : "s"} · select one to edit its features.`;
            inventory.append(inventoryTitle, inventorySummary, list);
            panel.append(inventory);
          }
        }

        function renderWriting(panel: HTMLElement, error: string) {
          const addWriting = () => {
            orthographyEditing = null;
            orthographyEditorOpen = true;
            orthographyDraft = emptyOrthography();
            render();
          };
          const add = button("Add writing system", "language-button", addWriting);
          add.disabled = !selectedLanguage;
          panel.append(
            paneToolbar(
              "Writing",
              selectedLanguage
                ? `${selectedLanguage.name} · scripts, graphemes, and sound mappings`
                : "Select a language to document its writing systems.",
              add,
            ),
          );
          if (paneLoading) {
            panel.append(loadingMessage("Loading writing systems…"));
            return;
          }
          if (orthographyEditorOpen) {
            panel.append(orthographyForm(error));
            return;
          }
          if (error) panel.append(alertMessage(error));
          else if (!selectedLanguage) panel.append(emptyState("Select a language to document its writing systems."));
          else if (orthographies.length === 0)
            panel.append(
              emptyState(
                "No writing systems yet. Add one and map graphemes to sounds.",
                button("Add first writing system", "language-button secondary", addWriting),
              ),
            );
          else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of orthographies) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "language-item";
              rowButton.setAttribute("aria-label", `Edit writing system ${record.value.name}`);
              const name = document.createElement("strong");
              name.textContent = record.value.name;
              const status = document.createElement("small");
              status.textContent = record.value.status || "—";
              const count = document.createElement("span");
              count.textContent = `${record.value.mappings.length} mapping${record.value.mappings.length === 1 ? "" : "s"}`;
              rowButton.append(name, status, count);
              rowButton.onclick = () => {
                orthographyEditing = record;
                orthographyEditorOpen = true;
                orthographyDraft = normalizeOrthography(record.value);
                render();
              };
              item.append(rowButton);
              list.append(item);
            }
            const inventory = document.createElement("section");
            inventory.className = "language-pane-section";
            const inventoryTitle = document.createElement("h3");
            inventoryTitle.textContent = "Writing systems";
            const inventorySummary = document.createElement("p");
            inventorySummary.textContent = `${orthographies.length} system${orthographies.length === 1 ? "" : "s"} · select one to edit its mappings.`;
            inventory.append(inventoryTitle, inventorySummary, list);
            panel.append(inventory);
          }
        }

        function openLinkedLexeme(lexemeId: string) {
          const target = records.find((record) => record.id === lexemeId);
          pendingLexemeId = lexemeId;
          pane = "lexicon";
          sampleEditorOpen = false;
          paradigmEditorOpen = false;
          search = "";
          statusFilter = "";
          tagFilter = "";
          homonymsOnly = false;
          page = 0;
          if (target) {
            editing = target;
            editorOpen = true;
            draft = normalizeLexeme(target.value);
          }
          void loadRecords();
        }

        function grammarContext(): GrammarPaneContext {
          return {
            languageName: selectedLanguage?.name,
            ownerId: selectedLanguage?.id,
            records: context.records,
            confirm: (message) => window.confirm(message),
            render,
            choices: {
              lexemes: records.map((record) => ({ id: record.id, lemma: record.value.lemma })),
              samples: samples.map((record) => ({ id: record.id, title: sampleTitle(record.value) })),
              paradigms: paradigms.map((record) => ({ id: record.id, name: record.value.name })),
              examples: records.flatMap((record) =>
                record.value.senses.flatMap((sense) =>
                  sense.examples.map((example) => ({
                    lexemeId: record.id,
                    exampleId: example.id,
                    lemma: record.value.lemma,
                    text: example.text,
                  })),
                ),
              ),
            },
          };
        }

        function renderGrammar(panel: HTMLElement, error: string) {
          renderGrammarPane(panel, grammarUi, grammarContext(), error, paneLoading);
        }

        function captureParadigm(form: HTMLFormElement) {
          const data = new FormData(form);
          paradigmDraft.name = String(data.get("name") ?? "");
          paradigmDraft.kind = (String(data.get("kind") ?? "inflection") || "inflection") as ParadigmKind;
          paradigmDraft.partOfSpeech = String(data.get("partOfSpeech") ?? "");
          paradigmDraft.notes = String(data.get("notes") ?? "");
          paradigmDraft.slots = paradigmDraft.slots.map((slot, index) => ({
            ...slot,
            label: String(data.get(`slot-label-${index}`) ?? ""),
            features: String(data.get(`slot-features-${index}`) ?? "") || undefined,
          }));
          paradigmDraft.rules = paradigmDraft.rules.map((rule, index) => ({
            ...rule,
            name: String(data.get(`rule-name-${index}`) ?? ""),
            kind: (String(data.get(`rule-kind-${index}`) ?? paradigmDraft.kind) || paradigmDraft.kind) as ParadigmKind,
            match: String(data.get(`rule-match-${index}`) ?? "") || undefined,
            notes: String(data.get(`rule-notes-${index}`) ?? "") || undefined,
            operations: rule.operations.map((operation, operationIndex) => ({
              ...operation,
              slotId: String(data.get(`op-slot-${index}-${operationIndex}`) ?? ""),
              op: (String(data.get(`op-kind-${index}-${operationIndex}`) ?? "suffix") ||
                "suffix") as MorphOperationKind,
              from: String(data.get(`op-from-${index}-${operationIndex}`) ?? "") || undefined,
              value: String(data.get(`op-value-${index}-${operationIndex}`) ?? "") || undefined,
            })),
          }));
        }

        function selectControl(name: string, value: string, options: { id: string; label: string }[], label: string) {
          const control = document.createElement("select");
          control.name = name;
          control.setAttribute("aria-label", label);
          for (const option of options) {
            control.append(new Option(option.label, option.id, option.id === value, option.id === value));
          }
          return control;
        }

        async function persistLexemeForms(record: ModuleRecord<LexemeValue>, forms: LexemeValue["forms"]) {
          if (!selectedLanguage) return;
          const value = normalizeLexeme({ ...record.value, forms });
          const updated = await context.records.update(
            "lexemes",
            record.id,
            selectedLanguage.id,
            serializeLexeme(value),
            { expectedRevision: record.revision, requestId: crypto.randomUUID() },
          );
          const next = { ...updated, value: normalizeLexeme(updated.value) };
          records = records.map((item) => (item.id === next.id ? next : item));
        }

        function paradigmForm(error = "") {
          const form = document.createElement("form");
          form.className = "language-editor";
          form.append(
            field("Name", input("name", paradigmDraft.name)),
            field("Kind", selectControl("kind", paradigmDraft.kind, PARADIGM_KINDS, "Paradigm kind")),
            field("Part of speech (optional)", input("partOfSpeech", paradigmDraft.partOfSpeech, "language-pos")),
            field("Notes (optional)", textarea("notes", paradigmDraft.notes)),
          );
          const posList = datalist("language-pos", PART_OF_SPEECH_SUGGESTIONS);
          form.append(posList);
          const slots = document.createElement("section");
          slots.className = "language-group";
          slots.append(
            groupHead("Slots", () => {
              captureParadigm(form);
              paradigmDraft.slots.push(emptySlot());
              replaceEditor(form, paradigmForm(error), "[name=name]");
            }),
          );
          if (paradigmDraft.slots.length === 0) {
            slots.append(emptyMessage("Add cells such as 1sg, plural, or comparative."));
          }
          for (const [index, slot] of paradigmDraft.slots.entries()) {
            slots.append(
              row(
                [
                  field("Slot label", input(`slot-label-${index}`, slot.label)),
                  field("Features (optional)", input(`slot-features-${index}`, slot.features)),
                ],
                () => {
                  captureParadigm(form);
                  const removed = paradigmDraft.slots[index]?.id;
                  paradigmDraft.slots.splice(index, 1);
                  for (const rule of paradigmDraft.rules) {
                    rule.operations = rule.operations.filter((item) => item.slotId !== removed);
                  }
                  replaceEditor(form, paradigmForm(error), "[name=name]");
                },
              ),
            );
          }
          form.append(slots);
          const rules = document.createElement("section");
          rules.className = "language-group";
          rules.append(
            groupHead("Rules", () => {
              captureParadigm(form);
              paradigmDraft.rules.push(emptyRule(paradigmDraft.kind));
              replaceEditor(form, paradigmForm(error), "[name=name]");
            }),
          );
          if (paradigmDraft.rules.length === 0) {
            rules.append(emptyMessage("Add an inflection or derivation rule. More specific suffix matches win."));
          }
          const slotOptions = paradigmDraft.slots
            .filter((slot) => slot.label.trim())
            .map((slot) => ({ id: slot.id, label: slot.label }));
          for (const [index, rule] of paradigmDraft.rules.entries()) {
            const block = document.createElement("section");
            block.className = "language-group";
            const head = document.createElement("div");
            head.className = "language-group-head";
            const heading = document.createElement("h3");
            heading.textContent = rule.name || `Rule ${index + 1}`;
            head.append(
              heading,
              button("Remove", "language-button secondary language-danger", () => {
                captureParadigm(form);
                paradigmDraft.rules.splice(index, 1);
                replaceEditor(form, paradigmForm(error), "[name=name]");
              }),
            );
            block.append(
              head,
              field("Rule name", input(`rule-name-${index}`, rule.name)),
              field("Kind", selectControl(`rule-kind-${index}`, rule.kind, PARADIGM_KINDS, "Rule kind")),
              field("Match lemma ending (optional)", input(`rule-match-${index}`, rule.match)),
              field("Notes (optional)", textarea(`rule-notes-${index}`, rule.notes, 2)),
            );
            for (const [operationIndex, operation] of rule.operations.entries()) {
              block.append(
                row(
                  [
                    field(
                      "Slot",
                      selectControl(
                        `op-slot-${index}-${operationIndex}`,
                        operation.slotId,
                        slotOptions,
                        "Operation slot",
                      ),
                    ),
                    field(
                      "Operation",
                      selectControl(
                        `op-kind-${index}-${operationIndex}`,
                        operation.op,
                        OPERATION_KINDS,
                        "Operation kind",
                      ),
                    ),
                    field("Replace from (optional)", input(`op-from-${index}-${operationIndex}`, operation.from)),
                    field(
                      "Affix or replacement (optional)",
                      input(`op-value-${index}-${operationIndex}`, operation.value),
                    ),
                  ],
                  () => {
                    captureParadigm(form);
                    paradigmDraft.rules[index].operations.splice(operationIndex, 1);
                    replaceEditor(form, paradigmForm(error), "[name=name]");
                  },
                ),
              );
            }
            block.append(
              button("Add operation", "language-button secondary", () => {
                captureParadigm(form);
                paradigmDraft.rules[index].operations.push(emptyOperation(paradigmDraft.slots[0]?.id ?? ""));
                replaceEditor(form, paradigmForm(error), "[name=name]");
              }),
            );
            rules.append(block);
          }
          form.append(rules);
          const preview = document.createElement("section");
          preview.className = "language-group";
          const previewHead = document.createElement("h3");
          previewHead.textContent = "Generated preview";
          preview.append(
            previewHead,
            emptyMessage(
              "This table is computed from the current rules. Saving a rule never rewrites authored word forms.",
            ),
          );
          const lexemeSelect = document.createElement("select");
          lexemeSelect.name = "previewLexemeId";
          lexemeSelect.setAttribute("aria-label", "Preview lexeme");
          lexemeSelect.append(new Option("Type a stem", "", !previewLexemeId, !previewLexemeId));
          for (const record of records) {
            lexemeSelect.append(
              new Option(record.value.lemma, record.id, record.id === previewLexemeId, record.id === previewLexemeId),
            );
          }
          lexemeSelect.onchange = () => {
            captureParadigm(form);
            previewLexemeId = String(new FormData(form).get("previewLexemeId") ?? "");
            const chosen = records.find((record) => record.id === previewLexemeId);
            previewStem = chosen?.value.lemma ?? previewStem;
            replaceEditor(form, paradigmForm(error), "[name=name]");
          };
          const stemInput = input(
            "previewStem",
            previewStem || records.find((record) => record.id === previewLexemeId)?.value.lemma || "",
          );
          stemInput.onchange = () => {
            previewStem = stemInput.value;
          };
          preview.append(field("Preview lexeme (optional)", lexemeSelect), field("Stem", stemInput));
          const stem = previewStem || records.find((record) => record.id === previewLexemeId)?.value.lemma || "";
          const previewLexeme = records.find((record) => record.id === previewLexemeId);
          const previewParadigmId = paradigmEditing?.id ?? "";
          preview.append(
            formPreviewTable(
              normalizeParadigm(paradigmDraft),
              stem,
              previewLexeme?.value.forms ?? [],
              previewParadigmId,
              previewLexeme && previewParadigmId
                ? {
                    onPin: (slot, formValue) => {
                      captureParadigm(form);
                      previewStem = String(new FormData(form).get("previewStem") ?? previewStem);
                      void persistLexemeForms(
                        previewLexeme,
                        pinOverride(previewLexeme.value.forms, previewParadigmId, slot, formValue),
                      ).then(
                        () => replaceEditor(form, paradigmForm(error), "[name=name]"),
                        (cause) => render(cause instanceof Error ? cause.message : String(cause)),
                      );
                    },
                    onClear: (slot) => {
                      captureParadigm(form);
                      previewStem = String(new FormData(form).get("previewStem") ?? previewStem);
                      void persistLexemeForms(
                        previewLexeme,
                        clearOverride(previewLexeme.value.forms, previewParadigmId, slot),
                      ).then(
                        () => replaceEditor(form, paradigmForm(error), "[name=name]"),
                        (cause) => render(cause instanceof Error ? cause.message : String(cause)),
                      );
                    },
                  }
                : undefined,
            ),
          );
          form.append(preview);
          if (error) form.append(alertMessage(error));
          const actions = document.createElement("div");
          actions.className = "language-actions";
          const left = document.createElement("span");
          if (paradigmEditing) {
            left.append(
              button("Delete", "language-button secondary language-danger", async () => {
                if (!selectedLanguage || !paradigmEditing || !window.confirm(`Delete “${paradigmEditing.value.name}”?`))
                  return;
                try {
                  await context.records.delete("paradigms", paradigmEditing.id, selectedLanguage.id, {
                    expectedRevision: paradigmEditing.revision,
                    requestId: crypto.randomUUID(),
                  });
                  paradigmEditing = null;
                  paradigmEditorOpen = false;
                  paradigmDraft = emptyParadigm();
                  await loadForms();
                } catch (cause) {
                  render(cause instanceof Error ? cause.message : String(cause));
                }
              }),
            );
          }
          const right = document.createElement("span");
          right.append(
            button("Cancel", "language-button secondary", () => {
              paradigmEditing = null;
              paradigmEditorOpen = false;
              paradigmDraft = emptyParadigm();
              render();
            }),
          );
          const save = document.createElement("button");
          save.type = "submit";
          save.className = "language-button";
          save.textContent = "Save paradigm";
          right.append(save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage) return;
            captureParadigm(form);
            previewStem = String(new FormData(form).get("previewStem") ?? "");
            previewLexemeId = String(new FormData(form).get("previewLexemeId") ?? "");
            const value = normalizeParadigm(paradigmDraft);
            if (!value.name) {
              form.querySelector<HTMLInputElement>("[name=name]")?.focus();
              render("Name is required.");
              return;
            }
            paradigmDraft = value;
            try {
              const payload = serializeParadigm(value);
              if (paradigmEditing) {
                const updated = await context.records.update(
                  "paradigms",
                  paradigmEditing.id,
                  selectedLanguage.id,
                  payload,
                  { expectedRevision: paradigmEditing.revision, requestId: crypto.randomUUID() },
                );
                paradigmEditing = { ...updated, value: normalizeParadigm(updated.value) };
              } else {
                const created = await context.records.create("paradigms", selectedLanguage.id, payload, {
                  requestId: crypto.randomUUID(),
                });
                paradigmEditing = { ...created, value: normalizeParadigm(created.value) };
              }
              paradigmEditorOpen = true;
              paradigmDraft = paradigmEditing.value;
              await loadForms();
            } catch (cause) {
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          return form;
        }

        function renderForms(panel: HTMLElement, error: string) {
          const addParadigm = () => {
            paradigmEditing = null;
            paradigmEditorOpen = true;
            paradigmDraft = emptyParadigm();
            previewStem = "";
            previewLexemeId = "";
            render();
          };
          const add = button("Add paradigm", "language-button", addParadigm);
          add.disabled = !selectedLanguage;
          panel.append(
            paneToolbar(
              "Forms",
              selectedLanguage
                ? `${selectedLanguage.name} · paradigms, rules, and generated forms`
                : "Select a language to document its morphology.",
              add,
            ),
          );
          if (paneLoading) {
            panel.append(loadingMessage("Loading paradigms…"));
            return;
          }
          if (paradigmEditorOpen) {
            panel.append(paradigmForm(error));
            return;
          }
          if (error) panel.append(alertMessage(error));
          else if (!selectedLanguage) panel.append(emptyState("Select a language to document its paradigms."));
          else if (paradigms.length === 0) {
            panel.append(
              emptyState(
                "No paradigms yet. Add an inflection or derivation table, then preview generated forms.",
                button("Add first paradigm", "language-button secondary", addParadigm),
              ),
            );
          } else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of paradigms) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "language-item";
              rowButton.setAttribute("aria-label", `Edit paradigm ${record.value.name}`);
              const name = document.createElement("strong");
              name.textContent = record.value.name;
              const kind = document.createElement("small");
              kind.textContent = record.value.kind;
              const detail = document.createElement("span");
              detail.textContent = `${record.value.slots.length} slot${record.value.slots.length === 1 ? "" : "s"} · ${record.value.rules.length} rule${record.value.rules.length === 1 ? "" : "s"}`;
              rowButton.append(name, kind, detail);
              rowButton.onclick = () => {
                paradigmEditing = record;
                paradigmEditorOpen = true;
                paradigmDraft = normalizeParadigm(record.value);
                render();
              };
              item.append(rowButton);
              list.append(item);
            }
            const inventory = document.createElement("section");
            inventory.className = "language-pane-section";
            const inventoryTitle = document.createElement("h3");
            inventoryTitle.textContent = "Paradigm library";
            const inventorySummary = document.createElement("p");
            inventorySummary.textContent = `${paradigms.length} paradigm${paradigms.length === 1 ? "" : "s"} · select one to edit rules or preview forms.`;
            inventory.append(inventoryTitle, inventorySummary, list);
            panel.append(inventory);
          }
        }

        function captureSample(form: HTMLFormElement) {
          const data = new FormData(form);
          sampleDraft.title = String(data.get("title") ?? "");
          sampleDraft.kind = (String(data.get("kind") ?? "sentence") || "sentence") as SampleKind;
          sampleDraft.text = String(data.get("text") ?? "");
          sampleDraft.translation = String(data.get("translation") ?? "");
          sampleDraft.transliteration = String(data.get("transliteration") ?? "");
          sampleDraft.notes = String(data.get("notes") ?? "");
          sampleDraft.tokens = sampleDraft.tokens.map((token, index) => ({
            ...token,
            text: String(data.get(`token-text-${index}`) ?? ""),
            gloss: String(data.get(`token-gloss-${index}`) ?? "") || undefined,
            grammar: String(data.get(`token-grammar-${index}`) ?? "") || undefined,
            lexemeId: String(data.get(`token-lexeme-${index}`) ?? "") || undefined,
          }));
        }

        function bindSampleRefs(root: HTMLElement) {
          for (const control of root.querySelectorAll<HTMLButtonElement>(".sample-ref")) {
            control.onclick = () => {
              const lexemeId = control.dataset.lexemeId;
              if (lexemeId) openLinkedLexeme(lexemeId);
            };
          }
        }

        function sampleForm(error = "") {
          const form = document.createElement("form");
          form.className = "language-editor";
          const kindSelect = document.createElement("select");
          kindSelect.name = "kind";
          kindSelect.setAttribute("aria-label", "Sample kind");
          for (const item of SAMPLE_KINDS) {
            kindSelect.append(
              new Option(item.label, item.id, item.id === sampleDraft.kind, item.id === sampleDraft.kind),
            );
          }
          form.append(
            field("Title (optional)", input("title", sampleDraft.title)),
            field("Kind", kindSelect),
            field("Text", textarea("text", sampleDraft.text, sampleDraft.kind === "paragraph" ? 6 : 3)),
            field("Transliteration (optional)", textarea("transliteration", sampleDraft.transliteration, 2)),
            field("Translation (optional)", textarea("translation", sampleDraft.translation, 2)),
            field("Notes (optional)", textarea("notes", sampleDraft.notes, 2)),
          );
          const tokens = document.createElement("section");
          tokens.className = "language-group";
          const tokenHead = document.createElement("div");
          tokenHead.className = "language-group-head";
          const tokenTitle = document.createElement("h3");
          tokenTitle.textContent = "Interlinear tokens";
          tokenHead.append(
            tokenTitle,
            button("Tokenize text", "language-button secondary", () => {
              captureSample(form);
              sampleDraft.tokens = tokenizeSample(sampleDraft.text, sampleDraft.tokens);
              replaceEditor(form, sampleForm(error), "[name=title]");
            }),
            button("Add", "language-button secondary", () => {
              captureSample(form);
              sampleDraft.tokens.push(emptyToken());
              replaceEditor(form, sampleForm(error), "[name=title]");
            }),
          );
          tokens.append(tokenHead);
          tokens.append(
            emptyMessage(
              "Tokenize splits the sample on whitespace. Matching surface forms keep their glosses, grammar tags, and lexeme links.",
            ),
          );
          const lexemeOptions = records.map((record) => ({ id: record.id, label: record.value.lemma }));
          for (const [index, token] of sampleDraft.tokens.entries()) {
            const lexemeSelect = document.createElement("select");
            lexemeSelect.name = `token-lexeme-${index}`;
            lexemeSelect.setAttribute("aria-label", `Lexeme for token ${index + 1}`);
            lexemeSelect.append(new Option("None", "", !token.lexemeId, !token.lexemeId));
            for (const option of lexemeOptions) {
              lexemeSelect.append(
                new Option(option.label, option.id, option.id === token.lexemeId, option.id === token.lexemeId),
              );
            }
            tokens.append(
              row(
                [
                  field("Form", input(`token-text-${index}`, token.text)),
                  field("Gloss (optional)", input(`token-gloss-${index}`, token.gloss)),
                  field("Grammar (optional)", input(`token-grammar-${index}`, token.grammar)),
                  field("Lexeme (optional)", lexemeSelect),
                ],
                () => {
                  captureSample(form);
                  sampleDraft.tokens.splice(index, 1);
                  replaceEditor(form, sampleForm(error), "[name=title]");
                },
              ),
            );
          }
          form.append(tokens);
          const preview = document.createElement("section");
          preview.className = "sample-block";
          const previewTitle = document.createElement("h3");
          previewTitle.textContent = "Readable preview";
          preview.append(previewTitle);
          const previewBody = document.createElement("div");
          const paintPreview = () => {
            captureSample(form);
            const html = samplePreviewHtml(normalizeSample(sampleDraft));
            previewBody.replaceChildren();
            if (html) {
              previewBody.innerHTML = html;
              bindSampleRefs(previewBody);
            } else {
              previewBody.append(emptyMessage("Add text or tokens to see the rendered sample."));
            }
          };
          preview.append(previewBody);
          paintPreview();
          form.append(preview);
          form.addEventListener("input", paintPreview);
          if (error) form.append(alertMessage(error));
          const actions = document.createElement("div");
          actions.className = "language-actions";
          const left = document.createElement("span");
          if (sampleEditing) {
            left.append(
              button("Delete", "language-button secondary language-danger", async () => {
                if (
                  !selectedLanguage ||
                  !sampleEditing ||
                  !window.confirm(`Delete “${sampleTitle(sampleEditing.value)}”?`)
                )
                  return;
                try {
                  await context.records.delete("samples", sampleEditing.id, selectedLanguage.id, {
                    expectedRevision: sampleEditing.revision,
                    requestId: crypto.randomUUID(),
                  });
                  sampleEditing = null;
                  sampleEditorOpen = false;
                  sampleDraft = emptySample();
                  await loadSamples();
                } catch (cause) {
                  render(cause instanceof Error ? cause.message : String(cause));
                }
              }),
            );
          }
          const right = document.createElement("span");
          right.append(
            button("Cancel", "language-button secondary", () => {
              sampleEditing = null;
              sampleEditorOpen = false;
              sampleDraft = emptySample();
              render();
            }),
          );
          const save = document.createElement("button");
          save.type = "submit";
          save.className = "language-button";
          save.textContent = "Save sample";
          right.append(save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage) return;
            captureSample(form);
            const value = normalizeSample(sampleDraft);
            if (!value.text.trim()) {
              form.querySelector<HTMLTextAreaElement>("[name=text]")?.focus();
              render("Text is required.");
              return;
            }
            sampleDraft = value;
            try {
              const payload = serializeSample(value);
              if (sampleEditing) {
                const updated = await context.records.update(
                  "samples",
                  sampleEditing.id,
                  selectedLanguage.id,
                  payload,
                  { expectedRevision: sampleEditing.revision, requestId: crypto.randomUUID() },
                );
                sampleEditing = { ...updated, value: normalizeSample(updated.value) };
              } else {
                const created = await context.records.create("samples", selectedLanguage.id, payload, {
                  requestId: crypto.randomUUID(),
                });
                sampleEditing = { ...created, value: normalizeSample(created.value) };
              }
              sampleEditorOpen = true;
              sampleDraft = sampleEditing.value;
              await loadSamples();
            } catch (cause) {
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          return form;
        }

        function renderSamples(panel: HTMLElement, error: string) {
          const addSample = (kind: SampleKind = "sentence") => {
            sampleEditing = null;
            sampleEditorOpen = true;
            sampleDraft = emptySample(kind);
            render();
          };
          const add = button("Add sample", "language-button", () => addSample());
          add.disabled = !selectedLanguage;
          panel.append(
            paneToolbar(
              "Samples",
              selectedLanguage
                ? `${selectedLanguage.name} · examples, translations, and interlinear notes`
                : "Select a language to collect examples and usage.",
              add,
            ),
          );
          if (paneLoading) {
            panel.append(loadingMessage("Loading samples…"));
            return;
          }
          if (sampleEditorOpen) {
            panel.append(sampleForm(error));
            return;
          }
          if (error) panel.append(alertMessage(error));
          else if (!selectedLanguage)
            panel.append(emptyState("Select a language to collect sample sentences and paragraphs."));
          else {
            const summary = document.createElement("p");
            summary.className = "language-pane-summary";
            summary.textContent = `${samples.length} sample${samples.length === 1 ? "" : "s"} · grouped by kind for quick browsing.`;
            panel.append(summary);
            const nav = document.createElement("div");
            nav.className = "grammar-nav";
            for (const group of groupSamples(samples)) {
              const block = document.createElement("section");
              block.className = "language-group";
              const head = document.createElement("div");
              head.className = "language-group-head";
              const heading = document.createElement("h3");
              heading.textContent = group.label;
              head.append(
                heading,
                button(`Add ${group.label.toLowerCase()}`, "language-button secondary", () => addSample(group.id)),
              );
              block.append(head);
              if (group.samples.length === 0) {
                block.append(emptyMessage(`No ${group.label.toLowerCase()} yet.`));
              } else {
                const list = document.createElement("ul");
                list.className = "lexeme-list";
                for (const record of group.samples) {
                  const item = document.createElement("li");
                  const rowButton = document.createElement("button");
                  rowButton.type = "button";
                  rowButton.className = "language-item";
                  rowButton.setAttribute("aria-label", `Edit sample ${sampleTitle(record.value)}`);
                  const name = document.createElement("strong");
                  name.textContent = sampleTitle(record.value);
                  const preview = document.createElement("span");
                  preview.textContent =
                    record.value.translation || record.value.text.trim().split("\n")[0] || "No text yet";
                  const count = document.createElement("small");
                  count.textContent = `${record.value.tokens.length} token${record.value.tokens.length === 1 ? "" : "s"}`;
                  rowButton.append(name, preview, count);
                  rowButton.onclick = () => {
                    sampleEditing = record;
                    sampleEditorOpen = true;
                    sampleDraft = normalizeSample(record.value);
                    render();
                  };
                  item.append(rowButton);
                  list.append(item);
                }
                block.append(list);
              }
              nav.append(block);
            }
            panel.append(nav);
          }
        }

        function rememberFocus() {
          const active = document.activeElement;
          if (
            !(
              active instanceof HTMLInputElement ||
              active instanceof HTMLTextAreaElement ||
              active instanceof HTMLSelectElement
            ) ||
            !root.contains(active)
          ) {
            return;
          }
          focusName = active.getAttribute("name") || active.getAttribute("aria-label") || "";
          focusOffset =
            "selectionStart" in active && typeof active.selectionStart === "number" ? active.selectionStart : 0;
        }

        function restoreFocus() {
          if (!focusName) return;
          const control =
            root.querySelector<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
              `[name="${CSS.escape(focusName)}"]`,
            ) ??
            root.querySelector<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>(
              `[aria-label="${CSS.escape(focusName)}"]`,
            );
          if (!control) return;
          control.focus();
          if ("setSelectionRange" in control && typeof control.setSelectionRange === "function") {
            try {
              control.setSelectionRange(focusOffset, focusOffset);
            } catch {
              /* not a text field */
            }
          }
        }

        function fillLanguageList(list: HTMLElement) {
          list.replaceChildren();
          const needle = languageQuery.trim().toLocaleLowerCase();
          const visible = needle
            ? languageSummaries.filter((language) => language.name.toLocaleLowerCase().includes(needle))
            : languageSummaries;
          for (const language of visible) {
            const item = document.createElement("li");
            const languageButton = document.createElement("button");
            languageButton.type = "button";
            const name = document.createElement("span");
            name.className = "language-list-name";
            name.textContent = language.name;
            const meta = document.createElement("span");
            meta.className = "language-list-meta";
            meta.textContent = selectedLanguage?.id === language.id ? "Selected language" : "Open language";
            languageButton.append(name, meta);
            if (selectedLanguage?.id === language.id) languageButton.setAttribute("aria-current", "page");
            languageButton.onclick = () => {
              if (!tryLeaveOverview((message) => window.confirm(message))) return;
              if (!tryLeaveGrammar(grammarUi, (message) => window.confirm(message))) return;
              selectedLanguage = language;
              resetEditors();
              search = "";
              statusFilter = "";
              tagFilter = "";
              sort = "lemma";
              homonymsOnly = false;
              page = 0;
              void loadPane();
            };
            item.append(languageButton);
            list.append(item);
          }
          const listMessage = (message: HTMLElement) => {
            const item = document.createElement("li");
            item.append(message);
            list.replaceChildren(item);
          };
          if (languageLoading) {
            listMessage(emptyMessage("Loading languages…"));
          } else if (languageLoadError) {
            listMessage(alertMessage(languageLoadError));
          } else if (languageSummaries.length === 0) {
            listMessage(emptyMessage("No languages yet. Create one to start."));
          } else if (visible.length === 0) {
            listMessage(emptyMessage("No languages match that filter."));
          }
        }

        function render(error = "") {
          if (cancelled) return;
          rememberFocus();
          root.replaceChildren(style);
          const panels: HTMLElement[] = [];
          if (!context.embedded) {
            const languagesPanel = document.createElement("aside");
            languagesPanel.className = "language-panel language-sidebar";
            languagesPanel.setAttribute("aria-busy", String(languageLoading));
            const sidebarHead = document.createElement("div");
            sidebarHead.className = "language-sidebar-head";
            const sidebarTitle = document.createElement("div");
            const sidebarKicker = document.createElement("p");
            sidebarKicker.className = "language-sidebar-kicker";
            sidebarKicker.textContent = "Language studio";
            const languagesTitle = document.createElement("h2");
            languagesTitle.textContent = "Languages";
            sidebarTitle.append(sidebarKicker, languagesTitle);
            const createLanguageButton = button("Create language", "language-button secondary", () => {
              creatingLanguage = true;
              languageCreateName = "";
              languageCreateError = "";
              render();
              root.querySelector<HTMLInputElement>('[name="languageCreateName"]')?.focus();
            });
            sidebarHead.append(sidebarTitle, createLanguageButton);
            languagesPanel.append(sidebarHead);
            const sidebarIntro = document.createElement("p");
            sidebarIntro.className = "language-sidebar-intro";
            sidebarIntro.textContent = "Choose a language to shape its words, sounds, writing, and grammar.";
            languagesPanel.append(sidebarIntro);
            const languageSearch = input("languageQuery", languageQuery);
            languageSearch.type = "search";
            languageSearch.oninput = () => {
              languageQuery = languageSearch.value;
              fillLanguageList(languagesList);
            };
            languagesPanel.append(field("Filter languages", languageSearch));
            if (creatingLanguage) {
              const createForm = document.createElement("form");
              createForm.className = "language-create";
              const createInput = input("languageCreateName", languageCreateName);
              createInput.autocomplete = "off";
              createForm.append(field("Language name", createInput));
              if (languageCreateError) createForm.append(alertMessage(languageCreateError));
              const createActions = document.createElement("div");
              createActions.className = "language-create-actions";
              createActions.append(
                button("Cancel", "language-button secondary", () => {
                  creatingLanguage = false;
                  languageCreateName = "";
                  languageCreateError = "";
                  render();
                }),
              );
              const saveLanguage = document.createElement("button");
              saveLanguage.type = "submit";
              saveLanguage.className = "language-button";
              saveLanguage.textContent = "Create";
              createActions.append(saveLanguage);
              createForm.append(createActions);
              createInput.oninput = () => {
                languageCreateName = createInput.value;
                languageCreateError = "";
              };
              createForm.onsubmit = async (event) => {
                event.preventDefault();
                languageCreateName = createInput.value.trim();
                if (!languageCreateName) {
                  languageCreateError = "Language name is required.";
                  render();
                  root.querySelector<HTMLInputElement>('[name="languageCreateName"]')?.focus();
                  return;
                }
                saveLanguage.disabled = true;
                saveLanguage.textContent = "Creating…";
                try {
                  const created = await context.entities.create({ name: languageCreateName, type: "language" });
                  languageSummaries = [created, ...languageSummaries.filter((language) => language.id !== created.id)];
                  languageListLoaded = true;
                  languageLoading = false;
                  selectedLanguage = created;
                  creatingLanguage = false;
                  languageCreateName = "";
                  languageCreateError = "";
                  resetEditors();
                  search = "";
                  statusFilter = "";
                  tagFilter = "";
                  page = 0;
                  render();
                  void loadPane();
                } catch (cause) {
                  languageCreateError = cause instanceof Error ? cause.message : String(cause);
                  render();
                  root.querySelector<HTMLInputElement>('[name="languageCreateName"]')?.focus();
                }
              };
              languagesPanel.append(createForm);
            }
            const languagesList = document.createElement("ul");
            languagesList.className = "language-list";
            fillLanguageList(languagesList);
            languagesPanel.append(languagesList);
            panels.push(languagesPanel);
          }
          if (!languageListLoaded && !languageLoading) {
            languageLoading = true;
            void loadLanguages();
          }

          const lexiconPanel = document.createElement("main");
          lexiconPanel.className = "language-panel language-main";
          panels.push(lexiconPanel);
          lexiconPanel.id = "language-pane";
          lexiconPanel.setAttribute("role", "tabpanel");
          lexiconPanel.setAttribute("aria-labelledby", `language-tab-${pane}`);
          lexiconPanel.setAttribute(
            "aria-busy",
            String(pane === "overview" ? overviewLoading : pane === "lexicon" ? lexiconLoading : paneLoading),
          );
          const tabs = document.createElement("div");
          tabs.className = "language-tabs";
          tabs.setAttribute("role", "tablist");
          tabs.setAttribute("aria-label", "Language workspace");
          const tabButtons: HTMLButtonElement[] = [];
          for (const [id, label] of [
            ["overview", "Overview"],
            ["lexicon", "Lexicon"],
            ["sounds", "Sounds"],
            ["writing", "Writing"],
            ["grammar", "Grammar"],
            ["forms", "Forms"],
            ["samples", "Samples"],
          ] as const) {
            const tab = button(label, "", () => {
              if (pane === id) return;
              if (!tryLeaveOverview((message) => window.confirm(message))) return;
              if (!tryLeaveGrammar(grammarUi, (message) => window.confirm(message))) return;
              pane = id;
              resetEditors();
              void loadPane();
            });
            tab.setAttribute("role", "tab");
            tab.id = `language-tab-${id}`;
            tab.setAttribute("aria-controls", "language-pane");
            tab.setAttribute("aria-selected", String(pane === id));
            tab.tabIndex = pane === id ? 0 : -1;
            tab.addEventListener("keydown", (event) => {
              if (
                event.key !== "ArrowLeft" &&
                event.key !== "ArrowRight" &&
                event.key !== "Home" &&
                event.key !== "End"
              ) {
                return;
              }
              event.preventDefault();
              const current = tabButtons.indexOf(tab);
              const next =
                event.key === "Home"
                  ? 0
                  : event.key === "End"
                    ? tabButtons.length - 1
                    : (current + (event.key === "ArrowRight" ? 1 : -1) + tabButtons.length) % tabButtons.length;
              tabButtons[next]?.focus();
              tabButtons[next]?.click();
            });
            tabButtons.push(tab);
            tabs.append(tab);
          }
          lexiconPanel.append(tabs);
          if (pane === "overview") {
            renderOverview(lexiconPanel, error);
            root.append(...panels);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "sounds") {
            renderSounds(lexiconPanel, error);
            root.append(...panels);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "writing") {
            renderWriting(lexiconPanel, error);
            root.append(...panels);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "grammar") {
            renderGrammar(lexiconPanel, error);
            root.append(...panels);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "forms") {
            renderForms(lexiconPanel, error);
            root.append(...panels);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "samples") {
            renderSamples(lexiconPanel, error);
            root.append(...panels);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const titleBlock = document.createElement("div");
          titleBlock.className = "language-toolbar-title";
          const eyebrow = document.createElement("p");
          eyebrow.className = "language-toolbar-eyebrow";
          eyebrow.textContent = "Focused projection";
          const title = document.createElement("h2");
          title.textContent = "Lexicon";
          const subtitle = document.createElement("p");
          subtitle.className = "language-toolbar-subtitle";
          subtitle.textContent = selectedLanguage
            ? `${selectedLanguage.name} · words, meanings, and usage`
            : "Select a language to begin building its lexicon.";
          titleBlock.append(eyebrow, title, subtitle);
          const actions = document.createElement("div");
          actions.className = "language-toolbar-actions";
          const file = document.createElement("input");
          file.type = "file";
          file.accept = "application/json,.json";
          file.className = "file-input";
          file.setAttribute("aria-label", "Import lexicon JSON");
          file.onchange = () => {
            const chosen = file.files?.[0];
            file.value = "";
            if (chosen) void importLexicon(chosen);
          };
          const add = button("Add word", "language-button", () => {
            editing = null;
            editorOpen = true;
            draft = emptyLexeme();
            homonymCount = 0;
            render();
            root.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
          });
          add.disabled = !selectedLanguage;
          const exportButton = button("Export JSON", "language-button secondary", () => void exportLexicon());
          exportButton.disabled = !selectedLanguage;
          const importButton = button("Import JSON", "language-button secondary", () => file.click());
          importButton.disabled = !selectedLanguage;
          actions.append(file, importButton, exportButton, add);
          toolbar.append(titleBlock, actions);
          lexiconPanel.append(toolbar);
          if (editorOpen) {
            lexiconPanel.append(editForm(error));
            root.append(...panels);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (selectedLanguage) {
            const searchInput = input("search", search);
            searchInput.className = "language-search";
            searchInput.type = "search";
            searchInput.oninput = () => {
              search = searchInput.value;
              scheduleLoad();
            };
            const statusInput = input("statusFilter", statusFilter, "language-filter-status");
            statusInput.oninput = () => {
              statusFilter = statusInput.value.trim();
              scheduleLoad();
            };
            const tagInput = input("tagFilter", tagFilter);
            tagInput.oninput = () => {
              tagFilter = tagInput.value.trim();
              scheduleLoad();
            };
            const sortSelect = document.createElement("select");
            sortSelect.name = "sort";
            sortSelect.setAttribute("aria-label", "Sort lexicon");
            for (const [value, label] of [
              ["lemma", "Sort by lemma"],
              ["status", "Sort by status"],
              ["updatedAt", "Sort by updated"],
            ] as const) {
              sortSelect.append(new Option(label, value, value === sort, value === sort));
            }
            sortSelect.onchange = () => {
              sort = sortSelect.value as ModuleRecordQuery["sort"];
              scheduleLoad();
            };
            const homonymLabel = document.createElement("label");
            homonymLabel.className = "language-check";
            const homonym = document.createElement("input");
            homonym.type = "checkbox";
            homonym.checked = homonymsOnly;
            homonym.onchange = () => {
              homonymsOnly = homonym.checked;
              scheduleLoad();
            };
            homonymLabel.append(homonym, document.createTextNode("Homonyms only"));
            const filterLists = document.createElement("datalist");
            filterLists.id = "language-filter-status";
            filterLists.append(...STATUS_SUGGESTIONS.map((item) => new Option(item)));
            const searchRow = document.createElement("div");
            searchRow.className = "language-search-row";
            searchRow.append(field("Search lemma or meaning", searchInput));
            lexiconPanel.append(searchRow);
            const activeFilterCount = [search, statusFilter, tagFilter, homonymsOnly ? "homonyms" : ""].filter(
              Boolean,
            ).length;
            const filterPanel = document.createElement("details");
            filterPanel.className = "language-filter-panel";
            filterPanel.open = activeFilterCount > 0;
            const filterSummary = document.createElement("summary");
            filterSummary.textContent = activeFilterCount
              ? `Filters · ${activeFilterCount} active`
              : "Filters and sorting";
            const filters = document.createElement("div");
            filters.className = "language-filters";
            filters.append(
              field("Status", statusInput),
              field("Tag", tagInput),
              field("Sort", sortSelect),
              homonymLabel,
              filterLists,
            );
            const filterActions = document.createElement("div");
            filterActions.className = "language-filter-actions";
            const filterHint = document.createElement("span");
            filterHint.className = "language-status";
            filterHint.textContent = "Use filters to narrow the working set.";
            const clearFilters = button("Clear filters", "language-button secondary", clearLexiconFilters);
            clearFilters.disabled = activeFilterCount === 0;
            filterActions.append(filterHint, clearFilters);
            filters.append(filterActions);
            filterPanel.append(filterSummary, filters);
            lexiconPanel.append(filterPanel);
          }
          if (error) {
            const message = document.createElement("p");
            message.className = "language-status error";
            message.setAttribute("role", "alert");
            message.textContent = error;
            lexiconPanel.append(message);
          } else if (!selectedLanguage) {
            const empty = emptyMessage("Select a language to view its lexicon.");
            lexiconPanel.append(empty);
          } else if (lexiconLoading) {
            const loading = emptyMessage("Loading lexicon…");
            loading.classList.add("language-loading");
            lexiconPanel.append(loading);
          } else if (records.length === 0) {
            const filtered = Boolean(search || statusFilter || tagFilter || homonymsOnly);
            lexiconPanel.append(
              emptyState(
                filtered ? "No words match these filters." : "No words yet.",
                filtered
                  ? button("Clear filters", "language-button secondary", clearLexiconFilters)
                  : button("Add word", "language-button", () => {
                      editing = null;
                      editorOpen = true;
                      draft = emptyLexeme();
                      homonymCount = 0;
                      render();
                      root.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
                    }),
              ),
            );
          } else {
            const resultSummary = document.createElement("p");
            resultSummary.className = "language-results";
            resultSummary.setAttribute("role", "status");
            const firstResult = page * 50 + 1;
            const lastResult = page * 50 + records.length;
            resultSummary.textContent = `Showing ${firstResult}–${lastResult}${hasNextPage ? "+" : ""} words`;
            lexiconPanel.append(resultSummary);
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of records) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "language-item lexeme-row";
              rowButton.setAttribute("aria-label", `Edit ${record.value.lemma || "word"}`);
              const lemma = document.createElement("strong");
              lemma.textContent = record.value.lemma;
              const part = document.createElement("small");
              part.className = "lexeme-part";
              part.textContent = record.value.partOfSpeech || "—";
              const meaning = document.createElement("span");
              meaning.className = "lexeme-meaning";
              meaning.textContent = firstGloss(record.value) || "No gloss yet";
              const status = document.createElement("small");
              status.className = "lexeme-status";
              status.textContent = [record.value.status, record.value.tags[0]].filter(Boolean).join(" · ") || "—";
              rowButton.append(lemma, part, meaning, status);
              rowButton.onclick = () => {
                editing = record;
                editorOpen = true;
                draft = normalizeLexeme(record.value);
                void refreshHomonyms(draft.lemma).then(() => {
                  render();
                  root.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
                });
              };
              item.append(rowButton);
              list.append(item);
            }
            lexiconPanel.append(list);
            if (page > 0 || hasNextPage) {
              const paging = document.createElement("div");
              paging.className = "language-actions";
              const previous = button("Previous", "language-button secondary", () => {
                page = Math.max(0, page - 1);
                void loadRecords();
              });
              previous.disabled = page === 0;
              const next = button("Next", "language-button secondary", () => {
                page += 1;
                void loadRecords();
              });
              next.disabled = !hasNextPage;
              paging.append(previous, next);
              lexiconPanel.append(paging);
            }
          }
          root.append(...panels);
          element.replaceChildren(root);
          restoreFocus();
        }

        render();
        return () => {
          cancelled = true;
          request += 1;
          if (searchTimer !== null) window.clearTimeout(searchTimer);
          clearOverviewAutosave();
          element.replaceChildren();
        };
      },
    },
  ],
};
