import { GRAMMAR_SYSTEM_IDS, type GrammarSectionId, type GrammarSystemId } from "./types.ts";

export type GrammarEditorKind = "choice" | "inventory" | "strategy" | "clause" | "paradigm";

export type GrammarSectionDescriptor = {
  id: GrammarSectionId;
  label: string;
  orientation: string;
  emptyTitle: string;
  emptyBody: string;
  emptyActions: string[];
};

export type GrammarSystemDescriptor = {
  id: GrammarSystemId;
  sectionId: Exclude<GrammarSectionId, "agreement" | "other">;
  label: string;
  hint: string;
  learnMore: string;
  searchAliases: string[];
  scope: "initial" | "later";
  editorKind: GrammarEditorKind;
  summaryId: GrammarSystemId;
  emptyAction: string;
  dependencies: GrammarSystemId[];
};

export const GRAMMAR_SECTIONS: GrammarSectionDescriptor[] = [
  {
    id: "syntax",
    label: "Syntax",
    orientation: "Describe how words and phrases are normally arranged. You can add exceptions later.",
    emptyTitle: "Nothing configured yet.",
    emptyBody: "Syntax covers word order, adjective placement, adpositions, and related structural choices.",
    emptyActions: ["Configure basic word order", "Browse syntax systems"],
  },
  {
    id: "nouns",
    label: "Nouns",
    orientation: "Start with the distinctions your language actually uses. You can add exceptions later.",
    emptyTitle: "Nothing configured yet.",
    emptyBody:
      "Noun grammar can describe things such as number, case, gender or noun classes, articles, and possession. A simple language may only need number and possession.",
    emptyActions: ["Configure Number", "Browse noun systems"],
  },
  {
    id: "pronouns",
    label: "Pronouns",
    orientation: "Start with person and number. Add gender, case, or other distinctions only if you use them.",
    emptyTitle: "No pronoun systems configured yet.",
    emptyBody: "Pronouns are edited as paradigms. Begin with personal pronouns, then add demonstratives if needed.",
    emptyActions: ["Configure personal pronouns", "Browse pronoun systems"],
  },
  {
    id: "verbs",
    label: "Verbs",
    orientation: "Describe how verbs encode time, event structure, modality, and participants.",
    emptyTitle: "Nothing configured yet.",
    emptyBody: "Start with how verbs are marked, then add tense, aspect, or mood only if the language uses them.",
    emptyActions: ["Configure verb marking", "Browse verb systems"],
  },
  {
    id: "modifiers",
    label: "Modifiers & Comparison",
    orientation:
      "Describe adjectives, comparison, and related modifier behavior. Adjective position lives under Syntax.",
    emptyTitle: "Nothing configured yet.",
    emptyBody: "This section covers how adjectives behave and how comparison is formed, not where adjectives appear.",
    emptyActions: ["Configure adjective behavior", "Browse modifier systems"],
  },
  {
    id: "clauses",
    label: "Clause Types",
    orientation: "Describe how statements, questions, commands, and related clause types are formed.",
    emptyTitle: "Nothing configured yet.",
    emptyBody: "Clause types cover questions, commands, negation, and relative clauses as constructions.",
    emptyActions: ["Configure yes/no questions", "Browse clause systems"],
  },
  {
    id: "agreement",
    label: "Agreement",
    orientation: "Agreement means one word changes based on grammatical features of another.",
    emptyTitle: "No agreement systems are defined.",
    emptyBody:
      "Agreement means that one word changes based on grammatical features of another, such as a verb changing for the person and number of its subject. If your language does not use agreement, you can mark this section as not used.",
    emptyActions: ["Add agreement system", "Mark as not used"],
  },
  {
    id: "other",
    label: "Other Rules",
    orientation:
      "Use this for grammatical features that do not fit Daena's built-in grammar systems. If a feature becomes common enough, it may eventually deserve its own dedicated editor.",
    emptyTitle: "No custom rules yet.",
    emptyBody: "Other Rules is the escape hatch for unsupported or unusual grammar.",
    emptyActions: ["Add a custom rule"],
  },
];

