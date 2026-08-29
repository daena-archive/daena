import { RECENT_ROOT_LIMIT } from "./model.ts";

const recentByProject = new Map<string, string[]>();

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
