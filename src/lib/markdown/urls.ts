const SAFE_HREF = /^(?:https?:|mailto:|#|daena:\/\/entity\/)/i;
const SAFE_SRC = /^(?:https?:|#|assets\/)/i;

export function safeHref(value: string): string {
  const trimmed = value.trim();
  return SAFE_HREF.test(trimmed) ? trimmed : "";
}

export function safeSrc(value: string): string {
  const trimmed = value.trim();
  return SAFE_SRC.test(trimmed) ? trimmed : "";
}

export function entityIdFromHref(value: string): string | null {
  const match = value.trim().match(/^daena:\/\/entity\/(.+)$/i);
  if (!match) return null;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return match[1];
  }
}
