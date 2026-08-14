# Daena Language Module — Grammar System Product Specification

## Authority and implementation posture

This document is the product and implementation authority for the Language
module's Grammar area. It must remain consistent with `ARCHITECTURE.md`,
`STORAGE.md`, and the public module-record contract.

The implementation described here is an alpha hard cut. Replace the existing
generic grammar-topic record shape and UI; do not add a legacy reader,
converter, dual-write path, feature flag, or data migration. Existing alpha
projects or fixtures containing the old `{ title, section, body, links }`
grammar records are incompatible and must be recreated or have those records
removed.

## Purpose

The grammar area of Daena's Language module should function as a guided language-design workspace, not as a collection of generic categorized notes.

The current structure — `grammar`, `nouns`, `pronouns`, `verbs`, `modifiers`, `clauses`, `agreements`, and `others` — is useful as a broad taxonomy, but the shared creation flow makes fundamentally different grammatical systems feel identical. A word-order rule, a pronoun paradigm, a noun case system, and subject–verb agreement should not be created or edited through the same form.

The desired product should:

- guide users through designing grammatical systems;
- use specialized editors for different kinds of grammar;
- explain linguistic concepts without requiring prior linguistics knowledge;
- support both simple and highly detailed constructed languages;
- avoid forcing users to configure systems their language does not use;
- make the current grammar of a language easy to inspect at a glance;
- preserve a flexible escape hatch for unusual or unsupported features;
- allow complexity to be introduced progressively rather than all at once.

The core product principle is:

> Ask what grammatical system exists first, then present the workflow appropriate for that system.

---

# 1. Overall Grammar Experience

## 1.1 Grammar Home

The Grammar area should open to an overview rather than a generic list of rules.

The page should answer two questions immediately:

1. What grammatical systems has this language defined?
2. What important areas remain undescribed?

Suggested layout:

```text
Grammar
────────────────────────────────────────

Define how sentences and words behave in this language.
You do not need to configure every system.

[ Syntax ]       4 systems configured
[ Nouns ]        3 systems configured
[ Pronouns ]     Basic paradigm configured
[ Verbs ]        5 systems configured
[ Modifiers ]    2 systems configured
[ Clauses ]      3 systems configured
[ Agreement ]    None configured
[ Other Rules ]  2 custom rules
```

Below the section cards, optionally show a compact grammar summary:

```text
At a glance

Basic word order     SOV
Adjective position   Before noun
Case system          Nominative–Accusative, 5 cases
Number               Singular / Plural
Verb tense           Past / Present / Future
Questions            Final particle
Negation             Pre-verbal particle
```

This should be a read-only summary generated from configured systems, not a separate data-entry surface.

---

## 1.2 Section Navigation

Retain recognizable high-level areas, but rename some of them for clarity.

Recommended final navigation:

1. **Syntax**
2. **Nouns**
3. **Pronouns**
4. **Verbs**
5. **Modifiers & Comparison**
6. **Clause Types**
7. **Agreement**
8. **Other Rules**

### Changes from the current design

- Rename `grammar` to **Syntax**.
  - "Grammar" already names the whole area.
  - The existing section is more naturally the place for word order and structural syntax.

- Rename `modifiers` to **Modifiers & Comparison**.
  - This clarifies that adjectives, adverbs, comparison, and related systems belong here.

- Rename `clauses` to **Clause Types**.
  - The term should describe what the user actually configures.

- Rename `agreements` to **Agreement**.
  - Treat it as a grammatical system rather than a bucket of arbitrary rules.

- Rename `others` to **Other Rules**.
  - Make its purpose explicit as an escape hatch.

---

# 2. Shared UX Principles

## 2.1 Systems, Not Generic Rules

Most grammar entries should be presented as named grammatical systems.

For example:

```text
Nouns

Number                 Singular / Plural
Case                    5 cases
Gender / noun classes   Not used
Definiteness            Articles
Possession              Configured
```

The user opens one system and configures it through a workflow designed specifically for it.

Do not present a universal:

```text
+ Add rule
Title
Description
Examples
```

except inside **Other Rules**.

---

## 2.2 Configured / Not Configured / Not Used

Each grammatical system should support three meaningful states:

- **Not configured**
  - The user has not decided or documented it yet.

- **Configured**
  - The system has meaningful data.

- **Not used**
  - The language intentionally lacks that grammatical feature.

This distinction matters.

For example:

```text
Case
Not configured
```

means "I have not designed this yet."

While:

```text
Case
Not used
Noun roles are primarily expressed through word order and adpositions.
```

means the user has made a design decision.

Where useful, allow an optional note when marking a system as not used.

These are the canonical states throughout the product:

- `unconfigured` — shown as **Not configured**;
- `configured` — shown as **Configured**, followed by a generated summary; and
- `not-used` — shown as **Not used**, with an optional explanation.

Do not introduce parallel state labels such as "Not decided", "No", or "Not
yet defined" inside individual editors. Yes/no questions may be used as
friendly prompts, but they must map to the canonical states rather than create
another persisted state model.

---

## 2.3 Progressive Disclosure

Do not show advanced linguistic dimensions until the user enables them.

Example for pronouns:

Initial editor:

```text
Pronoun system

Persons
[x] First
[x] Second
[x] Third

Numbers
[x] Singular
[x] Plural

[ Create paradigm ]
```

Advanced dimensions remain hidden behind:

```text
+ Add another distinction
```

which may offer:

- gender;
- case;
- inclusive / exclusive;
- formality;
- animacy;
- custom dimension.

The paradigm grows only when the user explicitly adds complexity.

---

## 2.4 Inline Help

Every system should provide three levels of assistance.

### Level 1 — short hint

Always visible and concise.

Example:

> Basic word order describes the usual order of subject, object, and verb in a simple statement.

### Level 2 — option explanations

Shown alongside selectable choices.

Example:

```text
SOV — Subject → Object → Verb
"The hunter the deer sees."

SVO — Subject → Verb → Object
"The hunter sees the deer."
```

Examples should prioritize illustrating structure rather than sounding natural in English.

### Level 3 — Learn more

Optional expandable help for users who want linguistic context.

It may explain:

