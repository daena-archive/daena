/**
 * Module schema-overlay compatibility.
 *
 * Overlay-capable modules offer the workbench. Language and Maps stay
 * "Managed by extension" until their specialized surfaces are ready (or forever
 * for Maps provider fields). Houses may author custom Types, but Tree only
 * understands the Person/House contract.
 */

export const HOUSES_PLUGIN_ID = "daena.houses";
export const LANGUAGE_PLUGIN_ID = "daena.language";
export const MAPS_PLUGIN_ID = "daena.maps";
export const LORE_PLUGIN_ID = "daena.lore";
export const TIMELINE_PLUGIN_ID = "daena.timeline";
export const WRITING_PLUGIN_ID = "daena.writing";

/** Tree contract types — the only entity types Houses hydrates as nodes. */
export const TREE_PERSON_TYPE = "daena.lore:person";
export const TREE_HOUSE_TYPE = "daena.houses:house";

/**
 * Language Overview still builds its field list from the packaged manifest
 * (`packages/modules/language/.../Overview.svelte`), not the merged project
 * schema. Keep this false until that workspace renders merged schema; the shell
 * treats Language as managed even if the manifest gains `schema.overlay`.
 */
export const LANGUAGE_SCHEMA_OVERLAY_READY = false;

export type HousesTypeTreeRole = "tree-house" | "collection-only";

/** True when `typeId` is the Tree House contract type (qualified or local). */
export function isTreeCompatibleHouseType(typeId: string): boolean {
  const id = typeId.trim();
  if (!id) return false;
  if (id === TREE_HOUSE_TYPE || id === "house") return true;
  const colon = id.lastIndexOf(":");
  return colon >= 0 && id.slice(colon + 1) === "house" && id.startsWith("daena.houses:");
}

/** True when `typeId` is the Tree Person contract type (qualified or local). */
export function isTreeCompatiblePersonType(typeId: string): boolean {
  const id = typeId.trim();
  if (!id) return false;
  if (id === TREE_PERSON_TYPE || id === "person") return true;
  const colon = id.lastIndexOf(":");
  return colon >= 0 && id.slice(colon + 1) === "person" && id.startsWith("daena.lore:");
}

/**
 * Classify a type in the Houses workbench for Tree vs collection-only display.
 * Only the contract House type is Tree-compatible here; Person and every other
 * id (including `daena.lore:person` if present on a Houses overlay) are
 * collection-only in this editor.
 */
export function housesTypeTreeRole(typeId: string): HousesTypeTreeRole {
  if (isTreeCompatibleHouseType(typeId)) return "tree-house";
  return "collection-only";
}

/**
 * Whether the Fields & Types workbench may open for this plugin.
 * Capability alone is not enough for Language until `LANGUAGE_SCHEMA_OVERLAY_READY`.
 */
export function schemaOverlayWorkbenchAllowed(
  pluginId: string,
  capabilities: readonly string[] | null | undefined,
): boolean {
  if (!(capabilities ?? []).includes("schema.overlay")) return false;
  if (pluginId === LANGUAGE_PLUGIN_ID && !LANGUAGE_SCHEMA_OVERLAY_READY) return false;
  return true;
}

/** Author-facing projection labels for the type editor (§5.3). */
export function projectionLabelsForModuleType(pluginId: string | null | undefined, typeId: string): string[] {
  if (!pluginId || !typeId.trim()) return [];
  if (pluginId === HOUSES_PLUGIN_ID) {
    if (housesTypeTreeRole(typeId) === "tree-house") return ["Houses collection", "Tree"];
    return ["Houses collection only"];
  }
  if (pluginId === LORE_PLUGIN_ID) {
    const labels = ["Library", "Wiki", "Graph"];
    if (isTreeCompatiblePersonType(typeId)) labels.push("Tree");
    return labels;
  }
  if (pluginId === TIMELINE_PLUGIN_ID) return ["Timeline"];
  if (pluginId === WRITING_PLUGIN_ID) return ["Writing Studio"];
  return [];
}

/** Clear reasons for plugins that intentionally omit / block overlay. */
export function managedSchemaPluginReason(pluginId: string): string {
  if (pluginId === MAPS_PLUGIN_ID) {
    return "Maps provider and internal fields stay extension-managed. Author-facing map metadata would use a separate schema later.";
  }
  if (pluginId === LANGUAGE_PLUGIN_ID) {
    return "Language keeps a specialized workspace that still reads packaged fields; overlay stays unavailable until that workspace renders merged schema.";
  }
  return "Schema structure is owned by this extension and is not project-customizable.";
}

/** Plugins expected to declare `schema.overlay` under §5.8. */
export const OVERLAY_EXPECTED_PLUGIN_IDS = [
  LORE_PLUGIN_ID,
  TIMELINE_PLUGIN_ID,
  WRITING_PLUGIN_ID,
  HOUSES_PLUGIN_ID,
] as const;

/** Plugins expected to remain managed (no overlay) under §5.8. */
export const MANAGED_EXPECTED_PLUGIN_IDS = [LANGUAGE_PLUGIN_ID, MAPS_PLUGIN_ID] as const;
