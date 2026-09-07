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
      value: { ...component.value },
    })),
  };
}