- terminology;
- unusual alternatives;
- common combinations;
- caveats;
- cross-linguistic examples;
- what the setting does not imply.

The normal editing workflow should never require opening this help.

---

## 2.5 Examples

Most systems should allow examples.

Examples should be visually secondary to the grammatical configuration.

Recommended example format:

```text
Example

Nar bel tor.
nar   bel   tor
I     bread eat

"I eat bread."
```

Support as much or as little detail as the user wants:

- example sentence only;
- translation;
- gloss;
- notes.

Do not require interlinear glossing.

---

## 2.6 "Start Simple" Guidance

Every major section should begin with a short orientation message.

For example:

> Start with the distinctions your language actually uses. You can add exceptions, irregular forms, and advanced distinctions later.

Avoid presenting every possible grammatical feature as a checklist the user is expected to complete.

---

# 3. Syntax

## Purpose

Syntax describes the structural ordering of words and phrases.

Suggested landing page:

```text
Syntax

Describe how words and phrases are normally arranged.

Basic word order             SOV
Adjective position           Before noun
Adpositions                  Postpositions
Possessive position          Possessor before noun
Relative clause position     Not configured
Information structure        Not configured
```

Recommended systems:

1. Basic Word Order
2. Adjective Position
3. Adpositions
4. Possessive Position
5. Relative Clause Position
6. Flexible Word Order / Information Structure
7. Custom Syntax Rules

---

## 3.1 Basic Word Order

### Hint

> Choose the usual order of subject, object, and verb in a simple declarative sentence. This describes the default pattern, not every possible sentence.

### Primary choices

- SOV
- SVO
- VSO
- VOS
- OVS
- OSV
- Flexible
- Custom

Each choice should show its expansion.

Example card:

```text
SOV
Subject → Object → Verb

"The hunter the deer sees."
```

### Follow-up

After selecting a pattern:

```text
How strong is this ordering?

( ) Strict
( ) Strongly preferred
( ) Default, but flexible
( ) Mostly determined by context
```

Then:

```text
What can cause the order to change?
[ optional notes ]
```

Allow examples and exceptions below.

### Flexible order

If the user chooses `Flexible`, ask what influences order:

- topic;
- focus;
- emphasis;
- definiteness;
- animacy;
- discourse context;
- custom.

Do not require any.

---

## 3.2 Adjective Position

### Hint

> Define where attributive adjectives usually appear relative to the noun they describe.

Choices:

- before noun;
- after noun;
- either position;
- position changes meaning;
- custom.

Example:

```text
Before noun
"red house"

After noun
"house red"
```

Optional follow-up:

```text
Does adjective position change in special situations?
```

Allow notes and examples.

---

## 3.3 Adpositions

### Hint

> Adpositions express relationships such as "in", "to", "from", or "with". They may appear before or after the noun phrase.

Choices:

- prepositions;
- postpositions;
- both;
- other strategy;
- not used.

If `both` is selected, allow the user to describe when each appears.

Case marking is not an alternative value for adposition order: languages may
use case and adpositions together. When adpositions are not used, the optional
not-used note may refer the user to **Nouns → Case** or another strategy.

---

## 3.4 Possessive Position

Choices:

- possessor before noun;
- possessor after noun;
- either;
- encoded morphologically;
- multiple strategies;
- custom.

Example:

```text
Possessor before noun
"the king's sword"

Possessor after noun
"the sword of the king"
```

This section describes ordering only. Detailed possession morphology belongs under **Nouns → Possession**.

---

## 3.5 Relative Clause Position

Choices:

- before noun;
- after noun;
- internally headed;
- multiple strategies;
- custom.

Keep detailed relative-clause behavior under **Clause Types**.

This system only defines placement.

---

# 4. Nouns

## Purpose

The Nouns section describes grammatical systems that affect nouns and noun phrases.

Landing page:

```text
Nouns

Number                 Singular / Plural
Case                    5 cases
Gender / noun classes   Not used
Definiteness            Definite article
Possession              Configured
Classifiers             Not used
Declension classes      Not configured
```

Recommended systems:

1. Number
2. Case
3. Gender / Noun Classes
4. Definiteness & Articles
5. Possession
6. Classifiers
7. Declension Classes
8. Other Nominal Rules

---

## 4.1 Number

### Hint

> Number distinguishes quantities grammatically, such as one, two, or many. A language does not need to distinguish only singular and plural.

Initial choices:

- Singular
- Plural
- Dual
- Trial
- Paucal
- Collective
- Custom

The user selects the categories their language distinguishes.

Example:

```text
Number categories

[x] Singular
[x] Plural
[ ] Dual
[ ] Paucal

How is number usually expressed?

( ) Affix
( ) Separate word
( ) Stem change
( ) Multiple strategies
( ) Usually unmarked
```

For each category, allow:

- label;
- meaning;
- marker;
- position;
- examples;
- notes.

Do not force one morphological mechanism per language. A user should be able to describe mixed strategies.

---

## 4.2 Case

### Hint

> Cases mark the grammatical role or relationship of a noun. If your language expresses these relationships mainly through word order or adpositions, you may not need grammatical case.

Landing state:

```text
Does this language use grammatical case?

[ Configure case ] [ Mark as not used ]
```

Leaving the system untouched means **Not configured**.

If yes:

```text
Cases

Nominative     Subject
Accusative     Direct object
Genitive       Possession
Dative         Recipient / goal
Locative       Location

+ Add case
```

### Add Case workflow

Start with common templates:

- nominative;
- accusative;
- ergative;
- absolutive;
- genitive;
- dative;
- instrumental;
- locative;
- ablative;
- allative;
- vocative;
- custom.

Selecting one pre-fills its usual description, but everything remains editable.

Each case contains:

```text
Name
Abbreviation
Primary function
Additional functions
How it is marked
Examples
Notes
```

The product should not imply that case names have universal exact meanings.

---

## 4.3 Gender / Noun Classes

### Hint

> Some languages group nouns into grammatical classes that can affect articles, adjectives, pronouns, or verbs. These classes do not need to correspond to natural gender.

Initial choice:

- no grammatical classes;
- gender system;
- noun class system;
- custom classification.

