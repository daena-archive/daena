export const EPOCH_MIN = -100_000;
export const EPOCH_MAX = 100_000;
export const EPOCH_STEP = 10;

export function formatEpoch(offset: number): string {
  if (offset === 0) return "Reference epoch";
  return `${offset > 0 ? "+" : ""}${offset.toLocaleString()} years`;
}

export function clampEpoch(offset: number, step = 1) {
  const snapped = step > 1 ? Math.round(offset / step) * step : Math.round(offset);
  return Math.min(EPOCH_MAX, Math.max(EPOCH_MIN, snapped));
}

export function parseEpochYears(raw: string) {
  const digits = raw.replace(/[^\d]/g, "");
  const value = digits ? Number(digits) : 0;
  return Math.min(EPOCH_MAX, value);
}
