import {
  PROFILE_SCHEMA_VERSION,
  type ProfileAllocation,
  type ProfileComponent,
  type ProfileComponentKind,
  type ProfileDocument,
  type ProfilePresetOrigin,
} from "./profile.ts";

export type ProfilePreset = {
  id: ProfilePresetOrigin;
  name: string;
  allocation?: ProfileAllocation;
  components: ProfileComponent[];
};

function componentId(kind: ProfileComponentKind, name: string): string {
  return `${kind}-${name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")}`;
}

function attribute(name: string, min: number, max: number, value: number): ProfileComponent {
  return {
    id: componentId("attribute", name),
    kind: "attribute",
    name,
    min,
    max,
    value: { type: "number", value },
  };
}

function abilityModifier(name: string): ProfileComponent {
  const source = componentId("attribute", name);
  return {
    id: componentId("derived", `${name} Modifier`),
    kind: "derived",
    name: `${name} Modifier`,
    formula: `floor(({${source}} - 10) / 2)`,
    dependencies: [source],
    value: { type: "number", value: null },
  };
}

function skill(name: string): ProfileComponent {
  return {
    id: componentId("skill", name),
    kind: "skill",
    name,
    value: { type: "number", value: null },
  };
}

function textField(kind: ProfileComponentKind, name: string): ProfileComponent {
  return {
    id: componentId(kind, name),
    kind,
    name,
    value: { type: "text", value: null },
  };
}

function numberField(kind: ProfileComponentKind, name: string): ProfileComponent {
  return {
    id: componentId(kind, name),
    kind,
    name,
    value: { type: "number", value: null },
  };
}

function booleanField(kind: ProfileComponentKind, name: string): ProfileComponent {
  return {
    id: componentId(kind, name),
    kind,
    name,
    value: { type: "boolean", value: null },
  };
}

function resource(name: string): ProfileComponent {
  return {
    id: componentId("resource", name),
    kind: "resource",
    name,
    value: { type: "resource", current: null, max: null, unit: null },
  };
}

function proficiency(name: string, scale: string[]): ProfileComponent {
  return {
    id: componentId("proficiency", name),
    kind: "proficiency",
    name,
    scale: [...scale],
    value: { type: "rank", value: null },
  };
}

const DND_PROFICIENCY_SCALE = ["Untrained", "Proficient", "Expert", "Master"];
const FANTASY_PROFICIENCY_SCALE = ["Untrained", "Novice", "Competent", "Skilled", "Expert", "Master", "Legendary"];

const DND_SKILLS = [
  "Acrobatics",
  "Animal Handling",
  "Arcana",
  "Athletics",
  "Deception",
  "History",
  "Insight",
  "Intimidation",
  "Investigation",
  "Medicine",
  "Nature",
  "Perception",
  "Performance",
  "Persuasion",
  "Religion",
  "Sleight of Hand",
  "Stealth",
  "Survival",
];

const FANTASY_SKILLS = [
  "Athletics",
  "Acrobatics",
  "Endurance",
  "Riding",
  "Swimming",
  "Climbing",
  "Swordsmanship",
  "Archery",
  "Polearms",
  "Unarmed Combat",
  "Shield Fighting",
  "Tactics",
  "History",
  "Politics",
  "Geography",
  "Theology",
  "Naturalism",
  "Arcana",
  "Diplomacy",
  "Deception",
  "Intimidation",
  "Persuasion",
  "Leadership",
  "Etiquette",
  "Smithing",
  "Crafting",
  "Cooking",
  "Hunting",
  "Herbalism",
  "Medicine",
  "Survival",
  "Navigation",
  "Spellcraft",
  "Ritualism",
  "Enchantment",
  "Alchemy",
  "Divination",
  "Summoning",
];

const SCIFI_SKILLS = [
  "Firearms",
  "Melee Combat",
  "Tactical Combat",
  "Marksmanship",
  "Heavy Weapons",
  "Defense",
  "Zero-G Combat",
  "Engineering",
  "Electronics",
  "Robotics",
  "Cybernetics",
  "Programming",
  "Systems Maintenance",
  "Fabrication",
  "Physics",
  "Biology",
  "Chemistry",
  "Astronomy",
  "Xenobiology",
  "Medicine",
  "Piloting",
  "Navigation",
  "Astrogation",
  "Flight Operations",
  "Ship Handling",
  "Diplomacy",
  "Negotiation",
  "Leadership",
  "Intimidation",
  "Espionage",
  "Command",
  "Survival",
  "Scavenging",
  "Exploration",
  "Tracking",
  "Field Medicine",
];

