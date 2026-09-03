import type { WorkspaceSection } from "$lib/modules/workspace";

export type ShellBreadcrumb = {
  key: string;
  label: string;
  action?: "section" | "view";
};

export function breadcrumbViewLabel(options: {
  section: WorkspaceSection;
  view: string;
  tabLabel?: string | null;
}): string | null {
  const { section, view, tabLabel } = options;
  if (section === "lore") {
    if (view === "wiki") return "Wiki";
    if (view === "graph") return "Graph";
    return null;
  }
  if (section === "houses") return view === "tree" ? "Tree" : null;
  if (section === "writing" || section === "timeline") {
    if (view === "timeline") return null;
    const label = tabLabel?.trim() ?? "";
    return label || null;
  }
  return null;
}

export function shellBreadcrumbs(options: {
  home?: boolean;
  settingsLabel?: string | null;
  sectionLabel: string;
  viewLabel?: string | null;
  entityName?: string | null;
  pluginLabel?: string | null;
}): ShellBreadcrumb[] {
  if (options.home) return [{ key: "home", label: "Home" }];
  if (options.settingsLabel) return [{ key: "settings", label: options.settingsLabel }];
  const items: ShellBreadcrumb[] = [{ key: "section", label: options.sectionLabel, action: "section" }];
  if (options.pluginLabel) {
    items.push({ key: "plugin", label: options.pluginLabel });
    return items;
  }
  if (options.viewLabel) items.push({ key: "view", label: options.viewLabel, action: "view" });
  if (options.entityName) items.push({ key: "entity", label: options.entityName });
  return items;
}