Then allow defining classes:

```text
Classes

Masculine
Feminine
Neuter

+ Add class
```

Each class may include:

- name;
- abbreviation;
- typical membership;
- exceptions;
- examples.

Do not put agreement behavior here. Agreement belongs under **Agreement**, though this system supplies the categories used there.

---

## 4.4 Definiteness & Articles

Possible strategies:

- definite article;
- indefinite article;
- both;
- affixes;
- demonstratives;
- context only;
- other;
- no grammatical definiteness distinction.

If articles exist, configure:

- form;
- position;
- examples.

Avoid requiring one article form if articles inflect.

Article agreement is configured under **Agreement → Noun → Article**. This
editor may link to that system but must not store a second copy of the
agreement rules.

---

## 4.5 Possession

Distinguish major strategies:

- possessive pronouns;
- genitive marking;
- possessor marking;
- possessed-noun marking;
- linking particle;
- word order only;
- multiple strategies.

Optional advanced distinction:

```text
Does the language distinguish alienable and inalienable possession?
[ No ] [ Yes ]
```

Do not show this unless the user expands advanced options.

---

# 5. Pronouns

## Purpose

Pronouns should primarily use paradigm-based editing.

Landing page:

```text
Pronouns

Personal pronouns       Configured
Possessive pronouns     Not configured
Demonstratives          Configured
Reflexives              Not configured
Relative pronouns       Not used
Interrogative pronouns  Not configured
```

---

## 5.1 Personal Pronouns

### Hint

> Start with person and number. Add distinctions such as gender, case, or inclusive/exclusive forms only if your language uses them.

### Step 1 — choose dimensions

```text
Person

[x] First
[x] Second
[x] Third

Number

[x] Singular
[x] Plural

+ Add distinction
```

Additional distinctions:

- dual;
- inclusive / exclusive;
- gender;
- noun class;
- case;
- animacy;
- formality;
- proximity;
- custom dimension.

### Step 2 — generated paradigm

Example:

|            | Singular | Plural |
| ---------- | -------- | ------ |
| 1st person | na       | nar    |
| 2nd person | ta       | tar    |
| 3rd person | sa       | sar    |

Cells should be directly editable.

A cell may optionally contain:

- primary form;
- alternate forms;
- notes;
- example.

### Missing forms

Allow cells to be explicitly marked:

- same as another form;
- no distinct form;
- omitted / zero;
- not applicable.

Do not force every paradigm cell to contain a unique string.

---

## 5.2 Possessive Pronouns

Allow the user to specify whether possessive forms are:

- independent pronouns;
- determiners;
- affixes;
- derived from personal pronouns;
- multiple strategies.

If they systematically derive from personal pronouns, allow a concise rule instead of requiring another full paradigm.

---

## 5.3 Demonstratives

Suggested initial dimensions:

```text
Distance distinctions

[x] Proximal      "this"
[x] Distal        "that"
[ ] Medial
[ ] Very distant
```

Optional dimensions:

- number;
- gender/class;
- visibility;
- elevation;
- direction;
- discourse status.

Generate a paradigm only from selected dimensions.

---

# 6. Verbs

## Purpose

The Verbs section should describe how verbs encode time, event structure, modality, participants, and related categories.

Landing page:

```text
Verbs

Verb marking strategy      Affixes
Tense                      Past / Present / Future
Aspect                     Perfective / Imperfective
Mood                       Indicative / Imperative
Argument indexing          Subject · Person / Number
Negative verb forms        Not configured
Voice                      Not configured
Conjugation classes        2 classes
```

Recommended systems:

1. Verb Marking Strategy
2. Tense
3. Aspect
4. Mood
5. Argument Indexing
6. Negative Verb Forms
7. Voice
8. Non-finite Forms
9. Conjugation Classes
10. Irregular Verbs

---

## 6.1 Verb Marking Strategy

This should appear near the top because it influences the rest of the experience.

### Hint

> Languages can express grammatical information by changing the verb, adding particles or auxiliaries, or combining several strategies.

Choices:

- verb usually does not change;
- prefixes;
- suffixes;
- other affixes;
- stem changes;
- auxiliary verbs;
- particles;
- multiple strategies;
- custom.

This selection should guide wording elsewhere, but should not restrict what the user can configure.

---

## 6.2 Tense

### Hint

> Tense locates an event in time. Some languages grammatically mark several tenses; others rely mostly on context or aspect.

Initial choice:

```text
Does this language grammatically mark tense?

[ Configure tense ] [ Mark as not used ]
```

Leaving the system untouched means **Not configured**.

If yes, allow selecting or adding:

- past;
- present;
- future;
- recent past;
- remote past;
- near future;
- remote future;
- custom.

Each tense can define:

- meaning;
- marker or construction;
- interaction with aspect;
- examples;
- notes.

Avoid assuming past/present/future is the universal default.

---

## 6.3 Aspect

Templates may include:

- perfective;
- imperfective;
- progressive;
- habitual;
- perfect;
- prospective;
- iterative;
- custom.

The UI should explain each briefly.

Example:

```text
Perfective
Presents an event as a bounded whole.

Imperfective
Presents an event as ongoing, habitual, or internally structured.
```

The user should be allowed to define their own meanings.

---

## 6.4 Mood

Templates:

- indicative;
- imperative;
- subjunctive;
- conditional;
- optative;
- potential;
- irrealis;
- jussive;
- custom.

Do not overload the user with the full list initially. Show common choices first and place the rest under **More**.

---

## 6.5 Argument Indexing

This system describes forms on or around the verb that index one or more
participants. "Agreement" is one possible analysis, but not all argument
indexing is best treated as agreement. The persisted system is therefore named
**Argument Indexing**; plain-language help may say that the verb changes based
on its participants.

Start with:

```text
Do verbs change based on their participants?

[ No ]
[ Subject only ]
[ Object only ]
[ Subject and object ]
[ Other ]
```

If subject-only agreement is selected, generate a paradigm from the language's defined person/number categories.

Example:

|     | Singular | Plural |
| --- | -------- | ------ |
| 1st | -m       | -men   |
| 2nd | -t       | -ten   |
| 3rd | -s       | -sen   |

