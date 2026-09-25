# Lore Profiles

This document is the Lore Profile product spec. The current product is section
27. Section 28 is planned scope; the delivery plan is
[`plans/profile-presets.md`](../plans/profile-presets.md). Architectural
constraints are [ADR 0006](../adr/0006-lore-profiles.md).

## 1. Overview

**Profiles** are an optional structured capability of the **Lore** module that allows any Lore entity to have measurable attributes, skills, traits, proficiencies, resources, and other characteristics.

Profiles are designed for RPG-like character sheets, but are **not tied to characters or RPG rules**. Any entity can have a Profile:

- Person
- Creature
- Faction
- Organization
- Kingdom
- Culture
- Settlement
- Artifact
- Building
- Ship
- Weapon
- Deity
- Concept
- Custom entity types

Profiles use a generic schema and can be configured through presets or completely customized by the user.

Bundled presets are data, grouped by genre (D&D, Fantasy, Sci-Fi) or left
ungenred (Custom, Culture, Concept). A preset names the qualified entity types
it is for; Custom names none. See sections 8–12. What ships today is section 27.

Profiles integrate directly with **Timeline**, allowing values to change throughout history and allowing users to inspect an entity's Profile at a specific point in time.

---

# 2. Goals

Profiles should:

- Provide an RPG-style structured representation of Lore entities.
- Allow users to allocate points to predefined or custom characteristics.
- Support both numeric and qualitative progression.
- Work with every entity type rather than only People.
- Provide useful presets without locking users into a particular ruleset.
- Allow presets to be modified after creation.
- Integrate with Timeline to represent historical development.
- Preserve historical values rather than overwriting previous states.
- Support derived values calculated from other Profile values.
- Allow users to define their own categories, scales, formulas, and progression rules.
- Remain useful for worldbuilding even when no RPG mechanics are being used.

Profiles should **not** attempt to become a full tabletop RPG engine.

---

# 3. Core Model

A Profile consists of reusable **Profile Components**.

```text
Profile
├── Attributes
├── Skills
├── Proficiencies
├── Traits
├── Resources
├── Derived Values
├── Conditions
├── Tags
└── Metadata
```

Not every Profile needs every component.

A Profile may contain any combination of these.

## 3.1 Attributes

Attributes represent broad capabilities or characteristics.

Examples:

- Strength
- Intelligence
- Agility
- Technology
- Resolve
- Authority
- Military Power
- Wealth

Attributes normally use numeric values but may optionally use another defined scale.

Example:

```text
Strength: 14
Intelligence: 12
Resolve: 8
```

Attributes can have:

- Name
- Description
- Value
- Minimum
- Maximum
- Starting value
- Allocation cost
- Category
- Display order
- Icon
- Tags
- Optional formula
- Optional Timeline history

---

## 3.2 Skills

Skills represent learned, specialized, or practiced capabilities.

Examples:

- Swordsmanship
- Diplomacy
- Medicine
- Sailing
- Engineering
- Investigation
- Stealth
- Arcane Theory

Skills are independent Profile components and may optionally reference an Attribute.

Example:

```text
Strength
  Swordsmanship
  Athletics

Intelligence
  Medicine
  Arcana
  Investigation
```

The relationship is informational by default. It does not automatically impose a game mechanic.

A preset may define formulas using the relationship.

---

## 3.3 Proficiencies

Proficiencies indicate competence with a specific field, object, technology, weapon, language, profession, or discipline.

Examples:

```text
Longswords
Heavy Armor
Elvish
Plasma Weapons
Ancient Runes
Navigation
```

Proficiency may be represented as:

- Boolean
- Rank
- Numeric value
- Custom scale

Example:

```text
Swordsmithing: Expert
Elvish: Fluent
Plasma Rifles: 4/5
```

Proficiencies should remain separate from Skills because a character may be skilled in a broad discipline while having proficiency with a particular tool or domain.

---

## 3.4 Traits

Traits are qualitative characteristics rather than conventional numerical statistics.

Examples:

- Brave
- Ambitious
- Ruthless
- Diplomatic
- Immortal
- Telepathic
- Wealthy
- Cursed
- Mechanically Augmented

Traits may contain:

- Name
- Description
- Optional severity/rank
- Optional tags
- Start date
- End date
- Timeline history

Traits do not require a numerical value.

---

## 3.5 Resources

Resources represent quantities or pools that an entity possesses.

Examples:

- Gold
- Wealth
- Reputation
- Influence
- Mana
- Energy
- Ammunition
- Population
- Military Strength
- Political Capital

Resources may have:

- Current value
- Maximum value
- Unit
- Regeneration/change rules
- Timeline history

Resources are especially useful for non-character entities.

Example:

```text
Kingdom
Population: 2.4M
Treasury: 820,000 gold
Military Strength: 74
Political Influence: 61
```

---

## 3.6 Derived Values

Derived Values are calculated from other Profile values.

Examples:

