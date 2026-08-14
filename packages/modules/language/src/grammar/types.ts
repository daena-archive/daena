export const GRAMMAR_SCHEMA_VERSION = 1 as const;

export type GrammarStatus = "unconfigured" | "configured" | "not-used";

export type GrammarSectionId =
  "syntax" | "nouns" | "pronouns" | "verbs" | "modifiers" | "clauses" | "agreement" | "other";

export const GRAMMAR_SYSTEM_IDS = [
  "syntax.basic-word-order",
  "syntax.adjective-position",
  "syntax.adpositions",
  "syntax.possessive-position",
  "syntax.relative-clause-position",
  "nouns.number",
  "nouns.case",
  "nouns.classes",
  "nouns.definiteness",
  "nouns.possession",
  "pronouns.personal",
  "pronouns.demonstratives",
  "verbs.marking-strategy",
  "verbs.tense",
  "verbs.aspect",
  "verbs.mood",
  "verbs.argument-indexing",
  "verbs.negative-forms",
  "modifiers.adjective-behavior",
  "modifiers.comparative",
  "modifiers.superlative",
  "clauses.yes-no-questions",
  "clauses.content-questions",
  "clauses.imperatives",
  "clauses.negation",
  "clauses.relative-clauses",
] as const;

export type GrammarSystemId = (typeof GRAMMAR_SYSTEM_IDS)[number];

export type GrammarRecordKind = "system" | "agreement" | "custom-rule" | "section-state";

export type GrammarLinkKind = "lexeme" | "lexeme-example" | "sample" | "paradigm";

export type GrammarExample = {
  id: string;
  text: string;
  translation?: string;
  gloss?: string;
  notes?: string;
};

export type GrammarLink = {
  id: string;
  kind: GrammarLinkKind;
  targetId: string;
  secondaryId?: string;
  label?: string;
};

export type ParadigmAxisValue = {
  id: string;
  label: string;
  description?: string;
};

export type ParadigmAxis = {
  id: string;
  label: string;
  values: ParadigmAxisValue[];
};

export type ParadigmCellState = "form" | "same-as" | "zero" | "not-applicable";

export type ParadigmCell = {
  id: string;
  coordinates: Record<string, string>;
  state: ParadigmCellState;
  form?: string;
  alternateForms?: string[];
  sameAsCellId?: string;
  notes?: string;
  exampleId?: string;
};

export type EmptyConfig = Record<string, never>;

export type WordOrderPattern = "sov" | "svo" | "vso" | "vos" | "ovs" | "osv" | "flexible" | "custom";
export type WordOrderStrength = "strict" | "strongly-preferred" | "default-flexible" | "context";
export type WordOrderInfluence = "topic" | "focus" | "emphasis" | "definiteness" | "animacy" | "discourse" | "custom";

export type BasicWordOrderConfig = {
  order: WordOrderPattern;
  customOrder?: string;
  strength?: WordOrderStrength;
  influences: WordOrderInfluence[];
  customInfluence?: string;
  changeNotes?: string;
};

export type PositionChoice = "before" | "after" | "either" | "meaning-changes" | "custom";
export type PositionConfig = {
  position: PositionChoice;
  customPosition?: string;
  alternatePositions: PositionChoice[];
  conditions?: string;
};

export type PossessivePositionChoice =
  "possessor-before" | "possessor-after" | "either" | "morphological" | "multiple" | "custom";

export type PossessivePositionConfig = {
  position: PossessivePositionChoice;
  customPosition?: string;
  alternatePositions: PossessivePositionChoice[];
  conditions?: string;
};

export type RelativeClausePositionChoice = "before" | "after" | "internally-headed" | "multiple" | "custom";

export type RelativeClausePositionConfig = {
  position: RelativeClausePositionChoice;
  customPosition?: string;
  alternatePositions: RelativeClausePositionChoice[];
  conditions?: string;
};

export type AdpositionStrategy = "prepositions" | "postpositions" | "both" | "other";

export type AdpositionsConfig = {
  strategy: AdpositionStrategy;
  distributionNotes?: string;
};

export type NumberCategoryId = "singular" | "plural" | "dual" | "trial" | "paucal" | "collective" | "custom";
export type MarkingStrategy = "affix" | "separate-word" | "stem-change" | "multiple" | "unmarked" | "custom";

export type NumberCategory = {
  id: string;
  templateId?: NumberCategoryId;
  label: string;
  meaning?: string;
  marker?: string;
  position?: string;
  notes?: string;
};

export type NumberConfig = {
  categories: NumberCategory[];
  markingStrategies: MarkingStrategy[];
};

export type CaseTemplateId =
  | "nominative"
  | "accusative"
  | "ergative"
  | "absolutive"
  | "genitive"
  | "dative"
  | "instrumental"
  | "locative"
  | "ablative"
  | "allative"
  | "vocative"
  | "custom";

