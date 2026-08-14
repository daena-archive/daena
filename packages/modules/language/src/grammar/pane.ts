import {
  GRAMMAR_SECTIONS,
  grammarGlance,
  grammarStatusLabel,
  grammarSystemDescriptor,
  searchGrammar,
  sectionCardSummary,
  summarizeSystem,
  systemsForSection,
} from "../grammar.ts";
import { alertMessage, button, emptyMessage, emptyState, field, input, textarea } from "../ui";
import { persistGrammarRecord, deleteGrammarRecord, type GrammarRecordsApi } from "./repository.ts";
import { configuredMinimum } from "./normalize.ts";
import { isChoiceSystem, renderChoiceEditor } from "./choice.ts";
import { isInventorySystem, referencedCategoryIds, renderInventoryEditor } from "./inventory.ts";
import { isStrategySystem, renderStrategyEditor } from "./strategy.ts";
import { isClauseSystem, renderClauseEditor } from "./clause.ts";
import { isParadigmSystem, renderParadigmEditor } from "./paradigm.ts";
import { renderAgreementEditor, summarizeAgreement } from "./agreement.ts";
import { renderCustomRuleEditor } from "./rules.ts";
import { GRAMMAR_STARTER_STEPS, nextStarterSystem, remainingStarterSystems, starterPosition, starterStepLabel } from "./starter.ts";
import {
  applyStoredVersion,
  confirmGrammarLeave,
  keepDraftAfterConflict,
  openAgreementEditor,
  openAgreementNotUsedEditor,
  openCustomRuleEditor,
  openSystemEditor,
  setSystemStatus,
  type GrammarEditSession,
  type GrammarUiState,
} from "./session.ts";
import type {
  GrammarExample,
  GrammarLink,
  GrammarSectionId,
  GrammarSystemId,
  GrammarSystemRecord,
  ParadigmConfig,
} from "./types.ts";

export type GrammarLinkChoices = {
  lexemes: { id: string; lemma: string }[];
  samples: { id: string; title: string }[];
  paradigms: { id: string; name: string }[];
  examples: { lexemeId: string; exampleId: string; lemma: string; text: string }[];
};

export type GrammarPaneContext = {
  languageName?: string;
  ownerId?: string;
  records: GrammarRecordsApi;
  confirm: (message: string) => boolean;
  render: () => void;
  choices: GrammarLinkChoices;
};

function heading(text: string, id?: string) {
  const node = document.createElement("h2");
  node.textContent = text;
  if (id) {
    node.id = id;
    node.tabIndex = -1;
  }
  return node;
}

export function tryLeaveGrammar(state: GrammarUiState, confirm: (message: string) => boolean) {
  return confirmGrammarLeave(state.editing, confirm);
}

export function goHome(state: GrammarUiState, confirm: (message: string) => boolean) {
  if (!tryLeaveGrammar(state, confirm)) return false;
  state.editing = null;
  state.section = null;
  state.query = "";
  state.starterCurrent = undefined;
  return true;
}

export function goSection(state: GrammarUiState, sectionId: GrammarSectionId, confirm: (message: string) => boolean) {
  if (!tryLeaveGrammar(state, confirm)) return false;
  state.editing = null;
  state.section = sectionId;
  state.query = "";
  return true;
}

function startAgreementEditor(state: GrammarUiState, ctx: GrammarPaneContext) {
  if (!tryLeaveGrammar(state, ctx.confirm)) return;
  if (state.index.sectionStates.get("agreement")) {
    if (
      !ctx.confirm(
        "This section is marked not used. Create an agreement system anyway? Saving it will clear the not-used marker.",
      )
    ) {
      return;
    }
  }
  state.section = "agreement";
  state.query = "";
  state.starterCurrent = undefined;
  state.editing = openAgreementEditor(state.index);
  state.focusTarget = "#grammar-editor-heading";
  ctx.render();
}

function startGrammarStarter(state: GrammarUiState, ctx: GrammarPaneContext) {
  if (!tryLeaveGrammar(state, ctx.confirm)) return;
  const next = nextStarterSystem(state.index);
  if (!next) {
    state.starterDismissed = true;
    ctx.render();
    return;
  }
  state.query = "";
  state.starterCurrent = next;
  state.section = grammarSystemDescriptor(next)?.sectionId ?? null;
  state.editing = openSystemEditor(state.index, next);
  state.focusTarget = "#grammar-editor-heading";
  ctx.render();
}