Do not assume that the entries must be complete verb forms. Let the user label the paradigm as:

- endings;
- prefixes;
- full forms;
- auxiliary forms;
- custom.

For complex polypersonal systems, allow the user to switch to a more flexible table rather than forcing a simple matrix.

If the user chooses to analyze the same behavior as agreement, link to a
single **Agreement** system and derive the display here from it. Never persist
independent copies of the same person/number rules in both sections.

---

## 6.6 Negative Verb Forms

### Hint

> Describe negative morphology or special negative forms of the verb. Configure
> the language's primary clause-negation strategy under Clause Types.

Choices:

- affix;
- negative auxiliary;
- special negative verb;
- stem change;
- no special verb form;
- multiple strategies;
- custom.

Then allow:

- marker/form;
- changes by tense/mood;
- examples.

**Clause Types → Negation** owns the primary clause-negation construction,
including negative particles and their placement. It may reference the
negative verb forms defined here. Never require the same marker or rule to be
entered twice.

---

# 7. Modifiers & Comparison

## Purpose

This section describes adjectives, adverbs, degree, and comparison.

Landing page:

```text
Modifiers & Comparison

Adjectives               Configured
Adjective agreement      See Agreement
Adverbs                  Not configured
Comparatives             Particle
Superlatives             Affix
Degree                    Not configured
```

Recommended systems:

1. Adjective Behavior
2. Adverb Formation
3. Comparative
4. Superlative
5. Degree
6. Other Modifier Rules

---

## 7.1 Adjective Behavior

Do not duplicate adjective position from Syntax.

Instead describe how adjectives behave morphologically:

- invariant;
- agree with noun;
- behave like verbs;
- behave like nouns;
- multiple classes;
- custom.

If `agree with noun` is selected, direct the user toward configuring the actual agreement under **Agreement**.

---

## 7.2 Comparative

### Hint

> Comparatives express meanings such as "taller", "more beautiful", or "better".

Strategies:

- synthetic form;
- comparative particle;
- comparative affix;
- verb meaning "exceed";
- special construction;
- multiple strategies;
- custom.

Example:

```text
Synthetic
tall → taller

Particle
more + tall

Exceed construction
A exceeds B in height
```

Allow irregular comparative forms as examples or exceptions.

---

## 7.3 Superlative

Strategies:

- dedicated superlative morphology;
- intensifier;
- comparative construction;
- definite construction;
- no dedicated superlative;
- custom.

---

# 8. Clause Types

## Purpose

Clause Types describes how different kinds of sentences and subordinate structures are formed.

Landing page:

```text
Clause Types

Declaratives          Default syntax
Yes/no questions      Final particle
Content questions     In-situ question words
Imperatives           Verb form
Negation              Pre-verbal particle
Relative clauses      Post-nominal
Coordination          Configured
Subordination         Not configured
Conditionals          Not configured
```

Recommended systems:

1. Declarative Clauses
2. Yes/No Questions
3. Content Questions
4. Imperatives
5. Negation
6. Relative Clauses
7. Coordination
8. Subordination
9. Conditionals
10. Complement Clauses

---

## 8.1 Declarative Clauses

Keep this lightweight.

Use it mainly for deviations from the default syntax.

Hint:

> Basic word order is configured under Syntax. Use this section for clause-level behavior that applies specifically to ordinary statements.

---

## 8.2 Yes/No Questions

Choices:

- intonation only;
- question particle;
- word-order change;
- verb morphology;
- auxiliary;
- multiple strategies;
- custom.

If `question particle` is selected:

```text
Particle
[ ma ]

Position
( ) Beginning of clause
( ) End of clause
( ) Before verb
( ) After verb
( ) Other
```

Examples below.

---

## 8.3 Content Questions

Configure question-word behavior:

- remain in normal position;
- move to beginning;
- move to another fixed position;
- special clause structure;
- mixed;
- custom.

Allow linking or defining common interrogatives:

- who;
- what;
- where;
- when;
- why;
- how.

These should not necessarily become lexicon entries automatically unless the user chooses to create/link them.

---

## 8.4 Imperatives

Possible strategies:

- bare verb;
- special verb form;
- particle;
- auxiliary;
- word-order change;
- multiple forms based on politeness/number;
- custom.

Optional advanced configuration:

- singular vs plural imperative;
- positive vs negative imperative;
- polite imperative.

---

## 8.5 Negation

This view owns the primary clause-negation strategy and clause behavior. It
must reference, rather than duplicate, morphology configured under **Verbs →
Negative Verb Forms**.

For example:

```text
Clause negation

Primary strategy
Pre-verbal particle "ne"

Negative questions
Particle remains before the verb.

Negative imperatives
Use "na" instead.
```

If negative verb forms are configured under Verbs, show a read-only reference
to that system here and let the user add the clause construction around it.

Avoid forcing the same information to be entered twice.

---

# 9. Agreement

## Purpose

Agreement should represent relationships between grammatical elements rather than arbitrary prose rules.

Landing page:

```text
Agreement

Subject → Verb
Person, Number

Noun → Adjective
Gender, Number

Noun → Article
Case, Number

+ Add agreement system
```

---

## 9.1 Creating an Agreement System

### Step 1 — Controller

Ask:

> Which element determines the grammatical features?

Examples:

- subject;
- object;
- noun;
- possessor;
- custom.

### Step 2 — Target

Ask:

> Which element changes to match it?

Examples:

- verb;
- adjective;
- article;
- pronoun;
- participle;
- custom.

### Step 3 — Features

Present features already defined elsewhere in the language first:

- person;
- number;
- gender;
- noun class;
- case;
- animacy;
- definiteness;
- custom.

Example:

```text
Subject → Verb

The verb agrees with the subject in:

[x] Person
[x] Number
[ ] Gender
```

### Step 4 — Behavior

Allow:

- full agreement;
- partial agreement;
- conditional agreement;
- default form;
- exceptions.

Example:

```text
Third-person verbs agree in number only.
Past-tense verbs do not mark person.
```

The editor should encourage structured choices but always retain a notes area for exceptions.

---

# 10. Other Rules

## Purpose