export const PROFILE_PRESETS: ProfilePreset[] = [
  { id: "custom", name: "Custom", components: [] },
  {
    id: "dnd",
    name: "D&D",
    components: [
      attribute("Strength", 1, 30, 10),
      attribute("Dexterity", 1, 30, 10),
      attribute("Constitution", 1, 30, 10),
      attribute("Intelligence", 1, 30, 10),
      attribute("Wisdom", 1, 30, 10),
      attribute("Charisma", 1, 30, 10),
      ...["Strength", "Dexterity", "Constitution", "Intelligence", "Wisdom", "Charisma"].map(abilityModifier),
      ...DND_SKILLS.map(skill),
      ...["Armor", "Weapons", "Tools", "Languages", "Saving Throws", "Skills"].map((name) =>
        proficiency(name, DND_PROFICIENCY_SCALE),
      ),
      numberField("attribute", "Level"),
      numberField("attribute", "Experience"),
      textField("tag", "Class"),
      textField("tag", "Subclass"),
      textField("tag", "Species"),
      textField("tag", "Background"),
      textField("tag", "Alignment"),
      booleanField("condition", "Inspiration"),
      resource("Hit Points"),
      numberField("attribute", "Armor Class"),
      numberField("attribute", "Initiative"),
      numberField("attribute", "Speed"),
      numberField("attribute", "Proficiency Bonus"),
      numberField("attribute", "Passive Perception"),
      textField("tag", "Spellcasting Ability"),
    ],
  },
  {
    id: "fantasy",
    name: "Fantasy",
    allocation: { pool: 40 },
    components: [
      attribute("Strength", 0, 20, 5),
      attribute("Agility", 0, 20, 5),
      attribute("Endurance", 0, 20, 5),
      attribute("Intelligence", 0, 20, 5),
      attribute("Willpower", 0, 20, 5),
      attribute("Perception", 0, 20, 5),
      attribute("Presence", 0, 20, 5),
      attribute("Magic", 0, 20, 5),
      ...FANTASY_SKILLS.map(skill),
      ...["Weapons", "Armor", "Tools", "Languages", "Magical Schools", "Professions", "Scholarly Disciplines"].map(
        (name) => proficiency(name, FANTASY_PROFICIENCY_SCALE),
      ),
      resource("Wealth"),
      resource("Reputation"),
      resource("Influence"),
      resource("Mana"),
      resource("Renown"),
      resource("Political Power"),
    ],
  },
  {
    id: "scifi",
    name: "Sci-Fi",
    components: [
      attribute("Strength", 0, 20, 5),
      attribute("Agility", 0, 20, 5),
      attribute("Endurance", 0, 20, 5),
      attribute("Intelligence", 0, 20, 5),
      attribute("Awareness", 0, 20, 5),
      attribute("Willpower", 0, 20, 5),
      attribute("Presence", 0, 20, 5),
      attribute("Technical Aptitude", 0, 20, 5),
      ...SCIFI_SKILLS.map(skill),
      ...[
        "Weapon Systems",
        "Vehicle Types",
        "Starship Classes",
        "Operating Systems",
        "Programming Languages",
        "Cybernetics",
        "Industrial Equipment",
        "Alien Technologies",
      ].map((name) => proficiency(name, FANTASY_PROFICIENCY_SCALE)),
      resource("Credits"),
      resource("Reputation"),
      resource("Influence"),
      resource("Energy"),
      resource("Ammunition"),
      resource("Fuel"),
      resource("Data"),
      resource("Population"),
      resource("Fleet Strength"),
    ],
  },
];

export function profilePreset(id: string | undefined): ProfilePreset | undefined {
  return PROFILE_PRESETS.find((preset) => preset.id === id);
}

export function profilePresetLabel(id: string | undefined): string {
  return profilePreset(id)?.name ?? "Custom";
}

export function profileFromPreset(id: ProfilePresetOrigin): ProfileDocument {
  const preset = profilePreset(id);
  if (!preset) throw new Error("Unknown Profile preset");
  return {
    schemaVersion: PROFILE_SCHEMA_VERSION,
    presetOrigin: preset.id,
    ...(preset.allocation ? { allocation: { ...preset.allocation } } : {}),
    components: preset.components.map((component) => ({
      ...component,
      scale: component.scale ? [...component.scale] : undefined,
      dependencies: component.dependencies ? [...component.dependencies] : undefined,
      value: { ...component.value },
    })),
  };
}

export const DND_ABILITY_KEYS = [
  "strength",
  "dexterity",
  "constitution",
  "intelligence",
  "wisdom",
  "charisma",
] as const;

export type DndAbilityKey = (typeof DND_ABILITY_KEYS)[number];

export type DndAncestry = {
  id: string;
  name: string;
  lineage: string;
  speed: number;
  abilities: Partial<Record<DndAbilityKey, number>>;
};

const SPECIES_ID = componentId("tag", "Species");
const SPEED_ID = componentId("attribute", "Speed");

