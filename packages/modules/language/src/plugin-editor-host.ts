import type { EditorAssetServices, EditorHost } from "../../../../src/lib/editor/host.ts";
import { mimeTypeFor } from "../../../../src/lib/editor/image-attrs.ts";
import type { Asset } from "../../../../src/lib/project/types.ts";
import type { AssetReplaceHandle } from "../../../plugin-sdk/src/index.ts";

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

/** The slice of the webview document the file chooser uses. */
export type FileChooserDocument = Pick<Document, "createElement" | "body" | "defaultView">;

export interface BrokerEditorHostOptions {
  rpc: EditorHostRpc;
  /** GET for read URLs; the plugin webview passes `window.fetch`. */
  fetch: (url: string) => Promise<Response>;
  /** Asks the shell to open an external link (the shell checks the scheme). */
  openLink: (href: string) => void;
  /** The asset namespace this host owns. The Language plugin passes `language`. */
  namespace: string;
  /** The entity the webview was launched for. Its own assets back the image fallback in `readAssetBytesByPath`. */
  launchEntityId: string | null;
  /** Sends the bytes of a begun upload. The plugin webview passes the SDK's `uploadAssetChunks`. */
  uploadChunks: (transfer: AssetReplaceHandle, bytes: Uint8Array) => Promise<void>;
  /** Hosts the hidden file input. The plugin webview passes `document`. */
  document: FileChooserDocument;
  /** Hashes the uploaded bytes. The plugin webview passes `crypto`. */
  crypto: Pick<Crypto, "subtle">;
  /** Called after an upload commits or an asset's sharing changes, so the shell refreshes that entity's views. */
  onEntityChanged: (entityId: string) => void;
}

const ENTITY_GET_METHOD = "entity.get";
const ASSET_LIST_METHOD = "asset.list";
const SHARED_ASSET_LIST_METHOD = "asset.shared.list";
const OWN_ASSET_READ_METHOD = "asset.read.begin";
const ASSET_UPDATE_METHOD = "asset.update";
const ASSET_UPLOAD_BEGIN_METHOD = "asset.upload.begin";
const ASSET_UPLOAD_COMMIT_METHOD = "asset.upload.commit";
const ASSET_TRANSFER_CANCEL_METHOD = "asset.transfer.cancel";

/**
 * Some engines never fire `cancel` on a file input. They return focus to the page
 * when the chooser closes, so a focus with no change event is read as a cancel
 * once this grace period has passed. The grace covers a change event that
 * arrives just after focus.
 */
const FOCUS_RETURN_GRACE_MS = 1000;

/** Largest file the chunked upload path accepts. Larger files are refused before the broker is asked for a transfer. */
const MAX_UPLOAD_BYTES = 64 * 1024 * 1024;

/**
 * Opens the system file chooser through a hidden `<input type="file">`. Call it
 * synchronously inside the click that starts the upload, because browsers only
 * open a chooser from a user gesture. Resolves null when the user cancels.
 */
export function openFileChooser(
  doc: FileChooserDocument,
  options: { focusGraceMs?: number } = {},
): Promise<File | null> {
  const graceMs = options.focusGraceMs ?? FOCUS_RETURN_GRACE_MS;
  return new Promise((resolve) => {
    const input = doc.createElement("input");
    input.type = "file";
    input.hidden = true;
    doc.body.appendChild(input);
    const view = doc.defaultView;
    let settled = false;
    let focusTimer: ReturnType<typeof setTimeout> | undefined;

    const chosen = () => input.files?.[0] ?? null;
    const settle = (file: File | null) => {
      if (settled) return;
      settled = true;
      clearTimeout(focusTimer);
      input.removeEventListener("change", onChange);
      input.removeEventListener("cancel", onCancel);
      view?.removeEventListener("focus", onFocus);
      input.remove();
      resolve(file);
    };
    const onChange = () => settle(chosen());
    const onCancel = () => settle(null);
    const onFocus = () => {
      clearTimeout(focusTimer);
      focusTimer = setTimeout(() => settle(chosen()), graceMs);
    };

    input.addEventListener("change", onChange);
    input.addEventListener("cancel", onCancel);
    view?.addEventListener("focus", onFocus);
    input.click();
  });
}

async function sha256Hex(subtle: Pick<SubtleCrypto, "digest">, bytes: Uint8Array): Promise<string> {
  const digest = new Uint8Array(await subtle.digest("SHA-256", bytes));
  let hex = "";
  for (const byte of digest) hex += byte.toString(16).padStart(2, "0");
  return `sha256:${hex}`;
}

/**
 * The editor host for the isolated Language webview. Asset bytes come from the
 * broker's asset reads, never from Tauri IPC, and external links are handed to
 * the shell through the state channel.
 */
