import { workspaceSectionDescription, type WorkspaceSection } from "./workspace.ts";
import type { EntityTypeDefinition, PluginAdminEntry } from "$lib/project/client";

export type NavigationRenderer = "workspace" | "maps" | "host" | "webview";

export const MAP_HOST_SURFACE = "daena.maps/editor";

export function workspaceDescription(target: WorkspaceSection) {
  return workspaceSectionDescription(target);
}

export function schemaEntityTypeIds(schema: { entityTypes: EntityTypeDefinition[] }): string[] {
  return schema.entityTypes.map((entityType) => entityType.id);
}

export function workspaceSectionLabel(target: WorkspaceSection) {
  return target === "lore"
    ? "Lore library"
    : target === "timeline"
      ? "Timeline"
      : target === "writing"
        ? "Writing Studio"
        : target === "language"
          ? "Languages"
          : target === "houses"
            ? "Houses"
            : "Maps";
}

export function viewRenderer(
  plugin: PluginAdminEntry,
  view: PluginAdminEntry["views"][number],
): Exclude<NavigationRenderer, "workspace"> {
  if (view.renderer?.type === "host-surface") {
    if (view.renderer.id === MAP_HOST_SURFACE && view.renderer.major === 1) return "maps";
    return "webview";
  }
  if (view.renderer?.type === "sandboxed") return "webview";
  if (view.renderer?.type === "declarative") return "host";
  return plugin.kind === "sandboxed" ? "webview" : "host";
}