export const DND_ANCESTRIES: DndAncestry[] = [
  { id: "hill-dwarf", name: "Hill Dwarf", lineage: "Dwarf", speed: 25, abilities: { constitution: 2, wisdom: 1 } },
  {
    id: "mountain-dwarf",
    name: "Mountain Dwarf",
    lineage: "Dwarf",
    speed: 25,
    abilities: { strength: 2, constitution: 2 },
  },
  { id: "high-elf", name: "High Elf", lineage: "Elf", speed: 30, abilities: { dexterity: 2, intelligence: 1 } },
  { id: "wood-elf", name: "Wood Elf", lineage: "Elf", speed: 35, abilities: { dexterity: 2, wisdom: 1 } },
  { id: "drow", name: "Drow", lineage: "Elf", speed: 30, abilities: { dexterity: 2, charisma: 1 } },
  {
    id: "lightfoot-halfling",
    name: "Lightfoot Halfling",
    lineage: "Halfling",
    speed: 25,
    abilities: { dexterity: 2, charisma: 1 },
  },
  {
    id: "stout-halfling",
    name: "Stout Halfling",
    lineage: "Halfling",
    speed: 25,
    abilities: { dexterity: 2, constitution: 1 },
  },
  {
    id: "human",
    name: "Human",
    lineage: "Human",
    speed: 30,
    abilities: { strength: 1, dexterity: 1, constitution: 1, intelligence: 1, wisdom: 1, charisma: 1 },
  },
  { id: "dragonborn", name: "Dragonborn", lineage: "Dragonborn", speed: 30, abilities: { strength: 2, charisma: 1 } },
  {
    id: "forest-gnome",
    name: "Forest Gnome",
    lineage: "Gnome",
    speed: 25,
    abilities: { intelligence: 2, dexterity: 1 },
  },
  {
    id: "rock-gnome",
    name: "Rock Gnome",
    lineage: "Gnome",
    speed: 25,
    abilities: { intelligence: 2, constitution: 1 },
  },
  { id: "half-elf", name: "Half-Elf", lineage: "Half-Elf", speed: 30, abilities: { charisma: 2 } },
  { id: "half-orc", name: "Half-Orc", lineage: "Half-Orc", speed: 30, abilities: { strength: 2, constitution: 1 } },
  { id: "tiefling", name: "Tiefling", lineage: "Tiefling", speed: 30, abilities: { intelligence: 1, charisma: 2 } },
];

export function dndAncestriesByLineage(): { lineage: string; ancestries: DndAncestry[] }[] {
  const groups: { lineage: string; ancestries: DndAncestry[] }[] = [];
  for (const ancestry of DND_ANCESTRIES) {
    const group = groups.find((candidate) => candidate.lineage === ancestry.lineage);
    if (group) group.ancestries.push(ancestry);
    else groups.push({ lineage: ancestry.lineage, ancestries: [ancestry] });
  }
  return groups;
}

export function matchingDndAncestry(profile: ProfileDocument): DndAncestry | undefined {
  const species = profile.components.find((component) => component.id === SPECIES_ID);
  if (!species || species.value.type !== "text" || !species.value.value) return undefined;
  const name = species.value.value;
  return DND_ANCESTRIES.find((ancestry) => ancestry.name === name);
}

function clampAbility(component: ProfileComponent, value: number): number {
  if (component.min !== undefined) value = Math.max(component.min, value);
  if (component.max !== undefined) value = Math.min(component.max, value);
  return value;
}

export function applyDndAncestry(profile: ProfileDocument, ancestryId: string): ProfileDocument {
  const next = ancestryId ? DND_ANCESTRIES.find((ancestry) => ancestry.id === ancestryId) : undefined;
  if (ancestryId && !next) return profile;
  const previous = matchingDndAncestry(profile);
  return {
    ...profile,
    components: profile.components.map((component) => {
      if (component.id === SPECIES_ID && component.value.type === "text") {
        return { ...component, value: { type: "text", value: next?.name ?? null } };
      }
      if (component.id === SPEED_ID && component.value.type === "number") {
        if (next) return { ...component, value: { type: "number", value: next.speed } };
        if (previous && component.value.value === previous.speed) {
          return { ...component, value: { type: "number", value: null } };
        }
        return component;
      }
      if (component.kind !== "attribute" || component.value.type !== "number") return component;
      const key = DND_ABILITY_KEYS.find((ability) => component.id === `attribute-${ability}`);
      if (!key) return component;
      const delta = (next?.abilities[key] ?? 0) - (previous?.abilities[key] ?? 0);
      if (!delta) return component;
      const current = component.value.value ?? 10;
      return { ...component, value: { type: "number", value: clampAbility(component, current + delta) } };
    }),
  };
}