function dismissGrammarStarter(state: GrammarUiState, ctx: GrammarPaneContext) {
  if (!tryLeaveGrammar(state, ctx.confirm)) return;
  state.starterDismissed = true;
  state.starterCurrent = undefined;
  state.editing = null;
  ctx.render();
}

function advanceStarter(state: GrammarUiState, current: GrammarSystemId) {
  const next = nextStarterSystem(state.index, current);
  if (!next) {
    state.starterDismissed = true;
    state.starterCurrent = undefined;
    state.editing = null;
    state.section = null;
    state.focusTarget = '[data-grammar-id="section:syntax"]';
    return;
  }
  state.starterCurrent = next;
  state.section = grammarSystemDescriptor(next)?.sectionId ?? state.section;
  state.editing = openSystemEditor(state.index, next);
  state.focusTarget = "#grammar-editor-heading";
}

function grammarFocusSelector(draft: GrammarEditSession["draft"], recordId?: string) {
  if (draft.recordKind === "system") return `[data-grammar-id="system:${draft.systemId}"]`;
  if (draft.recordKind === "agreement") return `[data-grammar-id="agreement:${recordId ?? ""}"]`;
  if (draft.recordKind === "custom-rule") return `[data-grammar-id="rule:${recordId ?? ""}"]`;
  return '[data-grammar-id="section:agreement"]';
}

function applyGrammarFocus(panel: HTMLElement, state: GrammarUiState) {
  const target = state.focusTarget;
  state.focusTarget = undefined;
  if (!target) return;
  panel.querySelector<HTMLElement>(target)?.focus();
}

export function goSystem(state: GrammarUiState, systemId: GrammarSystemId, confirm: (message: string) => boolean) {
  if (!tryLeaveGrammar(state, confirm)) return false;
  state.query = "";
  state.starterCurrent = undefined;
  state.section = grammarSystemDescriptor(systemId)?.sectionId ?? state.section;
  state.editing = openSystemEditor(state.index, systemId);
  state.focusTarget = "#grammar-editor-heading";
  return true;
}

export async function saveGrammarEditor(state: GrammarUiState, ctx: GrammarPaneContext) {
  if (!ctx.ownerId || !state.editing || state.editing.locked) return "This system cannot be edited.";
  const session = state.editing;
  const result = await persistGrammarRecord(ctx.records, ctx.ownerId, session);
  if (result.ok) {
    state.index = result.index;
    if (state.starterCurrent && session.draft.recordKind === "system") {
      advanceStarter(state, session.draft.systemId);
    } else {
      state.editing = null;
      state.section = session.originSection;
      state.focusTarget = grammarFocusSelector(session.draft, result.record?.id ?? session.recordId);
    }
    return "";
  }
  if (result.stale) {
    state.index = result.index;
    state.editing.conflict = true;
    state.editing.validationMessage =
      "This record changed since you opened it. Your draft is kept. Load the stored version or overwrite it.";
    return state.editing.validationMessage;
  }
  state.editing.validationMessage = result.error;
  state.editing.validationFocus = result.issues?.[0]?.path;
  return result.error;
}

export async function deleteGrammarEditor(state: GrammarUiState, ctx: GrammarPaneContext) {
  if (!ctx.ownerId || !state.editing?.recordId || state.editing.locked) return "";
  const session = state.editing;
  const recordId = session.recordId;
  if (!recordId) return "";
  const title =
    session.draft.recordKind === "system"
      ? grammarSystemDescriptor(session.draft.systemId)?.label ?? "this system"
      : "title" in session.draft
        ? session.draft.title
        : "this record";
  if (!ctx.confirm(`Delete “${title}”?`)) return "";
  const result = await deleteGrammarRecord(ctx.records, ctx.ownerId, {
    recordId,
    revision: session.revision,
  });
  if (result.ok) {
    state.index = result.index;
    state.editing = null;
    state.section = session.originSection;
    state.focusTarget = grammarFocusSelector(session.draft, session.recordId);
    return "";
  }
  if (result.stale) {
    state.index = result.index;
    state.editing.conflict = true;
    state.editing.validationMessage = "This record changed since you opened it. Your draft is kept.";
    return state.editing.validationMessage;
  }
  return result.error;
}

function statusControl(session: GrammarEditSession, onChange: (status: GrammarSystemRecord["status"]) => void) {
  if (session.draft.recordKind !== "system") return null;
  const group = document.createElement("fieldset");
  group.className = "grammar-status";
  const legend = document.createElement("legend");
  legend.textContent = "Status";
  group.append(legend);
  for (const status of ["unconfigured", "configured", "not-used"] as const) {
    const label = document.createElement("label");
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = "status";
    radio.value = status;
    radio.checked = session.draft.status === status;
    radio.disabled = session.locked;
    radio.onchange = () => onChange(status);
    label.append(radio, ` ${grammarStatusLabel(status)}`);
    group.append(label);
  }
  return group;
}