Other Rules is the deliberate escape hatch for grammar that does not fit the built-in systems.

Hint:

> Use this for grammatical features that do not fit Daena's built-in grammar systems. If a feature becomes common enough, it may eventually deserve its own dedicated editor.

This is the one section where a generic rule workflow is appropriate.

Each rule:

```text
Title
Category / tags
Description
Examples
Notes
```

Suggested optional tags:

- syntax;
- morphology;
- phonology interaction;
- discourse;
- irregularity;
- historical;
- custom.

Do not force a category.

---

# 11. Section-Level Empty States

Empty states should educate without overwhelming.

Example for Nouns:

```text
Nouns

Nothing configured yet.

Noun grammar can describe things such as number, case,
gender or noun classes, articles, and possession.

A simple language may only need number and possession.

[ Configure Number ]
[ Browse noun systems ]
```

Example for Agreement:

```text
Agreement

No agreement systems are defined.

Agreement means that one word changes based on grammatical
features of another, such as a verb changing for the person
and number of its subject.

If your language does not use agreement, you can mark this
section as not used.

[ Add agreement system ]
[ Mark as not used ]
```

The goal is to provide a useful starting point instead of an intimidating blank page.

---

# 12. Suggested Onboarding Flow

Do not force users through a full grammar wizard.

Instead, optionally offer a lightweight "Grammar Starter" action.

```text
Start your grammar

Choose a few foundational systems now. Everything can be
changed later.

1. Basic word order
2. Noun number
3. Pronouns
4. Verb tense
5. Questions
6. Negation

[ Start ]
[ I'll configure grammar manually ]
```

The starter should take the user through only high-value systems.

Recommended initial path:

1. Basic word order
2. Adjective position
3. Number
4. Personal pronouns
5. Tense behavior
6. Questions
7. Negation

At completion, take the user to the normal Grammar overview.

This should be optional and dismissible.

---

# 13. Cross-Section Relationships

The product should avoid making the user enter the same decision repeatedly.

Examples:

- **Syntax → Adjective Position** defines where adjectives appear.
- **Modifiers → Adjective Behavior** defines how adjectives morphologically behave.
- **Agreement → Noun → Adjective** defines which features adjectives agree in.

These are related but distinct.

Likewise:

- **Nouns → Gender / Noun Classes** defines the available classes.
- **Pronouns** may use those classes as a paradigm dimension.
- **Agreement** may use those classes as agreement features.

When one section depends on another, show a contextual reference:

```text
Gender agreement
Uses the noun classes defined under Nouns.

[ View noun classes ]
```

Do not clone the underlying data into multiple editors.

---

# 14. Summary and Readability

Users should be able to understand the language's grammar without opening every editor.

Each system should have a concise generated summary.

Examples:

```text
Basic word order
SOV · Default but flexible

Number
Singular, plural, dual · suffix marking

Case
5 cases · suffix marking

Personal pronouns
3 persons · singular/plural · gender distinction in 3rd person

Tense
Past, present, future · verb suffixes

Questions
Yes/no questions use final particle "ma"
```

These summaries should appear:

- on section landing pages;
- on the Grammar overview;
- anywhere Daena later provides a language summary.

---

# 15. Search and Discoverability

Because grammar grows large, users should be able to search systems by concept.

Search terms such as:

```text
plural
past tense
questions
adjective order
possession
ergative
```

should lead to the relevant system even if the exact section name differs.

When the user searches for a grammatical concept that is not configured, Daena should still surface the system:

```text
Case
Nouns → Case
Not configured
```

This makes the grammar model discoverable rather than requiring the user to know the taxonomy.

---

# 16. Advanced Features Should Stay Secondary

The product should support advanced conlanging without presenting advanced linguistics as the default experience.

Examples of features that should be available but progressively disclosed:

- clusivity;
- evidentiality;
- switch-reference;
- obviation;
- inverse systems;
- direct/inverse alignment;
- polypersonal agreement;
- noun incorporation;
- applicatives;
- valency-changing morphology;
- serial verbs;
- converbs;
- grammaticalized honorific systems;
- complex case syncretism.

These can initially live under:

- advanced options inside an existing system;
- custom dimensions;
- Other Rules.

A feature should become a dedicated first-class editor only when the product can provide a genuinely better workflow than structured notes.

---

# 17. Features to Avoid in the First Product Iteration

Do not attempt to make Daena:

- validate whether a language is typologically plausible;
- automatically enforce linguistic universals;
- generate a complete grammar from a few settings;
- reject unusual combinations;
- force every grammatical category into a predefined taxonomy;
- automatically generate words or conjugations unless explicitly requested;
- require IPA, glosses, or linguistic notation;
- behave like an academic linguistics tool by default.

Daena should help users describe the language they intend to create, including deliberately unusual languages.

---

# 18. Product Tone

The Grammar module should feel like a knowledgeable assistant embedded in a design tool.

Good:

> Case marks grammatical roles on nouns. If your language relies mostly on word order or adpositions, you may not need it.

Avoid:

> Configure morphological case declension parameters.

Good:

> Does the verb change depending on who performs the action?

Avoid:

> Enable person-indexing morphology.

Technical terminology may still appear, but plain-language explanations should accompany it.

---

# 19. Recommended First Product Scope

The initial specialized grammar iteration should focus on systems that provide the largest UX improvement over generic notes.

## Syntax

- Basic word order
- Adjective position
- Adpositions
- Possessive position

## Nouns

- Number
- Case
- Gender / noun classes
- Definiteness
- Possession

## Pronouns

- Personal pronoun paradigm
- Demonstratives

## Verbs

- Verb marking strategy
- Tense
- Aspect
- Mood
- Argument indexing
- Negative verb forms

## Modifiers & Comparison

- Adjective behavior
- Comparative
- Superlative

## Clause Types

- Yes/no questions
- Content questions
- Imperatives
- Negation
- Relative clauses

## Agreement

- Controller → target agreement definitions

## Other Rules

- Generic structured rules for everything else

This scope is already enough to make grammar feel like a real language-design tool rather than a categorized notebook.

---

# 20. Final Product Model

The desired grammar experience can be summarized as:

