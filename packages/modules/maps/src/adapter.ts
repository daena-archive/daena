/** Provider-neutral boundary for the bundled map editor.
 *
 * FMG is intentionally not imported here. The production implementation will
 * be supplied by the pinned, packaged FMG wrapper; the JSON adapter below is a
 * deterministic contract fixture used until that bundle is vendored.
 */
export const FMG_PROVIDER = "azgaar-fmg" as const;

export type NormalizedPoint = readonly [number, number];
export type MapAnchor =
  | { kind: "point"; point: NormalizedPoint }
  | {
      kind: "provider-feature";
      provider: string;
      featureKind: string;
      featureId: string;
      fallbackPoint: NormalizedPoint;
    }
  | { kind: "path"; points: readonly NormalizedPoint[] }
  | { kind: "area"; rings: readonly (readonly NormalizedPoint[])[] };

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
  point: NormalizedPoint;
};

export type FeatureQuery = { kind?: string; text?: string };
export type LoadedMap = { provider: string; sourceHash: string; dirty: boolean };
export type OverlayFrame = { locations: readonly MapAnchor[]; date?: unknown };
export type ProviderEvent =
  | { type: "ready" }
  | { type: "dirty"; dirty: boolean }
  | { type: "selection-changed"; anchor: MapAnchor | null }
  | { type: "source-changed" }
  | { type: "viewport-changed" }
  | { type: "fatal-error"; message: string };

export type MapEditorSaveResult = { bytes: Uint8Array; hash: string };

export interface MapProviderAdapter {
  capabilities(): Promise<ProviderCapabilities>;
  load(source: Uint8Array): Promise<LoadedMap>;
  serialize(): Promise<Uint8Array>;
  /** Begin an editing session. When `source` is provided the provider loads
   * it; when omitted (new empty source) the provider is expected to have a
   * freshly generated map already. Marks the session clean on success. */
  open(session: MapEditorSession, source?: Uint8Array): Promise<void>;
  /** Serialize the editor state. Does not commit to the host. */
  save(session: MapEditorSession): Promise<MapEditorSaveResult>;
  /** End the session. No-op when the session is clean; tears down provider
   * state when dirty. Returns whether unsaved work was discarded. */
  close(session: MapEditorSession): Promise<boolean>;
  listFeatures(query?: FeatureQuery): Promise<ProviderFeature[]>;
  captureSelection(): Promise<MapAnchor | null>;
  resolveAnchor(anchor: MapAnchor): Promise<{ resolved: boolean; point: NormalizedPoint | null }>;
  focus(anchor: MapAnchor): Promise<void>;
  setSemanticOverlay(frame: OverlayFrame): Promise<void>;
  subscribe(listener: (event: ProviderEvent) => void): () => void;
  dispose(): Promise<void>;
}

export type MapEditorDiagnosticCode = "provider-unavailable" | "asset-missing" | "asset-malformed" | "asset-conflict";

export type MapEditorDiagnostic = {
  code: MapEditorDiagnosticCode;
  message: string;
  recoverable: boolean;
};

export type MapEditorAsset = { assetId: string; contentHash: string; revision: number };
export type MapEditorSession = { mapId: string; asset: MapEditorAsset; dirty: boolean };

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function point(value: unknown): value is NormalizedPoint {
  return (
    Array.isArray(value) &&
    value.length === 2 &&
    value.every((part) => typeof part === "number" && Number.isFinite(part) && part >= 0 && part <= 1)
  );
}

function anchor(value: unknown): value is MapAnchor {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Record<string, unknown>;
  if (candidate.kind === "point") return point(candidate.point);
  if (candidate.kind === "provider-feature")
    return (
      candidate.provider === FMG_PROVIDER &&
      typeof candidate.featureKind === "string" &&
      typeof candidate.featureId === "string" &&
      point(candidate.fallbackPoint)
    );
  if (candidate.kind === "path")
    return Array.isArray(candidate.points) && candidate.points.length >= 2 && candidate.points.every(point);
  if (candidate.kind === "area")
    return (
      Array.isArray(candidate.rings) &&
      candidate.rings.length > 0 &&
      candidate.rings.every(
        (ring) => Array.isArray(ring) && ring.length >= 4 && ring[0] === ring[ring.length - 1] && ring.every(point),
      )
    );
  return false;
}

