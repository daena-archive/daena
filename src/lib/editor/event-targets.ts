export function getExternalLinkAnchor(target: EventTarget | null): HTMLAnchorElement | null {
  if (!target) return null;
  const el = target as HTMLElement;
  if ((el as any)?.closest) {
    const found = (el as HTMLElement).closest<HTMLAnchorElement>("a[href]:not([data-entity-id])");
    if (found) return found;
  }
  const parent = (target as any)?.parentElement as HTMLElement | null;
  if (parent?.closest) return parent.closest<HTMLAnchorElement>("a[href]:not([data-entity-id])");
  return null;
}

export function getSpoilerEl(target: EventTarget | null): HTMLElement | null {
  if (!target) return null;
  const el = target as HTMLElement;
  if ((el as any)?.closest) {
    const found = (el as HTMLElement).closest<HTMLElement>("span[data-spoiler]");
    if (found) return found;
  }
  const parent = (target as any)?.parentElement as HTMLElement | null;
  return parent?.closest?.("span[data-spoiler]") ?? null;
}