```text
Grammar
│
├── Syntax
│   ├── Basic Word Order
│   ├── Adjective Position
│   ├── Adpositions
│   ├── Possessive Position
│   └── Relative Clause Position
│
├── Nouns
│   ├── Number
│   ├── Case
│   ├── Gender / Noun Classes
│   ├── Definiteness
│   └── Possession
│
├── Pronouns
│   ├── Personal Pronouns
│   ├── Possessive Pronouns
│   └── Demonstratives
│
├── Verbs
│   ├── Marking Strategy
│   ├── Tense
│   ├── Aspect
│   ├── Mood
│   ├── Argument Indexing
│   ├── Negative Verb Forms
│   └── Voice
│
├── Modifiers & Comparison
│   ├── Adjective Behavior
│   ├── Adverbs
│   ├── Comparative
│   └── Superlative
│
├── Clause Types
│   ├── Declaratives
│   ├── Questions
│   ├── Imperatives
│   ├── Negation
│   ├── Relative Clauses
│   ├── Coordination
│   └── Subordination
│
├── Agreement
│   └── Controller → Target Systems
│
└── Other Rules
    └── Free-form structured grammar rules
```

The defining change is not merely adding more fields. It is giving each grammatical concept an interaction model that matches what the user is trying to design.

A user designing basic word order should choose and describe an ordering system.

A user designing pronouns should build a paradigm.

A user designing case should define a case inventory.

A user designing agreement should connect controllers, targets, and grammatical features.

A user describing something Daena does not yet understand should still be able to record it freely.

That combination of specialized workflows, progressive disclosure, contextual education, and an unrestricted fallback should make the Grammar area approachable for casual worldbuilders while remaining useful for increasingly sophisticated conlangs.

---

# 21. Verified Implementation Baseline

The Grammar area is the hard-cut specialized system described in this
document, not the former Markdown topic list:

- `packages/modules/language/src/grammar/` holds discriminated records,
  catalog, normalizers, specialized editors, Agreement, Other Rules, and
  Grammar Starter. `grammar.ts` is the public barrel; `index.ts` only
  orchestrates the Language pane.
- Collection `grammar` remains Language-owned module records. The packaged
  manifest schema is CommandSchema-safe (a single object with
  `additionalProperties: false`). The authored contract with `oneOf`
  branches keyed by `recordKind` and `systemId` lives in
  `GRAMMAR_VALUE_SCHEMA` and is enforced by Language normalizers and tests.
- There is no `language-v2` migration, legacy topic reader, converter, or
  dual-write path. `{ title, section, body, links }` records are rejected.
- The core round-trip test
  `language_grammar_records_round_trip_and_rebuild_from_checkpoint`
  persists system, agreement, and custom-rule values and rebuilds them
  after deleting `.daena/`.

The existing `paradigms` collection is a morphology rule engine: its slots and
operations generate lexeme forms. It is not a suitable storage model for a
descriptive personal-pronoun or argument-indexing matrix. Grammar matrices
must use the grammar record model below. They may optionally reference a
morphology paradigm when the user deliberately connects descriptive grammar
to form generation.

The architecture already provides the required persistence boundary. Grammar
data remains Language-owned module records attached to a language entity,
validated by the manifest schema, written through revision-aware record APIs,
stored authoritatively in SQLite, and exported through normal deterministic
project checkpoints. No new Tauri command, direct filesystem access, storage
silo, or private database is needed.

---

# 22. Hard-Cut Record Contract

Keep the collection ID `grammar`, but replace its schema and every old caller
in one change. Every new record has a required `schemaVersion: 1` and a
`recordKind` discriminator.

## 22.1 Common value objects

Define and bound these shared objects in TypeScript and in the manifest JSON
Schema:

```text
GrammarStatus
  "unconfigured" | "configured" | "not-used"

GrammarExample
  id
  text
  translation?
  gloss?
  notes?

GrammarLink
  id
  kind: "lexeme" | "lexeme-example" | "sample" | "paradigm"
  targetId
  secondaryId?
  label?

ParadigmAxis
  id
  label
  values: [{ id, label, description? }]

ParadigmCell
  id
  coordinates: { [axisId]: valueId }
  state: "form" | "same-as" | "zero" | "not-applicable"
  form?
  alternateForms?
  sameAsCellId?
  notes?
  exampleId?
```

Apply explicit limits consistent with the rest of the module: bounded strings,
notes, examples, links, axes, axis values, cells, categories, and custom
options. Normalizers must trim text, normalize line endings, discard malformed
references, reject unknown discriminators, and never silently convert an
unknown fixed system into **Other Rules**.

## 22.2 Fixed system records

A fixed system record has:

```text
recordKind: "system"
schemaVersion: 1
systemId: stable catalog ID
status: GrammarStatus
config: system-specific discriminated object
notes: string
examples: GrammarExample[]
links: GrammarLink[]
```

There may be at most one record for each `(language entity, systemId)`.
Absence and an explicit `unconfigured` record both render as **Not
configured**; save an explicit record only when preserving a draft is useful.
The UI must prevent duplicate creation. If malformed external data contains
duplicates, loading must choose no winner silently: show a diagnostic and
disable edits for that system until the conflict is resolved.

Use stable dotted IDs, independent of display labels:

- `syntax.basic-word-order`
- `syntax.adjective-position`
- `syntax.adpositions`
- `syntax.possessive-position`
- `syntax.relative-clause-position`
- `nouns.number`
- `nouns.case`
- `nouns.classes`
- `nouns.definiteness`
- `nouns.possession`
- `pronouns.personal`
- `pronouns.demonstratives`
- `verbs.marking-strategy`
- `verbs.tense`
- `verbs.aspect`
- `verbs.mood`
- `verbs.argument-indexing`
- `verbs.negative-forms`
- `modifiers.adjective-behavior`
- `modifiers.comparative`
- `modifiers.superlative`
- `clauses.yes-no-questions`
- `clauses.content-questions`
- `clauses.imperatives`
- `clauses.negation`
- `clauses.relative-clauses`

Add later systems to the catalog without changing existing IDs.

## 22.3 Dynamic agreement records