/** Small JSON fixture adapter. It models selector stability and lifecycle
 * events without importing FMG internals or accessing the host environment. */
export class JsonProviderAdapter implements MapProviderAdapter {
  private source: Record<string, unknown> | undefined;
  private selection: MapAnchor | null = null;
  private dirty = false;
  private listeners = new Set<(event: ProviderEvent) => void>();

  async capabilities(): Promise<ProviderCapabilities> {
    return {
      provider: FMG_PROVIDER,
      adapterVersion: 1,
      featureKinds: ["burg", "state", "province", "river", "marker"],
      supportsEditing: true,
    };
  }

  async load(bytes: Uint8Array): Promise<LoadedMap> {
    let value: unknown;
    try {
      value = JSON.parse(decoder.decode(bytes));
    } catch {
      throw new Error("FMG fixture source is malformed");
    }
    if (!value || typeof value !== "object" || !Array.isArray((value as Record<string, unknown>).features))
      throw new Error("FMG fixture source is malformed");
    const features = (value as Record<string, unknown>).features as unknown[];
    if (
      !features.every(
        (item) =>
          item &&
          typeof item === "object" &&
          typeof (item as Record<string, unknown>).kind === "string" &&
          typeof (item as Record<string, unknown>).id === "string" &&
          point((item as Record<string, unknown>).point),
      )
    )
      throw new Error("FMG fixture feature is malformed");
    this.source = structuredClone(value) as Record<string, unknown>;
    this.dirty = false;
    this.emit({ type: "ready" });
    return { provider: FMG_PROVIDER, sourceHash: await digest(bytes), dirty: false };
  }

  async serialize(): Promise<Uint8Array> {
    if (!this.source) throw new Error("map is not loaded");
    return encoder.encode(JSON.stringify(this.source));
  }

  async open(session: MapEditorSession, source?: Uint8Array): Promise<void> {
    if (source) {
      await this.load(source);
    } else if (!this.source) {
      throw new Error("map is not loaded");
    }
    session.dirty = false;
  }

  async save(_session: MapEditorSession): Promise<MapEditorSaveResult> {
    const bytes = await this.serialize();
    return { bytes, hash: await digest(bytes) };
  }

  async close(session: MapEditorSession): Promise<boolean> {
    return session.dirty;
  }

  async listFeatures(query: FeatureQuery = {}): Promise<ProviderFeature[]> {
    const features = (this.source?.features ?? []) as Array<Record<string, unknown>>;
    return features
      .filter(
        (feature) =>
          (!query.kind || feature.kind === query.kind) &&
          (!query.text ||
            String(feature.label ?? "")
              .toLocaleLowerCase()
              .includes(query.text.toLocaleLowerCase())),
      )
      .map((feature) => ({
        kind: feature.kind as string,
        id: feature.id as string,
        label: feature.label as string | undefined,
        point: feature.point as NormalizedPoint,
      }));
  }

  async captureSelection(): Promise<MapAnchor | null> {
    return this.selection;
  }

  async resolveAnchor(value: MapAnchor): Promise<{ resolved: boolean; point: NormalizedPoint | null }> {
    if (!anchor(value)) return { resolved: false, point: null };
    if (value.kind !== "provider-feature")
      return {
        resolved: true,
        point: value.kind === "point" ? value.point : value.kind === "path" ? value.points[0] : value.rings[0][0],
      };
    const feature = (await this.listFeatures({ kind: value.featureKind })).find((item) => item.id === value.featureId);
    return feature ? { resolved: true, point: feature.point } : { resolved: false, point: value.fallbackPoint };
  }

