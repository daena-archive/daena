import type {
  DaenaModule,
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
import {
  emptyGrammarTopic,
  grammarLinkMarkup,
  grammarMarkdownToHtml,
  GRAMMAR_SECTIONS,
  groupGrammarTopics,
  normalizeGrammarTopic,
  serializeGrammarTopic,
  type GrammarSectionId,
  type GrammarTopic,
} from "./grammar";
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

type Pane = "lexicon" | "sounds" | "writing" | "grammar" | "forms" | "samples";

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
        let searchTimer: number | null = null;
        let pane: Pane = "lexicon";
        let phonemes: ModuleRecord<PhonemeValue>[] = [];
        let phonemeEditing: ModuleRecord<PhonemeValue> | null = null;
        let phonemeEditorOpen = false;
        let phonemeDraft: PhonemeValue = emptyPhoneme();
        let phonologyRecord: ModuleRecord<PhonologyNotes> | null = null;
        let phonologyDraft: PhonologyNotes = emptyPhonologyNotes();
        let orthographies: ModuleRecord<OrthographyValue>[] = [];
        let orthographyEditing: ModuleRecord<OrthographyValue> | null = null;
        let orthographyEditorOpen = false;
        let orthographyDraft: OrthographyValue = emptyOrthography();
        let grammarTopics: ModuleRecord<GrammarTopic>[] = [];
        let grammarEditing: ModuleRecord<GrammarTopic> | null = null;
        let grammarEditorOpen = false;
        let grammarDraft: GrammarTopic = emptyGrammarTopic();
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
        let focusName = "";
        let focusOffset = 0;

        const root = document.createElement("section");
        root.className = "language-workspace";
        const style = document.createElement("style");
        style.textContent = `
          .language-workspace{display:grid;grid-template-columns:minmax(200px,240px) minmax(0,1fr);gap:14px;height:100%;min-height:0;color:var(--ink)}
          .language-panel{display:flex;flex-direction:column;min-width:0;min-height:0;overflow:auto;border:1px solid var(--line);border-radius:14px;background:var(--surface);padding:18px 18px 20px;box-shadow:var(--shadow-sm,0 2px 8px rgba(38,42,33,.05))}
          .language-panel h2,.language-panel h3{margin:0;font-family:var(--font-display);font-weight:500}
          .language-panel h2{font-size:24px;line-height:1.15}.language-panel h3{font-size:16px;line-height:1.3}
          .language-list,.lexeme-list{display:grid;gap:6px;margin:12px 0 0;padding:0;list-style:none}
          .language-list button{width:100%;padding:10px 12px;border:1px solid #ebe7de;border-radius:9px;background:var(--surface);color:inherit;text-align:left;cursor:pointer;box-shadow:0 1px 2px rgba(38,42,33,.03)}
          .language-list button:hover{border-color:#e5d8c6;background:var(--surface-muted)}
          .language-list button[aria-current=page]{border-color:#d8c3a5;background:var(--surface-muted);box-shadow:inset 3px 0 var(--accent),0 1px 2px rgba(38,42,33,.03);color:var(--ink)}
          .language-toolbar{display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap}
          .language-toolbar-actions{display:flex;flex-wrap:wrap;gap:8px}
          .language-filters{display:grid;grid-template-columns:minmax(160px,1.5fr) repeat(3,minmax(110px,.75fr));gap:10px 12px;align-items:end;margin-top:14px}
          .language-filters .language-check{grid-column:1/-1;padding:2px 0 0}
          .language-search,.language-filters input,.language-filters select,.language-field input,.language-field textarea,.language-field select{box-sizing:border-box;width:100%;min-width:0;padding:9px 10px;border:1px solid var(--line);border-radius:8px;background:var(--surface);color:var(--ink);font:inherit}
          .language-field textarea{min-height:4.5em;resize:vertical}
          .language-check{display:flex;align-items:center;gap:8px;color:var(--ink-soft);font-size:12px}
          .language-tabs{position:sticky;top:0;z-index:1;display:flex;flex-wrap:wrap;gap:6px;margin:0 0 8px;padding:0 0 12px;background:var(--surface)}
          .language-tabs button{padding:7px 12px;border:1px solid var(--line);border-radius:999px;background:transparent;color:var(--ink-soft);cursor:pointer}
          .language-tabs button:hover{border-color:#d8c3a5;color:var(--ink);background:var(--surface-muted)}
          .language-tabs button[aria-current=page]{border-color:var(--accent-dark);background:var(--surface-muted);color:var(--accent-dark)}
          .language-chart-wrap{overflow-x:auto;margin:8px 0 4px}
          .language-chart,.paradigm-preview{width:100%;border-collapse:collapse;font-size:12px}
          .language-chart th,.language-chart td,.paradigm-preview th,.paradigm-preview td{border:1px solid var(--line);padding:8px;text-align:center;min-width:52px}
          .paradigm-preview th,.paradigm-preview td{text-align:left}
          .language-chart th,.paradigm-preview th{background:var(--surface-muted);font-weight:600;color:var(--ink-soft)}
          .language-chart button{border:0;background:transparent;color:inherit;font:inherit;cursor:pointer}
          .language-chart .is-empty{color:var(--ink-faint)}
          .grammar-preview,.sample-block{padding:14px;border:1px solid var(--line);border-radius:10px;background:var(--surface-muted);font-size:13px;line-height:1.55}
          .grammar-preview h1,.grammar-preview h2,.grammar-preview h3,.sample-block h3{margin:0 0 8px;font-family:var(--font-display);font-weight:500}
          .grammar-preview p,.grammar-preview ul,.grammar-preview ol{margin:0 0 8px}
          .grammar-ref,.sample-ref{padding:0;border:0;border-bottom:1px dotted var(--accent-dark);background:transparent;color:var(--accent-dark);font:inherit;cursor:pointer}
          .grammar-nav{display:grid;gap:12px;margin-top:14px}
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
          .lexeme-row{grid-template-columns:minmax(0,1.1fr) minmax(0,.55fr) minmax(0,1.4fr) minmax(0,.55fr)}
          .language-item:hover,.lexeme-row:hover{border-color:#e5d8c6;background:var(--surface-muted)}
          .language-item strong,.lexeme-row strong{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
          .language-item small,.lexeme-row small{color:var(--ink-faint)}
          .language-item span,.lexeme-row span{min-width:0;color:var(--ink-soft);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
          .language-button{padding:8px 12px;border:1px solid var(--accent-dark);border-radius:8px;background:var(--accent-dark);color:#fff;cursor:pointer}
          .language-button:hover{filter:brightness(1.06)}
          .language-button.secondary{background:transparent;color:var(--accent-dark)}
          .language-button.secondary:hover{background:var(--surface-muted)}
          .language-button:disabled{opacity:.45;cursor:not-allowed;filter:none}
          .language-button:focus-visible,.language-tabs button:focus-visible,.language-list button:focus-visible,.language-item:focus-visible,.lexeme-row:focus-visible,.grammar-ref:focus-visible,.sample-ref:focus-visible{outline:3px solid rgba(180,119,63,.24);outline-offset:2px}
          .language-empty,.language-status{margin:0;color:var(--ink-soft);font-size:12px;line-height:1.6}
          .language-status.error{color:#a14f42}
          .language-empty-card{display:grid;gap:12px;justify-items:start;margin:18px 0;padding:20px;border:1px dashed var(--line);border-radius:12px;background:var(--surface-muted)}
          .language-editor{display:grid;gap:14px;margin-top:16px;min-width:0}
          .language-field{display:grid;gap:6px;min-width:0;color:var(--ink-soft);font-size:11px;letter-spacing:.01em}
          .language-actions{position:sticky;bottom:0;display:flex;justify-content:space-between;gap:10px;flex-wrap:wrap;padding:12px 0 2px;background:linear-gradient(180deg,transparent,var(--surface) 10px)}
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
            .language-workspace,.language-filters,.lexeme-row,.language-item{grid-template-columns:1fr}
            .language-item span,.lexeme-row span,.lexeme-row small{white-space:normal}
            .language-inline{flex-direction:column;align-items:stretch}
          }
        `;

        async function loadRecords() {
          if (!selectedLanguage) {
            records = [];
            paradigms = [];
            render();
            return;
          }
          const token = ++request;
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
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
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
          const posList = document.createElement("datalist");
          posList.id = "language-pos";
          posList.append(...PART_OF_SPEECH_SUGGESTIONS.map((item) => new Option(item)));
          const statusList = document.createElement("datalist");
          statusList.id = "language-status";
          statusList.append(...STATUS_SUGGESTIONS.map((item) => new Option(item)));
          lists.append(posList, statusList);
          form.append(lists);
          form.append(
            field("Lemma", input("lemma", draft.lemma)),
            field("Part of speech (optional)", input("partOfSpeech", draft.partOfSpeech, "language-pos")),
            field("Status (optional)", input("status", draft.status, "language-status")),
            field("Tags — comma or line separated (optional)", textarea("tags", draft.tags.join("\n"), 2)),
          );
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
          pronunciations.className = "language-group";
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
          forms.className = "language-group";
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
          senses.className = "language-group";
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
            preview.className = "language-group";
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
          save.textContent = "Save word";
          right.append(save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage) return;
            capture(form);
            const value = normalizeLexeme(draft);
            if (!value.lemma) {
              form.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
              render("Lemma is required.");
              return;
            }
            draft = value;
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
              await loadRecords();
              await refreshHomonyms(draft.lemma);
              render();
            } catch (cause) {
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
          orthographyEditing = null;
          orthographyEditorOpen = false;
          orthographyDraft = emptyOrthography();
          grammarEditing = null;
          grammarEditorOpen = false;
          grammarDraft = emptyGrammarTopic();
          paradigmEditing = null;
          paradigmEditorOpen = false;
          paradigmDraft = emptyParadigm();
          previewStem = "";
          previewLexemeId = "";
          sampleEditing = null;
          sampleEditorOpen = false;
          sampleDraft = emptySample();
        }

        async function loadPane() {
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
            render();
            return;
          }
          const token = ++request;
          try {
            const [inventory, notes] = await Promise.all([
              context.records.list<PhonemeValue>("phonemes", selectedLanguage.id, { limit: 100, sort: "symbol" }),
              context.records.list<PhonologyNotes>("phonology", selectedLanguage.id, { limit: 1 }),
            ]);
            if (!cancelled && token === request) {
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
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
          }
        }

        async function loadWriting() {
          if (!selectedLanguage) {
            orthographies = [];
            render();
            return;
          }
          const token = ++request;
          try {
            const [systems, inventory] = await Promise.all([
              context.records.list<OrthographyValue>("orthographies", selectedLanguage.id, {
                limit: 100,
                sort: "name",
              }),
              context.records.list<PhonemeValue>("phonemes", selectedLanguage.id, { limit: 100, sort: "symbol" }),
            ]);
            if (!cancelled && token === request) {
              orthographies = systems.map((record) => ({ ...record, value: normalizeOrthography(record.value) }));
              phonemes = inventory.map((record) => ({ ...record, value: normalizePhoneme(record.value) }));
              if (orthographyEditing) {
                const current = orthographies.find((record) => record.id === orthographyEditing?.id);
                if (current) orthographyEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
          }
        }

        async function loadGrammar() {
          if (!selectedLanguage) {
            grammarTopics = [];
            records = [];
            render();
            return;
          }
          const token = ++request;
          try {
            const [topics, lexemes] = await Promise.all([
              context.records.list<GrammarTopic>("grammar", selectedLanguage.id, { limit: 100, sort: "title" }),
              context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
            ]);
            if (!cancelled && token === request) {
              grammarTopics = topics.map((record) => ({ ...record, value: normalizeGrammarTopic(record.value) }));
              records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
              if (grammarEditing) {
                const current = grammarTopics.find((record) => record.id === grammarEditing?.id);
                if (current) grammarEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
          }
        }

        async function loadForms() {
          if (!selectedLanguage) {
            paradigms = [];
            records = [];
            render();
            return;
          }
          const token = ++request;
          try {
            const [tables, lexemes] = await Promise.all([
              context.records.list<Paradigm>("paradigms", selectedLanguage.id, { limit: 100, sort: "name" }),
              context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
            ]);
            if (!cancelled && token === request) {
              paradigms = tables.map((record) => ({ ...record, value: normalizeParadigm(record.value) }));
              records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
              if (paradigmEditing) {
                const current = paradigms.find((record) => record.id === paradigmEditing?.id);
                if (current) paradigmEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
          }
        }

        async function loadSamples() {
          if (!selectedLanguage) {
            samples = [];
            records = [];
            render();
            return;
          }
          const token = ++request;
          try {
            const [items, lexemes] = await Promise.all([
              context.records.list<Sample>("samples", selectedLanguage.id, { limit: 100, sort: "title" }),
              context.records.list<LexemeValue>("lexemes", selectedLanguage.id, { limit: 500, sort: "lemma" }),
            ]);
            if (!cancelled && token === request) {
              samples = items.map((record) => ({ ...record, value: normalizeSample(record.value) }));
              records = lexemes.map((record) => ({ ...record, value: normalizeLexeme(record.value) }));
              if (sampleEditing) {
                const current = samples.find((record) => record.id === sampleEditing?.id);
                if (current) sampleEditing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
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
          wrap.className = "language-group";
          const heading = document.createElement("h3");
          heading.textContent = caption;
          wrap.append(heading);
          if (!chart.columns.length) {
            wrap.append(
              emptyMessage(
                "Add place and manner (or height and backness) to place sounds on this chart. Incomplete inventories are allowed.",
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
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const title = document.createElement("h2");
          title.textContent = selectedLanguage ? `${selectedLanguage.name} sounds` : "Sounds";
          const add = button("Add sound", "language-button", () => {
            phonemeEditing = null;
            phonemeEditorOpen = true;
            phonemeDraft = emptyPhoneme();
            render();
          });
          add.disabled = !selectedLanguage;
          toolbar.append(title, add);
          panel.append(toolbar);
          if (!selectedLanguage) {
            panel.append(emptyMessage("Select a language to document its sounds."));
            return;
          }
          if (phonemeEditorOpen) {
            panel.append(phonemeForm(error));
            return;
          }
          const notes = document.createElement("form");
          notes.className = "language-editor";
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
          panel.append(notes);
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
              emptyMessage(
                "No sounds yet. Add consonants and vowels; charts stay empty until place, manner, height, or backness is filled in.",
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
            panel.append(list);
          }
        }

        function renderWriting(panel: HTMLElement, error: string) {
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const title = document.createElement("h2");
          title.textContent = selectedLanguage ? `${selectedLanguage.name} writing` : "Writing";
          const add = button("Add writing system", "language-button", () => {
            orthographyEditing = null;
            orthographyEditorOpen = true;
            orthographyDraft = emptyOrthography();
            render();
          });
          add.disabled = !selectedLanguage;
          toolbar.append(title, add);
          panel.append(toolbar);
          if (orthographyEditorOpen) {
            panel.append(orthographyForm(error));
            return;
          }
          if (error) panel.append(alertMessage(error));
          else if (!selectedLanguage) panel.append(emptyMessage("Select a language to document its writing systems."));
          else if (orthographies.length === 0)
            panel.append(emptyMessage("No writing systems yet. Add one and map graphemes to sounds."));
          else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of orthographies) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "language-item";
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
            panel.append(list);
          }
        }

        function exampleChoices() {
          return records.flatMap((record) =>
            record.value.senses.flatMap((sense) =>
              sense.examples.map((example) => ({
                lexemeId: record.id,
                lemma: record.value.lemma,
                exampleId: example.id,
                text: example.text,
              })),
            ),
          );
        }

        function openLinkedLexeme(lexemeId: string) {
          const target = records.find((record) => record.id === lexemeId);
          pendingLexemeId = lexemeId;
          pane = "lexicon";
          grammarEditorOpen = false;
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

        function bindGrammarRefs(root: HTMLElement) {
          for (const control of root.querySelectorAll<HTMLButtonElement>(".grammar-ref")) {
            control.onclick = () => {
              const lexemeId = control.dataset.lexemeId;
              if (lexemeId) openLinkedLexeme(lexemeId);
            };
          }
        }

        function captureGrammar(form: HTMLFormElement) {
          const data = new FormData(form);
          grammarDraft.title = String(data.get("title") ?? "");
          grammarDraft.section = (String(data.get("section") ?? "other") || "other") as GrammarSectionId;
          grammarDraft.body = String(data.get("body") ?? "");
        }

        function grammarForm(error = "") {
          const form = document.createElement("form");
          form.className = "language-editor";
          const sectionSelect = document.createElement("select");
          sectionSelect.name = "section";
          sectionSelect.setAttribute("aria-label", "Grammar section");
          for (const item of GRAMMAR_SECTIONS) {
            sectionSelect.append(
              new Option(item.label, item.id, item.id === grammarDraft.section, item.id === grammarDraft.section),
            );
          }
          form.append(
            field("Title", input("title", grammarDraft.title)),
            field("Section", sectionSelect),
            field("Notes (Markdown)", textarea("body", grammarDraft.body, 10)),
          );
          const preview = document.createElement("div");
          preview.className = "grammar-preview";
          preview.setAttribute("aria-label", "Markdown preview");
          preview.innerHTML = grammarMarkdownToHtml(grammarDraft.body) || "<p>Preview appears as you write.</p>";
          bindGrammarRefs(preview);
          form.append(preview);
          form.querySelector<HTMLTextAreaElement>("[name=body]")!.oninput = (event) => {
            preview.innerHTML =
              grammarMarkdownToHtml((event.target as HTMLTextAreaElement).value) ||
              "<p>Preview appears as you write.</p>";
            bindGrammarRefs(preview);
          };
          const links = document.createElement("section");
          links.className = "language-group";
          const head = document.createElement("div");
          head.className = "language-group-head";
          const heading = document.createElement("h3");
          heading.textContent = "Links to words";
          head.append(heading);
          links.append(head);
          for (const [index, item] of grammarDraft.links.entries()) {
            const lexeme = records.find((record) => record.id === item.lexemeId);
            const label = item.label || lexeme?.value.lemma || "Missing word";
            links.append(
              row([field("Linked word", input(`link-label-${index}`, label))], () => {
                captureGrammar(form);
                grammarDraft.links.splice(index, 1);
                replaceEditor(form, grammarForm(error), "[name=title]");
              }),
            );
          }
          const addLexeme = document.createElement("select");
          addLexeme.setAttribute("aria-label", "Link a lexeme");
          addLexeme.append(new Option("Link a word…", ""));
          for (const record of records) {
            addLexeme.append(new Option(record.value.lemma, record.id));
          }
          addLexeme.onchange = () => {
            const lexemeId = addLexeme.value;
            const lexeme = records.find((record) => record.id === lexemeId);
            if (!lexemeId || !lexeme) return;
            captureGrammar(form);
            const link = {
              id: crypto.randomUUID(),
              kind: "lexeme" as const,
              lexemeId,
              label: lexeme.value.lemma,
            };
            grammarDraft.links.push(link);
            grammarDraft.body = `${grammarDraft.body.trim()}\n\n${grammarLinkMarkup(link)}\n`;
            replaceEditor(form, grammarForm(error), "[name=title]");
          };
          const addExample = document.createElement("select");
          addExample.setAttribute("aria-label", "Link an example");
          addExample.append(new Option("Link an example…", ""));
          for (const example of exampleChoices()) {
            addExample.append(
              new Option(`${example.lemma}: ${example.text}`, `${example.lexemeId}:${example.exampleId}`),
            );
          }
          addExample.onchange = () => {
            const [lexemeId, exampleId] = addExample.value.split(":");
            const example = exampleChoices().find((item) => item.lexemeId === lexemeId && item.exampleId === exampleId);
            if (!lexemeId || !exampleId || !example) return;
            captureGrammar(form);
            const link = {
              id: crypto.randomUUID(),
              kind: "example" as const,
              lexemeId,
              exampleId,
              label: example.text,
            };
            grammarDraft.links.push(link);
            grammarDraft.body = `${grammarDraft.body.trim()}\n\n${grammarLinkMarkup(link)}\n`;
            replaceEditor(form, grammarForm(error), "[name=title]");
          };
          links.append(addLexeme, addExample);
          form.append(links);
          if (error) form.append(alertMessage(error));
          const actions = document.createElement("div");
          actions.className = "language-actions";
          const left = document.createElement("span");
          if (grammarEditing) {
            left.append(
              button("Delete", "language-button secondary language-danger", async () => {
                if (!selectedLanguage || !grammarEditing || !window.confirm(`Delete “${grammarEditing.value.title}”?`))
                  return;
                try {
                  await context.records.delete("grammar", grammarEditing.id, selectedLanguage.id, {
                    expectedRevision: grammarEditing.revision,
                    requestId: crypto.randomUUID(),
                  });
                  grammarEditing = null;
                  grammarEditorOpen = false;
                  grammarDraft = emptyGrammarTopic();
                  await loadGrammar();
                } catch (cause) {
                  render(cause instanceof Error ? cause.message : String(cause));
                }
              }),
            );
          }
          const right = document.createElement("span");
          right.append(
            button("Cancel", "language-button secondary", () => {
              grammarEditing = null;
              grammarEditorOpen = false;
              grammarDraft = emptyGrammarTopic();
              render();
            }),
          );
          const save = document.createElement("button");
          save.type = "submit";
          save.className = "language-button";
          save.textContent = "Save topic";
          right.append(save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage) return;
            captureGrammar(form);
            const value = normalizeGrammarTopic(grammarDraft);
            if (!value.title) {
              form.querySelector<HTMLInputElement>("[name=title]")?.focus();
              render("Title is required.");
              return;
            }
            grammarDraft = value;
            try {
              const payload = serializeGrammarTopic(value);
              if (grammarEditing) {
                const updated = await context.records.update(
                  "grammar",
                  grammarEditing.id,
                  selectedLanguage.id,
                  payload,
                  { expectedRevision: grammarEditing.revision, requestId: crypto.randomUUID() },
                );
                grammarEditing = { ...updated, value: normalizeGrammarTopic(updated.value) };
              } else {
                const created = await context.records.create("grammar", selectedLanguage.id, payload, {
                  requestId: crypto.randomUUID(),
                });
                grammarEditing = { ...created, value: normalizeGrammarTopic(created.value) };
              }
              grammarEditorOpen = true;
              grammarDraft = grammarEditing.value;
              await loadGrammar();
            } catch (cause) {
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          return form;
        }

        function renderGrammar(panel: HTMLElement, error: string) {
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const title = document.createElement("h2");
          title.textContent = selectedLanguage ? `${selectedLanguage.name} grammar` : "Grammar";
          const add = button("Add topic", "language-button", () => {
            grammarEditing = null;
            grammarEditorOpen = true;
            grammarDraft = emptyGrammarTopic("word-order");
            render();
          });
          add.disabled = !selectedLanguage;
          toolbar.append(title, add);
          panel.append(toolbar);
          if (grammarEditorOpen) {
            panel.append(grammarForm(error));
            return;
          }
          if (error) panel.append(alertMessage(error));
          else if (!selectedLanguage) panel.append(emptyMessage("Select a language to document its grammar."));
          else {
            const nav = document.createElement("div");
            nav.className = "grammar-nav";
            for (const group of groupGrammarTopics(grammarTopics)) {
              const block = document.createElement("section");
              block.className = "language-group";
              const head = document.createElement("div");
              head.className = "language-group-head";
              const heading = document.createElement("h3");
              heading.textContent = group.label;
              head.append(
                heading,
                button("Add", "language-button secondary", () => {
                  grammarEditing = null;
                  grammarEditorOpen = true;
                  grammarDraft = emptyGrammarTopic(group.id);
                  render();
                }),
              );
              block.append(head);
              if (group.topics.length === 0) {
                block.append(emptyMessage("No notes in this section yet."));
              } else {
                const list = document.createElement("ul");
                list.className = "lexeme-list";
                for (const record of group.topics) {
                  const item = document.createElement("li");
                  const rowButton = document.createElement("button");
                  rowButton.type = "button";
                  rowButton.className = "language-item";
                  const topicTitle = document.createElement("strong");
                  topicTitle.textContent = record.value.title;
                  const preview = document.createElement("span");
                  preview.textContent = record.value.body.trim().split("\n")[0] || "No notes yet";
                  rowButton.append(topicTitle, preview);
                  rowButton.onclick = () => {
                    grammarEditing = record;
                    grammarEditorOpen = true;
                    grammarDraft = normalizeGrammarTopic(record.value);
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
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const title = document.createElement("h2");
          title.textContent = selectedLanguage ? `${selectedLanguage.name} forms` : "Forms";
          const add = button("Add paradigm", "language-button", () => {
            paradigmEditing = null;
            paradigmEditorOpen = true;
            paradigmDraft = emptyParadigm();
            previewStem = "";
            previewLexemeId = "";
            render();
          });
          add.disabled = !selectedLanguage;
          toolbar.append(title, add);
          panel.append(toolbar);
          if (paradigmEditorOpen) {
            panel.append(paradigmForm(error));
            return;
          }
          if (error) panel.append(alertMessage(error));
          else if (!selectedLanguage) panel.append(emptyMessage("Select a language to document its paradigms."));
          else if (paradigms.length === 0) {
            panel.append(
              emptyMessage("No paradigms yet. Add an inflection or derivation table, then preview generated forms."),
            );
          } else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of paradigms) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "language-item";
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
            panel.append(list);
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
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const title = document.createElement("h2");
          title.textContent = selectedLanguage ? `${selectedLanguage.name} samples` : "Samples";
          const add = button("Add sample", "language-button", () => {
            sampleEditing = null;
            sampleEditorOpen = true;
            sampleDraft = emptySample("sentence");
            render();
          });
          add.disabled = !selectedLanguage;
          toolbar.append(title, add);
          panel.append(toolbar);
          if (sampleEditorOpen) {
            panel.append(sampleForm(error));
            return;
          }
          if (error) panel.append(alertMessage(error));
          else if (!selectedLanguage)
            panel.append(emptyMessage("Select a language to collect sample sentences and paragraphs."));
          else {
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
                button("Add", "language-button secondary", () => {
                  sampleEditing = null;
                  sampleEditorOpen = true;
                  sampleDraft = emptySample(group.id);
                  render();
                }),
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
            languageButton.textContent = language.name;
            if (selectedLanguage?.id === language.id) languageButton.setAttribute("aria-current", "page");
            languageButton.onclick = () => {
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
          if (languageSummaries.length === 0) {
            list.replaceChildren(emptyMessage("Create a Language from the main workspace first."));
          } else if (visible.length === 0) {
            list.replaceChildren(emptyMessage("No languages match that filter."));
          }
        }

        function render(error = "") {
          if (cancelled) return;
          rememberFocus();
          root.replaceChildren(style);
          const languagesPanel = document.createElement("aside");
          languagesPanel.className = "language-panel";
          const languagesTitle = document.createElement("h2");
          languagesTitle.textContent = "Languages";
          languagesPanel.append(languagesTitle);
          const languageSearch = input("languageQuery", languageQuery);
          languageSearch.type = "search";
          languageSearch.oninput = () => {
            languageQuery = languageSearch.value;
            fillLanguageList(languagesList);
          };
          languagesPanel.append(field("Filter languages", languageSearch));
          const languagesList = document.createElement("ul");
          languagesList.className = "language-list";
          if (languageSummaries.length) fillLanguageList(languagesList);
          void context.entities.list({ type: "language", limit: 500 }).then((languages) => {
            if (cancelled) return;
            languageSummaries = languages;
            fillLanguageList(languagesList);
            if (!selectedLanguage && languages.length) {
              selectedLanguage = languages.find((language) => language.id === context.focusEntityId) ?? languages[0];
              void loadPane();
            }
          });
          languagesPanel.append(languagesList);

          const lexiconPanel = document.createElement("main");
          lexiconPanel.className = "language-panel";
          const tabs = document.createElement("div");
          tabs.className = "language-tabs";
          tabs.setAttribute("role", "tablist");
          tabs.setAttribute("aria-label", "Language workspace");
          for (const [id, label] of [
            ["lexicon", "Lexicon"],
            ["sounds", "Sounds"],
            ["writing", "Writing"],
            ["grammar", "Grammar"],
            ["forms", "Forms"],
            ["samples", "Samples"],
          ] as const) {
            const tab = button(label, "", () => {
              pane = id;
              resetEditors();
              void loadPane();
            });
            tab.setAttribute("role", "tab");
            if (pane === id) tab.setAttribute("aria-current", "page");
            tabs.append(tab);
          }
          lexiconPanel.append(tabs);
          if (pane === "sounds") {
            renderSounds(lexiconPanel, error);
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "writing") {
            renderWriting(lexiconPanel, error);
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "grammar") {
            renderGrammar(lexiconPanel, error);
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "forms") {
            renderForms(lexiconPanel, error);
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (pane === "samples") {
            renderSamples(lexiconPanel, error);
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          const toolbar = document.createElement("div");
          toolbar.className = "language-toolbar";
          const title = document.createElement("h2");
          title.textContent = selectedLanguage ? `${selectedLanguage.name} lexicon` : "Lexicon";
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
            lexiconPanel.replaceChildren(toolbar, editForm());
            lexiconPanel.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
          });
          add.disabled = !selectedLanguage;
          const exportButton = button("Export JSON", "language-button secondary", () => void exportLexicon());
          exportButton.disabled = !selectedLanguage;
          const importButton = button("Import JSON", "language-button secondary", () => file.click());
          importButton.disabled = !selectedLanguage;
          actions.append(file, importButton, exportButton, add);
          toolbar.append(title, actions);
          lexiconPanel.append(toolbar);
          if (editorOpen) {
            lexiconPanel.append(editForm(error));
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
            restoreFocus();
            return;
          }
          if (selectedLanguage) {
            const filters = document.createElement("div");
            filters.className = "language-filters";
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
            filters.append(
              field("Search lexicon", searchInput),
              field("Status", statusInput),
              field("Tag", tagInput),
              field("Sort", sortSelect),
              homonymLabel,
              filterLists,
            );
            lexiconPanel.append(filters);
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
          } else if (records.length === 0) {
            const filtered = Boolean(search || statusFilter || tagFilter || homonymsOnly);
            lexiconPanel.append(
              emptyState(
                filtered ? "No words match these filters." : "No words yet.",
                filtered
                  ? undefined
                  : button("Add word", "language-button", () => {
                      editing = null;
                      editorOpen = true;
                      draft = emptyLexeme();
                      homonymCount = 0;
                      lexiconPanel.replaceChildren(toolbar, editForm());
                      lexiconPanel.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
                    }),
              ),
            );
          } else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of records) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "language-item lexeme-row";
              const lemma = document.createElement("strong");
              lemma.textContent = record.value.lemma;
              const part = document.createElement("small");
              part.textContent = record.value.partOfSpeech || "—";
              const meaning = document.createElement("span");
              meaning.textContent = firstGloss(record.value) || "No gloss yet";
              const status = document.createElement("small");
              status.textContent = [record.value.status, record.value.tags[0]].filter(Boolean).join(" · ") || "—";
              rowButton.append(lemma, part, meaning, status);
              rowButton.onclick = () => {
                editing = record;
                editorOpen = true;
                draft = normalizeLexeme(record.value);
                void refreshHomonyms(draft.lemma).then(() => {
                  lexiconPanel.replaceChildren(toolbar, editForm());
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
          root.append(languagesPanel, lexiconPanel);
          element.replaceChildren(root);
          restoreFocus();
        }

        render();
        return () => {
          cancelled = true;
          request += 1;
          if (searchTimer !== null) window.clearTimeout(searchTimer);
          element.replaceChildren();
        };
      },
    },
  ],
};