function exampleEditor(examples: GrammarExample[], locked: boolean, onChange: (next: GrammarExample[]) => void) {
  const section = document.createElement("section");
  section.className = "language-group";
  const head = document.createElement("h3");
  head.textContent = "Examples";
  section.append(head, emptyMessage("Add a sentence, and optionally a translation, gloss, or notes."));
  for (const [index, example] of examples.entries()) {
    const card = document.createElement("div");
    card.className = "grammar-example";
    const text = textarea(`example-text-${index}`, example.text, 2);
    text.placeholder = "Nar bel tor.";
    text.disabled = locked;
    text.oninput = () => {
      examples[index] = { ...examples[index], text: text.value };
    };
    const translation = input(`example-translation-${index}`, example.translation ?? "");
    translation.placeholder = "I eat bread.";
    translation.disabled = locked;
    translation.oninput = () => {
      examples[index] = { ...examples[index], translation: translation.value };
    };
    const gloss = input(`example-gloss-${index}`, example.gloss ?? "");
    gloss.placeholder = "1sg bread eat";
    gloss.disabled = locked;
    gloss.oninput = () => {
      examples[index] = { ...examples[index], gloss: gloss.value };
    };
    const notes = textarea(`example-notes-${index}`, example.notes ?? "", 2);
    notes.disabled = locked;
    notes.oninput = () => {
      examples[index] = { ...examples[index], notes: notes.value };
    };
    card.append(
      field("Example", text),
      field("Translation (optional)", translation),
      field("Gloss (optional)", gloss),
      field("Notes (optional)", notes),
    );
    if (!locked) {
      card.append(
        button("Remove example", "language-button secondary language-danger", () => {
          onChange(examples.filter((_, item) => item !== index));
        }),
      );
    }
    section.append(card);
  }
  if (!locked) {
    section.append(
      button("Add example", "language-button secondary", () => {
        onChange([...examples, { id: crypto.randomUUID(), text: "" }]);
      }),
    );
  }
  return section;
}

function linkEditor(
  links: GrammarLink[],
  locked: boolean,
  choices: GrammarLinkChoices,
  onChange: (next: GrammarLink[]) => void,
) {
  const section = document.createElement("section");
  section.className = "language-group";
  const head = document.createElement("h3");
  head.textContent = "Links";
  section.append(head);
  for (const [index, link] of links.entries()) {
    const row = document.createElement("div");
    row.className = "language-inline";
    const label = document.createElement("span");
    label.textContent = `${link.kind}: ${link.label || link.targetId}`;
    row.append(label);
    if (!locked) {
      row.append(
        button("Remove", "language-button secondary language-danger", () => {
          onChange(links.filter((_, item) => item !== index));
        }),
      );
    }
    section.append(row);
  }
  if (locked) return section;
  const add = document.createElement("select");
  add.setAttribute("aria-label", "Link a record");
  add.append(new Option("Link a word, sample, or paradigm…", ""));
  for (const lexeme of choices.lexemes) {
    add.append(new Option(`Word: ${lexeme.lemma}`, JSON.stringify({ kind: "lexeme", targetId: lexeme.id, label: lexeme.lemma })));
  }
  for (const example of choices.examples) {
    add.append(
      new Option(
        `Example: ${example.lemma} — ${example.text}`,
        JSON.stringify({
          kind: "lexeme-example",
          targetId: example.lexemeId,
          secondaryId: example.exampleId,
          label: example.text,
        }),
      ),
    );
  }
  for (const sample of choices.samples) {
    add.append(new Option(`Sample: ${sample.title}`, JSON.stringify({ kind: "sample", targetId: sample.id, label: sample.title })));
  }
  for (const paradigm of choices.paradigms) {
    add.append(
      new Option(`Paradigm: ${paradigm.name}`, JSON.stringify({ kind: "paradigm", targetId: paradigm.id, label: paradigm.name })),
    );
  }
  add.onchange = () => {
    if (!add.value) return;
    const parsed = JSON.parse(add.value) as GrammarLink;
    onChange([...links, { ...parsed, id: crypto.randomUUID() }]);
  };
  section.append(add);
  return section;
}

