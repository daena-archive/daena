const LEGACY_KEYS: Record<string, string> = {
  language: "daena-language-tour-completed",
};

export function guideDismissedKey(guideId: string, projectId?: string | null): string {
  return projectId ? `daena-guide:${guideId}:${projectId}` : `daena-guide:${guideId}`;
}

export function isGuideDismissed(guideId: string, projectId?: string | null): boolean {
  try {
    if (globalThis.localStorage?.getItem(guideDismissedKey(guideId, projectId))) return true;
    const legacy = LEGACY_KEYS[guideId];
    return Boolean(legacy && globalThis.localStorage?.getItem(legacy));
  } catch {
    return false;
  }
}

export function dismissGuide(guideId: string, projectId?: string | null): void {
  try {
    globalThis.localStorage?.setItem(guideDismissedKey(guideId, projectId), "true");
  } catch {
    /* ignore quota / private mode */
  }
}
