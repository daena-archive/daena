# Lore Profiles

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

Initial presets:

1. **D&D**
2. **Fantasy**
3. **Sci-Fi**
4. **Custom**

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

Presets are predefined Profile templates.

A preset defines:

- Components
- Categories
- Default values
- Scales
- Skills
- Proficiencies
- Derived formulas
- Optional allocation rules
- Recommended entity types
- Display organization

Presets are **templates, not restrictions**.

After creating a Profile from a preset, the user can:

- Add components
- Remove components
- Rename components
- Change scales
- Change values
- Add formulas
- Add custom categories

Preset updates must never overwrite user customizations.

---

# 9. D&D Preset

The D&D preset should provide a recognizable tabletop-RPG character sheet without making the entire Profile system dependent on D&D rules.

## 9.1 Attributes

### Core Ability Scores

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

## 9.2 Skills

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

## 9.3 Proficiencies

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

## 9.4 Character Properties

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

## 9.5 Combat Values

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

---

# 10. Fantasy Preset

The Fantasy preset should be system-neutral and suitable for original fantasy worlds.

## 10.1 Attributes

Default attributes:

| Attribute    | Purpose                                       |
| ------------ | --------------------------------------------- |
| Strength     | Physical power                                |
| Agility      | Speed, coordination, reflexes                 |
| Endurance    | Physical resilience                           |
| Intelligence | Reasoning and knowledge                       |
| Willpower    | Mental resilience                             |
| Perception   | Awareness and senses                          |
| Presence     | Social force and personality                  |
| Magic        | Capacity to interact with supernatural forces |

Default scale:

```text
0–20
```

## 10.2 Skills

### Physical

- Athletics
- Acrobatics
- Endurance
- Riding
- Swimming
- Climbing

### Combat

- Swordsmanship
- Archery
- Polearms
- Unarmed Combat
- Shield Fighting
- Tactics

### Knowledge

- History
- Politics
- Geography
- Theology
- Naturalism
- Arcana

### Social

- Diplomacy
- Deception
- Intimidation
- Persuasion
- Leadership
- Etiquette

### Practical

- Smithing
- Crafting
- Cooking
- Hunting
- Herbalism
- Medicine
- Survival
- Navigation

### Magic

- Spellcraft
- Ritualism
- Enchantment
- Alchemy
- Divination
- Summoning

Users may remove or rename any of these.

## 10.3 Proficiencies

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

## 10.4 Traits

Suggested examples:

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

Traits are examples, not required fields.

## 10.5 Resources

Optional:

- Wealth
- Reputation
- Influence
- Mana
- Renown
- Political Power

---

# 11. Sci-Fi Preset

The Sci-Fi preset should support characters, crews, factions, ships, colonies, corporations, and other science-fiction entities.

## 11.1 Attributes

Default attributes:

| Attribute          | Purpose                               |
| ------------------ | ------------------------------------- |
| Strength           | Physical force                        |
| Agility            | Coordination and reaction             |
| Endurance          | Physical resilience                   |
| Intelligence       | Reasoning and analysis                |
| Awareness          | Perception and sensory capability     |
| Willpower          | Mental resilience                     |
| Presence           | Social influence                      |
| Technical Aptitude | Ability to work with advanced systems |

Default scale:

```text
0–20
```

## 11.2 Skills

### Combat

- Firearms
- Melee Combat
- Tactical Combat
- Marksmanship
- Heavy Weapons
- Defense
- Zero-G Combat

### Technical

- Engineering
- Electronics
- Robotics
- Cybernetics
- Programming
- Systems Maintenance
- Fabrication

### Science

- Physics
- Biology
- Chemistry
- Astronomy
- Xenobiology
- Medicine

### Spaceflight

- Piloting
- Navigation
- Astrogation
- Flight Operations
- Ship Handling

### Social

- Diplomacy
- Negotiation
- Leadership
- Intimidation
- Espionage
- Command

### Survival

- Survival
- Scavenging
- Exploration
- Tracking
- Field Medicine

## 11.3 Proficiencies

Examples:

- Weapon Systems
- Vehicle Types
- Starship Classes
- Operating Systems
- Programming Languages
- Cybernetics
- Industrial Equipment
- Alien Technologies

## 11.4 Traits

Examples:

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

## 11.5 Resources

Examples:

- Credits
- Reputation
- Influence
- Energy
- Ammunition
- Fuel
- Data
- Population
- Fleet Strength

---

# 12. Custom Preset

The Custom preset starts with an empty Profile.

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

---

# 13. Profiles for Non-Person Entities

Profiles must not contain implicit assumptions that an entity represents a person.

Examples:

### Faction

```text
Military Power
Political Influence
Economic Power
Intelligence
Stability

Resources
  Treasury
  Population
  Territory

Traits
  Expansionist
  Isolationist
  Religious
```

### Kingdom

```text
Attributes
  Stability
  Technology
  Infrastructure
  Military Power
  Administrative Capacity

Resources
  Population
  Treasury
  Food
  Territory
```

### Ship

```text
Attributes
  Speed
  Maneuverability
  Hull Strength
  Sensor Capability
  Firepower

Resources
  Fuel
  Ammunition
  Crew

Traits
  Damaged
  Veteran Crew
  Experimental
```

### Artifact

```text
Attributes
  Power
  Durability
  Resonance

Skills
  Binding
  Manipulation

Traits
  Cursed
  Sentient
  Ancient

Resources
  Charges
```

The same underlying Profile system should support all of these.

---

# 14. Entity Templates

Profiles should optionally be saved as **Entity Profile Templates**.

A template defines which components should be created for a particular kind of entity.

Examples:

```text
Person → Fantasy Character
Faction → Fantasy Faction
Ship → Sci-Fi Ship
Kingdom → Fantasy Kingdom
Artifact → Magical Artifact
```

Templates are separate from the global presets.

A preset provides the building blocks; an entity template selects and arranges them.

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

A user creates a Profile using **Fantasy**.

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

# 27. Initial Scope

The initial implementation should include:

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

- D&D
- Fantasy
- Sci-Fi
- Custom

### Timeline

- Historical Profile changes
- Event-linked modifications
- Historical Profile inspection
- Change history

### Entity Support

- All Lore entity types
- No person-specific assumptions

---

# 28. Future Scope

Potential future extensions:

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
- Profile charts and analytics
- Cross-entity statistics
- Historical statistical analysis

These should not be required for the initial implementation.

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