  async focus(value: MapAnchor): Promise<void> {
    this.selection = value;
    this.emit({ type: "selection-changed", anchor: value });
    this.emit({ type: "viewport-changed" });
  }
  async setSemanticOverlay(_frame: OverlayFrame): Promise<void> {}
  subscribe(listener: (event: ProviderEvent) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
  async dispose(): Promise<void> {
    this.listeners.clear();
    this.source = undefined;
    this.selection = null;
    this.dirty = false;
  }

  markDirty(): void {
    if (!this.dirty) {
      this.dirty = true;
      this.emit({ type: "dirty", dirty: true });
    }
  }
  private emit(event: ProviderEvent): void {
    for (const listener of this.listeners) listener(event);
  }
}

type BrowserProvider = {
  provider: string;
  capabilities?: () => Promise<ProviderCapabilities>;
  load: (source: Uint8Array) => Promise<void>;
  serialize: () => Promise<Uint8Array> | Uint8Array;
  listFeatures?: (query?: FeatureQuery) => Promise<ProviderFeature[]>;
  captureSelection?: () => Promise<MapAnchor | null>;
  resolveAnchor?: (value: MapAnchor) => Promise<{ resolved: boolean; point: NormalizedPoint | null }>;
  focus?: (value: MapAnchor) => Promise<void>;
  setSemanticOverlay?: (frame: OverlayFrame) => Promise<void>;
  subscribe?: (listener: (event: ProviderEvent) => void) => () => void;
  dispose?: () => Promise<void>;
};

/** Production adapter facade. The bridge is injected by the vendored FMG
 * shell and is the only browser-global dependency of the Maps module. */
export class FmgBrowserAdapter implements MapProviderAdapter {
  private readonly provider: BrowserProvider;
  constructor(provider: BrowserProvider) {
    if (provider.provider !== FMG_PROVIDER) throw new Error("unsupported map provider");
    this.provider = provider;
  }
  async capabilities(): Promise<ProviderCapabilities> {
    return (
      this.provider.capabilities?.() ?? {
        provider: FMG_PROVIDER,
        adapterVersion: 1,
        featureKinds: ["burg", "state", "province", "river", "marker"],
        supportsEditing: true,
      }
    );
  }
  async load(source: Uint8Array): Promise<LoadedMap> {
    await this.provider.load(source);
    return { provider: FMG_PROVIDER, sourceHash: await digest(source), dirty: false };
  }
  async serialize(): Promise<Uint8Array> {
    return this.provider.serialize();
  }
  async open(session: MapEditorSession, source?: Uint8Array): Promise<void> {
    if (source) await this.provider.load(source);
    session.dirty = false;
  }
  async save(_session: MapEditorSession): Promise<MapEditorSaveResult> {
    const bytes = await this.provider.serialize();
    return { bytes, hash: await digest(bytes) };
  }
  async close(session: MapEditorSession): Promise<boolean> {
    if (!session.dirty) return false;
    await this.provider.dispose?.();
    return true;
  }
  async listFeatures(query?: FeatureQuery): Promise<ProviderFeature[]> {
    return this.provider.listFeatures?.(query) ?? [];
  }
  async captureSelection(): Promise<MapAnchor | null> {
    return this.provider.captureSelection?.() ?? null;
  }
  async resolveAnchor(value: MapAnchor): Promise<{ resolved: boolean; point: NormalizedPoint | null }> {
    return this.provider.resolveAnchor?.(value) ?? { resolved: false, point: null };
  }
  async focus(value: MapAnchor): Promise<void> {
    await this.provider.focus?.(value);
  }
  async setSemanticOverlay(frame: OverlayFrame): Promise<void> {
    await this.provider.setSemanticOverlay?.(frame);
  }
  subscribe(listener: (event: ProviderEvent) => void): () => void {
    return this.provider.subscribe?.(listener) ?? (() => undefined);
  }
  async dispose(): Promise<void> {
    await this.provider.dispose?.();
  }
}

async function digest(bytes: Uint8Array): Promise<string> {
  const hash = await globalThis.crypto.subtle.digest("SHA-256", new Uint8Array(bytes).buffer as ArrayBuffer);
  return `sha256:${Array.from(new Uint8Array(hash), (value) => value.toString(16).padStart(2, "0")).join("")}`;
}

export class UnavailableProviderAdapter implements MapProviderAdapter {
  readonly provider = FMG_PROVIDER;
  constructor(
    private readonly diagnostic: MapEditorDiagnostic = {
      code: "provider-unavailable",
      message: "The pinned FMG editor bundle is not included in this build.",
      recoverable: false,
    },
  ) {}
  getDiagnostic(): MapEditorDiagnostic {
    return this.diagnostic;
  }
  private unavailable(): never {
    throw new Error(this.diagnostic.message);
  }
  async capabilities(): Promise<ProviderCapabilities> {
    return this.unavailable();
  }
  async load(_source: Uint8Array): Promise<LoadedMap> {
    return this.unavailable();
  }
  async serialize(): Promise<Uint8Array> {
    return this.unavailable();
  }
  async listFeatures(_query?: FeatureQuery): Promise<ProviderFeature[]> {
    return this.unavailable();
  }
  async captureSelection(): Promise<MapAnchor | null> {
    return this.unavailable();
  }
  async resolveAnchor(_anchor: MapAnchor): Promise<{ resolved: boolean; point: NormalizedPoint | null }> {
    return this.unavailable();
  }
  async focus(_anchor: MapAnchor): Promise<void> {
    return this.unavailable();
  }
  async setSemanticOverlay(_frame: OverlayFrame): Promise<void> {
    return this.unavailable();
  }
  subscribe(_listener: (event: ProviderEvent) => void): () => void {
    return () => undefined;
  }
  async dispose(): Promise<void> {}
  async open(_session: MapEditorSession, _source?: Uint8Array): Promise<void> {
    return this.unavailable();
  }
  async save(_session: MapEditorSession): Promise<MapEditorSaveResult> {
    return this.unavailable();
  }
  async close(_session: MapEditorSession): Promise<boolean> {
    return false;
  }
}

export function diagnosticForAssetState(state: "missing" | "malformed" | "conflict"): MapEditorDiagnostic {
  const messages = {
    missing: "The map source asset is missing.",
    malformed: "The map source asset is malformed.",
    conflict: "The map source changed outside Daena.",
  } as const;
  return { code: `asset-${state}`, message: messages[state], recoverable: true };
}

export type MapEditorControllerEvent =
  | { type: "dirty"; dirty: boolean }
  | { type: "saved"; revision: number; contentHash: string }
  | { type: "conflict"; diagnostic: MapEditorDiagnostic }
  | { type: "fatal"; message: string };

/** Owns the editing session state on top of a provider adapter: tracks
 * dirty/saved transitions, forwards provider signals, and reports whether
 * closing discarded unsaved work. Session mutations flow through here so the
 * bridge can publish deterministic editor state to the shell. */
export class MapEditorController {
  private readonly listeners = new Set<(event: MapEditorControllerEvent) => void>();
  private session: MapEditorSession | null = null;
  private unsubscribeProvider: (() => void) | null = null;