```text
Strength Modifier = floor((Strength - 10) / 2)

Combat Rating = Strength + Swordsmanship

Political Influence = Reputation + Diplomacy + Wealth

Ship Integrity = Hull Strength × Structural Integrity
```

A Derived Value contains:

- Name
- Formula
- Dependencies
- Unit
- Formatting
- Optional minimum/maximum

Derived Values are read-only unless explicitly overridden by the user.

Changing an input automatically recalculates dependent values.

---

## 3.7 Conditions

Conditions represent temporary or persistent states.

Examples:

- Injured
- Poisoned
- Exhausted
- Blessed
- Cursed
- Captured
- Disgraced
- Missing

Conditions may have:

- Start date
- End date
- Severity
- Description
- Tags
- Timeline origin

Conditions are inherently historical and should integrate closely with Timeline.

---

## 3.8 Tags

Tags provide lightweight categorization without requiring a structured value.

Examples:

```text
Human
Noble
Military
Magic User
Artificial Intelligence
Ancient
```

Tags may be used for filtering, searching, formulas, and preset-specific behavior.

---

# 4. Values and Scales

Profile components should support multiple value types.

## Numeric

```text
Strength: 14
```

Supports:

- Integer
- Decimal
- Minimum
- Maximum
- Points
- Units

## Ranked

```text
Swordsmanship: Expert
```

A ranked field references a predefined ordered scale.

Example:

```text
Untrained
Novice
Apprentice
Adept
Expert
Master
Legendary
```

## Boolean

```text
Immortal: Yes
```

## Enumerated

```text
Faction Alignment: Neutral
```

## Text

Used for custom descriptive properties where measurement is inappropriate.

## Resource

Numeric value with optional maximum and unit.

---

# 5. Point Allocation

Profiles may optionally define a point-allocation system.

A Profile template can specify:

- Available points
- Cost per point
- Starting value
- Minimum
- Maximum
- Point refund rules
- Category-specific costs
- Prerequisites

Example:

```text
Fantasy Profile

Starting Points: 40

Strength        5
Agility         7
Intelligence    8
Resolve         6
```

The user can allocate remaining points through the Profile editor.

Point allocation is **optional**.

A Profile may instead be freely edited or populated exclusively through Timeline events.

---

# 6. Progression

Progression describes how Profile values change over time.

Progression can occur through:

1. Direct editing
2. Point allocation
3. Timeline events
4. Manual adjustments attached to Timeline
5. Derived calculations
6. Custom progression rules

Example:

```text
Swordsmanship

Age 12    Novice
Age 16    Apprentice
Age 22    Adept
Age 31    Expert
Age 48    Master
```

The Profile should preserve the historical changes.

The current value is simply the latest applicable state.

---

# 7. Timeline Integration

Timeline is the authoritative mechanism for **historical Profile state**.

Profile changes may be attached to Timeline events.

Example:

### Event

**Battle of Red Vale — 247 AE**

Effects:

```text
Swordsmanship +2
Leadership +1
Reputation +3
Condition: Injured
```

Another event:

**Three-year apprenticeship — 239–242 AE**

Effects:

```text
Alchemy +3
Herbalism +2
Arcane Theory +1
```

The Profile should not store only the latest value. It should retain the changes necessary to reconstruct historical state.

---

## 7.1 Historical Profile View

Users can select a date on the Timeline and view:

> **Profile — Year 247**

The Profile displays values as they existed at that point.

Moving the selected date changes the displayed values.

Example:

```text
Year 240
Swordsmanship: 3

Year 250
Swordsmanship: 6

Year 270
Swordsmanship: 8
```

This should work for:

- Attributes
- Skills
- Proficiencies
- Resources
- Traits
- Conditions
- Derived values

---

## 7.2 Profile Change History

Each historical change should display:

- Previous value
- New value
- Date
- Event
- Optional reason
- Optional author/editor

Example:

```text
Swordsmanship
3 → 5
Battle of Red Vale
247 AE
```

Users may manually create a change without creating a full Timeline event when appropriate.

However, manually dated changes should still participate in historical reconstruction.

---

# 8. Presets

Presets are Profile templates stored as data.

## 8.1 Preset Architecture

Presets are `module_records` entries in a `profile-preset` collection on
`daena.lore`. Bundled presets and user-created presets use the same storage
and code path. Presets are never hardcoded constants once that collection
ships.

The collection is project-scoped, not entity-owned. Do not create a sentinel,
nil, or hidden entity to own presets. `module_records.owner_entity_id` is a
required foreign key to `entities`, and a checkpoint rejects a record whose
owner entity is missing. A fake owner would also delete every preset when that
entity is deleted. Shipping the collection requires a project owner scope
whose rows have no entity owner, including on checkpoint restore. `profile`
and `profile-change` stay entity-owned.

A preset defines:

- Name
- Description
- Icon
- Entity types (qualified runtime ids, such as `daena.lore:person` or an overlay `daena.lore:…` id)
- Genre (grouping label: D&D, Fantasy, Sci-Fi, or user-defined; omit for ungenred presets)
- Components
- Default values
- Scales
- Derived formulas
- Optional allocation rules

