import { RECENT_ROOT_LIMIT, clampTreeLimits, DEFAULT_TREE_LIMITS, type TreeLimits } from "./model.ts";

const recentByProject = new Map<string, string[]>();
const LIMITS_STORAGE_KEY = "daena:houses.tree.limits";

export function rememberRecentRoot(projectId: string, rootId: string): string[] {
  if (!projectId || !rootId) return recentRoots(projectId);
  const existing = recentByProject.get(projectId) ?? [];
  const next = [rootId, ...existing.filter((id) => id !== rootId)].slice(0, RECENT_ROOT_LIMIT);
  recentByProject.set(projectId, next);
  return next;
}

export function recentRoots(projectId: string): string[] {
  return [...(recentByProject.get(projectId) ?? [])];
}

export function replaceRecentRoots(projectId: string, ids: string[]): string[] {
  const next = [...new Set(ids)].slice(0, RECENT_ROOT_LIMIT);
  if (!projectId) return next;
  recentByProject.set(projectId, next);
  return next;
}

export function readTreeLimits(storage: Pick<Storage, "getItem"> | null = defaultStorage()): TreeLimits {
  if (!storage) return { ...DEFAULT_TREE_LIMITS };
  try {
    const raw = storage.getItem(LIMITS_STORAGE_KEY);
    if (!raw) return { ...DEFAULT_TREE_LIMITS };
    const parsed = JSON.parse(raw) as Partial<TreeLimits>;
    return clampTreeLimits(parsed);
  } catch {
    return { ...DEFAULT_TREE_LIMITS };
  }
}

export function writeTreeLimits(
  limits: Partial<TreeLimits>,
  storage: Pick<Storage, "setItem"> | null = defaultStorage(),
): TreeLimits {
  const next = clampTreeLimits(limits);
  if (!storage) return next;
  try {
    storage.setItem(LIMITS_STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* ignore quota / private-mode failures */
  }
  return next;
}

function defaultStorage(): Pick<Storage, "getItem" | "setItem"> | null {
  try {
    if (typeof localStorage === "undefined") return null;
    return localStorage;
  } catch {
    return null;
  }
}
