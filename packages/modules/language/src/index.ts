import type {
  DaenaModule,
  EntitySummary,
  ModuleContext,
  ModuleManifest,
  ModuleRecord,
  UUID,
} from "../../../module-api/src/index";
import manifestJson from "../manifest.json";

const manifest = manifestJson as unknown as ModuleManifest;

type LexemeValue = {
  lemma: string;
  partOfSpeech?: string;
  meanings: string[];
  pronunciation?: string;
  notes?: string;
  example?: { text: string; translation?: string };
};

function field(label: string, name: string, value = "", multiline = false) {
  const wrapper = document.createElement("label");
  wrapper.className = "language-field";
  const title = document.createElement("span");
  title.textContent = label;
  const input = multiline ? document.createElement("textarea") : document.createElement("input");
  input.name = name;
  input.value = value;
  if (input instanceof HTMLTextAreaElement) input.rows = 3;
  wrapper.append(title, input);
  return wrapper;
}

function read(form: HTMLFormElement, name: string) {
  return String(new FormData(form).get(name) ?? "").trim();
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
        let draft: LexemeValue | null = null;
        let search = "";
        let page = 0;
        let hasNextPage = false;
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
          .language-toolbar{display:flex;align-items:center;justify-content:space-between;gap:12px}.language-search{width:100%;box-sizing:border-box;margin-top:14px;padding:9px 10px;border:1px solid var(--line);border-radius:8px;background:var(--paper);color:inherit}
          .lexeme-row{display:grid;grid-template-columns:minmax(100px,1fr) minmax(90px,.6fr) minmax(140px,1.4fr);gap:12px;padding:10px;border-bottom:1px solid var(--line);border-radius:0}.lexeme-row:hover{background:var(--paper-strong)}.lexeme-row small{color:var(--ink-faint)}
          .language-button{padding:8px 12px;border:1px solid var(--accent-dark);border-radius:8px;background:var(--accent-dark);color:white;cursor:pointer}.language-button.secondary{background:transparent;color:var(--accent-dark)}
          .language-empty,.language-status{margin:18px 0;color:var(--ink-soft);font-size:12px;line-height:1.6}.language-status.error{color:#a14f42}
          .language-editor{display:grid;gap:12px;margin-top:18px}.language-field{display:grid;gap:6px;color:var(--ink-soft);font-size:11px}.language-field input,.language-field textarea{box-sizing:border-box;width:100%;padding:9px;border:1px solid var(--line);border-radius:8px;background:var(--paper);color:var(--ink);font:inherit}.language-actions{display:flex;justify-content:space-between;gap:10px}.language-actions span{display:flex;gap:8px}.language-danger{border-color:#a14f42!important;color:#a14f42!important}
          @media(max-width:760px){.language-workspace{grid-template-columns:1fr}.lexeme-row{grid-template-columns:1fr}.lexeme-row small{display:block}}
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
              limit: 51,
              offset: page * 50,
            });
            if (!cancelled && token === request) {
              hasNextPage = result.length > 50;
              records = result.slice(0, 50);
              render();
            }
          } catch (cause) {
            if (!cancelled && token === request) render(cause instanceof Error ? cause.message : String(cause));
          }
        }

        function editForm(error = "") {
          const form = document.createElement("form");
          form.className = "language-editor";
          const value = draft ?? editing?.value;
          form.append(
            field("Lemma", "lemma", value?.lemma),
            field("Part of speech (optional)", "partOfSpeech", value?.partOfSpeech),
            field("Meanings — one per line (optional)", "meanings", value?.meanings.join("\n"), true),
            field("Pronunciation (optional)", "pronunciation", value?.pronunciation),
            field("Notes (optional)", "notes", value?.notes, true),
            field("Example (optional)", "example", value?.example?.text, true),
            field("Example translation (optional)", "translation", value?.example?.translation, true),
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
            const remove = document.createElement("button");
            remove.type = "button";
            remove.className = "language-button secondary language-danger";
            remove.textContent = "Delete";
            remove.onclick = async () => {
              if (!selectedLanguage || !editing || !window.confirm(`Delete “${editing.value.lemma}”?`)) return;
              try {
                await context.records.delete("lexemes", editing.id, selectedLanguage.id, {
                  expectedRevision: editing.revision,
                  requestId: crypto.randomUUID(),
                });
                editing = null;
                editorOpen = false;
                draft = null;
                await loadRecords();
              } catch (cause) {
                render(cause instanceof Error ? cause.message : String(cause));
              }
            };
            left.append(remove);
          }
          const right = document.createElement("span");
          const cancel = document.createElement("button");
          cancel.type = "button";
          cancel.className = "language-button secondary";
          cancel.textContent = "Cancel";
          cancel.onclick = () => {
            editing = null;
            editorOpen = false;
            draft = null;
            render();
          };
          const save = document.createElement("button");
          save.type = "submit";
          save.className = "language-button";
          save.textContent = "Save word";
          right.append(cancel, save);
          actions.append(left, right);
          form.append(actions);
          form.onsubmit = async (event) => {
            event.preventDefault();
            if (!selectedLanguage) return;
            const lemma = read(form, "lemma");
            if (!lemma) {
              form.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
              const existing = form.querySelector(".language-status.error");
              if (existing) existing.textContent = "Lemma is required.";
              else {
                const message = document.createElement("p");
                message.className = "language-status error";
                message.setAttribute("role", "alert");
                message.textContent = "Lemma is required.";
                form.insertBefore(message, actions);
              }
              return;
            }
            const example = read(form, "example");
            const translation = read(form, "translation");
            const value: LexemeValue = {
              lemma,
              partOfSpeech: read(form, "partOfSpeech") || undefined,
              meanings: read(form, "meanings")
                .split("\n")
                .map((item) => item.trim())
                .filter(Boolean),
              pronunciation: read(form, "pronunciation") || undefined,
              notes: read(form, "notes") || undefined,
              example: example ? { text: example, translation: translation || undefined } : undefined,
            };
            draft = value;
            try {
              if (editing) {
                await context.records.update("lexemes", editing.id, selectedLanguage.id, value, {
                  expectedRevision: editing.revision,
                  requestId: crypto.randomUUID(),
                });
              } else {
                await context.records.create("lexemes", selectedLanguage.id, value, {
                  requestId: crypto.randomUUID(),
                });
              }
              editing = null;
              editorOpen = false;
              draft = null;
              await loadRecords();
            } catch (cause) {
              render(cause instanceof Error ? cause.message : String(cause));
            }
          };
          return form;
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
              const button = document.createElement("button");
              button.type = "button";
              button.textContent = language.name;
              if (selectedLanguage?.id === language.id) button.setAttribute("aria-current", "page");
              button.onclick = () => {
                selectedLanguage = language;
                editing = null;
                editorOpen = false;
                draft = null;
                search = "";
                page = 0;
                void loadRecords();
              };
              item.append(button);
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
          const add = document.createElement("button");
          add.type = "button";
          add.className = "language-button";
          add.textContent = "Add word";
          add.disabled = !selectedLanguage;
          add.onclick = () => {
            editing = null;
            editorOpen = true;
            draft = null;
            lexiconPanel.replaceChildren(toolbar, editForm());
            lexiconPanel.querySelector<HTMLInputElement>("[name=lemma]")?.focus();
          };
          toolbar.append(title, add);
          lexiconPanel.append(toolbar);
          if (editorOpen) {
            lexiconPanel.append(editForm(error));
            root.append(languagesPanel, lexiconPanel);
            element.replaceChildren(root);
            return;
          }
          if (selectedLanguage && !editing) {
            const input = document.createElement("input");
            input.className = "language-search";
            input.type = "search";
            input.value = search;
            input.placeholder = "Search lemma or meaning";
            input.setAttribute("aria-label", "Search lexicon");
            input.oninput = () => {
              search = input.value;
              page = 0;
              if (searchTimer !== null) window.clearTimeout(searchTimer);
              searchTimer = window.setTimeout(() => void loadRecords(), 180);
            };
            lexiconPanel.append(input);
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
            empty.textContent = search ? "No words match this search." : "No words yet. Add the first word.";
            lexiconPanel.append(empty);
          } else {
            const list = document.createElement("ul");
            list.className = "lexeme-list";
            for (const record of records) {
              const item = document.createElement("li");
              const button = document.createElement("button");
              button.type = "button";
              button.className = "lexeme-row";
              const lemma = document.createElement("strong");
              lemma.textContent = record.value.lemma;
              const part = document.createElement("small");
              part.textContent = record.value.partOfSpeech || "—";
              const meaning = document.createElement("span");
              meaning.textContent = record.value.meanings[0] || "No meaning yet";
              button.append(lemma, part, meaning);
              button.onclick = () => {
                editing = record;
                editorOpen = true;
                draft = null;
                lexiconPanel.replaceChildren(toolbar, editForm());
              };
              item.append(button);
              list.append(item);
            }
            lexiconPanel.append(list);
            if (page > 0 || hasNextPage) {
              const paging = document.createElement("div");
              paging.className = "language-actions";
              const previous = document.createElement("button");
              previous.type = "button";
              previous.className = "language-button secondary";
              previous.textContent = "Previous";
              previous.disabled = page === 0;
              previous.onclick = () => {
                page = Math.max(0, page - 1);
                void loadRecords();
              };
              const next = document.createElement("button");
              next.type = "button";
              next.className = "language-button secondary";
              next.textContent = "Next";
              next.disabled = !hasNextPage;
              next.onclick = () => {
                page += 1;
                void loadRecords();
              };
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
