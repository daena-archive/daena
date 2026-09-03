import type { ExternalImportPageItem, StagedObject } from "$lib/project/client";

export function displayError(cause: unknown): string {
  const message = cause instanceof Error ? cause.message : String(cause);
  return message.replace(/^external_import\.[^:]+:\s*/, "");
}

export function pageItemTitle(item: ExternalImportPageItem): string {
  if (item.kind === "object") return item.value.title;
  if (item.kind === "asset") return item.value.filename;
  if (item.kind === "unsupported") return item.value.source_path;
  return item.value.message;
}

export function pageItemSubtitle(item: ExternalImportPageItem): string {
  if (item.kind === "object" || item.kind === "asset" || item.kind === "unsupported") {
    return item.value.source_path;
  }
  return item.value.source_path ?? item.value.code;
}

export function stagedDocumentFormat(object: StagedObject): string {
  const sourceFormat = object.metadata?.document_format;
  return typeof sourceFormat === "string" ? sourceFormat : (object.body?.format ?? object.source_kind);
}

export function relationshipSources(object: StagedObject): string[] {
  return [
    ...new Set(
      (object.links ?? [])
        .filter((link) => link.resolution === "resolved" && (link.kind === "internal" || link.kind === "embed"))
        .map((link) => link.kind),
    ),
  ].sort();
}

export function previewValue(value: unknown): string {
  const rendered = typeof value === "string" ? value : JSON.stringify(value);
  if (!rendered) return "";
  return rendered.length > 90 ? `${rendered.slice(0, 87)}…` : rendered;
}
