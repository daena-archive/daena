import { fromStore } from "svelte/store";
import { displayLanguage, type DisplayLanguage } from "./index.ts";
import ja from "./profile-ja.json" with { type: "json" };
import { BUNDLED_PROFILE_PRESET_SEEDS, canonicalPresetOrigin } from "../lore/profilePresetSeed.ts";

const catalogs: Record<DisplayLanguage, Readonly<Record<string, string>>> = { en: {}, ja };
const language = fromStore(displayLanguage);
const seeds = new Map(BUNDLED_PROFILE_PRESET_SEEDS.map((seed) => [seed.id, seed]));

function translate(value: string) {
  const catalog = catalogs[language.current];
  return Object.hasOwn(catalog, value) ? catalog[value] : value;
}

export function presetCopy(origin: string | undefined, field: "name" | "genre" | "description", value: string) {
  const seed = origin && seeds.get(canonicalPresetOrigin(origin));
  return seed && seed[field] === value ? translate(value) : value;
}

export function profileComponentCopy(origin: string | undefined, id: string, value: string) {
  const seed = origin && seeds.get(canonicalPresetOrigin(origin));
  const component = seed && seed.components.find((candidate) => candidate.id === id);
  return component && (component.name === value || component.scale?.includes(value)) ? translate(value) : value;
}