export function createBrokerEditorHost(options: BrokerEditorHostOptions): EditorHost {
  const { rpc, namespace } = options;
  const mimeByPath = new Map<string, { mime_type?: string }>();
  // Uploads to one entity run one at a time: each begin needs the entity revision that the previous commit changed.
  const uploadQueues = new Map<string, Promise<unknown>>();

  function queueUpload<T>(entityId: string, task: () => Promise<T>): Promise<T> {
    const previous = uploadQueues.get(entityId) ?? Promise.resolve();
    const run = previous.then(task);
    const settled = run.then(
      () => undefined,
      () => undefined,
    );
    uploadQueues.set(entityId, settled);
    void settled.then(() => {
      if (uploadQueues.get(entityId) === settled) uploadQueues.delete(entityId);
    });
    return run;
  }

  async function tryBegin(begin: () => Promise<SharedAssetReadBegin | null>): Promise<SharedAssetReadBegin | null> {
    try {
      const begun = await begin();
      if (!begun || typeof begun.url !== "string" || begun.url.length === 0) return null;
      return begun;
    } catch {
      return null;
    }
  }

  /** Begins a read of a portable `assets/...` path: project-shared first, then the launch entity's own assets. */
  async function beginPathRead(path: string): Promise<SharedAssetReadBegin | null> {
    const shared = await tryBegin(() => rpc.call<SharedAssetReadBegin | null>(SHARED_ASSET_READ_METHOD, { path }));
    let begun = shared;
    if (!begun && options.launchEntityId) {
      const entityId = options.launchEntityId;
      begun = await tryBegin(async () => {
        const own = await rpc.call<Asset[]>(ASSET_LIST_METHOD, { entityId, namespace });
        const match = own.find((asset) => asset.path === path);
        if (!match) return null;
        return rpc.call<SharedAssetReadBegin | null>(OWN_ASSET_READ_METHOD, { assetId: match.id, namespace });
      });
    }
    if (!begun) return null;
    mimeByPath.set(path, { mime_type: begun.mimeType });
    return begun;
  }

  /** GETs a begun read. Throws on transport failure or a non-OK status. */
  async function readUrl(url: string): Promise<Uint8Array> {
    const response = await options.fetch(url);
    if (!response.ok) throw new Error(`asset read failed with status ${response.status}`);
    return new Uint8Array(await response.arrayBuffer());
  }

  async function entityRevision(entityId: string): Promise<string> {
    const entity = await rpc.call<{ revision?: unknown } | null>(ENTITY_GET_METHOD, { id: entityId });
    if (!entity || typeof entity.revision !== "string") throw new Error("The language could not be found.");
    return entity.revision;
  }

  async function cancelTransfer(handle: string): Promise<void> {
    try {
      await rpc.call(ASSET_TRANSFER_CANCEL_METHOD, { handle });
    } catch {
      // The transfer expires on its own; the upload error is the one the user needs to see.
    }
  }

  /** Begins, sends and commits one file. Runs inside the entity's upload queue. */
  async function uploadFile(
    input: { entityId: string; namespace: string },
    file: File,
  ): Promise<{ asset: Asset; filename: string }> {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const mimeType = file.type || mimeTypeFor(file.name);
    // Read fresh for every upload: the entity revision changes whenever an asset is added.
    const expectedRevision = await entityRevision(input.entityId);
    const transfer = await rpc.call<AssetReplaceHandle>(ASSET_UPLOAD_BEGIN_METHOD, {
      entityId: input.entityId,
      namespace: input.namespace,
      filename: file.name,
      mimeType,
      size: bytes.byteLength,
      expectedRevision,
    });
    try {
      await options.uploadChunks(transfer, bytes);
      const contentHash = await sha256Hex(options.crypto.subtle, bytes);
      const asset = await rpc.call<Asset>(ASSET_UPLOAD_COMMIT_METHOD, { handle: transfer.handle, contentHash });
      return { asset, filename: file.name };
    } catch (cause) {
      await cancelTransfer(transfer.handle);
      throw cause;
    }
  }

  const assets: EditorAssetServices = {
    listAssets(entityId) {
      return rpc.call<Asset[]>(ASSET_LIST_METHOD, { entityId, namespace });
    },
    listSharedAssets() {
      return rpc.call<Asset[]>(SHARED_ASSET_LIST_METHOD, {});
    },
    async readAssetBytes(asset) {
      const begun =
        asset.namespace === namespace
          ? await rpc.call<SharedAssetReadBegin>(OWN_ASSET_READ_METHOD, { assetId: asset.id, namespace })
          : await rpc.call<SharedAssetReadBegin>(SHARED_ASSET_READ_METHOD, { path: asset.path });
      return readUrl(begun.url);
    },
    async updateAssetMetadata(assetId, update, expectedRevision) {
      const updated = await rpc.call<Asset>(ASSET_UPDATE_METHOD, {
        assetId,
        namespace,
        referenceScope: update.referenceScope,
        expectedRevision,
      });
      if (updated?.entity_id) options.onEntityChanged(updated.entity_id);
      return updated;
    },
    async pickAndRegisterAsset(input) {
      // The chooser opens before the first await so the click that started the upload still counts as a user gesture.
      const file = await openFileChooser(options.document);
      if (!file) return null;
      if (file.size > MAX_UPLOAD_BYTES) {
        throw new Error(`${file.name} is larger than 64 MiB and cannot be attached.`);
      }
      const picked = await queueUpload(input.entityId, () => uploadFile(input, file));
      options.onEntityChanged(input.entityId);
      return picked;
    },
  };

  return {
    async readAssetBytesByPath(path) {
      const begun = await beginPathRead(path);
      if (!begun) return null;
      try {
        return await readUrl(begun.url);
      } catch {
        return null;
      }
    },
    async getAssetByPath(path) {
      const cached = mimeByPath.get(path);
      if (cached) return cached;
      const begun = await beginPathRead(path);
      return begun ? (mimeByPath.get(path) ?? null) : null;
    },
    async openExternal(href) {
      options.openLink(href);
    },
    assets,
  };
}
