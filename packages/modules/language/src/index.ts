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

const manifest = manifestJson as unknown as ModuleManifest;

function field(label: string, control: HTMLElement) {
  const wrapper = document.createElement("label");
  wrapper.className = "language-field";
  const title = document.createElement("span");
  title.textContent = label;
  wrapper.append(title, control);
  return wrapper;
}

function input(name: string, value = "", list?: string) {
  const control = document.createElement("input");
  control.name = name;
  control.value = value;
  if (list) control.setAttribute("list", list);
  return control;
}

function textarea(name: string, value = "", rows = 3) {
  const control = document.createElement("textarea");
  control.name = name;
  control.value = value;
  control.rows = rows;
  return control;
}

function button(label: string, className: string, onclick: () => void) {
  const control = document.createElement("button");
  control.type = "button";
  control.className = className;
  control.textContent = label;
  control.onclick = onclick;
  return control;
}

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

        function groupHead(title: string, add: () => void) {
          const head = document.createElement("div");
          head.className = "language-group-head";
          const heading = document.createElement("h3");
          heading.textContent = title;
          head.append(heading, button("Add", "language-button secondary", add));
          return head;
        }

        function row(fields: HTMLElement[], remove: () => void) {
          const wrap = document.createElement("div");
          wrap.className = "language-inline";
          wrap.append(...fields, button("Remove", "language-button secondary language-danger", remove));
          return wrap;
        }

        function replaceEditor(current: HTMLElement, next: HTMLElement) {
          current.replaceWith(next);
          next.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
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
            await loadRecords();
          } catch (cause) {
            render(cause instanceof Error ? cause.message : String(cause));
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
                editing = null;
                editorOpen = false;
                draft = emptyLexeme();
                search = "";
                statusFilter = "";
                tagFilter = "";
                sort = "lemma";
                homonymsOnly = false;
                page = 0;
                void loadRecords();
              };
              item.append(languageButton);
              languagesList.append(item);
            }
            if (!selectedLanguage && languages[0]) {
              selectedLanguage = languages[0];
              void loadRecords();
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