Agreement is not one fixed form. Each controller-to-target relationship is a
separate record:

```text
recordKind: "agreement"
schemaVersion: 1
title
controller: { kind, customLabel? }
target: { kind, customLabel? }
features: [{ sourceSystemId?, categoryId?, label }]
behavior: "full" | "partial" | "conditional"
defaultForm?
conditions?
exceptions?
notes
examples
links
```

Feature references point to canonical category IDs from systems such as
`nouns.number` or `nouns.classes`; display labels are resolved at render time.
A custom feature stores its own label. Deleting or renaming a referenced
category must produce a visible broken-reference diagnostic, not erase the
agreement feature.

To persist the decision that the entire Agreement section is not used, allow:

```text
recordKind: "section-state"
schemaVersion: 1
sectionId: "agreement"
status: "not-used"
note?
```

Creating an agreement record clears that marker after confirmation.

## 22.4 Other Rules records

The only free-form record is:

```text
recordKind: "custom-rule"
schemaVersion: 1
title
tags: string[]
body
examples
links
```

Do not route unknown `systemId` values or invalid system configs into this
shape. Other Rules is an explicit user choice, not a compatibility fallback.

## 22.5 System-specific config contracts

Implement each config as a discriminated TypeScript type and matching JSON
Schema branch. The first-scope fields are:

- Basic word order: order, strength, influences, change notes.
- Adjective, possessive, and relative-clause position: primary position,
  optional alternate positions, conditions.
- Adpositions: strategy, distribution notes.
- Number: ordered categories with stable IDs, meanings, marking strategies,
  markers, and positions.
- Case: ordered case inventory with stable IDs, abbreviations, functions, and
  marking strategies.
- Gender/noun classes: system kind and ordered classes with stable IDs,
  membership notes, and exceptions.
- Definiteness: strategies and article forms with position; no agreement copy.
- Possession: strategies and optional alienability distinction.
- Personal pronouns: selected axes and cells using the common paradigm objects.
- Demonstratives: distance axis plus optional selected axes and cells.
- Verb marking strategy: one or more strategies and notes.
- Tense, aspect, and mood: ordered categories with stable IDs, meanings,
  markers/constructions, and interaction notes.
- Argument indexing: indexed participants, representation kind, axes/cells or
  a flexible table, plus an optional agreement-record reference.
- Negative verb forms: strategies, forms, and tense/mood conditions.
- Adjective behavior: behavior kinds and optional Agreement references.
- Comparative and superlative: strategies, markers/constructions, and
  irregular examples.
- Yes/no questions: strategies, particle/form, and placement.
- Content questions: question-word behavior and optional lexeme links.
- Imperatives: strategies and enabled number, polarity, and politeness
  distinctions.
- Clause negation: primary strategies, particle placement, negative-question
  behavior, negative-imperative behavior, and optional negative-verb-form
  reference.
- Relative clauses: relativization strategies, head behavior, resumptives or
  gaps, and a read-only reference to Syntax placement.

`not-used` records must keep `config` empty and may keep only the common note.
A `configured` record must pass that system's minimum meaningful-data check.
The Save action must explain missing requirements and focus the first invalid
control.

---

# 23. Catalog, Derivation, and Cross-System Rules

Create one static catalog as the source of UI identity and educational copy.
Each descriptor contains:

- stable system and section IDs;
- display label and short hint;
- search aliases;
- scope tier (`initial` or `later`);
- editor kind;
- allowed dependencies and references;
- empty-state action labels; and
- a pure summary function identifier.

Do not persist labels, section names, hints, option descriptions, or generated
summaries. Persist stable IDs and user-authored values only. This keeps copy
changes from becoming data migrations.

Generated section and Grammar-home summaries must be pure functions of the
catalog and normalized records. They must not be separately editable or
stored. If referenced data is absent or invalid, return a bounded diagnostic
summary rather than inventing a value.

The canonical ownership rules are:

- Syntax owns constituent placement.
- Noun, pronoun, verb, and modifier systems own their categories and forms.
- Agreement owns controller-to-target relationships.
- Clause Types owns clause constructions, including primary clause negation.
- Verbs owns only negative verb morphology and special negative forms.
- Other Rules owns only explicitly created unsupported features.

Cross-section controls store references by stable record, system, category, or
cell ID. They never copy the referenced object. Editors may offer a shortcut
to create a missing dependency, but the dependency is saved through its own
normal record workflow.

Grammar search is catalog-aware. Search the catalog labels, aliases, hints,
configured summaries, and custom-rule titles/tags in memory so unconfigured
systems are discoverable. Do not depend only on record search, because absent
systems have no persisted record.

---

# 24. Agent Implementation Plan

Agents must implement this as sequential, testable vertical slices. Do not
parallelize work that edits `packages/modules/language/src/index.ts`,
`packages/modules/language/src/grammar.ts`, or the Language manifest. Preserve
unrelated worktree changes, especially changes outside the Language module.

## Phase 0 — Contract hard cut

1. Replace the legacy types and helpers in
   `packages/modules/language/src/grammar.ts` with the discriminated records,
   bounded normalizers, catalog IDs, dependency checks, and summary helpers
   defined above.
2. Replace the `grammar` collection schema in
   `packages/modules/language/manifest.json`. Use `oneOf` branches keyed by
   `recordKind`, and further discriminate fixed-system configs by `systemId`.
   Set `additionalProperties: false` at every authored object level.
3. Keep `language-v1` as the only packaged namespace migration. Do not add a
   `language-v2` migration: there is no old grammar-data compatibility
   contract.
4. Delete legacy-only Markdown topic rendering and section fallback behavior.
   Keep reusable safe text/link helpers only if the new editors use them.
5. Rewrite `scripts/language-grammar.test.mjs` around the new records. Add
   fixtures for every discriminator, every fixed config type, bounds,
   malformed values, duplicate detection, dependency diagnostics, summaries,
   and schema/normalizer agreement.
6. Rewrite the core grammar round-trip fixture to use new values and verify
   checkpoint flush, removal of `.daena/`, and reconstruction from portable
   files. Include agreement and custom-rule records, not only fixed systems.

Exit criteria:

