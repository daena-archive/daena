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
import { alertMessage, button, emptyMessage, field, groupHead, input, replaceEditor, row, textarea } from "./ui";

const manifest = manifestJson as unknown as ModuleManifest;

type Pane = "lexicon" | "sounds" | "writing";

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

        const root = document.createElement("section");
        root.className = "language-workspace";
        const style = document.createElement("style");
        style.textContent = `
          .language-workspace{display:grid;grid-template-columns:minmax(190px,260px) minmax(0,1fr);gap:18px;padding:4px;color:var(--ink)}
          .language-panel{border:1px solid var(--line);border-radius:14px;background:var(--paper);padding:18px;box-shadow:0 8px 24px rgba(62,42,25,.05)}
          .language-panel h2,.language-panel h3{margin:0;font-family:var(--font-display);font-weight:500}.language-panel h2{font-size:25px}.language-panel h3{font-size:19px}
          .language-list,.lexeme-list{display:grid;gap:7px;margin:14px 0 0;padding:0;list-style:none}.language-list button,.lexeme-row{width:100%;border:1px solid transparent;border-radius:8px;background:transparent;color:inherit;text-align:left;cursor:pointer}
          .language-list button{padding:9px}.language-list button[aria-current=page]{border-color:var(--line);background:var(--paper-strong);color:var(--accent-dark)}
          .language-toolbar{display:flex;align-items:center;justify-content:space-between;gap:12px;flex-wrap:wrap}
          .language-toolbar-actions{display:flex;flex-wrap:wrap;gap:8px}
          .language-filters{display:grid;grid-template-columns:minmax(140px,1.4fr) repeat(3,minmax(90px,.7fr)) auto;gap:8px;margin-top:14px}
          .language-search,.language-filters input,.language-filters select{box-sizing:border-box;width:100%;padding:9px 10px;border:1px solid var(--line);border-radius:8px;background:var(--paper);color:inherit;font:inherit}
          .language-check{display:flex;align-items:center;gap:6px;color:var(--ink-soft);font-size:11px;white-space:nowrap}
          .language-tabs{display:flex;gap:6px;margin:0 0 14px}
          .language-tabs button{padding:7px 11px;border:1px solid var(--line);border-radius:999px;background:transparent;color:inherit;cursor:pointer}
          .language-tabs button[aria-current=page]{border-color:var(--accent-dark);background:var(--paper-strong);color:var(--accent-dark)}
          .language-chart{width:100%;border-collapse:collapse;margin:12px 0;font-size:12px}
          .language-chart th,.language-chart td{border:1px solid var(--line);padding:8px;text-align:center;min-width:52px}
          .language-chart th{background:var(--paper-strong);font-weight:600;color:var(--ink-soft)}
          .language-chart button{border:0;background:transparent;color:inherit;font:inherit;cursor:pointer}
          .language-chart .is-empty{color:var(--ink-faint)}
          .lexeme-row{display:grid;grid-template-columns:minmax(100px,1.1fr) minmax(70px,.5fr) minmax(120px,1.3fr) minmax(70px,.5fr);gap:12px;padding:10px;border-bottom:1px solid var(--line);border-radius:0}.lexeme-row:hover{background:var(--paper-strong)}.lexeme-row small{color:var(--ink-faint)}
          .language-button{padding:8px 12px;border:1px solid var(--accent-dark);border-radius:8px;background:var(--accent-dark);color:white;cursor:pointer}.language-button.secondary{background:transparent;color:var(--accent-dark)}
          .language-empty,.language-status{margin:18px 0;color:var(--ink-soft);font-size:12px;line-height:1.6}.language-status.error{color:#a14f42}
          .language-editor{display:grid;gap:14px;margin-top:18px}.language-field{display:grid;gap:6px;color:var(--ink-soft);font-size:11px}.language-field input,.language-field textarea,.language-field select{box-sizing:border-box;width:100%;padding:9px;border:1px solid var(--line);border-radius:8px;background:var(--paper);color:var(--ink);font:inherit}
          .language-actions{display:flex;justify-content:space-between;gap:10px;flex-wrap:wrap}.language-actions span{display:flex;gap:8px;flex-wrap:wrap}.language-danger{border-color:#a14f42!important;color:#a14f42!important}
          .language-group{display:grid;gap:10px;padding:12px;border:1px solid var(--line);border-radius:10px;background:var(--paper-strong,transparent)}
          .language-group-head{display:flex;justify-content:space-between;align-items:center;gap:8px}
          .language-inline{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:8px}
          .file-input{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)}
          @media(max-width:760px){.language-workspace,.language-filters,.lexeme-row{grid-template-columns:1fr}.lexeme-row small{display:block}}
        `;

        async function loadRecords() {
          if (!selectedLanguage) {
            records = [];
            render();
            return;
          }
          const token = ++request;
          try {
            const result = await context.records.list<LexemeValue>("lexemes", selectedLanguage.id, {
              query: search || undefined,
              status: statusFilter || undefined,
              tag: tagFilter || undefined,
              sort,
              homonymsOnly: homonymsOnly || undefined,
              limit: 51,
              offset: page * 50,
            });
            if (!cancelled && token === request) {
              hasNextPage = result.length > 50;
              records = result.slice(0, 50).map((record) => ({
                ...record,
                value: normalizeLexeme(record.value),
              }));
              if (editing) {
                const current = records.find((record) => record.id === editing?.id);
                if (current) editing = current;
              }
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
          }
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
                const lemma = draft.lemma;
                editing = null;
                editorOpen = true;
                draft = { ...emptyLexeme(), lemma };
                homonymCount = 0;
                render();
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
        }

        async function loadPane() {
          if (pane === "sounds") return loadSounds();
          if (pane === "writing") return loadWriting();
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
          wrap.append(table);
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
              rowButton.className = "lexeme-row";
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
              rowButton.className = "lexeme-row";
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

        function render(error = "") {
          if (cancelled) return;
          root.replaceChildren(style);
          const languagesPanel = document.createElement("aside");
          languagesPanel.className = "language-panel";
          const languagesTitle = document.createElement("h2");
          languagesTitle.textContent = "Languages";
          languagesPanel.append(languagesTitle);
          const languagesList = document.createElement("ul");
          languagesList.className = "language-list";
          void context.entities.list({ type: "language", limit: 500 }).then((languages) => {
            if (cancelled) return;
            languagesList.replaceChildren();
            for (const language of languages) {
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
              languagesList.append(item);
            }
            if (!selectedLanguage && languages[0]) {
              selectedLanguage = languages[0];
              void loadPane();
            }
            if (languages.length === 0) {
              const empty = document.createElement("p");
              empty.className = "language-empty";
              empty.textContent = "Create a Language from the main workspace first.";
              languagesList.replaceChildren(empty);
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
            return;
          }
          if (pane === "writing") {
            renderWriting(lexiconPanel, error);
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
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
            return;
          }
          if (selectedLanguage) {
            const filters = document.createElement("div");
            filters.className = "language-filters";
            const searchInput = input("search", search);
            searchInput.className = "language-search";
            searchInput.type = "search";
            searchInput.setAttribute("aria-label", "Search lexicon");
            searchInput.oninput = () => {
              search = searchInput.value;
              scheduleLoad();
            };
            const statusInput = input("statusFilter", statusFilter, "language-status");
            statusInput.setAttribute("aria-label", "Filter by status");
            statusInput.oninput = () => {
              statusFilter = statusInput.value.trim();
              scheduleLoad();
            };
            const tagInput = input("tagFilter", tagFilter);
            tagInput.setAttribute("aria-label", "Filter by tag");
            tagInput.oninput = () => {
              tagFilter = tagInput.value.trim();
              scheduleLoad();
            };
            const sortSelect = document.createElement("select");
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
            filterLists.id = "language-status";
            filterLists.append(...STATUS_SUGGESTIONS.map((item) => new Option(item)));
            filters.append(searchInput, statusInput, tagInput, sortSelect, homonymLabel, filterLists);
            lexiconPanel.append(filters);
          }
          if (error) {
            const message = document.createElement("p");
            message.className = "language-status error";
            message.setAttribute("role", "alert");
            message.textContent = error;
            lexiconPanel.append(message);
          } else if (!selectedLanguage) {
            const empty = document.createElement("p");
            empty.className = "language-empty";
            empty.textContent = "Select a language to view its lexicon.";
            lexiconPanel.append(empty);
          } else if (records.length === 0) {
            const empty = document.createElement("p");
            empty.className = "language-empty";
            empty.textContent =
              search || statusFilter || tagFilter || homonymsOnly
                ? "No words match these filters."
                : "No words yet. Add the first word.";
            lexiconPanel.append(empty);
          } else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of records) {
              const item = document.createElement("li");
              const rowButton = document.createElement("button");
              rowButton.type = "button";
              rowButton.className = "lexeme-row";
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