Declared entity types are matched against the entity's qualified type. A
preset with no declared entity types is universal and always suggested.
Custom is the only bundled universal preset. Culture and Concept are ungenred,
but they declare a type and are not universal.

Unless a section states a starting value, numeric attributes start at the
minimum of their scale.

Trait lists in sections 9–12 are examples for the author. They are not preset
components and must not be seeded. Skill lists, proficiency lists, attribute
tables, resource lists, and derived formulas are components unless a section
says otherwise.

All presets remain accessible regardless of entity-type scoping. An author
may apply any preset to any entity. Suggestions are hints, not a claim that
every entity of a declared type is that kind of thing.

## 8.2 Bundled and User Presets

**Bundled presets** ship with Daena and are seeded as `profile-preset`
records on project creation. Bundled presets:

- May be edited (the project's copy diverges from the shipped default).
- May be reset to the shipped default.
- May not be deleted (they may be hidden).

**User presets** are created by the author:

- From scratch (blank).
- By duplicating a bundled or user preset.
- By using "Save as Preset" from an existing entity's Profile.

User presets may be renamed, edited, and deleted.

## 8.3 Copy-on-Create

Presets are **templates, not restrictions**.

Creating a Profile from a preset copies the preset's components into a new
independent Profile instance. The instance retains a `presetOrigin`
reference (the preset record ID) for display purposes only.

After creation, the user can:

- Add components
- Remove components
- Rename components
- Change scales
- Change values
- Add formulas
- Add custom categories

Preset updates never overwrite existing Profile instances.

## 8.4 Preset Picker

When creating a Profile on an entity, the picker shows:

**Suggested** — presets whose declared entity types include the current
entity's qualified type, plus universal presets. Custom is here because it
declares no types. It is not a separate action.

**All Presets** — every other preset, grouped by genre. Presets with no genre
appear together. Do not invent a genre to hold them.

## 8.5 Preset Management

Authors may manage presets through a dedicated surface:

- Browse all presets grouped by genre with entity-type badges.
- Create a new preset or duplicate an existing one.
- Edit name, description, icon, genre, entity types, and components.
- Preview a preset before applying.
- Reset a bundled preset to its shipped default, including making it visible again.
- Hide or unhide a bundled preset. Hidden presets stay in the project and do not appear in the picker.
- Delete user presets (with confirmation). Deletion does not change existing Profile instances.

---

# 9. D&D Presets

The D&D genre provides presets for character, faction, and location entities.
These presets should provide recognizable tabletop-RPG structures without
making the entire Profile system dependent on D&D rules.

## 9.1 D&D Character

Entity types: `daena.lore:person`.

### Attributes — Core Ability Scores

| Attribute    | Default Scale |
| ------------ | ------------- |
| Strength     | 1–30          |
| Dexterity    | 1–30          |
| Constitution | 1–30          |
| Intelligence | 1–30          |
| Wisdom       | 1–30          |
| Charisma     | 1–30          |

Default starting value: 10.

Default derived modifier:

```text
Modifier = floor((Score - 10) / 2)
```

The implementation should allow users to modify the formula.

### Skills

Default skill set:

- Acrobatics
- Animal Handling
- Arcana
- Athletics
- Deception
- History
- Insight
- Intimidation
- Investigation
- Medicine
- Nature
- Perception
- Performance
- Persuasion
- Religion
- Sleight of Hand
- Stealth
- Survival

Each skill may reference its default governing ability.

Example:

```text
Stealth → Dexterity
History → Intelligence
Persuasion → Charisma
```

### Proficiencies

Default categories:

- Armor
- Weapons
- Tools
- Languages
- Saving Throws
- Skills

Ranks:

```text
Untrained
Proficient
Expert
Master
```

The exact game-specific meaning may be customized.

### Character Properties

Optional predefined properties:

- Level
- Experience
- Class
- Subclass
- Species
- Background
- Alignment
- Inspiration

These should be ordinary Profile fields rather than special hard-coded properties.

### Combat Values

Optional Derived Values / Resources:

- Hit Points
- Maximum Hit Points
- Armor Class
- Initiative
- Speed
- Proficiency Bonus
- Passive Perception
- Spellcasting Ability

These should be implemented using the generic value system.

## 9.2 D&D Faction

Entity types: `daena.lore:faction`.

### Attributes

| Attribute         | Scale | Purpose                         |
| ----------------- | ----- | ------------------------------- |
| Influence         | 0–20  | Political and social reach      |
| Military Strength | 0–20  | Combat capability and readiness |
| Wealth            | 0–20  | Economic resources              |
| Secrecy           | 0–20  | Ability to operate covertly     |
| Reach             | 0–20  | Geographic or institutional scope |

### Resources

- Gold
- Members
- Strongholds
- Allies

### Traits

Example traits (not seeded):

- Lawful
- Chaotic
- Good
- Evil
- Secretive
- Militant
- Mercantile
- Religious
- Criminal
- Noble

### Specialization

One optional text field. Example values, not seeded components: Assassination,
Espionage, Trade, War, Diplomacy.

## 9.3 D&D Location

Entity types: `daena.lore:place`.

### Attributes

| Attribute          | Scale | Purpose                     |
| ------------------ | ----- | --------------------------- |
| Danger Level       | 0–20  | Threat to visitors          |
| Accessibility      | 0–20  | Ease of reaching or entering |
| Magical Saturation | 0–20  | Ambient magical energy      |

### Resources

- Treasure (resource with unit: gold)
- Population (numeric, optional for settlements)

### Traits

Example traits (not seeded):

- Dungeon
- Wilderness
- Urban
- Planar
- Cursed
- Sacred
- Fortified
- Ruined
- Hidden

---

# 10. Fantasy Presets

The Fantasy genre provides system-neutral presets suitable for original
fantasy worlds. Presets cover characters, factions, kingdoms, places, and
artifacts.

## 10.1 Fantasy Character

Entity types: `daena.lore:person`.

Point allocation: 40 points. Default starting value: 5.

### Attributes

| Attribute    | Scale | Purpose                                       |
| ------------ | ----- | --------------------------------------------- |
| Strength     | 0–20  | Physical power                                |
| Agility      | 0–20  | Speed, coordination, reflexes                 |
| Endurance    | 0–20  | Physical resilience                           |
| Intelligence | 0–20  | Reasoning and knowledge                       |
| Willpower    | 0–20  | Mental resilience                             |
| Perception   | 0–20  | Awareness and senses                          |
| Presence     | 0–20  | Social force and personality                  |
| Magic        | 0–20  | Capacity to interact with supernatural forces |

### Skills

#### Physical

- Athletics
- Acrobatics
- Endurance
- Riding
- Swimming
- Climbing

#### Combat

- Swordsmanship
- Archery
- Polearms
- Unarmed Combat
- Shield Fighting
- Tactics

#### Knowledge

- History
- Politics
- Geography
- Theology
- Naturalism
- Arcana

#### Social

- Diplomacy
- Deception
- Intimidation
- Persuasion
- Leadership
- Etiquette

#### Practical

- Smithing
- Crafting
- Cooking
- Hunting
- Herbalism
- Medicine
- Survival
- Navigation

#### Magic

- Spellcraft
- Ritualism
- Enchantment
- Alchemy
- Divination
- Summoning

Users may remove or rename any of these.

### Proficiencies

Examples:

- Weapons
- Armor
- Tools
- Languages
- Magical Schools
- Professions
- Scholarly Disciplines

Default proficiency scale:

```text
Untrained
Novice
Competent
Skilled
Expert
Master
Legendary
```

### Traits

Example traits (not seeded):

- Brave
- Cunning
- Ambitious
- Compassionate
- Ruthless
- Honorable
- Curious
- Superstitious
- Immortal
- Blessed
- Cursed

These are examples, not preset components.

### Resources

Optional:

- Wealth
- Reputation
- Influence
- Mana
- Renown
- Political Power

## 10.2 Fantasy Faction

Entity types: `daena.lore:faction`.

### Attributes

| Attribute          | Scale | Purpose                           |
| ------------------ | ----- | --------------------------------- |
| Military Power     | 0–100 | Armed strength and readiness      |
| Political Influence| 0–100 | Ability to affect governance      |
| Economic Power     | 0–100 | Trade, production, wealth         |
| Intelligence       | 0–100 | Information gathering and secrecy |
| Stability          | 0–100 | Internal cohesion and loyalty     |

### Resources

- Treasury (resource with unit)
- Population (numeric)
- Territory (numeric)
- Armies (numeric)

### Traits

Example traits (not seeded):

- Expansionist
- Isolationist
- Religious
- Militaristic
- Mercantile
- Democratic
- Tyrannical
- Feudal
- Nomadic

### Derived Values

```text
Overall Power = avg(Military Power, Political Influence, Economic Power)
```

The result stays on the 0–100 scale. Do not sum the attributes.

## 10.3 Fantasy Kingdom

Entity types: `daena.lore:faction`, `daena.lore:place`.

Suggested for both types. A tavern or dungeon that is a place will see this
beside Fantasy Place. That is a hint, not a claim that the place is a kingdom.

### Attributes

| Attribute              | Scale | Purpose                          |
| ---------------------- | ----- | -------------------------------- |
| Stability              | 0–100 | Resistance to internal unrest    |
| Technology             | 0–100 | Advancement of tools and methods |
| Infrastructure         | 0–100 | Roads, cities, logistics         |
| Military Power         | 0–100 | Defensive and offensive strength |
| Administrative Capacity| 0–100 | Governance effectiveness         |

### Resources

- Population (numeric)
- Treasury (resource with unit: gold)
- Food (numeric)
- Territory (numeric)

### Traits

Example traits (not seeded):

- Feudal
- Imperial
- Theocratic
- Republic
- Magocratic
- Declining
- Rising
- At War
- Prosperous

### Derived Values

```text
Regional Influence = avg(Military Power, Stability)
Economic Output = avg(Infrastructure, Technology)
```

Both results stay on the 0–100 scale.

## 10.4 Fantasy Place

Entity types: `daena.lore:place`.

### Attributes

| Attribute        | Scale | Purpose                                 |
| ---------------- | ----- | --------------------------------------- |
| Strategic Value  | 0–20  | Military and political importance       |
| Defensibility    | 0–20  | Natural and constructed defenses        |
| Prosperity       | 0–20  | Economic health and trade activity      |
| Magical Resonance| 0–20  | Ambient supernatural energy             |

### Resources

- Population (numeric)
- Trade Volume (resource with unit)
- Garrison (numeric)

### Traits

Example traits (not seeded):

- Fortified
- Sacred
- Cursed
- Contested
- Ruined
- Hidden
- Port
- Capital
- Frontier
- Ancient

## 10.5 Fantasy Artifact

Entity types: `daena.lore:artifact`.

### Attributes

| Attribute  | Scale | Purpose                                |
| ---------- | ----- | -------------------------------------- |
| Power      | 0–20  | Raw magical or physical potency        |
| Durability | 0–20  | Resistance to damage and degradation   |
| Resonance  | 0–20  | Sensitivity to magical interaction     |

### Skills

Artifact capabilities:

- Binding
- Manipulation
- Protection
- Destruction
- Divination
- Communication

### Resources

- Charges (resource with max)

### Traits

Example traits (not seeded):

- Cursed
- Sentient
- Ancient
- Blessed
- Legendary
- Dormant
- Corrupting
- Bonded

### Derived Values

```text
Magical Potency = avg(Power, Resonance)
```

The result stays on the 0–20 scale.

---

# 11. Sci-Fi Presets

The Sci-Fi genre provides presets for characters, factions, starships,
and colonies or stations.

## 11.1 Sci-Fi Character

Entity types: `daena.lore:person`.

Default starting value: 5.

### Attributes

| Attribute          | Scale | Purpose                               |
| ------------------ | ----- | ------------------------------------- |
| Strength           | 0–20  | Physical force                        |
| Agility            | 0–20  | Coordination and reaction             |
| Endurance          | 0–20  | Physical resilience                   |
| Intelligence       | 0–20  | Reasoning and analysis                |
| Awareness          | 0–20  | Perception and sensory capability     |
| Willpower          | 0–20  | Mental resilience                     |
| Presence           | 0–20  | Social influence                      |
| Technical Aptitude | 0–20  | Ability to work with advanced systems |

### Skills

#### Combat

- Firearms
- Melee Combat
- Tactical Combat
- Marksmanship
- Heavy Weapons
- Defense
- Zero-G Combat

#### Technical

- Engineering
- Electronics
- Robotics
- Cybernetics
- Programming
- Systems Maintenance
- Fabrication

#### Science

- Physics
- Biology
- Chemistry
- Astronomy
- Xenobiology
- Medicine

#### Spaceflight

- Piloting
- Navigation
- Astrogation
- Flight Operations
- Ship Handling

#### Social

- Diplomacy
- Negotiation
- Leadership
- Intimidation
- Espionage
- Command

#### Survival

- Survival
- Scavenging
- Exploration
- Tracking
- Field Medicine

### Proficiencies

Examples:

- Weapon Systems
- Vehicle Types
- Starship Classes
- Operating Systems
- Programming Languages
- Cybernetics
- Industrial Equipment
- Alien Technologies

### Traits

Example traits (not seeded):

- Genetically Modified
- Cybernetically Augmented
- Telepathic
- Artificial Intelligence
- Enhanced
- Clone
- Alien
- Synthetic
- Immune
- Psionic

### Resources

- Credits
- Reputation
- Influence
- Energy
- Ammunition
- Fuel
- Data

## 11.2 Sci-Fi Faction

Entity types: `daena.lore:faction`.

### Attributes

| Attribute                | Scale | Purpose                                 |
| ------------------------ | ----- | --------------------------------------- |
| Military Strength        | 0–100 | Fleet and ground force capability       |
| Technological Advancement| 0–100 | Research and engineering sophistication  |
| Economic Power           | 0–100 | Industrial output and trade volume      |
| Intelligence Network     | 0–100 | Espionage and information capability    |
| Diplomatic Influence     | 0–100 | Standing in interstellar relations      |

### Resources

- Credits (resource with unit)
- Fleet Size (numeric)
- Population (numeric)
- Territory (numeric — systems or sectors)
- Research Output (numeric)

### Traits

Example traits (not seeded):

- Expansionist
- Isolationist
- Corporate
- Military Junta
- Democratic
- Theocratic
- Hive Mind
- Federation
- Empire
- Rebel Alliance

### Derived Values

```text
Power Index = avg(Military Strength, Technological Advancement, Economic Power)
```

The result stays on the 0–100 scale. Do not sum the attributes.

## 11.3 Starship

Entity types: `daena.lore:artifact`.

### Attributes

| Attribute          | Scale | Purpose                              |
| ------------------ | ----- | ------------------------------------ |
| Speed              | 0–20  | Sublight and FTL velocity            |
| Maneuverability    | 0–20  | Agility in combat and navigation     |
| Hull Strength      | 0–20  | Structural integrity and armor       |
| Sensor Capability  | 0–20  | Detection and scanning range         |
| Firepower          | 0–20  | Offensive weapon systems             |
| Stealth            | 0–20  | Ability to avoid detection           |

### Resources

- Fuel (resource with max)
- Ammunition (resource with max)
- Crew (resource with max)
- Cargo (resource with max and unit: tons)
- Shield Strength (resource with max)

### Traits

Example traits (not seeded):

- Damaged
- Veteran Crew
- Experimental
- Stealth-Capable
- Decommissioned
- Flagship
- Prototype
- Salvaged
- Alien Design

### Derived Values

```text
Combat Rating = avg(Firepower, Hull Strength, Maneuverability)
```

The result stays on the 0–20 scale. Do not mix Speed with Fuel; those are different quantities.

## 11.4 Colony / Station

Entity types: `daena.lore:place`.

### Attributes

| Attribute         | Scale | Purpose                             |
| ----------------- | ----- | ----------------------------------- |
| Infrastructure    | 0–100 | Built environment and facilities    |
| Life Support      | 0–100 | Environmental sustainability        |
| Defenses          | 0–100 | Military and shield installations   |
| Research Capacity | 0–100 | Scientific and engineering output   |
| Trade Hub         | 0–100 | Commercial activity and connections |

### Resources

- Population (numeric)
- Power Supply (resource with unit: MW)
- Food (resource with max)
- Water (resource with max)
- Atmosphere (resource — percentage)

### Traits

Example traits (not seeded):

- Self-Sustaining
- Under Siege
- Quarantined
- Mining Colony
- Research Station
- Military Outpost
- Trade Hub
- Frontier Settlement
- Orbital
- Planetary

### Derived Values

```text
Readiness = avg(Infrastructure, Life Support, Defenses)
```

The result stays on the 0–100 scale. Do not divide a stock by Population or add that ratio to a 0–100 attribute.

---

# 12. Custom and Ungenred Presets

## 12.1 Custom Preset

The Custom preset starts with an empty Profile. It declares no entity types,
so it is suggested for every entity, including overlay types.

The user defines:

- Component types
- Categories
- Attributes
- Skills
- Proficiencies
- Traits
- Resources
- Derived values
- Scales
- Units
- Formulas
- Allocation rules
- Progression rules

Example custom Profile:

```text
Entity: The Kingdom of Aras

Attributes
  Stability: 72
  Military Power: 81
  Technology: 43
  Infrastructure: 65

Resources
  Treasury: 820,000 gold
  Population: 2,400,000

Traits
  Militaristic
  Expanding
  Religious

Derived Values
  Regional Influence = Military Power + Stability
```

The Custom preset should impose no assumptions about the entity or setting.

## 12.2 Culture

Entity types: `daena.lore:culture`.

No genre. Suggested only for that type.

### Attributes

| Attribute           | Scale | Purpose                            |
| ------------------- | ----- | ---------------------------------- |
| Technological Level | 0–100 | Advancement of tools and knowledge |
| Military Tradition  | 0–100 | Martial capability and doctrine    |
| Artistic Achievement| 0–100 | Creative and cultural output       |
| Religious Devotion  | 0–100 | Spiritual practice and influence   |
| Expansionism        | 0–100 | Tendency to spread and colonize    |

### Resources

- Population (numeric)
- Territory (numeric)
- Sacred Sites (numeric)

### Traits

Example traits (not seeded):

- Nomadic
- Settled
- Seafaring
- Warlike
- Pacifist
- Matriarchal
- Caste-Based
- Oral Tradition
- Written Law
- Ancestor Worship

## 12.3 Concept

Entity types: `daena.lore:concept`.

No genre. Suggested only for that type.

### Attributes

| Attribute | Scale | Purpose                              |
| --------- | ----- | ------------------------------------ |
| Influence | 0–100 | Impact on world events and decisions |
| Spread    | 0–100 | Geographic and cultural reach        |
| Orthodoxy | 0–100 | Rigidity of doctrine or definition   |

### Resources

- Adherents (numeric)
- Institutions (numeric)
- Texts (numeric)

### Traits

Example traits (not seeded):

- Dominant
- Suppressed
- Evolving
- Ancient
- Revolutionary
- Underground
- State-Sponsored
- Heretical
- Universal
- Localized

---

# 13. Profiles for Non-Person Entities

Profiles must not contain implicit assumptions that an entity represents a
person. The genre presets in sections 9–12 demonstrate this principle:

- **D&D** provides Character, Faction, and Location presets.
- **Fantasy** provides Character, Faction, Kingdom, Place, and Artifact presets.
- **Sci-Fi** provides Character, Faction, Starship, and Colony/Station presets.
- Ungenred presets provide Custom, Culture, and Concept. Custom is the only universal preset.

Every preset declares which entity types it is designed for. When an author
creates a Profile on a non-person entity, the preset picker suggests
appropriate presets rather than character-oriented ones.

The underlying Profile system is entity-neutral. Any component type
(attributes, skills, proficiencies, traits, resources, derived values,
conditions, tags) can appear on any entity type. The presets select and
arrange components to be useful for a particular kind of entity.

---

# 14. User-Defined Presets

Authors may save any Profile as a reusable preset through "Save as Preset"
in the Profile editor. User-defined presets serve the same purpose as the
former Entity Profile Templates concept.

"Save as Preset" copies structure: component kinds, names, scales, units,
formulas, allocation rules, and starting defaults. It does not copy the
entity's current progressed values.

A user preset defines:

- Name
- Description
- Entity types (qualified runtime ids; omit for a universal preset)
- Genre (optional grouping label)
- Components, scales, formulas, and allocation rules

Examples:

```text
Person  → My World Character
Faction → Trade Guild
Faction → Noble House
Place   → Frontier Settlement
Artifact → Enchanted Weapon
```

User presets and bundled presets use the same data model and storage.
Authors may duplicate, rename, edit, and delete their own presets.

The distinction between "preset" and "template" is collapsed: a preset
with declared entity types is effectively a template for that entity type.

---

# 15. Derived Values and Formula System

The formula system should initially remain deliberately simple.

Supported operations should include:

- Addition
- Subtraction
- Multiplication
- Division
- Minimum
- Maximum
- Average
- Rounding
- Conditional values

Examples:

```text
Combat Rating =
    Strength +
    Swordsmanship

Political Influence =
    Diplomacy +
    Reputation +
    Wealth

Maximum HP =
    Constitution × 5
```

References should be made to Profile components rather than arbitrary database fields.

Derived values update automatically when dependencies change.

Circular dependencies should be detected and rejected.

---

# 16. Progression Rules

Progression rules are optional.

A component may define:

- Whether it can progress
- Whether it can regress
- Minimum
- Maximum
- Default increment
- Allowed event effects

Example:

```text
Swordsmanship

Starting Value: 0
Maximum: 10
Progression: +1 to +3
```

However, the Profile system should not automatically decide _why_ an entity improves.

The user or Timeline system determines the cause.

---

# 17. Timeline Event Effects

Timeline events may expose a **Profile Changes** section.

Example:

### Event: Siege of Vel Aran

```text
Participant: Aric

Changes
  Swordsmanship       +1
  Tactics             +2
  Reputation          +4
  Endurance           -1

Conditions
  Injured             Start
```

For organizations:

```text
Participant: House Valen

Changes
  Military Power      -8
  Political Influence -3
  Reputation          +2

Resources
  Treasury            -50,000
```

Events may affect multiple entities.

---

# 18. Profile Comparison

Users should be able to compare Profiles.

Example:

```text
           Aric     Darius
Strength     14       11
Agility      12       15
Sword        8         6
Diplomacy    4         9
```

Comparison should respect the active historical date.

Therefore users can compare:

> Aric in 240 AE vs Darius in 250 AE

rather than only comparing current values.

---

# 19. Profile History Visualization

Profiles should provide a history view for individual components.

Example:

```text
Swordsmanship

10 |                         ●
 9 |                    ●────
 8 |               ●────
 7 |          ●────
 6 |
 5 |     ●────
 4 |
 3 | ●───
   +----------------------------
     220 230 240 250 260 270
```

The graph should identify relevant Timeline events.

This is particularly useful for:

- Skill development
- Political power
- Wealth
- Reputation
- Population
- Military strength
- Technology

---

# 20. UI

## Profile Summary

The entity page should expose a **Profile** section when a Profile exists.

Example:

```text
┌───────────────────────────────┐
│ PROFILE                       │
├───────────────────────────────┤
│ Strength          14          │
│ Intelligence      12          │
│ Charisma           9          │
│                               │
│ SKILLS                        │
│ Swordsmanship      Expert     │
│ Diplomacy          Adept      │
│ Riding             Novice     │
│                               │
│ TRAITS                        │
│ Brave                         │
│ Ambitious                     │
└───────────────────────────────┘
```

The UI should support compact cards as well as a detailed editor.

## Editing

Users should be able to:

- Add component
- Remove component
- Edit value
- Allocate points
- View history
- Create a historical change
- Jump to the originating Timeline event
- Configure component
- Add custom component

---

# 21. Profile Configuration

Configuration should be accessible from the entity's Profile editor.

Users should be able to configure:

- Name
- Type
- Description
- Scale
- Minimum
- Maximum
- Units
- Default value
- Attribute relationship
- Formula
- Allocation cost
- Visibility
- Display order
- Icon
- Tags

Configuration should not require editing project files or code.

---

# 22. Search and Filtering

Profile values should eventually become available to Lore search.

Users should be able to search/filter entities by:

```text
Strength > 15

Swordsmanship >= Expert

Military Power > 70

Trait contains "Immortal"

Technology between 50–75
```

Historical filtering should also be possible:

```text
Show factions with Military Power > 80 in 500 AE
```

This functionality may be implemented after the initial Profile release, but the data model should not prevent it.

---

# 23. Preset Customization

Presets should be editable after creation.

Example:

A user creates a Profile using **Fantasy Character**.

They can then:

```text
Remove:
  Swimming

Rename:
  Magic → Arcane Potential

Add:
  Dreamwalking

Add:
  Corruption

Create formula:
  Sanity = Willpower - Corruption
```

The resulting Profile is independent of future preset modifications.

---

# 24. Data Integrity

The Profile system must maintain historical integrity.

Changing the current value must not silently rewrite previous historical states.

Removing a Profile component with historical data should require explicit confirmation.

Deleting a Timeline event that produced Profile changes should correctly recalculate affected historical states.

Derived values must always be recalculated from the applicable historical inputs.

Invalid formulas must not corrupt Profile data.

---

# 25. Import / Export

Profiles should be serializable as ordinary project data.

Import/export should preserve:

- Preset origin
- Customizations
- Values
- Historical changes
- Timeline references
- Formulas
- Scales
- Traits
- Resources

Profiles must remain understandable without the preset package that originally created them.

---

# 26. Design Principles

### Generic first

The Profile system must not contain assumptions such as:

```text
Every entity has Strength.
Every character has HP.
Every entity gains XP.
```

Those are preset-level decisions.

### Presets are starting points

Presets should provide useful defaults, not restrictions.

### Timeline owns historical change

Profiles describe state.

Timeline describes when and why that state changed.

### Numbers are optional

Not everything meaningful should become a number.

Traits, conditions, proficiencies, and descriptive fields are first-class components.

### Derived values are not stored truth

Derived values should be calculated from underlying Profile values whenever possible.

### Any entity can participate

A Profile can belong to any Lore entity.

### Don't become an RPG engine

The system should provide structured worldbuilding data without attempting to implement combat simulation, turn systems, dice engines, encounters, or complete tabletop rules.

---

# 27. Current product

The shipped Profile capability includes:

### Profile

- Create/delete Profile
- Preset selection
- Custom Profile
- Component management
- Numeric values
- Ranked values
- Boolean values
- Traits
- Resources
- Basic derived values
- Point allocation

### Presets

Shipped presets are hardcoded TypeScript constants:

- D&D (person-oriented)
- Fantasy (person-oriented)
- Sci-Fi (person-oriented)
- Custom (empty)

Presets are copy-on-create. Preset updates do not rewrite instances.

Shipped presets are person-centric. Non-person entities must use Custom.

### Timeline

- Historical Profile changes
- Event-linked modifications
- Historical Profile inspection
- Change history

### Entity Support

- All Lore entity types
- No person-specific assumptions in the core model

---

# 28. Planned Scope

The next delivery priority is data-driven and entity-oriented presets.
The delivery plan is [`plans/profile-presets.md`](../plans/profile-presets.md).

### Data-Driven Presets

- Presets stored as project-scoped `profile-preset` records, not entity-owned records and not hardcoded constants.
- `presetOrigin` changes from a hardcoded enum to a string (preset record ID).
- Bundled presets seeded on project creation.
- User-created presets via "Save as Preset" and preset management surface.
- Preset picker reads from records instead of code.

### Entity-Oriented Presets

- D&D: Character, Faction, Location.
- Fantasy: Character, Faction, Kingdom, Place, Artifact.
- Sci-Fi: Character, Faction, Starship, Colony/Station.
- Ungenred: Custom, Culture, Concept. Only Custom is universal.
- Each non-universal preset declares qualified entity type ids.
- Preset picker suggests matching presets. Suggestions are hints.
- Bundled presets may be hidden. They may not be deleted.

### Preset Management

- Browse, create, edit, duplicate, delete, reset presets.
- Entity-type scoping and genre grouping.

### Remaining Future Scope

Potential extensions not part of the current or planned product:

- Advanced formula language
- Conditional modifiers
- Automatic progression systems
- Dependency graphs
- Skill trees
- Perks
- Classes / archetypes
- Equipment modifiers
- Reputation systems
- Faction relationship modifiers
- Encounters and combat
- Dice integration
- AI-assisted Profile generation
- Profile comparison
- Profile charts and analytics
- Cross-entity statistics
- Historical statistical analysis

---

# 29. Desired End State

A Profile should feel like a **living statistical representation of an entity**.

For a person:

> "Who are they, what can they do, and how have they developed?"

For a faction:

> "How powerful are they, what resources do they possess, and how have they changed?"

For an artifact:

> "What properties does it possess and how have those properties evolved?"

Timeline provides the answer to:

> "When did this change happen, and why?"

The combination should let Daena represent not only what exists in a world, but **how its capabilities, characteristics, and power evolve throughout that world's history**.
