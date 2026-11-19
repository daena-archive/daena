import { localizeUi } from "../i18n/ui.ts";
import type { Entity, GitChange } from "$lib/project/client";

export type ChangeGroup = {
  id: string;
  kind: "entity" | "project" | "plugin" | "asset";
  title: string;
  subtitle: string;
  paths: string[];
};
export type SnapshotChangeGroup = {
  id: string;
  label: string;
  changes: GitChange[];
  kind: "added" | "modified" | "deleted" | "other";
};

export function friendly(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

export function snapshotMessageTitle(value: string) {
  return value.replaceAll("\r\n", "\n").split("\n")[0]?.trim() ?? "";
}

export function snapshotMessageBody(value: string) {
  return value.replaceAll("\r\n", "\n").split("\n").slice(1).join("\n").trim();
}

export function formatSnapshotMessage(value: string) {
  const lines = value.replaceAll("\r\n", "\n").split("\n");
  const title = lines.shift()?.trim() ?? "";
  const comments = lines.join("\n").trim();
  return comments ? `${title}\n\n${comments}` : title;
}

export function entityFileRole(path: string): string | null {
  const relative = path.replace(/^entities\/[^/]+\//, "");
  if (relative === "entity.json") return "Identity";
  if (relative === "document.md") return "Document";
  if (relative === "relationships.json") return "Relationships";
  if (relative === "assets.json") return "Asset index";
  if (relative.startsWith("fields/")) return "Fields";
  return null;
}

export function summarizeRoles(paths: string[], translate = localizeUi): string {
  const roles = [...new Set(paths.map(entityFileRole).filter((role): role is string => Boolean(role)))];
  return roles.length > 0 ? roles.map((role) => translate(role)).join(" · ") : translate(paths.length === 1 ? "{0} file" : "{0} files", paths.length);
}

export function changeStatus(status: string) {
  return status.slice(0, 1).toUpperCase();
}

export function changeKind(status: string): "added" | "modified" | "deleted" {
  const code = changeStatus(status);
  return code === "A" ? "added" : code === "D" ? "deleted" : "modified";
}

export function groupSnapshotChanges(changes: GitChange[], entityList: Entity[], translate = localizeUi): SnapshotChangeGroup[] {
  const groups = new Map<string, GitChange[]>();
  const entityNames = new Map(entityList.map((entity) => [entity.id, entity.name]));
  for (const change of changes) {
    const entityId = change.path.startsWith("entities/") ? change.path.split("/")[1] : null;
    const id = entityId ? `entity:${entityId}` : change.path.startsWith("plugins/") ? "Plugins" : change.path.startsWith("assets/") ? "Assets" : "Project";
    const bucket = groups.get(id) ?? [];
    bucket.push(change);
    groups.set(id, bucket);
  }
  return [...groups.entries()].map(([id, changes]) => {
    const entityId = id.startsWith("entity:") ? id.slice("entity:".length) : null;
    const name = entityId ? entityNames.get(entityId) : undefined;
    const identity = changes.find((change) => change.path.endsWith("/entity.json"));
    return {
      id,
      label: entityId ? (name ?? translate("Deleted entity ({0})", shortId(entityId))) : translate(id),
      changes,
      kind: !entityId ? "other" : name === undefined ? "deleted" : identity ? changeKind(identity.status) : "modified",
    };
  });
}

export function snapshotChangeLabel(path: string) {
  if (!path.startsWith("entities/")) return path;
  const [, , ...parts] = path.split("/");
  const relative = parts.join("/");
  const fileLabel =
    relative === "document.md"
      ? localizeUi("Document")
      : relative === "relationships.json"
        ? localizeUi("Relationships")
        : relative === "assets.json"
          ? localizeUi("Asset links")
          : relative === "entity.json"
            ? localizeUi("Identity")
            : relative.startsWith("fields/")
              ? localizeUi("Field · {0}", relative.slice("fields/".length))
              : relative;
  return fileLabel;
}

export function diffLineClass(line: string) {
  return line.startsWith("+++") || line.startsWith("---")
    ? "diff-file-header"
    : line.startsWith("+")
      ? "diff-added"
      : line.startsWith("-")
        ? "diff-removed"
        : line.startsWith("@@")
          ? "diff-hunk"
          : "diff-context";
}

export function isDiffMetadata(line: string) {
  return (
    line.startsWith("diff --git ") ||
    line.startsWith("new file mode ") ||
    line.startsWith("deleted file mode ") ||
    line.startsWith("old mode ") ||
    line.startsWith("new mode ") ||
    line.startsWith("similarity index ") ||
    line.startsWith("rename from ") ||
    line.startsWith("rename to ") ||
    line.startsWith("index ") ||
    line.startsWith("--- ") ||
    line.startsWith("+++ ") ||
    line.startsWith("Binary files ")
  );
}

export function shortId(id: string) {
  return id.length > 8 ? id.slice(0, 8) : id;
}

export function snapshotDateLabel(value: string, locale: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString(locale, {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function buildChangeGroups(paths: string[], entityList: Entity[], translate = localizeUi): ChangeGroup[] {
  const byId = new Map(entityList.map((entity) => [entity.id, entity]));
  const entityBuckets = new Map<string, string[]>();
  const pluginBuckets = new Map<string, string[]>();
  const assetBuckets = new Map<string, string[]>();
  const projectPaths: string[] = [];

  for (const path of paths) {
    if (path.startsWith("entities/")) {
      const entityId = path.split("/")[1];
      if (!entityId) continue;
      const bucket = entityBuckets.get(entityId) ?? [];
      bucket.push(path);
      entityBuckets.set(entityId, bucket);
      continue;
    }
    if (path.startsWith("plugins/") && path.endsWith(".json")) {
      const pluginId = path.slice("plugins/".length, -".json".length);
      const bucket = pluginBuckets.get(pluginId) ?? [];
      bucket.push(path);
      pluginBuckets.set(pluginId, bucket);
      continue;
    }
    if (path.startsWith("assets/")) {
      assetBuckets.set(path, [path]);
      continue;
    }
    projectPaths.push(path);
  }

  const groups: ChangeGroup[] = [];

  if (projectPaths.length > 0) {
    groups.push({
      id: "project",
      kind: "project",
      title: translate("Project settings"),
      subtitle: projectPaths
        .map((path) => (path === "project.json" ? translate("Project manifest") : path === ".gitignore" ? translate("Ignore rules") : path))
        .join(" · "),
      paths: projectPaths,
    });
  }

  for (const [entityId, entityPaths] of [...entityBuckets.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    const entity = byId.get(entityId);
    groups.push({
      id: `entity:${entityId}`,
      kind: "entity",
      title: entity?.name ?? translate("Deleted entity ({0})", shortId(entityId)),
      subtitle: [
        entity?.entity_type ?? translate(entity ? "Uncategorized" : "Unknown entity"),
        summarizeRoles(entityPaths, translate),
      ].join(" · "),
      paths: entityPaths,
    });
  }

  for (const [pluginId, pluginPaths] of [...pluginBuckets.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    groups.push({
      id: `plugin:${pluginId}`,
      kind: "plugin",
      title: pluginId,
      subtitle: translate("Plugin config"),
      paths: pluginPaths,
    });
  }

  for (const [assetPath, assetPaths] of [...assetBuckets.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    const parts = assetPath.split("/");
    const filename = parts[parts.length - 1] ?? assetPath;
    const folder = parts[1] ?? "files";
    groups.push({
      id: `asset:${assetPath}`,
      kind: "asset",
      title: filename,
      subtitle: translate("Asset · {0}", folder),
      paths: assetPaths,
    });
  }

  return groups;
}

export function selectedPathsFromGroups(groupIds: string[], groups: ChangeGroup[]) {
  const selected = new Set(groupIds);
  return groups.filter((group) => selected.has(group.id)).flatMap((group) => group.paths);
}