function renderEditor(panel: HTMLElement, state: GrammarUiState, ctx: GrammarPaneContext, error: string) {
  const session = state.editing!;
  const toolbar = document.createElement("div");
  toolbar.className = "language-toolbar";
  const back = button("Back", "language-button secondary", () => {
    if (!tryLeaveGrammar(state, ctx.confirm)) return;
    const origin = session.originSection;
    const focus = grammarFocusSelector(session.draft, session.recordId);
    state.editing = null;
    state.starterCurrent = undefined;
    state.section = origin;
    state.focusTarget = focus;
    ctx.render();
  });
  toolbar.append(back);
  if (state.starterCurrent && session.draft.recordKind === "system") {
    const progress = starterPosition(state.starterCurrent);
    const status = document.createElement("span");
    status.textContent = `Starter ${progress.current} of ${progress.total}`;
    toolbar.append(status);
    toolbar.append(
      button("Skip", "language-button secondary", () => {
        if (!tryLeaveGrammar(state, ctx.confirm)) return;
        advanceStarter(state, state.starterCurrent!);
        ctx.render();
      }),
      button("Exit starter", "language-button secondary", () => dismissGrammarStarter(state, ctx)),
    );
  }
  panel.append(toolbar);

  const form = document.createElement("form");
  form.className = "language-editor";
  let titleText = "Grammar";
  if (session.draft.recordKind === "system") titleText = grammarSystemDescriptor(session.draft.systemId)?.label ?? "System";
  else if (session.draft.recordKind === "agreement") titleText = session.recordId ? session.draft.title || "Agreement" : "New agreement system";
  else if (session.draft.recordKind === "custom-rule") titleText = session.recordId ? "Custom rule" : "New custom rule";
  else if (session.draft.recordKind === "section-state") titleText = "Agreement";
  const title = heading(titleText, "grammar-editor-heading");
  form.append(title);

  if (session.draft.recordKind === "system") {
    const descriptor = grammarSystemDescriptor(session.draft.systemId)!;
    form.append(emptyMessage(descriptor.hint));
    const learn = document.createElement("details");
    learn.className = "grammar-learn";
    learn.open = session.learnMoreOpen;
    learn.ontoggle = () => {
      session.learnMoreOpen = learn.open;
    };
    const summary = document.createElement("summary");
    summary.textContent = "Learn more";
    const body = document.createElement("p");
    body.className = "grammar-help";
    body.textContent = descriptor.learnMore;
    learn.append(summary, body);
    form.append(learn);
    const statuses = statusControl(session, (status) => {
      if (session.locked) return;
      if (session.draft.recordKind !== "system") return;
      if (session.draft.status === "configured" && status !== "configured") {
        if (!ctx.confirm("Reset this system’s configuration? Unsaved settings in this editor will be cleared.")) return;
      }
      session.draft = setSystemStatus(session.draft, status);
      ctx.render();
    });
    if (statuses) form.append(statuses);
    if (session.draft.status === "configured") {
      if (configuredMinimum(session.draft.systemId, session.draft.config)) {
        form.append(emptyMessage(summarizeSystem(session.draft.systemId, session.draft)));
      }
      const choice = renderChoiceEditor(session.draft, session.locked, (next, rerender) => {
        if (session.draft.recordKind !== "system") return;
        session.draft = next;
        if (rerender) ctx.render();
      });
      const inventory = renderInventoryEditor(
        session.draft,
        session.locked,
        { referencedIds: referencedCategoryIds(state.index, session.draft.systemId), confirm: ctx.confirm },
        (next, rerender) => {
          if (session.draft.recordKind !== "system") return;
          session.draft = next;
          if (rerender) ctx.render();
        },
      );
      const strategy = renderStrategyEditor(
        session.draft,
        session.locked,
        {
          agreements: state.index.agreements
            .filter((item) => item.value.recordKind === "agreement")
            .map((item) => ({ id: item.id, title: item.value.title })),
        },
        (next, rerender) => {
          if (session.draft.recordKind !== "system") return;
          session.draft = next;
          if (rerender) ctx.render();
        },
      );
      const negativeVerb = state.index.systems.get("verbs.negative-forms")?.value;
      const relativePosition = state.index.systems.get("syntax.relative-clause-position")?.value;
      const personalPronouns = state.index.systems.get("pronouns.personal")?.value;
      const paradigm = renderParadigmEditor(
        session.draft,
        session.locked,
        {
          confirm: ctx.confirm,
          referencedIds: referencedCategoryIds(state.index, session.draft.systemId),
          pronounAxes:
            personalPronouns?.recordKind === "system" ? (personalPronouns.config as ParadigmConfig).axes : undefined,
          agreements: state.index.agreements
            .filter((item) => item.value.recordKind === "agreement")
            .map((item) => ({ id: item.id, title: item.value.title })),
        },
        (next, rerender) => {
          if (session.draft.recordKind !== "system") return;
          session.draft = next;
          if (rerender) ctx.render();
        },
      );
      const clause = renderClauseEditor(
        session.draft,
        session.locked,
        {
          lexemes: ctx.choices.lexemes,
          negativeVerbSummary:
            negativeVerb?.recordKind === "system" ? summarizeSystem("verbs.negative-forms", negativeVerb) : undefined,
          relativePositionSummary:
            relativePosition?.recordKind === "system"
              ? summarizeSystem("syntax.relative-clause-position", relativePosition)
              : undefined,
        },
        (next, rerender) => {
          if (session.draft.recordKind !== "system") return;
          session.draft = next;
          if (rerender) ctx.render();
        },
      );
      if (choice) form.append(choice);
      else if (inventory) form.append(inventory);
      else if (strategy) form.append(strategy);
      else if (clause) form.append(clause);
      else if (paradigm) form.append(paradigm);
      else if (
        !configuredMinimum(session.draft.systemId, session.draft.config) &&
        !isChoiceSystem(session.draft.systemId) &&
        !isInventorySystem(session.draft.systemId) &&
        !isStrategySystem(session.draft.systemId) &&
        !isClauseSystem(session.draft.systemId) &&
        !isParadigmSystem(session.draft.systemId)
      ) {
        form.append(
          emptyMessage(
            "Specialized settings for this system will appear here. Mark it as not used if the language does not have this feature, or leave it not configured.",
          ),
        );
      }
    }
    if (session.draft.status === "not-used") {
      const note = textarea("notes", session.draft.notes, 3);
      note.disabled = session.locked;
      note.placeholder = "Noun roles are primarily expressed through word order and adpositions.";
      note.oninput = () => {
        if (session.draft.recordKind === "system") session.draft.notes = note.value;
      };
      form.append(field("Why it is not used (optional)", note));
    } else {
      const notes = textarea("notes", session.draft.notes, 4);
      notes.disabled = session.locked;
      notes.oninput = () => {
        if (session.draft.recordKind !== "section-state") session.draft.notes = notes.value;
      };
      form.append(field("Notes", notes));
    }
  } else if (session.draft.recordKind === "agreement") {
    form.append(
      renderAgreementEditor(session.draft, session.locked, state.index, (next, rerender) => {
        if (session.draft.recordKind !== "agreement") return;
        session.draft = next;
        if (rerender) ctx.render();
      }),
    );
    const notes = textarea("notes", session.draft.notes, 4);
    notes.disabled = session.locked;
    notes.oninput = () => {
      if (session.draft.recordKind === "agreement") session.draft.notes = notes.value;
    };
    form.append(field("Notes", notes));
  } else if (session.draft.recordKind === "custom-rule") {
    form.append(
      renderCustomRuleEditor(session.draft, session.locked, (next, rerender) => {
        if (session.draft.recordKind !== "custom-rule") return;
        session.draft = next;
        if (rerender) ctx.render();
      }),
    );
  } else if (session.draft.recordKind === "section-state") {
    form.append(emptyMessage("If your language does not use agreement, you can mark this section as not used."));
    const note = textarea("note", session.draft.note ?? "", 3);
    note.disabled = session.locked;
    note.oninput = () => {
      if (session.draft.recordKind === "section-state") session.draft.note = note.value;
    };
    form.append(field("Note (optional)", note));
  }

  if (session.draft.recordKind === "system" || session.draft.recordKind === "custom-rule" || session.draft.recordKind === "agreement") {
    const draft = session.draft;
    form.append(
      exampleEditor(draft.examples, session.locked, (examples) => {
        draft.examples = examples;
        ctx.render();
      }),
      linkEditor(draft.links, session.locked, ctx.choices, (links) => {
        draft.links = links;
        ctx.render();
      }),
    );
  }

  if (error || session.validationMessage) form.append(alertMessage(session.validationMessage || error));
  if (session.conflict && session.recordId) {
    const stored = [...state.index.systems.values(), ...state.index.customRules, ...state.index.agreements, ...state.index.sectionStates.values()].find(
      (item) => item.id === session.recordId,
    );
    const actions = document.createElement("div");
    actions.className = "language-inline";
    actions.append(
      button("Load stored version", "language-button secondary", () => {
        if (!stored) return;
        state.editing = applyStoredVersion(session, stored);
        ctx.render();
      }),
      button("Keep my draft", "language-button secondary", () => {
        if (!stored) return;
        state.editing = keepDraftAfterConflict(session, stored);
        ctx.render();
      }),
    );
    form.append(actions);
  }

  const actions = document.createElement("div");
  actions.className = "language-actions";
  const left = document.createElement("span");
  if (session.recordId && !session.locked) {
    const remove = button("Delete", "language-button secondary language-danger", async () => {
      const message = await deleteGrammarEditor(state, ctx);
      if (message && state.editing) state.editing.validationMessage = message;
      ctx.render();
    });
    remove.setAttribute("aria-label", `Delete ${titleText}`);
    left.append(remove);
  }
  const right = document.createElement("span");
  right.append(
    button("Cancel", "language-button secondary", () => {
      if (!tryLeaveGrammar(state, ctx.confirm)) return;
      const origin = session.originSection;
      const focus = grammarFocusSelector(session.draft, session.recordId);
      state.editing = null;
      state.starterCurrent = undefined;
      state.section = origin;
      state.focusTarget = focus;
      ctx.render();
    }),
  );
  const save = document.createElement("button");
  save.type = "submit";
  save.className = "language-button";
  save.textContent = "Save";
  save.disabled = session.locked;
  right.append(save);
  actions.append(left, right);
  form.append(actions);
  form.onsubmit = async (event) => {
    event.preventDefault();
    const message = await saveGrammarEditor(state, ctx);
    if (message && state.editing?.validationFocus) {
      state.focusTarget = `[name="${CSS.escape(state.editing.validationFocus)}"]`;
    } else if (message) {
      state.focusTarget = "#grammar-editor-heading";
    }
    ctx.render();
  };
  panel.append(form);
  if (!state.focusTarget) state.focusTarget = "#grammar-editor-heading";
}