export const GRAMMAR_CATALOG: GrammarSystemDescriptor[] = [
  {
    id: "syntax.basic-word-order",
    sectionId: "syntax",
    label: "Basic word order",
    hint: "Choose the usual order of subject, object, and verb in a simple declarative sentence. This describes the default pattern, not every possible sentence.",
    learnMore:
      "Word-order labels such as SOV describe a common pattern, not a law. Many languages allow other orders for topic, focus, or questions.",
    searchAliases: ["sov", "svo", "vso", "word order", "syntax", "constituent order"],
    scope: "initial",
    editorKind: "choice",
    summaryId: "syntax.basic-word-order",
    emptyAction: "Configure word order",
    dependencies: [],
  },
  {
    id: "syntax.adjective-position",
    sectionId: "syntax",
    label: "Adjective position",
    hint: "Define where attributive adjectives usually appear relative to the noun they describe.",
    learnMore: "This setting is about placement only. Morphological adjective behavior belongs under Modifiers.",
    searchAliases: ["adjective order", "adj", "before noun", "after noun"],
    scope: "initial",
    editorKind: "choice",
    summaryId: "syntax.adjective-position",
    emptyAction: "Configure adjective position",
    dependencies: [],
  },
  {
    id: "syntax.adpositions",
    sectionId: "syntax",
    label: "Adpositions",
    hint: 'Adpositions express relationships such as "in", "to", "from", or "with". They may appear before or after the noun phrase.',
    learnMore:
      "Case marking is not an alternative value here. Languages may use case and adpositions together. If adpositions are not used, see Nouns → Case.",
    searchAliases: ["preposition", "postposition", "adposition", "prepositions", "postpositions"],
    scope: "initial",
    editorKind: "choice",
    summaryId: "syntax.adpositions",
    emptyAction: "Configure adpositions",
    dependencies: [],
  },
  {
    id: "syntax.possessive-position",
    sectionId: "syntax",
    label: "Possessive position",
    hint: "Describe where a possessor appears relative to the possessed noun. Detailed possession morphology belongs under Nouns → Possession.",
    learnMore: 'English uses both "the king\'s sword" and "the sword of the king". Other languages pick one default.',
    searchAliases: ["possessor", "genitive order", "possession order"],
    scope: "initial",
    editorKind: "choice",
    summaryId: "syntax.possessive-position",
    emptyAction: "Configure possessive position",
    dependencies: [],
  },
  {
    id: "syntax.relative-clause-position",
    sectionId: "syntax",
    label: "Relative clause position",
    hint: "This system only defines where a relative clause appears relative to its head noun.",
    learnMore: "Detailed relative-clause behavior belongs under Clause Types.",
    searchAliases: ["relative clause order", "prenominal relative", "postnominal relative"],
    scope: "initial",
    editorKind: "choice",
    summaryId: "syntax.relative-clause-position",
    emptyAction: "Configure relative clause position",
    dependencies: [],
  },
  {
    id: "nouns.number",
    sectionId: "nouns",
    label: "Number",
    hint: "Number distinguishes quantities grammatically, such as one, two, or many. A language does not need to distinguish only singular and plural.",
    learnMore: "Dual, trial, and paucal are optional. Mixed marking strategies are allowed.",
    searchAliases: ["plural", "singular", "dual", "paucal", "number"],
    scope: "initial",
    editorKind: "inventory",
    summaryId: "nouns.number",
    emptyAction: "Configure Number",
    dependencies: [],
  },
  {
    id: "nouns.case",
    sectionId: "nouns",
    label: "Case",
    hint: "Cases mark the grammatical role or relationship of a noun. If your language expresses these relationships mainly through word order or adpositions, you may not need grammatical case.",
    learnMore:
      "Case names are convenient labels, not universal exact meanings. Ergative/absolutive is one alignment among several.",
    searchAliases: ["case", "nominative", "accusative", "ergative", "absolutive", "dative", "declension"],
    scope: "initial",
    editorKind: "inventory",
    summaryId: "nouns.case",
    emptyAction: "Configure case",
    dependencies: [],
  },
  {
    id: "nouns.classes",
    sectionId: "nouns",
    label: "Gender / noun classes",
    hint: "Some languages group nouns into grammatical classes that can affect articles, adjectives, pronouns, or verbs. These classes do not need to correspond to natural gender.",
    learnMore: "Agreement behavior belongs under Agreement. This system only defines the available classes.",
    searchAliases: ["gender", "noun class", "masculine", "feminine", "neuter"],
    scope: "initial",
    editorKind: "inventory",
    summaryId: "nouns.classes",
    emptyAction: "Configure noun classes",
    dependencies: [],
  },
  {
    id: "nouns.definiteness",
    sectionId: "nouns",
    label: "Definiteness & articles",
    hint: "Describe whether the language marks definite or indefinite reference, and how.",
    learnMore: "Article agreement is configured under Agreement → Noun → Article. Do not copy those rules here.",
    searchAliases: ["article", "definite", "indefinite", "the", "a"],
    scope: "initial",
    editorKind: "strategy",
    summaryId: "nouns.definiteness",
    emptyAction: "Configure definiteness",
    dependencies: [],
  },
  {
    id: "nouns.possession",
    sectionId: "nouns",
    label: "Possession",
    hint: "Describe how possession is marked on nouns or noun phrases. Ordering belongs under Syntax.",
    learnMore: "Alienable vs inalienable possession is an optional advanced distinction.",
    searchAliases: ["possession", "possessive", "genitive", "inalienable"],
    scope: "initial",
    editorKind: "strategy",
    summaryId: "nouns.possession",
    emptyAction: "Configure possession",
    dependencies: [],
  },
  {
    id: "pronouns.personal",
    sectionId: "pronouns",
    label: "Personal pronouns",
    hint: "Start with person and number. Add distinctions such as gender, case, or inclusive/exclusive forms only if your language uses them.",
    learnMore: "Cells may share a form, be zero, or be not applicable. Do not force a unique string in every cell.",
    searchAliases: ["pronoun", "pronouns", "person", "clusivity", "inclusive", "exclusive"],
    scope: "initial",
    editorKind: "paradigm",
    summaryId: "pronouns.personal",
    emptyAction: "Configure personal pronouns",
    dependencies: [],
  },
  {
    id: "pronouns.demonstratives",
    sectionId: "pronouns",
    label: "Demonstratives",
    hint: "Start with distance distinctions such as this and that. Add number, class, or other dimensions only if needed.",
    learnMore: "The paradigm is generated only from selected dimensions.",
    searchAliases: ["demonstrative", "this", "that", "proximal", "distal"],
    scope: "initial",
    editorKind: "paradigm",
    summaryId: "pronouns.demonstratives",
    emptyAction: "Configure demonstratives",
    dependencies: [],
  },
  {
    id: "verbs.marking-strategy",
    sectionId: "verbs",
    label: "Verb marking strategy",
    hint: "Languages can express grammatical information by changing the verb, adding particles or auxiliaries, or combining several strategies.",
    learnMore: "This selection guides wording elsewhere, but does not restrict what you can configure.",
    searchAliases: ["affix", "suffix", "prefix", "auxiliary", "particle", "conjugation"],
    scope: "initial",
    editorKind: "strategy",
    summaryId: "verbs.marking-strategy",
    emptyAction: "Configure verb marking",
    dependencies: [],
  },
  {
    id: "verbs.tense",
    sectionId: "verbs",
    label: "Tense",
    hint: "Tense locates an event in time. Some languages grammatically mark several tenses; others rely mostly on context or aspect.",
    learnMore: "Past/present/future is not a required default. You may mark tense as not used.",
    searchAliases: ["past tense", "present", "future", "tense"],
    scope: "initial",
    editorKind: "inventory",
    summaryId: "verbs.tense",
    emptyAction: "Configure tense",
    dependencies: [],
  },
  {
    id: "verbs.aspect",
    sectionId: "verbs",
    label: "Aspect",
    hint: "Aspect describes the internal structure of an event, such as completed, ongoing, or habitual.",
    learnMore:
      "Perfective presents an event as a bounded whole. Imperfective presents it as ongoing, habitual, or internally structured.",
    searchAliases: ["perfective", "imperfective", "progressive", "habitual", "aspect"],
    scope: "initial",
    editorKind: "inventory",
    summaryId: "verbs.aspect",
    emptyAction: "Configure aspect",
    dependencies: [],
  },
  {
    id: "verbs.mood",
    sectionId: "verbs",
    label: "Mood",
    hint: "Mood marks the speaker's attitude toward a clause, such as statement, command, or hypothetical.",
    learnMore: "Common choices appear first. Less common moods such as optative or jussive stay under More.",
    searchAliases: ["indicative", "imperative mood", "subjunctive", "irrealis", "mood"],
    scope: "initial",
    editorKind: "inventory",
    summaryId: "verbs.mood",
    emptyAction: "Configure mood",
    dependencies: [],
  },
  {
    id: "verbs.argument-indexing",
    sectionId: "verbs",
    label: "Argument indexing",
    hint: "Describe forms on or around the verb that index one or more participants. The verb may change based on who takes part.",
    learnMore:
      "Not all argument indexing is best treated as agreement. If you analyze the same behavior as agreement, link a single Agreement system instead of copying rules.",
    searchAliases: ["person marking", "polypersonal", "subject agreement", "object agreement", "indexing"],
    scope: "initial",
    editorKind: "paradigm",
    summaryId: "verbs.argument-indexing",
    emptyAction: "Configure argument indexing",
    dependencies: ["nouns.number", "pronouns.personal"],
  },
  {
    id: "verbs.negative-forms",
    sectionId: "verbs",
    label: "Negative verb forms",
    hint: "Describe negative morphology or special negative forms of the verb. Configure the language's primary clause-negation strategy under Clause Types.",
    learnMore: "Clause Types → Negation owns particles and clause behavior. Do not enter the same marker twice.",
    searchAliases: ["negative verb", "negation morphology", "negative auxiliary"],
    scope: "initial",
    editorKind: "strategy",
    summaryId: "verbs.negative-forms",
    emptyAction: "Configure negative verb forms",
    dependencies: [],
  },
  {
    id: "modifiers.adjective-behavior",
    sectionId: "modifiers",
    label: "Adjective behavior",
    hint: "Describe how adjectives behave morphologically. Placement is configured under Syntax → Adjective position.",
    learnMore: "If adjectives agree with nouns, configure the actual agreement under Agreement.",
    searchAliases: ["adjective", "adjectives", "invariant adjective"],
    scope: "initial",
    editorKind: "strategy",
    summaryId: "modifiers.adjective-behavior",
    emptyAction: "Configure adjective behavior",
    dependencies: [],
  },
  {
    id: "modifiers.comparative",
    sectionId: "modifiers",
    label: "Comparative",
    hint: 'Comparatives express meanings such as "taller", "more beautiful", or "better".',
    learnMore: "Synthetic forms, particles, affixes, and exceed-constructions are all valid strategies.",
    searchAliases: ["comparative", "comparison", "more", "than"],
    scope: "initial",
    editorKind: "strategy",
    summaryId: "modifiers.comparative",
    emptyAction: "Configure comparatives",
    dependencies: [],
  },
  {
    id: "modifiers.superlative",
    sectionId: "modifiers",
    label: "Superlative",
    hint: "Describe how the language expresses the highest degree, if it does so at all.",
    learnMore: "Some languages reuse the comparative or a definite construction instead of a dedicated superlative.",
    searchAliases: ["superlative", "most", "est"],
    scope: "initial",
    editorKind: "strategy",
    summaryId: "modifiers.superlative",
    emptyAction: "Configure superlatives",
    dependencies: [],
  },
  {
    id: "clauses.yes-no-questions",
    sectionId: "clauses",
    label: "Yes/no questions",
    hint: "Describe how polar questions are formed.",
    learnMore:
      "A final question particle is one common strategy among several, including intonation and word-order change.",
    searchAliases: ["questions", "yes no", "polar question", "question particle"],
    scope: "initial",
    editorKind: "clause",
    summaryId: "clauses.yes-no-questions",
    emptyAction: "Configure yes/no questions",
    dependencies: [],
  },
  {
    id: "clauses.content-questions",
    sectionId: "clauses",
    label: "Content questions",
    hint: "Configure where question words appear and which interrogatives the language uses.",
    learnMore: "Interrogatives need not become lexicon entries unless you choose to link them.",
    searchAliases: ["wh-question", "who", "what", "where", "content question", "questions"],
    scope: "initial",
    editorKind: "clause",
    summaryId: "clauses.content-questions",
    emptyAction: "Configure content questions",
    dependencies: [],
  },
  {
    id: "clauses.imperatives",
    sectionId: "clauses",
    label: "Imperatives",
    hint: "Describe how commands are formed.",
    learnMore: "Number, polarity, and politeness distinctions are optional advanced settings.",
    searchAliases: ["imperative", "command", "commands"],
    scope: "initial",
    editorKind: "clause",
    summaryId: "clauses.imperatives",
    emptyAction: "Configure imperatives",
    dependencies: [],
  },
  {
    id: "clauses.negation",
    sectionId: "clauses",
    label: "Negation",
    hint: "This view owns the primary clause-negation strategy. Negative verb morphology belongs under Verbs.",
    learnMore: "If negative verb forms are configured, reference them here instead of copying the marker.",
    searchAliases: ["negation", "negative", "not"],
    scope: "initial",
    editorKind: "clause",
    summaryId: "clauses.negation",
    emptyAction: "Configure negation",
    dependencies: ["verbs.negative-forms"],
  },
  {
    id: "clauses.relative-clauses",
    sectionId: "clauses",
    label: "Relative clauses",
    hint: "Describe how relative clauses are formed. Placement is configured under Syntax.",
    learnMore: "This editor may show a read-only reference to Syntax → Relative clause position.",
    searchAliases: ["relative clause", "relativizer", "resumptive"],
    scope: "initial",
    editorKind: "clause",
    summaryId: "clauses.relative-clauses",
    emptyAction: "Configure relative clauses",
    dependencies: ["syntax.relative-clause-position"],
  },
];

const BY_ID = new Map(GRAMMAR_CATALOG.map((item) => [item.id, item]));

export function grammarSystemDescriptor(id: string) {
  return BY_ID.get(id as GrammarSystemId);
}

export function grammarSectionDescriptor(id: string) {
  return GRAMMAR_SECTIONS.find((item) => item.id === id);
}

export function systemsForSection(sectionId: GrammarSectionId) {
  return GRAMMAR_CATALOG.filter((item) => item.sectionId === sectionId);
}

export function assertCatalogComplete() {
  const ids = new Set(GRAMMAR_CATALOG.map((item) => item.id));
  for (const id of GRAMMAR_SYSTEM_IDS) {
    if (!ids.has(id)) throw new Error(`missing catalog entry: ${id}`);
  }
  if (ids.size !== GRAMMAR_SYSTEM_IDS.length) throw new Error("catalog has extra or duplicate system ids");
}
