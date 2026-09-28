import type { ModuleContext, ProjectModuleRecord, UUID } from "../../../packages/module-api/src/index";
import {
  cloneProfileData,
  emptyProfile,
  emptyValue,
  PROFILE_COLLECTION,
  PROFILE_SCHEMA_VERSION,
  type ProfileComponent,
  type ProfileComponentKind,
  type ProfileDocument,
} from "./profile.ts";
import {
  BUNDLED_PROFILE_PRESET_SEEDS,
  CUSTOM_PRESET_ID,
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
  customized?: boolean;
  allocation?: ProfileDocument["allocation"];
  components: ProfileDocument["components"];
};

export type ProfilePresetFields = {
  name: string;
  description?: string;
  entityTypes?: string[];
  genre?: string;
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
    ...(seed.entityTypes?.length ? { entityTypes: [...seed.entityTypes] } : {}),
    ...(seed.genre ? { genre: seed.genre } : {}),
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
    ...(selected.document.allocation ? { allocation: cloneProfileData(selected.document.allocation) } : {}),
    components: cloneProfileData(selected.document.components ?? []),
  };
}

export function profilePresetLabel(id: string | undefined, presets?: readonly LoadedProfilePreset[]): string {
  const loaded = id ? presets?.find((preset) => preset.id === id) : undefined;
  if (loaded) return loaded.document.name;
  if (!id || isCustomPresetOrigin(id)) return "Custom";
  const seed = BUNDLED_PROFILE_PRESET_SEEDS.find((preset) => preset.id === id || preset.legacyKey === id);
  if (seed) return seed.name;
  return presets ? "" : "Custom";
}

async function listPresetRecords(context: ModuleContext) {
  const records: ProjectModuleRecord<ProfilePresetDocument>[] = [];
  let offset = 0;
  while (true) {
    const page = await context.projectRecords.list<ProfilePresetDocument>(PROFILE_PRESET_COLLECTION, {
      limit: 200,
      offset,
    });
    records.push(...page);
    if (page.length < 200) break;
    offset += page.length;
  }
  return records;
}

export async function loadAllProfilePresets(context: ModuleContext): Promise<LoadedProfilePreset[]> {
  const records = await listPresetRecords(context);
  return records
    .filter((record) => record.value)
    .map((record) => ({ id: record.id, revision: record.revision, document: record.value }));
}

export async function loadProfilePresets(context: ModuleContext): Promise<LoadedProfilePreset[]> {
  const records = await loadAllProfilePresets(context);
  return records.filter((record) => record.document.hidden !== true);
}