  constructor(private readonly adapter: MapProviderAdapter) {}

  getSession(): MapEditorSession | null {
    return this.session;
  }

  subscribe(listener: (event: MapEditorControllerEvent) => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  async open(session: MapEditorSession, source?: Uint8Array): Promise<void> {
    if (this.unsubscribeProvider) {
      this.unsubscribeProvider();
      this.unsubscribeProvider = null;
    }
    this.unsubscribeProvider = this.adapter.subscribe((event) => {
      if (event.type === "dirty") {
        if (this.session && this.session.dirty !== event.dirty) {
          this.session.dirty = event.dirty;
          this.emit({ type: "dirty", dirty: event.dirty });
        }
      } else if (event.type === "fatal-error") {
        this.emit({ type: "fatal", message: event.message });
      }
    });
    await this.adapter.open(session, source);
    this.session = { ...session, dirty: false };
  }

  async save(): Promise<MapEditorSaveResult> {
    if (!this.session) throw new Error("map editor is not open");
    return this.adapter.save(this.session);
  }

  confirmSaved(revision: number, contentHash: string): void {
    if (!this.session) return;
    this.session.asset = { ...this.session.asset, revision, contentHash };
    this.session.dirty = false;
    this.emit({ type: "saved", revision, contentHash });
  }

  markDirty(): void {
    if (this.session && !this.session.dirty) {
      this.session.dirty = true;
      this.emit({ type: "dirty", dirty: true });
    }
  }

  reportConflict(diagnostic: MapEditorDiagnostic): void {
    this.emit({ type: "conflict", diagnostic });
  }

  /** Ends the session. Returns whether unsaved work was discarded. */
  async close(): Promise<boolean> {
    const session = this.session;
    this.session = null;
    if (this.unsubscribeProvider) {
      this.unsubscribeProvider();
      this.unsubscribeProvider = null;
    }
    if (!session) return false;
    const discarded = await this.adapter.close(session);
    await this.adapter.dispose();
    return discarded;
  }

  private emit(event: MapEditorControllerEvent): void {
    for (const listener of this.listeners) listener(event);
  }
}
