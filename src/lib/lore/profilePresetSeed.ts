import type { ProfileAllocation, ProfileComponent, ProfileComponentKind } from "./profile.ts";

export const CUSTOM_PRESET_ID = "ef2bce76-ca39-59f5-ab9d-28a0b665eb15";
export const DND_PRESET_ID = "9771b70b-04c4-506e-949b-b26ea5b61235";
export const FANTASY_PRESET_ID = "1b2c999f-65b5-57b6-9d02-6ce2c40cc977";
export const SCIFI_PRESET_ID = "d9ff8d59-1cd3-52ae-8adb-5700fbff6ba1";
export const DND_FACTION_PRESET_ID = "e4baa36f-9591-5eb3-aa6c-96728fe77e61";
export const DND_LOCATION_PRESET_ID = "f7f1d532-b04f-5ef6-b440-eb4e21ab6283";
export const FANTASY_FACTION_PRESET_ID = "b324c23c-aba7-5734-9a21-84894ffa93c5";
export const FANTASY_KINGDOM_PRESET_ID = "c1b642b3-f218-53e7-a8c5-db41c327ec51";
export const FANTASY_PLACE_PRESET_ID = "a8d73270-c9a6-5e5a-8ed7-3bd22704b725";
export const FANTASY_ARTIFACT_PRESET_ID = "e9757343-bf50-5326-ae6f-915cd6894ab6";
export const SCIFI_FACTION_PRESET_ID = "25dae462-323c-5e93-8322-d7d42d2a9733";
export const STARSHIP_PRESET_ID = "6f753e3f-342c-583d-88dc-9cdc55cb42ef";
export const COLONY_PRESET_ID = "3aa17fe2-dd28-50ba-b54a-2e7da9c4a65d";
export const CULTURE_PRESET_ID = "6fefd437-d376-5772-a5c4-7ebf00da0a69";
export const CONCEPT_PRESET_ID = "26bf13cb-1970-5bfc-a728-69cabc9b47f0";

const PERSON = "daena.lore:person";
const FACTION = "daena.lore:faction";
const PLACE = "daena.lore:place";
const ARTIFACT = "daena.lore:artifact";
const CULTURE = "daena.lore:culture";
const CONCEPT = "daena.lore:concept";
const GENRE_DND = "D&D";
const GENRE_FANTASY = "Fantasy";
const GENRE_SCIFI = "Sci-Fi";

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
  legacyKey?: LegacyPresetOrigin;
  name: string;
  description: string;
  icon: string;
  entityTypes?: string[];
  genre?: string;
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

function resource(name: string, unit?: string): ProfileComponent {
  return {
    id: componentId("resource", name),
    kind: "resource",
    name,
    value: { type: "resource", current: null, max: null, unit: unit ?? null },
  };
}

