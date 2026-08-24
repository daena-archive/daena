export function isEmptyFieldValue(value: unknown): boolean {
  return (
    value === undefined ||
    value === null ||
    value === "" ||
    (typeof value === "string" && !value.trim()) ||
    (Array.isArray(value) && value.length === 0)
  );
}

export function shouldPersistFieldValue(value: unknown, exists: boolean): boolean {
  return exists || !isEmptyFieldValue(value);
}

export function isStructuredFieldValue(value: unknown): boolean {
  return value !== null && typeof value === "object";
}

export function restoreStructuredFieldValue(value: unknown, wasStructured: boolean, label: string): unknown {
  if (!wasStructured || typeof value !== "string") return value;

  let parsed: unknown;
  try {
    parsed = JSON.parse(value);
  } catch {
    throw new Error(`${label} must contain valid JSON.`);
  }

  if (!isStructuredFieldValue(parsed)) {
    throw new Error(`${label} must contain a JSON object or array.`);
  }
  return parsed;
}