export type CaseItem = {
  id: string;
  templateId?: CaseTemplateId;
  name: string;
  abbreviation?: string;
  primaryFunction: string;
  additionalFunctions?: string;
  marking?: string;
  notes?: string;
};

export type CaseConfig = {
  cases: CaseItem[];
};

export type NounClassKind = "gender" | "noun-class" | "custom";

export type NounClassItem = {
  id: string;
  name: string;
  abbreviation?: string;
  membership?: string;
  exceptions?: string;
};

export type NounClassesConfig = {
  kind: NounClassKind;
  classes: NounClassItem[];
};

export type DefinitenessStrategy =
  "definite-article" | "indefinite-article" | "both" | "affixes" | "demonstratives" | "context" | "other";

export type ArticleForm = {
  id: string;
  form: string;
  position?: string;
  notes?: string;
};

export type DefinitenessConfig = {
  strategies: DefinitenessStrategy[];
  articles: ArticleForm[];
};

export type PossessionStrategy =
  | "possessive-pronouns"
  | "genitive"
  | "possessor-marking"
  | "possessed-marking"
  | "linking-particle"
  | "word-order"
  | "multiple";

export type PossessionConfig = {
  strategies: PossessionStrategy[];
  alienability?: boolean;
  alienabilityNotes?: string;
};

export type ParadigmConfig = {
  axes: ParadigmAxis[];
  cells: ParadigmCell[];
};

export type DemonstrativeConfig = ParadigmConfig & {
  distances: string[];
};

export type VerbMarkingStrategy =
  | "invariant"
  | "prefixes"
  | "suffixes"
  | "other-affixes"
  | "stem-changes"
  | "auxiliaries"
  | "particles"
  | "multiple"
  | "custom";

export type VerbMarkingConfig = {
  strategies: VerbMarkingStrategy[];
  customStrategy?: string;
};

export type TamCategory = {
  id: string;
  templateId?: string;
  label: string;
  meaning?: string;
  marker?: string;
  interaction?: string;
  notes?: string;
};

export type TamConfig = {
  categories: TamCategory[];
};

export type ArgumentParticipants = "none" | "subject" | "object" | "subject-object" | "other";
export type ArgumentRepresentation =
  "endings" | "prefixes" | "full-forms" | "auxiliaries" | "flexible-table" | "custom";

export type ArgumentIndexingConfig = {
  participants: ArgumentParticipants;
  representation?: ArgumentRepresentation;
  axes: ParadigmAxis[];
  cells: ParadigmCell[];
  flexibleNotes?: string;
  agreementRecordId?: string;
};

export type NegativeVerbStrategy =
  "affix" | "negative-auxiliary" | "special-verb" | "stem-change" | "none" | "multiple" | "custom";

export type NegativeVerbForm = {
  id: string;
  form: string;
  conditions?: string;
  notes?: string;
};

export type NegativeVerbConfig = {
  strategies: NegativeVerbStrategy[];
  forms: NegativeVerbForm[];
};

export type AdjectiveBehaviorKind =
  "invariant" | "agree-with-noun" | "verb-like" | "noun-like" | "multiple-classes" | "custom";

export type AdjectiveBehaviorConfig = {
  behaviors: AdjectiveBehaviorKind[];
  customBehavior?: string;
  agreementRecordIds: string[];
};

export type ComparativeStrategy = "synthetic" | "particle" | "affix" | "exceed" | "special" | "multiple" | "custom";

export type SuperlativeStrategy = "dedicated" | "intensifier" | "comparative" | "definite" | "none" | "custom";

export type DegreeConfig = {
  strategies: string[];
  marker?: string;
  construction?: string;
};

export type YesNoQuestionStrategy =
  "intonation" | "particle" | "word-order" | "verb-morphology" | "auxiliary" | "multiple" | "custom";

export type ParticlePlacement = "clause-initial" | "clause-final" | "before-verb" | "after-verb" | "other";

export type YesNoQuestionsConfig = {
  strategies: YesNoQuestionStrategy[];
  particle?: string;
  placement?: ParticlePlacement;
};

export type ContentQuestionBehavior =
  "in-situ" | "fronted" | "fixed-position" | "special-structure" | "mixed" | "custom";

export type InterrogativeItem = {
  id: string;
  meaning: string;
  form?: string;
  lexemeId?: string;
};

export type ContentQuestionsConfig = {
  behavior: ContentQuestionBehavior;
  customBehavior?: string;
  interrogatives: InterrogativeItem[];
};

export type ImperativeStrategy =
  "bare-verb" | "special-form" | "particle" | "auxiliary" | "word-order" | "multiple" | "custom";

export type ImperativesConfig = {
  strategies: ImperativeStrategy[];
  numberDistinction?: boolean;
  polarityDistinction?: boolean;
  politenessDistinction?: boolean;
};

