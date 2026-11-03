import type { EditorHost } from "../../../../src/lib/editor/host.ts";

/** Broker method that opens a read of a project-scoped (`reference_scope: "project"`) asset. */
export const SHARED_ASSET_READ_METHOD = "asset.shared.read.begin";

/** Result of `asset.shared.read.begin`, the same shape as `asset.read.begin`. */
export interface SharedAssetReadBegin {
  handle?: string;
  url: string;
  size?: number;
  contentHash?: string;
  mimeType?: string;
  revision?: string;
}

export interface EditorHostRpc {
  call<T>(method: string, payload: unknown): Promise<T>;
}

export interface BrokerEditorHostOptions {
  rpc: EditorHostRpc;
  /** GET for the begin result's URL; the plugin webview passes `window.fetch`. */
  fetch: (url: string) => Promise<Response>;
  /** Asks the shell to open an external link (the shell checks the scheme). */
  openLink: (href: string) => void;
}

/**
 * The editor host for the isolated Language webview. Asset bytes come from the
 * broker's shared-asset read, never from Tauri IPC, and external links are
 * handed to the shell through the state channel.
 */
export function createBrokerEditorHost(options: BrokerEditorHostOptions): EditorHost {
  const mimeByPath = new Map<string, { mime_type?: string }>();

  async function beginRead(path: string): Promise<SharedAssetReadBegin | null> {
    try {
      const begun = await options.rpc.call<SharedAssetReadBegin | null>(SHARED_ASSET_READ_METHOD, { path });
      if (!begun || typeof begun.url !== "string" || begun.url.length === 0) return null;
      mimeByPath.set(path, { mime_type: begun.mimeType });
      return begun;
    } catch {
      return null;
    }
  }

  return {
    async readAssetBytesByPath(path) {
      const begun = await beginRead(path);
      if (!begun) return null;
      try {
        const response = await options.fetch(begun.url);
        if (!response.ok) return null;
        return new Uint8Array(await response.arrayBuffer());
      } catch {
        return null;
      }
    },
    async getAssetByPath(path) {
      const cached = mimeByPath.get(path);
      if (cached) return cached;
      const begun = await beginRead(path);
      return begun ? (mimeByPath.get(path) ?? null) : null;
    },
    async openExternal(href) {
      options.openLink(href);
    },
    assets: null,
  };
}
