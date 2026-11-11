export type ProjectionSandboxKey = "lore" | "timeline";

/**
 * Projections drawn as isolated plugin webviews in the content column: the Lore World graph and
 * Chronology. Each webview mounts its package entrypoint, so `viewId` is a shell-only id that the
 * manifests do not declare (validate_plugin_view rejects unlisted ids). That is why SandboxView is
 * mounted without a viewId for these surfaces. The page adds each projection's icon.
 */
export const PROJECTION_SANDBOXES: Record<
  ProjectionSandboxKey,
  {
    moduleName: string;
    pluginId: string;
    viewId: string;
    title: string;
    subtitle: string;
    surfaceKey: string;
  }
> = {
  lore: {
    moduleName: "Lore",
    pluginId: "daena.lore",
    viewId: "graph",
    title: "World graph",
    subtitle: "Explore entities and their connections",
    surfaceKey: "workspace:lore:graph",
  },
  timeline: {
    moduleName: "Timeline",
    pluginId: "daena.timeline",
    viewId: "chronology",
    title: "Chronology",
    subtitle: "Explore your world across time",
    surfaceKey: "workspace:timeline:timeline",
  },
};
