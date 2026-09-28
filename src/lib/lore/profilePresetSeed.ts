import type { ProfileAllocation, ProfileComponent, ProfileComponentKind } from "./profile.ts";

export const CUSTOM_PRESET_ID = "ef2bce76-ca39-59f5-ab9d-28a0b665eb15";
export const DND_PRESET_ID = "9771b70b-04c4-506e-949b-b26ea5b61235";
export const FANTASY_PRESET_ID = "1b2c999f-65b5-57b6-9d02-6ce2c40cc977";
export const SCIFI_PRESET_ID = "d9ff8d59-1cd3-52ae-8adb-5700fbff6ba1";

export const LEGACY_PRESET_ORIGINS = ["custom", "dnd", "fantasy", "scifi"] as const;
export type LegacyPresetOrigin = (typeof LEGACY_PRESET_ORIGINS)[number];

export const LEGACY_PRESET_IDS: Record<LegacyPresetOrigin, string> = {
  custom: CUSTOM_PRESET_ID,
  dnd: DND_PRESET_ID,
  fantasy: FANTASY_PRESET_ID,
  scifi: SCIFI_PRESET_ID,
};

const PRESET_ID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function canonicalPresetOrigin(value: string): string {
  if (value in LEGACY_PRESET_IDS) return LEGACY_PRESET_IDS[value as LegacyPresetOrigin];
  return value;
}

export function isPresetOrigin(value: string): boolean {
  return value in LEGACY_PRESET_IDS || PRESET_ID_RE.test(value);
}

export function isDndPresetOrigin(value: string | undefined): boolean {
  return value === "dnd" || value === DND_PRESET_ID;
}

export function isCustomPresetOrigin(value: string | undefined): boolean {
  return value === "custom" || value === CUSTOM_PRESET_ID;
}

export type BundledProfilePresetSeed = {
  id: string;
  legacyKey: LegacyPresetOrigin;
  name: string;
  description: string;
  icon: LegacyPresetOrigin;
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

export const BUNDLED_PROFILE_PRESET_SEEDS: BundledProfilePresetSeed[] = [
  { id: CUSTOM_PRESET_ID, legacyKey: "custom", name: "Custom", description: "Blank sheet. Field schema comes later.", icon: "custom", components: [] },
  {
    id: DND_PRESET_ID, legacyKey: "dnd", name: "D&D", description: "Six abilities, skills, and combat basics.", icon: "dnd",
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
    id: FANTASY_PRESET_ID, legacyKey: "fantasy", name: "Fantasy", description: "Eight attributes with a 40-point pool.", icon: "fantasy",
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
    id: SCIFI_PRESET_ID, legacyKey: "scifi", name: "Sci-Fi", description: "Technical scores, skills, and resources.", icon: "scifi",
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

