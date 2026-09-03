const ALLOWED_SCHEMES = new Set(["http", "https", "ftp", "ftps", "mailto", "tel", "callto", "sms", "cid", "xmpp"]);

export function looksLikeWebUrl(value: string) {
  const candidate = value.trim();
  if (!candidate || /\s/.test(candidate)) return false;
  if (/^https?:\/\//i.test(candidate) || /^www\./i.test(candidate)) return true;
  return /^[\w.-]+\.[a-z]{2,}([/:?#].*)?$/i.test(candidate);
}

export function normalizeHref(raw: string): string | null {
  const value = raw.trim();
  if (!value) return null;
  if (/[\u0000-\u001f\u007f]/.test(value)) return null;
  const scheme = value.match(/^([a-z][a-z0-9+.-]*):/i)?.[1]?.toLowerCase();
  if (scheme) return ALLOWED_SCHEMES.has(scheme) ? value : null;
  if (looksLikeWebUrl(value)) return `https://${value.replace(/^\/\//, "")}`;
  return value;
}