function renderSearch(home: HTMLElement, state: GrammarUiState, ctx: GrammarPaneContext) {
  const hits = searchGrammar(state.query, state.index);
  const list = document.createElement("div");
  list.className = "grammar-systems";
  if (hits.length === 0) list.append(emptyMessage("No matching grammar systems."));
  for (const hit of hits) {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "grammar-system";
    const headingNode = document.createElement("strong");
    headingNode.textContent = hit.label;
    const detail = document.createElement("span");
    const sectionLabel = GRAMMAR_SECTIONS.find((section) => section.id === hit.sectionId)?.label ?? "";
    detail.textContent = `${sectionLabel} · ${hit.status ? grammarStatusLabel(hit.status) : hit.summary}`;
    item.append(headingNode, detail);
    item.onclick = () => {
      if (hit.kind === "system" && hit.systemId) goSystem(state, hit.systemId, ctx.confirm);
      else if (hit.kind === "custom-rule") {
        if (!tryLeaveGrammar(state, ctx.confirm)) return;
        state.section = "other";
        state.query = "";
        state.editing = openCustomRuleEditor(state.index, hit.recordId);
      } else if (hit.kind === "agreement" && hit.recordId) {
        if (!tryLeaveGrammar(state, ctx.confirm)) return;
        state.section = "agreement";
        state.query = "";
        state.editing = openAgreementEditor(state.index, hit.recordId);
      } else {
        goSection(state, hit.sectionId, ctx.confirm);
      }
      ctx.render();
    };
    list.append(item);
  }
  home.append(list);
}