function derivedAverage(name: string, sources: readonly string[]): ProfileComponent {
  const ids = sources.map((source) => componentId("attribute", source));
  return {
    id: componentId("derived", name),
    kind: "derived",
    name,
    formula: `avg(${ids.map((id) => `{${id}}`).join(", ")})`,
    dependencies: ids,
    value: { type: "number", value: null },
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
  {
    id: CUSTOM_PRESET_ID,
    legacyKey: "custom",
    name: "Custom",
    description: "Blank sheet. Field schema comes later.",
    icon: "custom",
    components: [],
  },
  {
    id: DND_PRESET_ID,
    legacyKey: "dnd",
    name: "D&D Character",
    description: "Six abilities, skills, and combat basics.",
    icon: "dnd",
    entityTypes: [PERSON],
    genre: GENRE_DND,
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
    id: FANTASY_PRESET_ID,
    legacyKey: "fantasy",
    name: "Fantasy Character",
    description: "Eight attributes with a 40-point pool.",
    icon: "fantasy",
    entityTypes: [PERSON],
    genre: GENRE_FANTASY,
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
    id: SCIFI_PRESET_ID,
    legacyKey: "scifi",
    name: "Sci-Fi Character",
    description: "Technical scores, skills, and resources.",
    icon: "scifi",
    entityTypes: [PERSON],
    genre: GENRE_SCIFI,
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
    ],
  },
  {
    id: DND_FACTION_PRESET_ID,
    name: "D&D Faction",
    description: "Influence, strength, and resources for a group.",
    icon: "dnd",
    entityTypes: [FACTION],
    genre: GENRE_DND,
    components: [
      attribute("Influence", 0, 20, 0),
      attribute("Military Strength", 0, 20, 0),
      attribute("Wealth", 0, 20, 0),
      attribute("Secrecy", 0, 20, 0),
      attribute("Reach", 0, 20, 0),
      resource("Gold"),
      resource("Members"),
      resource("Strongholds"),
      resource("Allies"),
      textField("tag", "Specialization"),
    ],
  },
  {
    id: DND_LOCATION_PRESET_ID,
    name: "D&D Location",
    description: "Danger, access, and treasure for a place.",
    icon: "dnd",
    entityTypes: [PLACE],
    genre: GENRE_DND,
    components: [
      attribute("Danger Level", 0, 20, 0),
      attribute("Accessibility", 0, 20, 0),
      attribute("Magical Saturation", 0, 20, 0),
      resource("Treasure", "gold"),
      numberField("attribute", "Population"),
    ],
  },
  {
    id: FANTASY_FACTION_PRESET_ID,
    name: "Fantasy Faction",
    description: "Power, stability, and holdings.",
    icon: "fantasy",
    entityTypes: [FACTION],
    genre: GENRE_FANTASY,
    components: [
      attribute("Military Power", 0, 100, 0),
      attribute("Political Influence", 0, 100, 0),
      attribute("Economic Power", 0, 100, 0),
      attribute("Intelligence", 0, 100, 0),
      attribute("Stability", 0, 100, 0),
      resource("Treasury"),
      numberField("attribute", "Population"),
      numberField("attribute", "Territory"),
      numberField("attribute", "Armies"),
      derivedAverage("Overall Power", ["Military Power", "Political Influence", "Economic Power"]),
    ],
  },
  {
    id: FANTASY_KINGDOM_PRESET_ID,
    name: "Fantasy Kingdom",
    description: "A realm's stability, reach, and output.",
    icon: "fantasy",
    entityTypes: [FACTION, PLACE],
    genre: GENRE_FANTASY,
    components: [
      attribute("Stability", 0, 100, 0),
      attribute("Technology", 0, 100, 0),
      attribute("Infrastructure", 0, 100, 0),
      attribute("Military Power", 0, 100, 0),
      attribute("Administrative Capacity", 0, 100, 0),
      numberField("attribute", "Population"),
      resource("Treasury", "gold"),
      numberField("attribute", "Food"),
      numberField("attribute", "Territory"),
      derivedAverage("Regional Influence", ["Military Power", "Stability"]),
      derivedAverage("Economic Output", ["Infrastructure", "Technology"]),
    ],
  },
  {
    id: FANTASY_PLACE_PRESET_ID,
    name: "Fantasy Place",
    description: "Strategic value and local resources.",
    icon: "fantasy",
    entityTypes: [PLACE],
    genre: GENRE_FANTASY,
    components: [
      attribute("Strategic Value", 0, 20, 0),
      attribute("Defensibility", 0, 20, 0),
      attribute("Prosperity", 0, 20, 0),
      attribute("Magical Resonance", 0, 20, 0),
      numberField("attribute", "Population"),
      resource("Trade Volume"),
      numberField("attribute", "Garrison"),
    ],
  },
  {
    id: FANTASY_ARTIFACT_PRESET_ID,
    name: "Fantasy Artifact",
    description: "Power, charges, and capabilities.",
    icon: "fantasy",
    entityTypes: [ARTIFACT],
    genre: GENRE_FANTASY,
    components: [
      attribute("Power", 0, 20, 0),
      attribute("Durability", 0, 20, 0),
      attribute("Resonance", 0, 20, 0),
      ...["Binding", "Manipulation", "Protection", "Destruction", "Divination", "Communication"].map(skill),
      resource("Charges"),
      derivedAverage("Magical Potency", ["Power", "Resonance"]),
    ],
  },
  {
    id: SCIFI_FACTION_PRESET_ID,
    name: "Sci-Fi Faction",
    description: "Fleets, industry, and diplomatic reach.",
    icon: "scifi",
    entityTypes: [FACTION],
    genre: GENRE_SCIFI,
    components: [
      attribute("Military Strength", 0, 100, 0),
      attribute("Technological Advancement", 0, 100, 0),
      attribute("Economic Power", 0, 100, 0),
      attribute("Intelligence Network", 0, 100, 0),
      attribute("Diplomatic Influence", 0, 100, 0),
      resource("Credits"),
      numberField("attribute", "Fleet Size"),
      numberField("attribute", "Population"),
      numberField("attribute", "Territory"),
      numberField("attribute", "Research Output"),
      derivedAverage("Power Index", ["Military Strength", "Technological Advancement", "Economic Power"]),
    ],
  },
  {
    id: STARSHIP_PRESET_ID,
    name: "Starship",
    description: "Hull, firepower, and ship stores.",
    icon: "scifi",
    entityTypes: [ARTIFACT],
    genre: GENRE_SCIFI,
    components: [
      attribute("Speed", 0, 20, 0),
      attribute("Maneuverability", 0, 20, 0),
      attribute("Hull Strength", 0, 20, 0),
      attribute("Sensor Capability", 0, 20, 0),
      attribute("Firepower", 0, 20, 0),
      attribute("Stealth", 0, 20, 0),
      resource("Fuel"),
      resource("Ammunition"),
      resource("Crew"),
      resource("Cargo", "tons"),
      resource("Shield Strength"),
      derivedAverage("Combat Rating", ["Firepower", "Hull Strength", "Maneuverability"]),
    ],
  },
  {
    id: COLONY_PRESET_ID,
    name: "Colony / Station",
    description: "Infrastructure, life support, and readiness.",
    icon: "scifi",
    entityTypes: [PLACE],
    genre: GENRE_SCIFI,
    components: [
      attribute("Infrastructure", 0, 100, 0),
      attribute("Life Support", 0, 100, 0),
      attribute("Defenses", 0, 100, 0),
      attribute("Research Capacity", 0, 100, 0),
      attribute("Trade Hub", 0, 100, 0),
      numberField("attribute", "Population"),
      resource("Power Supply", "MW"),
      resource("Food"),
      resource("Water"),
      resource("Atmosphere", "%"),
      derivedAverage("Readiness", ["Infrastructure", "Life Support", "Defenses"]),
    ],
  },
  {
    id: CULTURE_PRESET_ID,
    name: "Culture",
    description: "Tradition, reach, and ways of living.",
    icon: "custom",
    entityTypes: [CULTURE],
    components: [
      attribute("Technological Level", 0, 100, 0),
      attribute("Military Tradition", 0, 100, 0),
      attribute("Artistic Achievement", 0, 100, 0),
      attribute("Religious Devotion", 0, 100, 0),
      attribute("Expansionism", 0, 100, 0),
      numberField("attribute", "Population"),
      numberField("attribute", "Territory"),
      numberField("attribute", "Sacred Sites"),
    ],
  },
  {
    id: CONCEPT_PRESET_ID,
    name: "Concept",
    description: "Influence, spread, and institutions.",
    icon: "custom",
    entityTypes: [CONCEPT],
    components: [
      attribute("Influence", 0, 100, 0),
      attribute("Spread", 0, 100, 0),
      attribute("Orthodoxy", 0, 100, 0),
      numberField("attribute", "Adherents"),
      numberField("attribute", "Institutions"),
      numberField("attribute", "Texts"),
    ],
  },
];
