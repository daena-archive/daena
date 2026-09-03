import type { GuideMode, GuideStep } from "./types.ts";

export const LORE_GUIDE_ID = "lore";

const createStep: GuideStep = {
  id: "create",
  title: "Create an entry",
  body: "Click New. A name and type are enough — person, place, or whatever this world needs.",
  target: '[data-guide="workspace-new"]',
  waitForTarget: true,
  action: "pause",
};

const libraryStep: GuideStep = {
  id: "library",
  title: "Library",
  body: "This list is the world bible. Open an entry to write it.",
  target: '[data-guide="workspace-view-library"]',
};

const libraryToInspector: GuideStep = {
  ...libraryStep,
  primaryLabel: "Show inspector",
  action: "inspector",
};

const libraryToWiki: GuideStep = {
  ...libraryStep,
  primaryLabel: "Show Wiki",
  action: "wiki",
};

const inspectorStep: GuideStep = {
  id: "inspector",
  title: "Inspector",
  body: "Fields, relationships, and assets for the open entry live here.",
  target: '[data-guide="workspace-inspector"]',
  primaryLabel: "Show Wiki",
  action: "wiki",
};

const wikiStep: GuideStep = {
  id: "wiki",
  title: "Wiki",
  body: "Wiki is a readable page for the same entries — useful for browsing the world.",
  target: '[data-guide="workspace-view-wiki"]',
  primaryLabel: "Show Graph",
  action: "graph",
};

const graphStep: GuideStep = {
  id: "graph",
  title: "Graph",
  body: "Graph shows how entries link. Click a node to open it.",
  target: '[data-guide="workspace-view-graph"]',
  primaryLabel: "Done",
  action: "complete",
};

export function loreGuideSteps(opts: {
  hasCollection: boolean;
  hasSelection: boolean;
  view: string;
  mode: GuideMode;
}): GuideStep[] {
  if (!opts.hasCollection) return [createStep];
  if (opts.mode === "hint") {
    if (opts.view === "wiki") return [wikiStep];
    if (opts.view === "graph") return [graphStep];
    if (opts.hasSelection) return [inspectorStep];
    return [libraryStep];
  }
  return opts.hasSelection
    ? [libraryToInspector, inspectorStep, wikiStep, graphStep]
    : [libraryToWiki, wikiStep, graphStep];
}