export type ClauseNegationStrategy = "particle" | "affix" | "auxiliary" | "special-verb" | "multiple" | "custom";

export type ClauseNegationConfig = {
  strategies: ClauseNegationStrategy[];
  particle?: string;
  placement?: ParticlePlacement;
  negativeQuestions?: string;
  negativeImperatives?: string;
};

export type RelativizationStrategy =
  "relative-pronoun" | "complementizer" | "gap" | "resumptive" | "internally-headed" | "multiple" | "custom";

export type RelativeClausesConfig = {
  strategies: RelativizationStrategy[];
  headBehavior?: string;
  resumptives?: string;
};

export type GrammarSystemConfig =
  | EmptyConfig
  | BasicWordOrderConfig
  | PositionConfig
  | PossessivePositionConfig
  | RelativeClausePositionConfig
  | AdpositionsConfig
  | NumberConfig
  | CaseConfig
  | NounClassesConfig
  | DefinitenessConfig
  | PossessionConfig
  | ParadigmConfig
  | DemonstrativeConfig
  | VerbMarkingConfig
  | TamConfig
  | ArgumentIndexingConfig
  | NegativeVerbConfig
  | AdjectiveBehaviorConfig
  | DegreeConfig
  | YesNoQuestionsConfig
  | ContentQuestionsConfig
  | ImperativesConfig
  | ClauseNegationConfig
  | RelativeClausesConfig;

export type GrammarSystemRecord = {
  recordKind: "system";
  schemaVersion: 1;
  systemId: GrammarSystemId;
  status: GrammarStatus;
  config: GrammarSystemConfig;
  notes: string;
  examples: GrammarExample[];
  links: GrammarLink[];
};

export type AgreementControllerKind = "subject" | "object" | "noun" | "possessor" | "custom";
export type AgreementTargetKind = "verb" | "adjective" | "article" | "pronoun" | "participle" | "custom";
export type AgreementBehavior = "full" | "partial" | "conditional";

export type AgreementFeature = {
  sourceSystemId?: GrammarSystemId;
  categoryId?: string;
  label: string;
};

export type AgreementEndpoint = {
  kind: AgreementControllerKind | AgreementTargetKind;
  customLabel?: string;
};

export type GrammarAgreementRecord = {
  recordKind: "agreement";
  schemaVersion: 1;
  title: string;
  controller: AgreementEndpoint;
  target: AgreementEndpoint;
  features: AgreementFeature[];
  behavior: AgreementBehavior;
  defaultForm?: string;
  conditions?: string;
  exceptions?: string;
  notes: string;
  examples: GrammarExample[];
  links: GrammarLink[];
};

export type GrammarCustomRuleRecord = {
  recordKind: "custom-rule";
  schemaVersion: 1;
  title: string;
  tags: string[];
  body: string;
  examples: GrammarExample[];
  links: GrammarLink[];
};

export type GrammarSectionStateRecord = {
  recordKind: "section-state";
  schemaVersion: 1;
  sectionId: "agreement";
  status: "not-used";
  note?: string;
};

export type GrammarRecord =
  GrammarSystemRecord | GrammarAgreementRecord | GrammarCustomRuleRecord | GrammarSectionStateRecord;

export type GrammarIssueCode =
  | "legacy-topic"
  | "unknown-kind"
  | "unknown-system"
  | "invalid-schema-version"
  | "invalid-status"
  | "empty-config-required"
  | "configured-minimum"
  | "malformed"
  | "duplicate-system"
  | "broken-reference";

export type GrammarIssue = {
  code: GrammarIssueCode;
  message: string;
  path?: string;
};

export type NormalizeOk = { ok: true; record: GrammarRecord; issues: GrammarIssue[] };
export type NormalizeErr = { ok: false; issues: GrammarIssue[] };
export type NormalizeResult = NormalizeOk | NormalizeErr;

export type LoadedGrammarRecord = {
  id: string;
  revision: string;
  value: GrammarRecord;
};

export type GrammarDiagnostic = GrammarIssue & {
  recordIds: string[];
  systemId?: GrammarSystemId;
};

export type IndexedGrammar = {
  systems: Map<GrammarSystemId, LoadedGrammarRecord>;
  duplicates: Map<GrammarSystemId, string[]>;
  agreements: LoadedGrammarRecord[];
  customRules: LoadedGrammarRecord[];
  sectionStates: Map<string, LoadedGrammarRecord>;
  rejected: { id: string; issues: GrammarIssue[] }[];
  diagnostics: GrammarDiagnostic[];
};

export type GrammarSearchHit = {
  kind: "system" | "custom-rule" | "agreement";
  systemId?: GrammarSystemId;
  sectionId: GrammarSectionId;
  label: string;
  status?: GrammarStatus;
  summary: string;
  recordId?: string;
};
