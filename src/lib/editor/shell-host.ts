import { openUrl } from "@tauri-apps/plugin-opener";
import { project } from "$lib/project/client";
import { setDefaultEditorHost, type EditorAssetServices, type EditorHost } from "$lib/editor/host";
import { mimeTypeFor } from "$lib/editor/image-attrs";
import {
  toAsyncEntityPage,
  toShellSortDirection,
  toShellSortField,
  type AsyncEntitySearchFn,
} from "$lib/entity-lifecycle/asyncEntityQuery.ts";

/** Main-webview implementation of the editor host, backed by Tauri IPC and the project client. */
const shellAssets: EditorAssetServices = {
  listAssets: (entityId) => project.listAssets(entityId),
  listSharedAssets: () => project.listSharedAssets(),
  readAssetBytes: async (asset) => Uint8Array.from(await project.readAssetBytes(asset.id)),
  updateAssetMetadata: (assetId, update, expectedRevision) =>
    project.updateAssetMetadata(assetId, update, expectedRevision),
  pickAndRegisterAsset: async ({ entityId, namespace }) => {
    const selection = await project.pickFile();
    const source = typeof selection === "string" ? selection : null;
    if (!source) return null;
    const filename = source.split(/[\\/]/).pop() ?? "asset";
    const asset = await project.registerAssetFile({
      entity_id: entityId,
      namespace,
      source_path: source,
      filename,
      mime_type: mimeTypeFor(filename),
    });
    return { asset, filename };
  },
};

/** Same query and paging as the shell's entity pickers, without a field-specific entity type. */
const shellSearchEntities: AsyncEntitySearchFn = async (query) => {
  const page = await project.queryEntities({
    query: query.text || undefined,
    entityTypes: query.entityTypes,
    excludedEntityTypes: query.excludedEntityTypes,
    sortField: toShellSortField(query.sortField),
    sortDirection: toShellSortDirection(query.sortDirection),
    offset: query.offset,
    limit: query.limit,
  });
  return toAsyncEntityPage(page, { excludeIds: query.excludeIds });
};

/** Names of the entities with these ids. A lookup that fails is left out rather than failing the others. */
async function shellResolveEntityNames(ids: string[]): Promise<Record<string, string>> {
  const found = await Promise.all(ids.map((id) => project.getEntity(id).catch(() => null)));
  const names: Record<string, string> = {};
  for (const entity of found) {
    if (entity && !entity.deleted) names[entity.id] = entity.name;
  }
  return names;
}

async function openExternal(href: string): Promise<void> {
  const isTauri = typeof window !== "undefined" && ((window as any).__TAURI__ || (window as any).__TAURI_INTERNALS__);
  if (isTauri) {
    try {
      await openUrl(href);
      return;
    } catch (e) {
      console.warn("openUrl failed, fallback to window.open", e);
    }
    try {
      window.open(href, "_blank", "noopener,noreferrer");
      return;
    } catch (e) {
      console.warn("window.open fallback failed", e);
    }
  } else {
    try {
      const w = window.open(href, "_blank", "noopener,noreferrer");
      if (w) return;
    } catch (e) {
      console.warn("window.open failed", e);
    }
    try {
      await openUrl(href);
      return;
    } catch (e) {
      console.warn("openUrl fallback failed", e);
    }
    try {
      const a = document.createElement("a");
      a.href = href;
      a.target = "_blank";
      a.rel = "noopener noreferrer";
      document.body.appendChild(a);
      a.click();
      a.remove();
    } catch (e) {
      console.warn("anchor fallback failed", e);
    }
  }
}

export const shellEditorHost: EditorHost = {
  readAssetBytesByPath: async (path) => {
    const bytes = await project.readAssetBytesByPath(path);
    return bytes ? Uint8Array.from(bytes) : null;
  },
  getAssetByPath: (path) => project.getAssetByPath(path),
  openExternal,
  openEntity: null,
  resolveEntityNames: shellResolveEntityNames,
  assets: shellAssets,
  searchEntities: shellSearchEntities,
};

/**
 * Sets the function that opens an entity for editor links, or clears it with null. The route calls
 * this once its selection state exists; until then editors in the main webview offer no Open action.
 */
export function setShellEntityOpener(opener: ((entityId: string) => void) | null): void {
  shellEditorHost.openEntity = opener;
}

/** Makes the shell host the default for every editor surface in the main webview. Call once at startup. */
export function installShellEditorHost(): void {
  setDefaultEditorHost(shellEditorHost);
}
