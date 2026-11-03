import type { Asset } from "$lib/project/types";

/**
 * Project asset operations the rich-text editor needs in order to browse,
 * upload, share and preview assets. Hosts that cannot offer assets pass
 * `assets: null` on the EditorHost, which hides the insert/replace UI.
 */
export interface EditorAssetServices {
  listAssets(entityId: string): Promise<Asset[]>;
  listSharedAssets(): Promise<Asset[]>;
  readAssetBytes(assetId: string): Promise<Uint8Array>;
  updateAssetMetadata(
    assetId: string,
    update: { referenceScope?: "entity" | "project" },
    expectedRevision: string,
  ): Promise<Asset>;
  pickFile(): Promise<string | string[] | null>;
  registerAssetFile(input: {
    entity_id: string;
    namespace: string;
    source_path: string;
    filename: string;
    mime_type: string;
  }): Promise<Asset>;
}

/**
 * Everything the editor may ask of its surrounding host. The editor itself
 * must not reach Tauri or the trusted project client directly.
 */
export interface EditorHost {
  /** Bytes for a portable `assets/...` path, or null when unavailable. */
  readAssetBytesByPath(path: string): Promise<Uint8Array | null>;
  /** Asset metadata for a portable `assets/...` path (only mime_type is read). */
  getAssetByPath(path: string): Promise<{ mime_type?: string } | null>;
  /** Opens an already-normalised http(s)/mailto/ftp URL outside the app. */
  openExternal(href: string): Promise<void>;
  /** Asset services, or null when the host offers no assets. */
  assets: EditorAssetServices | null;
}

let defaultHost: EditorHost | null = null;

/** Installs the host used by modules that have no explicit host (e.g. resolve.ts). */
export function setDefaultEditorHost(host: EditorHost | null): void {
  defaultHost = host;
}

export function defaultEditorHost(): EditorHost | null {
  return defaultHost;
}
