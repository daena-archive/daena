import {
  DEFAULT_COLLECTION_QUERY,
  workspaceModuleId,
  type CollectionQuery,
  type WorkspaceSection,
} from "$lib/modules/workspace";

export type WorkbenchPane = "collection" | "content" | "inspector";
export type WorkbenchLayoutPrefs = {
  visibility: Record<WorkbenchPane, boolean>;
  collectionWidth: number;
  inspectorWidth: number;
};
export const collectionPaneMin = 190;
export const collectionPaneMax = 380;
export const collectionPaneDefault = 245;
export const inspectorPaneMin = 230;
export const inspectorPaneMax = 440;
export const inspectorPaneDefault = 290;

export function workbenchLayoutStorageKey(sec: WorkspaceSection) {
  return `daena:workbench-layout:${workspaceModuleId(sec)}`;
}

export function clampWorkbenchPaneWidth(value: number, min: number, max: number, fallback: number) {
  return Number.isFinite(value) && value > 0 ? Math.max(min, Math.min(max, Math.round(value))) : fallback;
}

export function loadWorkbenchLayout(sec: WorkspaceSection): WorkbenchLayoutPrefs {
  try {
    const raw = localStorage.getItem(workbenchLayoutStorageKey(sec));
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<WorkbenchLayoutPrefs> & {
        visibility?: Partial<Record<WorkbenchPane, boolean>>;
      };
      return {
        visibility: {
          collection: parsed.visibility?.collection !== false,
          content: parsed.visibility?.content !== false,
          inspector: parsed.visibility?.inspector !== false,
        },
        collectionWidth: clampWorkbenchPaneWidth(
          Number(parsed.collectionWidth),
          collectionPaneMin,
          collectionPaneMax,
          collectionPaneDefault,
        ),
        inspectorWidth: clampWorkbenchPaneWidth(
          Number(parsed.inspectorWidth),
          inspectorPaneMin,
          inspectorPaneMax,
          inspectorPaneDefault,
        ),
      };
    }
  } catch {
    // Fall through to legacy global keys / defaults.
  }
  return {
    visibility: {
      collection: localStorage.getItem("daena:workbench-pane:collection") !== "false",
      content: localStorage.getItem("daena:workbench-pane:content") !== "false",
      inspector: localStorage.getItem("daena:workbench-pane:inspector") !== "false",
    },
    collectionWidth: clampWorkbenchPaneWidth(
      Number(localStorage.getItem("daena:workbench-pane-width:collection")),
      collectionPaneMin,
      collectionPaneMax,
      collectionPaneDefault,
    ),
    inspectorWidth: clampWorkbenchPaneWidth(
      Number(localStorage.getItem("daena:workbench-pane-width:inspector")),
      inspectorPaneMin,
      inspectorPaneMax,
      inspectorPaneDefault,
    ),
  };
}

export function saveWorkbenchLayout(sec: WorkspaceSection, layout: WorkbenchLayoutPrefs) {
  try {
    localStorage.setItem(workbenchLayoutStorageKey(sec), JSON.stringify(layout));
  } catch {
    // Ignore quota / private-mode failures; in-session layout still works.
  }
}

export function loadCollectionQuery(sec: WorkspaceSection): CollectionQuery {
  try {
    const raw = localStorage.getItem(`daena:collection-query:${sec}`);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        ...DEFAULT_COLLECTION_QUERY,
        ...parsed,
        pageSize: [25, 50, 100].includes(parsed.pageSize) ? parsed.pageSize : DEFAULT_COLLECTION_QUERY.pageSize,
        section: sec,
        excludedTypes: parsed.excludedTypes ?? [],
      };
    }
  } catch {}
  return {
    ...DEFAULT_COLLECTION_QUERY,
    section: sec,
    viewMode: sec === "language" ? "flat" : DEFAULT_COLLECTION_QUERY.viewMode,
  };
}