function renderHome(home: HTMLElement, state: GrammarUiState, ctx: GrammarPaneContext) {
  home.append(
    emptyMessage("Define how sentences and words behave in this language. You do not need to configure every system."),
  );
  const remaining = remainingStarterSystems(state.index);
  if (!state.starterDismissed && remaining.length) {
    const intro = document.createElement("section");
    intro.className = "language-empty-card";
    intro.setAttribute("data-grammar-id", "starter");
    intro.append(
      emptyMessage("Start your grammar"),
      emptyMessage("Choose a few foundational systems now. Everything can be changed later."),
    );
    const list = document.createElement("ol");
    list.className = "grammar-starter-list";
    for (const systemId of remaining) {
      const item = document.createElement("li");
      item.textContent = starterStepLabel(systemId);
      list.append(item);
    }
    intro.append(list);
    const actions = document.createElement("div");
    actions.className = "language-inline";
    const start = button(remaining.length === GRAMMAR_STARTER_STEPS.length ? "Start" : "Continue starter", "language-button", () => {
      startGrammarStarter(state, ctx);
    });
    start.setAttribute("data-grammar-id", "starter-start");
    const manual = button("I'll configure grammar manually", "language-button secondary", () => {
      dismissGrammarStarter(state, ctx);
    });
    actions.append(start, manual);
    intro.append(actions);
    home.append(intro);
  }
  const cards = document.createElement("div");
  cards.className = "grammar-cards";
  for (const section of GRAMMAR_SECTIONS) {
    const summary = sectionCardSummary(state.index, section.id);
    const card = document.createElement("button");
    card.type = "button";
    card.className = "grammar-card";
    card.setAttribute("data-grammar-id", `section:${section.id}`);
    const notUsed = summary.notUsed ? ` · ${summary.notUsed} not used` : "";
    card.setAttribute("aria-label", `${summary.label}: ${summary.detail}${notUsed}`);
    const headingNode = document.createElement("strong");
    headingNode.textContent = summary.label;
    const detail = document.createElement("span");
    detail.textContent = `${summary.detail}${notUsed}`;
    card.append(headingNode, detail);
    card.onclick = () => {
      goSection(state, section.id, ctx.confirm);
      ctx.render();
    };
    cards.append(card);
  }
  home.append(cards);
  const glance = document.createElement("dl");
  glance.className = "grammar-glance";
  glance.setAttribute("aria-label", "At a glance");
  for (const row of grammarGlance(state.index)) {
    const term = document.createElement("dt");
    term.textContent = row.label;
    const value = document.createElement("dd");
    value.textContent = row.value;
    glance.append(term, value);
  }
  home.append(glance);
}

