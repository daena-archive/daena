/** Link policy shared by the shell bridges for plugin webviews. Pure logic, no Tauri or Svelte imports. */

const MAX_HREF_LENGTH = 4096;
const OPENABLE_PROTOCOLS: ReadonlySet<string> = new Set(["http:", "https:", "mailto:"]);

/**
 * Returns the normalised href when the shell may open it outside the app
 * (http, https, mailto only), otherwise null.
 */
export function openableLinkHref(href: unknown): string | null {
  if (typeof href !== "string") return null;
  const trimmed = href.trim();
  if (trimmed.length === 0 || trimmed.length > MAX_HREF_LENGTH) return null;
  let url: URL;
  try {
    url = new URL(trimmed);
  } catch {
    return null;
  }
  return OPENABLE_PROTOCOLS.has(url.protocol) ? url.href : null;
}
