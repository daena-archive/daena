import { openUrl } from "@tauri-apps/plugin-opener";
import { project } from "$lib/project/client";
import { setDefaultEditorHost, type EditorAssetServices, type EditorHost } from "$lib/editor/host";

/** Main-webview implementation of the editor host, backed by Tauri IPC and the project client. */
const shellAssets: EditorAssetServices = {
  listAssets: (entityId) => project.listAssets(entityId),
  listSharedAssets: () => project.listSharedAssets(),
  readAssetBytes: async (assetId) => Uint8Array.from(await project.readAssetBytes(assetId)),
  updateAssetMetadata: (assetId, update, expectedRevision) =>
    project.updateAssetMetadata(assetId, update, expectedRevision),
  pickFile: () => project.pickFile(),
  registerAssetFile: (input) => project.registerAssetFile(input),
};

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
  assets: shellAssets,
};

/** Makes the shell host the default for every editor surface in the main webview. Call once at startup. */
export function installShellEditorHost(): void {
  setDefaultEditorHost(shellEditorHost);
}
