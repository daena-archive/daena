/** Provider-neutral types for the Maps module host surface.
 *
 * Authored maps use the native OpenLayers / physical editors in the host shell.
 * This module no longer embeds a third-party map webview.
 */
export { VECTOR_PROVIDER, PHYSICAL_PROVIDER, type MapAnchor, type NormalizedPoint } from "../../../plugin-sdk/src/maps";

export type ProviderCapabilities = {
  provider: string;
  adapterVersion: number;
  featureKinds: readonly string[];
  supportsEditing: boolean;
};

export type ProviderFeature = {
  kind: string;
  id: string;
  label?: string;
  point: readonly [number, number];
};

export type FeatureQuery = { kind?: string; text?: string };
export type LoadedMap = { provider: string; sourceHash: string; dirty: boolean };
export type OverlayFrame = { locations: readonly import("../../../plugin-sdk/src/maps").MapAnchor[]; date?: unknown };
export type ProviderEvent =
  | { type: "ready" }
  | { type: "dirty"; dirty: boolean }
  | { type: "selection-changed"; anchor: import("../../../plugin-sdk/src/maps").MapAnchor | null }
  | { type: "source-changed" }
  | { type: "viewport-changed" }
  | { type: "fatal-error"; message: string };

export type MapEditorSaveResult = { bytes: Uint8Array; hash: string };

export interface MapProviderAdapter {
  capabilities(): Promise<ProviderCapabilities>;
  load(source: Uint8Array): Promise<LoadedMap>;
  save(): Promise<MapEditorSaveResult>;
  isDirty(): Promise<boolean>;
  captureAnchor(): Promise<import("../../../plugin-sdk/src/maps").MapAnchor | null>;
  startPick(): Promise<void>;
  setOverlay(frame: OverlayFrame): Promise<void>;
  setDate(date: unknown): Promise<void>;
  focusLink(linkId: string): Promise<void>;
  queryFeatures(query: FeatureQuery): Promise<readonly ProviderFeature[]>;
  subscribe(listener: (event: ProviderEvent) => void): () => void;
}

/** Host-owned editors replace the former embedded map webview. */
export class UnavailableProviderAdapter implements MapProviderAdapter {
  readonly provider = "daena-openlayers";

  private fail(): never {
    throw new Error("Map editing runs in the Daena host shell, not a plugin webview.");
  }

  async capabilities(): Promise<ProviderCapabilities> {
    return {
      provider: this.provider,
      adapterVersion: 2,
      featureKinds: ["geojson-feature"],
      supportsEditing: false,
    };
  }

  async load(): Promise<LoadedMap> {
    this.fail();
  }
  async save(): Promise<MapEditorSaveResult> {
    this.fail();
  }
  async isDirty(): Promise<boolean> {
    return false;
  }
  async captureAnchor() {
    this.fail();
  }
  async startPick() {
    this.fail();
  }
  async setOverlay() {
    this.fail();
  }
  async setDate() {
    this.fail();
  }
  async focusLink() {
    this.fail();
  }
  async queryFeatures(): Promise<readonly ProviderFeature[]> {
    return [];
  }
  subscribe(): () => void {
    return () => {};
  }
}
