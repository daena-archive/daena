const HEX = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i;

export function normalizeHexColor(value: string): string | null {
  const match = value.trim().match(HEX);
  if (!match) return null;
  let hex = match[1].toLowerCase();
  if (hex.length === 3) hex = [...hex].map((unit) => unit + unit).join("");
  return `#${hex}`;
}

export function colorFromStyle(style: string): string | null {
  const match = String(style).match(/(?:^|;)\s*color\s*:\s*([^;]+)/i);
  if (!match) return null;
  return normalizeHexColor(match[1]);
}

export function sanitizeInlineStyle(style: string): string | null {
  const kept: string[] = [];
  for (const part of String(style)
    .split(";")
    .map((item) => item.trim())
    .filter(Boolean)) {
    if (/^text-align\s*:\s*(?:left|center|right)$/i.test(part)) {
      const align = part.replace(/^text-align\s*:\s*/i, "").toLowerCase();
      kept.push(`text-align: ${align}`);
      continue;
    }
    const colorMatch = part.match(/^color\s*:\s*(.+)$/i);
    if (colorMatch) {
      const color = normalizeHexColor(colorMatch[1]);
      if (color) kept.push(`color: ${color}`);
    }
  }
  return kept.length ? kept.join("; ") : null;
}