export async function migrateLegacyPresetOrigins(context: ModuleContext, entityIds: readonly string[]): Promise<void> {
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

function stableJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.entries(value as Record<string, unknown>)
      .filter(([, item]) => item !== undefined)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => `${JSON.stringify(key)}:${stableJson(item)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value) ?? "null";
}

export function presetDocumentMatchesSeed(stored: ProfilePresetDocument, seed: ProfilePresetDocument): boolean {
  return (
    stored.schemaVersion === seed.schemaVersion &&
    stored.name === seed.name &&
    stored.description === seed.description &&
    stored.icon === seed.icon &&
    stored.builtin === true &&
    (stored.genre ?? null) === (seed.genre ?? null) &&
    stableJson(stored.entityTypes ?? null) === stableJson(seed.entityTypes ?? null) &&
    stableJson(stored.allocation ?? null) === stableJson(seed.allocation ?? null) &&
    stableJson(stored.components) === stableJson(seed.components)
  );
}

export async function ensureBundledProfilePresets(context: ModuleContext): Promise<void> {
  const existing = await listPresetRecords(context);
  const byId = new Map(existing.map((record) => [record.id, record]));
  const failures: string[] = [];
  for (const seed of BUNDLED_PROFILE_PRESET_SEEDS) {
    const document = bundledPresetDocument(seed);
    const stored = byId.get(seed.id as UUID);
    try {
      if (!stored) {
        await context.projectRecords.create(PROFILE_PRESET_COLLECTION, document, { id: seed.id });
        continue;
      }
      if (stored.value?.customized) continue;
      if (stored.value && presetDocumentMatchesSeed(stored.value, document)) continue;
      if (stored.value && !bundledPresetNeedsMigration(stored.value, seed)) continue;
      const next = stored.value?.hidden ? { ...document, hidden: true } : document;
      await context.projectRecords.update(PROFILE_PRESET_COLLECTION, stored.id, next, {
        expectedRevision: stored.revision,
      });
    } catch (cause) {
      failures.push(cause instanceof Error ? cause.message : String(cause));
    }
  }
  if (failures.length) throw new Error(failures[0]);
}

const LEGACY_BUNDLED_NAMES: Partial<Record<LegacyPresetOrigin, string>> = {
  dnd: "D&D",
  fantasy: "Fantasy",
  scifi: "Sci-Fi",
};

export function bundledPresetNeedsMigration(stored: ProfilePresetDocument, seed: BundledProfilePresetSeed): boolean {
  const legacyName = seed.legacyKey ? LEGACY_BUNDLED_NAMES[seed.legacyKey] : undefined;
  return Boolean(legacyName) && stored.name === legacyName && !stored.entityTypes?.length && !stored.genre;
}

export function normalizePresetFields(fields: ProfilePresetFields): ProfilePresetFields {
  const name = fields.name.trim();
  if (!name) throw new Error("Preset name is required");
  const description = fields.description?.trim() ?? "";
  const genre = fields.genre?.trim() ?? "";
  const entityTypes = [
    ...new Set(fields.entityTypes?.map((id) => qualifiedProfileEntityType(id)).filter(Boolean) ?? []),
  ];
  return {
    name,
    ...(description ? { description } : {}),
    ...(entityTypes.length ? { entityTypes } : {}),
    ...(genre ? { genre } : {}),
  };
}

function withoutPresetValueProgress(
  component: ProfileComponent,
  origin: ProfileComponent | undefined,
): ProfileComponent {
  const value = origin ? cloneProfileData(origin.value) : startingPresetValue(component);
  if (value.type === "resource" && component.value.type === "resource") {
    value.unit = component.unit ?? component.value.unit ?? value.unit;
  }
  const next: ProfileComponent = {
    id: component.id,
    kind: component.kind,
    name: component.name,
    value,
  };
  if (component.scale) next.scale = [...component.scale];
  if (component.min !== undefined) next.min = component.min;
  if (component.max !== undefined) next.max = component.max;
  if (component.unit) next.unit = component.unit;
  if (component.formula) next.formula = component.formula;
  if (component.dependencies) next.dependencies = [...component.dependencies];
  if (component.decimals !== undefined) next.decimals = component.decimals;
  return next;
}

function startingPresetValue(component: ProfileComponent): ProfileComponent["value"] {
  if (component.value.type === "resource") {
    return { type: "resource", current: null, max: null, unit: component.unit ?? component.value.unit ?? null };
  }
  if (component.value.type === "number" && component.min !== undefined) return { type: "number", value: component.min };
  return emptyValue(component.value.type);
}

export function presetFromProfile(
  profile: ProfileDocument,
  fields: ProfilePresetFields,
  origin?: ProfilePresetDocument | null,
): ProfilePresetDocument {
  const meta = normalizePresetFields(fields);
  const originById = new Map((origin?.components ?? []).map((component) => [component.id, component]));
  return {
    schemaVersion: PROFILE_PRESET_SCHEMA_VERSION,
    ...meta,
    ...(profile.allocation ? { allocation: { ...profile.allocation } } : {}),
    components: profile.components.map((component) =>
      withoutPresetValueProgress(component, originById.get(component.id)),
    ),
  };
}

export function blankProfilePreset(fields: ProfilePresetFields): ProfilePresetDocument {
  return { schemaVersion: PROFILE_PRESET_SCHEMA_VERSION, ...normalizePresetFields(fields), components: [] };
}

export function applyPresetFields(preset: ProfilePresetDocument, fields: ProfilePresetFields): ProfilePresetDocument {
  const meta = normalizePresetFields(fields);
  return {
    schemaVersion: preset.schemaVersion || PROFILE_PRESET_SCHEMA_VERSION,
    ...meta,
    ...(preset.icon ? { icon: preset.icon } : {}),
    ...(preset.builtin ? { builtin: true, customized: true } : {}),
    ...(preset.hidden ? { hidden: true } : {}),
    ...(preset.allocation ? { allocation: cloneProfileData(preset.allocation) } : {}),
    components: cloneProfileData(preset.components ?? []),
  };
}

export function duplicatedProfilePreset(preset: ProfilePresetDocument, name?: string): ProfilePresetDocument {
  const copy = cloneProfileData(preset);
  delete copy.builtin;
  delete copy.hidden;
  delete copy.customized;
  return {
    ...copy,
    schemaVersion: PROFILE_PRESET_SCHEMA_VERSION,
    name: name?.trim() || `${preset.name} copy`,
  };
}

function loadedPreset(record: { id: UUID; revision: string; value: ProfilePresetDocument }): LoadedProfilePreset {
  return { id: record.id, revision: record.revision, document: record.value };
}

export async function createProfilePreset(
  context: ModuleContext,
  document: ProfilePresetDocument,
): Promise<LoadedProfilePreset> {
  const record = await context.projectRecords.create(PROFILE_PRESET_COLLECTION, document);
  return loadedPreset(record);
}

export async function updateProfilePreset(
  context: ModuleContext,
  preset: LoadedProfilePreset,
  document: ProfilePresetDocument,
): Promise<LoadedProfilePreset> {
  const record = await context.projectRecords.update(PROFILE_PRESET_COLLECTION, preset.id, document, {
    expectedRevision: preset.revision,
  });
  return loadedPreset(record);
}

export async function duplicateProfilePreset(
  context: ModuleContext,
  preset: LoadedProfilePreset,
  name?: string,
): Promise<LoadedProfilePreset> {
  return createProfilePreset(context, duplicatedProfilePreset(preset.document, name));
}

export async function setProfilePresetHidden(
  context: ModuleContext,
  preset: LoadedProfilePreset,
  hidden: boolean,
): Promise<LoadedProfilePreset> {
  const document = { ...preset.document };
  if (hidden) document.hidden = true;
  else delete document.hidden;
  return updateProfilePreset(context, preset, document);
}

export async function deleteProfilePreset(context: ModuleContext, preset: LoadedProfilePreset): Promise<void> {
  if (preset.document.builtin) throw new Error("Bundled presets cannot be deleted");
  await context.projectRecords.delete(PROFILE_PRESET_COLLECTION, preset.id, { expectedRevision: preset.revision });
}

export async function resetBundledProfilePreset(
  context: ModuleContext,
  preset: LoadedProfilePreset,
): Promise<LoadedProfilePreset> {
  const seed = BUNDLED_PROFILE_PRESET_SEEDS.find((candidate) => candidate.id === preset.id);
  if (!seed) throw new Error("Only bundled presets can be reset");
  return updateProfilePreset(context, preset, bundledPresetDocument(seed));
}

const PRESET_GENRE_ORDER = ["D&D", "Fantasy", "Sci-Fi"];

export function qualifiedProfileEntityType(entityType: string | null | undefined): string {
  const value = entityType?.trim() ?? "";
  if (!value || value.includes(":")) return value;
  return `daena.lore:${value}`;
}

export type ProfilePresetGroup = {
  genre: string | null;
  presets: LoadedProfilePreset[];
};

export function groupProfilePresets(
  presets: readonly LoadedProfilePreset[],
  entityType: string | null | undefined,
): { suggested: LoadedProfilePreset[]; others: ProfilePresetGroup[] } {
  const qualified = qualifiedProfileEntityType(entityType);
  const suggested: LoadedProfilePreset[] = [];
  const rest: LoadedProfilePreset[] = [];
  for (const preset of presets) {
    if (preset.document.hidden) continue;
    const types = preset.document.entityTypes;
    if (!types?.length || types.includes(qualified)) suggested.push(preset);
    else rest.push(preset);
  }
  const grouped = new Map<string | null, LoadedProfilePreset[]>();
  for (const preset of rest) {
    const genre = preset.document.genre ?? null;
    const list = grouped.get(genre) ?? [];
    list.push(preset);
    grouped.set(genre, list);
  }
  const others = [...grouped.entries()]
    .sort(([left], [right]) => genreRank(left) - genreRank(right) || (left ?? "").localeCompare(right ?? ""))
    .map(([genre, items]) => ({ genre, presets: items }));
  return { suggested, others };
}

function genreRank(genre: string | null): number {
  if (!genre) return PRESET_GENRE_ORDER.length;
  const index = PRESET_GENRE_ORDER.indexOf(genre);
  return index === -1 ? PRESET_GENRE_ORDER.length + 1 : index;
}

export function defaultProfilePresetId(
  presets: readonly LoadedProfilePreset[],
  entityType: string | null | undefined,
): string {
  const { suggested } = groupProfilePresets(presets, entityType);
  return (
    suggested.find((preset) => preset.id !== CUSTOM_PRESET_ID)?.id ??
    suggested.find((preset) => preset.id === CUSTOM_PRESET_ID)?.id ??
    presets[0]?.id ??
    ""
  );
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
