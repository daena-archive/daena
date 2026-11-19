import { en } from "./en.ts";

function builtInLabel(
  translated: Record<string, string>,
  builtIn: Record<string, string>,
  key: string,
  label: string,
): string {
  return Object.hasOwn(builtIn, key) && builtIn[key] === label ? (translated[key] ?? label) : label;
}

export function loreTypeLabel(translated: typeof en.loreTypes, id: string, label: string): string {
  return builtInLabel(translated, en.loreTypes, id.replace(/^daena\.lore:/, ""), label);
}

export function loreFieldLabel(translated: typeof en.loreFields, key: string, label: string): string {
  return builtInLabel(translated, en.loreFields, key, label);
}