- no source or test constructs `{ title, section, body, links }` as a grammar
  topic;
- old records fail the new manifest schema instead of being normalized;
- all fixed system IDs are unique and have a catalog entry, config validator,
  summary function, and search aliases; and
- generated manifest fixtures are updated and pass both contract validators.

## Phase 1 — Grammar state and shell

1. Move Grammar-specific state and DOM construction out of the monolithic
   `index.ts` into a `src/grammar/` feature directory. Keep `index.ts` as the
   module lifecycle and pane orchestrator.
2. Add a repository adapter that loads all grammar records for the selected
   language with pagination, normalizes them once, indexes them by kind and
   system ID, and preserves each module-record revision.
3. All saves, deletes, and section-state changes must use fresh request IDs and
   observed revisions. On stale-revision errors, keep the draft, reload the
   record, and show an explicit conflict action; do not silently overwrite.
4. Build the Grammar home, eight section cards, generated at-a-glance summary,
   configured/not-used counts, section empty states, catalog-aware search, and
   section navigation.
5. Add a shared editor shell for status controls, hints, Learn more content,
   notes, structured examples, links, validation, Save, Cancel, and Delete.
   The shell supplies behavior, not a universal field layout.
6. Preserve unsaved drafts when opening Learn more or dependency references.
   Prompt before navigation, status reset, destructive category removal, or
   closing an editor with changes.

Exit criteria:

- an empty language shows all catalog systems as **Not configured** without
  creating records;
- **Not used** survives reload and appears distinctly from an absent system;
- section and home summaries change immediately after a successful save;
- search returns configured and unconfigured systems; and
- stale revisions, duplicate fixed records, and broken references are visible
  and non-destructive.

## Phase 2 — Foundational specialized editors

Implement and verify one editor family at a time in this order:

1. choice editors: basic word order, adjective position, adpositions,
   possessive position, and relative-clause position;
2. inventory editors: number, case, gender/noun classes, tense, aspect, and
   mood;
3. strategy editors: definiteness, possession, verb marking, negative verb
   forms, adjective behavior, comparative, and superlative;
4. clause editors: yes/no questions, content questions, imperatives, clause
   negation, and relative clauses; and
5. paradigm editors: personal pronouns, demonstratives, and argument indexing.

For every editor:

- implement its minimum configured-data rule;
- cover all stated choices plus Custom;
- support reorderable user-defined categories where order is meaningful;
- assign stable IDs once and retain them through label edits and reordering;
- protect referenced categories/cells from silent deletion;
- include short help, option explanations, and optional Learn more copy;
- render structured examples without requiring translation or gloss; and
- add normalization, summary, and interaction tests before starting the next
  family.

The paradigm grid must derive its Cartesian cells from selected axes while
preserving existing cells by coordinate. Before removing an axis or value,
show how many populated cells and external references will be affected.
Require confirmation, then remove invalid cells and surface any remaining
broken external references. Do not generate morphology or lexemes implicitly.

Exit criteria:

- every system in Recommended First Product Scope has a dedicated editor;
- no initial-scope system falls back to a title/description form;
- every editor round-trips through module records and checkpoint rebuild; and
- dependent displays update from references without cloned data.

## Phase 3 — Agreement and Other Rules

1. Implement the controller → target agreement builder with defined categories
   offered before custom features.
2. Support full, partial, and conditional behavior, defaults, exceptions,
   notes, examples, and multiple independent agreement records.
3. Implement the Agreement section-level **Not used** marker and its
   confirmation behavior when creating the first relationship.
4. Implement Other Rules as the sole generic structured-rule editor, including
   optional tags, notes, examples, and links.
5. Add reference diagnostics for renamed/deleted categories and direct
   navigation to the owning system.

Exit criteria:

- agreement features retain stable references through label edits;
- deleting a referenced category never silently rewrites agreement;
- multiple agreement systems display independent summaries; and
- generic rules can be created only from Other Rules.

## Phase 4 — Starter, accessibility, and completeness

1. Add the optional Grammar Starter flow using the existing system editors in
   sequence. It must not have a second save path or separate draft model.
2. Add keyboard navigation and accessible names for section cards, status
   controls, dynamic inventory rows, paradigm headers/cells, help disclosures,
   diagnostics, and destructive confirmations.
3. Ensure focus moves to the editor heading on open, the first invalid field on
   validation failure, and the originating system card after save/cancel.
4. Verify narrow layouts and large paradigms. Large grids may scroll, but row
   and column context must remain understandable.
5. Add later-scope catalog entries only when their editor provides a better
   workflow than Other Rules. Do not expose inert checklist items.

Exit criteria:

- the Starter can be dismissed, resumed through normal editors, and completed
  without creating empty records;
- all workflows are operable without a pointer;
- complexity remains hidden until explicitly enabled; and
- no user-authored grammar value exists only in transient UI state after Save.

---

# 25. Verification Checklist

Each agent completes focused checks for its phase, then the final agent runs
the full relevant set through `rtk`:

```text
rtk node --experimental-strip-types scripts/language-grammar.test.mjs
rtk npm run check:manifest-fixtures
rtk npm run check
rtk npm run build
rtk cargo test --manifest-path src-tauri/Cargo.toml --locked --offline language_grammar
```

Add or rename the focused Rust test filter if the final test names differ; do
not skip the checkpoint reconstruction assertion.

Rendered verification must exercise the actual Language module in the desktop
host, not browser automation against the Tauri app. At minimum verify:

1. a new language's empty Grammar home;
2. configure, edit, mark not used, reset, and delete flows;
3. one inventory editor, one paradigm editor, clause negation, Agreement, and
   Other Rules;
4. category rename/removal with a dependent agreement reference;
5. stale-revision and duplicate-record diagnostics;
6. search for an unconfigured alias such as `ergative`;
7. Grammar Starter dismissal and completion;
8. keyboard-only operation and visible focus; and
9. close/reopen plus clean checkpoint rebuild after deleting `.daena/`.

The hard cut is complete only when legacy topic code and fixtures are gone,
the manifest accepts exactly the new contract, the rendered module uses
specialized editors for the initial scope, and clean portable data rebuilds to
the same grammar records and generated summaries.