function renderSection(home: HTMLElement, state: GrammarUiState, ctx: GrammarPaneContext) {
  const section = GRAMMAR_SECTIONS.find((item) => item.id === state.section)!;
  const headingNode = document.createElement("h3");
  headingNode.textContent = section.label;
  home.append(headingNode, emptyMessage(section.orientation));
  const systems = document.createElement("div");
  systems.className = "grammar-systems";

  if (section.id === "agreement") {
    const unused = state.index.sectionStates.get("agreement");
    const add = button("Add agreement system", "language-button", () => startAgreementEditor(state, ctx));
    if (unused?.value.recordKind === "section-state") {
      home.append(emptyMessage("Not used"));
      if (unused.value.note) home.append(emptyMessage(unused.value.note));
      home.append(
        add,
        button("Edit", "language-button secondary", () => {
          if (!tryLeaveGrammar(state, ctx.confirm)) return;
          state.editing = openAgreementNotUsedEditor(state.index);
          ctx.render();
        }),
      );
      return;
    }
    if (state.index.agreements.length === 0) {
      const mark = button("Mark as not used", "language-button secondary", () => {
        if (!tryLeaveGrammar(state, ctx.confirm)) return;
        state.editing = openAgreementNotUsedEditor(state.index);
        ctx.render();
      });
      home.append(emptyState(section.emptyBody, add, mark));
      return;
    }
    home.append(add);
    for (const record of state.index.agreements) {
      if (record.value.recordKind !== "agreement") continue;
      const item = document.createElement("button");
      item.type = "button";
      item.className = "grammar-system";
      item.setAttribute("data-grammar-id", `agreement:${record.id}`);
      item.setAttribute("aria-label", `${record.value.title}: ${summarizeAgreement(record.value)}`);
      const name = document.createElement("strong");
      name.textContent = record.value.title;
      const detail = document.createElement("span");
      detail.textContent = summarizeAgreement(record.value);
      item.append(name, detail);
      item.onclick = () => {
        if (!tryLeaveGrammar(state, ctx.confirm)) return;
        state.editing = openAgreementEditor(state.index, record.id);
        state.focusTarget = "#grammar-editor-heading";
        ctx.render();
      };
      systems.append(item);
    }
    home.append(systems);
    return;
  }

  if (section.id === "other") {
    const add = button("Add a custom rule", "language-button", () => {
      if (!tryLeaveGrammar(state, ctx.confirm)) return;
      state.editing = openCustomRuleEditor(state.index);
      ctx.render();
    });
    if (state.index.customRules.length === 0) {
      home.append(emptyState(section.emptyBody, add));
      return;
    }
    home.append(add);
    for (const record of state.index.customRules) {
      if (record.value.recordKind !== "custom-rule") continue;
      const item = document.createElement("button");
      item.type = "button";
      item.className = "grammar-system";
      item.setAttribute("data-grammar-id", `rule:${record.id}`);
      item.setAttribute("aria-label", record.value.title);
      const name = document.createElement("strong");
      name.textContent = record.value.title;
      const detail = document.createElement("span");
      detail.textContent = record.value.tags.join(", ") || record.value.body.split("\n")[0] || "Custom rule";
      item.append(name, detail);
      item.onclick = () => {
        if (!tryLeaveGrammar(state, ctx.confirm)) return;
        state.editing = openCustomRuleEditor(state.index, record.id);
        state.focusTarget = "#grammar-editor-heading";
        ctx.render();
      };
      systems.append(item);
    }
    home.append(systems);
    return;
  }

  const listed = systemsForSection(section.id);
  const noneConfigured = listed.every(
    (system) => !state.index.systems.has(system.id) && !state.index.duplicates.has(system.id),
  );
  if (noneConfigured) {
    const first = listed[0];
    const action = first
      ? button(first.emptyAction, "language-button", () => {
          goSystem(state, first.id, ctx.confirm);
          ctx.render();
        })
      : undefined;
    home.append(emptyState(section.emptyBody, action));
  }
  for (const system of listed) {
    const record = state.index.systems.get(system.id)?.value;
    const duplicate = state.index.duplicates.has(system.id);
    const item = document.createElement("button");
    item.type = "button";
    item.className = "grammar-system";
    item.setAttribute("data-grammar-id", `system:${system.id}`);
    const name = document.createElement("strong");
    name.textContent = system.label;
    const detail = document.createElement("span");
    const summary = duplicate
      ? "Duplicate records — edits disabled"
      : record?.recordKind === "system"
        ? summarizeSystem(system.id, record)
        : grammarStatusLabel("unconfigured");
    detail.textContent = summary;
    item.setAttribute("aria-label", `${system.label}: ${summary}`);
    item.append(name, detail);
    item.onclick = () => {
      goSystem(state, system.id, ctx.confirm);
      ctx.render();
    };
    systems.append(item);
  }
  home.append(systems);
}

