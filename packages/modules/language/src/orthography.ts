import type { PhonemeKind } from "./phonology";

export const WRITING_DIRECTIONS = ["ltr", "rtl", "vertical", "unspecified"] as const;
export const CHARACTER_GROUPS = ["ungrouped", "vowels", "consonants", "other"] as const;

export type WritingDirection = (typeof WRITING_DIRECTIONS)[number];
export type CharacterGroup = (typeof CHARACTER_GROUPS)[number];

export type OrthographySound = { kind: "phoneme"; phonemeId: string; symbol: string } | { kind: "ipa"; value: string };

export type OrthographyMapping = {
  id: string;
  writtenForm: string;
  sounds: OrthographySound[];
  romanization?: string;
  notes?: string;
  group: CharacterGroup;
};

export type OrthographySample = {
  id: string;
  writtenText: string;
  pronunciation?: string;
  translation?: string;
  notes?: string;
};

export type OrthographyValue = {
  name: string;
  direction: WritingDirection;
  description?: string;
  mappings: OrthographyMapping[];
  samples: OrthographySample[];
};

export type PhonemeOption = {
  id: string;
  symbol: string;
  ipa?: string;
  kind: PhonemeKind;
};

const TEXT = 500;
const LONG = 4000;
const MAX_MAPPINGS = 512;
const MAX_SOUNDS = 16;
const MAX_SAMPLES = 128;

function id() {
  return crypto.randomUUID();
}

function text(value: unknown, limit = TEXT) {
  return typeof value === "string" ? value.trim().slice(0, limit) : "";
}

function optional(value: unknown, limit = TEXT) {
  return text(value, limit) || undefined;
}

function direction(value: unknown): WritingDirection {
  return WRITING_DIRECTIONS.includes(value as WritingDirection) ? (value as WritingDirection) : "unspecified";
}

function group(value: unknown): CharacterGroup {
  return CHARACTER_GROUPS.includes(value as CharacterGroup) ? (value as CharacterGroup) : "ungrouped";
}

function sound(value: unknown): OrthographySound | null {
  const entry = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  if (entry.kind === "phoneme") {
    const phonemeId = text(entry.phonemeId);
    if (!phonemeId) return null;
    return { kind: "phoneme", phonemeId, symbol: text(entry.symbol) };
  }
  if (entry.kind === "ipa") {
    const ipa = text(entry.value);
    return ipa ? { kind: "ipa", value: ipa } : null;
  }
  return null;
}

export function emptyOrthography(): OrthographyValue {
  return { name: "", direction: "unspecified", mappings: [], samples: [] };
}

export function emptyOrthographyMapping(group: CharacterGroup = "ungrouped"): OrthographyMapping {
  return { id: id(), writtenForm: "", sounds: [], group };
}

export function emptyOrthographySample(): OrthographySample {
  return { id: id(), writtenText: "" };
}

export function normalizeOrthography(value: unknown): OrthographyValue {
  const record = value && typeof value === "object" ? (value as Record<string, unknown>) : {};
  const mappings = Array.isArray(record.mappings)
    ? record.mappings.slice(0, MAX_MAPPINGS).map((item): OrthographyMapping => {
        const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
        return {
          id: text(entry.id) || id(),
          writtenForm: text(entry.writtenForm),
          sounds: (Array.isArray(entry.sounds) ? entry.sounds : [])
            .map(sound)
            .filter((item): item is OrthographySound => item !== null)
            .slice(0, MAX_SOUNDS),
          romanization: optional(entry.romanization),
          notes: optional(entry.notes, LONG),
          group: group(entry.group),
        };
      })
    : [];
  const samples = Array.isArray(record.samples)
    ? record.samples.slice(0, MAX_SAMPLES).map((item): OrthographySample => {
        const entry = item && typeof item === "object" ? (item as Record<string, unknown>) : {};
        return {
          id: text(entry.id) || id(),
          writtenText: text(entry.writtenText, LONG),
          pronunciation: optional(entry.pronunciation, LONG),
          translation: optional(entry.translation, LONG),
          notes: optional(entry.notes, LONG),
        };
      })
    : [];
  return {
    name: text(record.name),
    direction: direction(record.direction),
    description: optional(record.description, LONG),
    mappings,
    samples,
  };
}

export function serializeOrthography(value: OrthographyValue): Record<string, unknown> {
  return normalizeOrthography(value);
}

export function representedPhonemeIds(value: OrthographyValue): Set<string> {
  return new Set(
    value.mappings.flatMap((mapping) =>
      mapping.sounds
        .filter((sound): sound is Extract<OrthographySound, { kind: "phoneme" }> => sound.kind === "phoneme")
        .map((sound) => sound.phonemeId),
    ),
  );
}

export function countPhonemeReferences(value: OrthographyValue, phonemeId: string): number {
  return value.mappings.reduce(
    (count, mapping) =>
      count + mapping.sounds.filter((sound) => sound.kind === "phoneme" && sound.phonemeId === phonemeId).length,
    0,
  );
}

export function orthographyCoverage(value: OrthographyValue, phonemeIds: string[]) {
  const available = new Set(phonemeIds);
  const represented = representedPhonemeIds(value);
  const representedCount = phonemeIds.filter((phonemeId) => represented.has(phonemeId)).length;
  return {
    represented: representedCount,
    total: available.size,
    unmapped: phonemeIds.filter((phonemeId) => !represented.has(phonemeId)),
  };
}

export function mappingFromPhoneme(phoneme: PhonemeOption): OrthographyMapping {
  const mappingGroup: CharacterGroup =
    phoneme.kind === "vowel" ? "vowels" : phoneme.kind === "consonant" ? "consonants" : "other";
  return {
    ...emptyOrthographyMapping(mappingGroup),
    sounds: [{ kind: "phoneme", phonemeId: phoneme.id, symbol: phoneme.ipa || phoneme.symbol }],
  };
}

export function validateOrthography(value: OrthographyValue): string | null {
  if (!value.name.trim()) return "Writing system name is required.";
  if (value.mappings.some((mapping) => !mapping.writtenForm.trim())) {
    return "Every character mapping needs a written form.";
  }
  if (value.samples.some((sample) => !sample.writtenText.trim())) {
    return "Every sample needs written text.";
  }
  return null;
}
