export function capabilityLabel(capability: string) {
  const labels: Record<string, string> = {
    "entity.read": "Read entities",
    "entity.write": "Create and edit entities",
    "entity.delete": "Delete entities",
    "document.read": "Read documents",
    "document.write": "Save and edit documents",
    "relationship.read": "Read relationships",
    "relationship.write": "Create and delete relationships",
    "search.query": "Search the whole world",
    "asset.register": "Register assets",
    "asset.read:self": "Read assets in own namespace",
    "field.read:self": "Read fields in own namespace",
    "field.write:self": "Write fields in own namespace",
  };
  return labels[capability] ?? capability;
}

export function shortDigest(digest: string) {
  return digest ? digest.slice(0, 12) : "";
}

export function installedAtLabel(timestamp: number) {
  return timestamp ? new Date(timestamp * 1000).toLocaleString() : "";
}

export function runtimeTimestampLabel(timestamp: string) {
  try {
    const ms = Number(BigInt(timestamp) / 1_000_000n);
    const date = new Date(ms);
    return Number.isFinite(ms) && ms > 0 && !Number.isNaN(date.getTime()) ? date.toLocaleString() : "Unknown";
  } catch {
    return "Unknown";
  }
}
