import { localizeUi } from "../i18n/ui.ts";
const CAPABILITY_LABELS: Record<string, string> = {
  "entity.read": "Read entities",
  "entity.write": "Create and edit entities",
  "entity.delete": "Delete entities",
  "document.read": "Read documents",
  "document.write": "Save and edit documents",
  "field.read:self": "Read fields in own namespace",
  "field.read:shared": "Read fields other plugins share",
  "field.write:self": "Write fields in own namespace",
  "record.read:self": "Read own records",
  "record.write:self": "Create and edit own records",
  "relationship.read": "Read relationships",
  "relationship.write": "Create and delete relationships",
  "asset.read:self": "Read assets in own namespace",
  "asset.read:shared": "Read shared project assets",
  "asset.write:self": "Update assets in own namespace",
  "asset.register": "Register assets",
  "text.transfer": "Hand large text to the app",
  "search.query": "Search the whole world",
  "schema.overlay": "Customize types and fields",
  "ai.text.generate": "Request text assistance",
  "ai.text.generate-structured": "Request structured text assistance",
  "clipboard.read": "Read the clipboard",
  "clipboard.write": "Write to the clipboard",
};

const CAPABILITY_DESCRIPTIONS: Record<string, string> = {
  "entity.read": "See names, types, and identity of entries in this project.",
  "entity.write": "Create and rename entries. Does not include deleting them.",
  "entity.delete": "Archive entries after you confirm in the host UI.",
  "document.read": "Read prose attached to entries this plugin can see.",
  "document.write": "Create or change prose documents.",
  "field.read:self": "Read structured fields this plugin owns.",
  "field.read:shared": "Read fields another plugin has marked as shared.",
  "field.write:self": "Write structured fields this plugin owns.",
  "record.read:self": "Read record collections this plugin owns.",
  "record.write:self": "Create, update, and delete records this plugin owns.",
  "relationship.read": "See links between entries.",
  "relationship.write": "Create and remove links using registered relationship types.",
  "asset.read:self": "Read metadata for files this plugin owns. Bytes are fetched separately.",
  "asset.read:shared": "Read project-visible files other plugins have shared.",
  "asset.write:self": "Replace or update files this plugin owns.",
  "asset.register": "Add a plugin-supplied file into this plugin’s namespace.",
  "text.transfer": "Send text too large for one message to the app, which decides whether to save it.",
  "search.query": "Run searches across the open project.",
  "schema.overlay": "Let authors customize this plugin’s types, fields, and templates.",
  "ai.text.generate": "Ask the host to generate unstructured text. Does not grant data or network access.",
  "ai.text.generate-structured": "Ask the host to generate structured text. Does not grant data or network access.",
  "clipboard.read": "Read clipboard contents through the host.",
  "clipboard.write": "Write clipboard contents through the host.",
};

function prefixedLabel(capability: string, prefix: string, format: (rest: string) => string) {
  if (!capability.startsWith(prefix)) return null;
  const rest = capability.slice(prefix.length).trim();
  return rest ? format(rest) : null;
}

export function capabilityLabel(capability: string) {
  const exact = CAPABILITY_LABELS[capability];
  if (exact) return localizeUi(exact);
  return (
    prefixedLabel(capability, "event.publish:", (type) => localizeUi("Publish {0} events", type)) ??
    prefixedLabel(capability, "event.subscribe:", (type) => localizeUi("Subscribe to {0} events", type)) ??
    prefixedLabel(capability, "host.surface:", (surface) => localizeUi("Use the {0} host surface", surface)) ??
    prefixedLabel(capability, "service.provide:", (name) => localizeUi("Provide the {0} service", name)) ??
    prefixedLabel(capability, "service.call:", (name) => localizeUi("Call the {0} service", name)) ??
    prefixedLabel(capability, "network:", (origin) => localizeUi("Make HTTPS requests to {0}", origin)) ??
    capability
  );
}

export function capabilityDescription(capability: string) {
  const exact = CAPABILITY_DESCRIPTIONS[capability];
  if (exact) return localizeUi(exact);
  return (
    prefixedLabel(
      capability,
      "event.publish:",
      (type) => localizeUi("Send {0} notifications to other plugins in this session.", type),
    ) ??
    prefixedLabel(
      capability,
      "event.subscribe:",
      (type) => localizeUi("Receive {0} notifications from the host or other plugins.", type),
    ) ??
    prefixedLabel(
      capability,
      "host.surface:",
      (surface) => localizeUi("Open the host-owned {0} surface. The plugin cannot implement this surface itself.", surface),
    ) ??
    prefixedLabel(capability, "service.provide:", (name) => localizeUi("Register as the provider for {0} in this project.", name)) ??
    prefixedLabel(capability, "service.call:", (name) => localizeUi("Call the {0} service through the host broker.", name)) ??
    prefixedLabel(
      capability,
      "network:",
      (origin) => localizeUi("Send brokered HTTPS requests only to {0}. No other network access is included.", origin),
    ) ??
    localizeUi("This plugin requested this capability.")
  );
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
    return Number.isFinite(ms) && ms > 0 && !Number.isNaN(date.getTime()) ? date.toLocaleString() : localizeUi("Unknown");
  } catch {
    return localizeUi("Unknown");
  }
}
