import type { ModuleContext, UUID } from "../../../packages/module-api/src/index";
import {
  emptyProfile,
  PROFILE_COLLECTION,
  PROFILE_SCHEMA_VERSION,
  type ProfileComponent,
  type ProfileComponentKind,
  type ProfileDocument,
} from "./profile.ts";
import {
  BUNDLED_PROFILE_PRESET_SEEDS,
  canonicalPresetOrigin,
  isCustomPresetOrigin,
  type BundledProfilePresetSeed,
  type LegacyPresetOrigin,
} from "./profilePresetSeed.ts";

export const PROFILE_PRESET_COLLECTION = "profile-preset";
export const PROFILE_PRESET_SCHEMA_VERSION = 1;

export type ProfilePresetDocument = {
  schemaVersion: number;
  name: string;
  description?: string;
  icon?: string;
  entityTypes?: string[];
  genre?: string;
  builtin?: boolean;
  hidden?: boolean;
  allocation?: ProfileDocument["allocation"];
  components: ProfileDocument["components"];
};

export type LoadedProfilePreset = {
  id: UUID;
  revision: string;
  document: ProfilePresetDocument;
};

export function bundledPresetDocument(seed: BundledProfilePresetSeed): ProfilePresetDocument {
  return {
    schemaVersion: PROFILE_PRESET_SCHEMA_VERSION,
    name: seed.name,
    description: seed.description,
    icon: seed.icon,
    builtin: true,
    ...(seed.allocation ? { allocation: { ...seed.allocation } } : {}),
    components: seed.components.map((component) => ({
      ...component,
      scale: component.scale ? [...component.scale] : undefined,
      dependencies: component.dependencies ? [...component.dependencies] : undefined,
      value: { ...component.value },
    })),
  };
}

export function profileFromSeed(seed: BundledProfilePresetSeed): ProfileDocument {
  const document = bundledPresetDocument(seed);
  return {
    schemaVersion: PROFILE_SCHEMA_VERSION,
    presetOrigin: seed.id,
    ...(document.allocation ? { allocation: { ...document.allocation } } : {}),
    components: document.components,
  };
}

export function profileFromPreset(id: string): ProfileDocument {
  const seed = BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === id || preset.legacyKey === id);
  if (!seed) throw new Error("Unknown Profile preset");
  return profileFromSeed(seed);
}

export function profileForPresetSelection(
  presets: readonly LoadedProfilePreset[],
  selectedId: string | undefined,
): ProfileDocument {
  const selected = selectedId ? presets.find((preset) => preset.id === selectedId) : undefined;
  if (!selected) return emptyProfile();
  return {
    schemaVersion: PROFILE_SCHEMA_VERSION,
    presetOrigin: selected.id,
    ...(selected.document.allocation ? { allocation: structuredClone(selected.document.allocation) } : {}),
    components: structuredClone(selected.document.components ?? []),
  };
}

export function profilePresetLabel(id: string | undefined): string {
  if (!id || isCustomPresetOrigin(id)) return "Custom";
  const seed = BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === id || preset.legacyKey === id);
  return seed?.name ?? "Custom";
}

export async function loadProfilePresets(context: ModuleContext): Promise<LoadedProfilePreset[]> {
  const records = await context.projectRecords.list<ProfilePresetDocument>(PROFILE_PRESET_COLLECTION);
  return records
    .filter((record) => record.value && record.value.hidden !== true)
    .map((record) => ({ id: record.id, revision: record.revision, document: record.value }));
}

export async function migrateLegacyPresetOrigins(
  context: ModuleContext,
  entityIds: readonly string[],
): Promise<void> {
  const failures: string[] = [];
  for (const entityId of entityIds) {
    try {
      const records = await context.records.list<ProfileDocument>(PROFILE_COLLECTION, entityId as UUID, { limit: 2 });
      const record = records[0];
      const origin = record?.value.presetOrigin;
      if (!record || typeof origin !== "string") continue;
      const canonical = canonicalPresetOrigin(origin);
      if (canonical === origin) continue;
      await context.records.update(
        PROFILE_COLLECTION,
        record.id,
        entityId as UUID,
        { ...record.value, presetOrigin: canonical },
        { expectedRevision: record.revision },
      );
    } catch (cause) {
      failures.push(cause instanceof Error ? cause.message : String(cause));
    }
  }
  if (failures.length) throw new Error(failures[0]);
}

export async function ensureBundledProfilePresets(context: ModuleContext): Promise<void> {
  const existing = await context.projectRecords.list<ProfilePresetDocument>(PROFILE_PRESET_COLLECTION);
  const ids = new Set(existing.map((record) => record.id));
  for (const seed of BUNDLED_PROFILE_PRESET_SEEDS) {
    if (ids.has(seed.id as UUID)) continue;
    await context.projectRecords.create(PROFILE_PRESET_COLLECTION, bundledPresetDocument(seed), { id: seed.id });
  }
}

export function legacyPresetOrigin(value: string | undefined): LegacyPresetOrigin | undefined {
  const seed = BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.legacyKey === value);
  return seed?.legacyKey;
}

export { canonicalPresetOrigin };

function componentId(kind: ProfileComponentKind, name: string): string {
  return `${kind}-${name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")}`;
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
