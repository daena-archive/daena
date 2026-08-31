export type LabelManifest = {
  id?: string;
  schemas?: ReadonlyArray<{
    entityTypes?: ReadonlyArray<{ id: string; name: string }>;
  }>;
};

export function humanizeTypeId(type: string): string {
  const short = type.includes(":") ? (type.split(":").pop() ?? type) : type;
  const spaced = short
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/[-_]+/g, " ")
    .trim()
    .replace(/\s+/g, " ");
  if (!spaced) return short || type;
  return spaced.replace(/\b\w/g, (char) => char.toUpperCase());
}

export function buildTypeLabelMap(manifests: ReadonlyArray<LabelManifest>): Map<string, string> {
  const map = new Map<string, string>();
  for (const manifest of manifests) {
    for (const schema of manifest.schemas ?? []) {
      for (const entityType of schema.entityTypes ?? []) {
        if (!entityType.id || !entityType.name) continue;
        map.set(entityType.id, entityType.name);
        if (manifest.id && !entityType.id.includes(":")) {
          map.set(`${manifest.id}:${entityType.id}`, entityType.name);
        } else if (entityType.id.includes(":")) {
          const short = entityType.id.split(":").pop() ?? entityType.id;
          if (short && !map.has(short)) map.set(short, entityType.name);
        }
      }
    }
  }
  return map;
}

export function resolveEntityTypeLabel(type: string | null | undefined, labels: ReadonlyMap<string, string>): string {
  if (!type) return "Unknown type";
  const labeled = labels.get(type);
  if (labeled) return labeled;
  if (type.includes(":")) {
    const short = type.split(":").pop() ?? type;
    const shortLabeled = labels.get(short);
    if (shortLabeled) return shortLabeled;
  }
  return humanizeTypeId(type);
}

export function createEntityTypeLabelResolver(manifests: ReadonlyArray<LabelManifest>) {
  const labels = buildTypeLabelMap(manifests);
  return (type: string | null | undefined) => resolveEntityTypeLabel(type, labels);
}
