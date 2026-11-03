import type { Asset } from "$lib/project/types";
import type { AsyncEntitySearchFn } from "$lib/entity-lifecycle/asyncEntityQuery.ts";

/**
 * Project asset operations the rich-text editor needs in order to browse,
 * upload, share and preview assets. Hosts that cannot offer assets pass
 * `assets: null` on the EditorHost, which hides the insert/replace UI.
 */
export interface EditorAssetServices {
  listAssets(entityId: string): Promise<Asset[]>;
  listSharedAssets(): Promise<Asset[]>;
  /** Bytes of an asset the dialog listed (own or shared). */
  readAssetBytes(asset: Asset): Promise<Uint8Array>;
  updateAssetMetadata(
    assetId: string,
    update: { referenceScope?: "entity" | "project" },
    expectedRevision: string,
  ): Promise<Asset>;
  /** Lets the user choose a file and registers it on `entityId` in `namespace`.
   *  Resolves null when the user cancels. */
  pickAndRegisterAsset(input: {
    entityId: string;
    namespace: string;
  }): Promise<{ asset: Asset; filename: string } | null>;
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
  /** Backend-paged entity search for entity links and mentions, or null when the host cannot search entities. */
  searchEntities: AsyncEntitySearchFn | null;
}

let defaultHost: EditorHost | null = null;

/** Installs the host used by modules that have no explicit host (e.g. resolve.ts). */
export function setDefaultEditorHost(host: EditorHost | null): void {
  defaultHost = host;
}

export function defaultEditorHost(): EditorHost | null {
  return defaultHost;
}