export function renderGrammarPane(panel: HTMLElement, state: GrammarUiState, ctx: GrammarPaneContext, error: string) {
  const toolbar = document.createElement("div");
  toolbar.className = "language-toolbar";
  toolbar.append(heading(ctx.languageName ? `${ctx.languageName} grammar` : "Grammar"));
  if (state.section || state.editing) {
    toolbar.append(
      button("All sections", "language-button secondary", () => {
        if (!goHome(state, ctx.confirm)) return;
        ctx.render();
      }),
    );
  }
  panel.append(toolbar);
  if (error) panel.append(alertMessage(error));
  if (!ctx.ownerId) {
    panel.append(emptyMessage("Select a language to document its grammar."));
    return;
  }
  if (state.editing) {
    renderEditor(panel, state, ctx, error);
    applyGrammarFocus(panel, state);
    return;
  }
  const search = input("grammar-search", state.query);
  search.placeholder = "Search grammar systems…";
  search.setAttribute("aria-label", "Search grammar systems");
  search.oninput = () => {
    state.query = search.value;
    ctx.render();
  };
  panel.append(search);
  const home = document.createElement("div");
  home.className = "grammar-home";
  for (const diagnostic of state.index.diagnostics) {
    const row = document.createElement("div");
    row.className = "grammar-diagnostic";
    row.setAttribute("role", "group");
    row.setAttribute("aria-label", "Grammar diagnostic");
    row.append(alertMessage(diagnostic.message));
    if (diagnostic.systemId) {
      const label = grammarSystemDescriptor(diagnostic.systemId)?.label ?? diagnostic.systemId;
      row.append(
        button(`Open ${label}`, "language-button secondary", () => {
          goSystem(state, diagnostic.systemId!, ctx.confirm);
          ctx.render();
        }),
      );
    } else if (diagnostic.recordIds[0] && state.index.agreements.some((item) => item.id === diagnostic.recordIds[0])) {
      row.append(
        button("Open agreement", "language-button secondary", () => {
          if (!tryLeaveGrammar(state, ctx.confirm)) return;
          state.section = "agreement";
          state.editing = openAgreementEditor(state.index, diagnostic.recordIds[0]);
          state.focusTarget = "#grammar-editor-heading";
          ctx.render();
        }),
      );
    }
    home.append(row);
  }
  if (state.query.trim()) renderSearch(home, state, ctx);
  else if (!state.section) renderHome(home, state, ctx);
  else renderSection(home, state, ctx);
  panel.append(home);
  applyGrammarFocus(panel, state);
}
