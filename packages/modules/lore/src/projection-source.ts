import type { ModuleManifest } from "../../../module-api/src/index";

export type LoreEntityRef = { id: string; name: string; type: string | null };

export type LoreRelationshipRef = { id: string; sourceId: string; targetId: string; type: string };

export type LoreMapLocation = { id: string; mapEntityId: string };

export type LoreFocusResult =
  | { status?: undefined; mapEntityId: string; linkId: string }
  | { status: "multiple-links"; locations: readonly LoreMapLocation[] };

/** Exactly the data and services the World graph reads. */
export interface LoreGraphSource {
  module: ModuleManifest;
  listEntities(entityTypes: readonly string[]): Promise<LoreEntityRef[]>;
  listRelationships(entityId: string): Promise<LoreRelationshipRef[]>;
  mapsNavigationEnabled(): boolean;
  focusEntity(entityId: string): Promise<LoreFocusResult>;
  openMap(input: { mapEntityId: string; linkId?: string }): Promise<void>;
  reportSurfaceMeta?(meta: { subtitle: string }): void;
}

export interface LoreRpc {
  call<T>(method: string, payload: unknown): Promise<T>;
}

type WireEntity = {
  id: string;
  name: string;
  entityType?: string | null;
  entity_type?: string | null;
  type?: string | null;
};

type WireRelationship = {
  id: string;
  sourceId?: string;
  source_id?: string;
  targetId?: string;
  target_id?: string;
  relationshipType?: string;
  relationship_type?: string;
};

const MAPS_NAVIGATION = "daena.maps/navigation";

function entityFromWire(entity: WireEntity): LoreEntityRef {
  return {
    id: entity.id,
    name: entity.name,
    type: entity.entityType ?? entity.entity_type ?? entity.type ?? null,
  };
}

function relationshipFromWire(relationship: WireRelationship): LoreRelationshipRef {
  return {
    id: relationship.id,
    sourceId: relationship.sourceId ?? relationship.source_id ?? "",
    targetId: relationship.targetId ?? relationship.target_id ?? "",
    type: relationship.relationshipType ?? relationship.relationship_type ?? "",
  };
}

function mapsProviderEnabled(manifests: readonly ModuleManifest[]): boolean {
  return manifests.some(
    (manifest) =>
      manifest.id === "daena.maps" &&
      (manifest.services?.provides ?? []).some((service) => service.name === MAPS_NAVIGATION && service.major === 1),
  );
}

export async function createLoreGraphSource(
  client: LoreRpc,
  bundledManifest: ModuleManifest,
): Promise<LoreGraphSource> {
  const manifests = await client.call<ModuleManifest[]>("schema.list", {});
  const mapsEnabled = mapsProviderEnabled(manifests);
  // The effective manifest carries qualified entity type ids (`daena.lore:person`)
  // and the project's schema overlay; the bundled manifest.json only has local ids,
  // which never match stored entities. Fall back to it only when schema.list omits lore.
  const effectiveModule = manifests.find((manifest) => manifest.id === bundledManifest.id) ?? bundledManifest;
  return {
    module: effectiveModule,
    mapsNavigationEnabled: () => mapsEnabled,
    async listEntities(entityTypes) {
      // Same query the broker-backed context sent before: type filter, no
      // excluded types, and deleted entities excluded by the host by default.
      const entities: LoreEntityRef[] = [];
      let offset = 0;
      while (true) {
        const page = await client.call<{ items: WireEntity[]; hasMore: boolean }>("entity.query", {
          entityTypes: [...entityTypes],
          excludedEntityTypes: [],
          offset,
          limit: 200,
        });
        entities.push(...page.items.map(entityFromWire));
        if (!page.hasMore || page.items.length === 0) return entities;
        offset += page.items.length;
      }
    },
    async listRelationships(entityId) {
      const relationships = await client.call<WireRelationship[]>("relationship.list", { entityId });
      return relationships.map(relationshipFromWire);
    },
    async focusEntity(entityId) {
      return client.call<LoreFocusResult>("service.call", {
        name: MAPS_NAVIGATION,
        major: 1,
        payload: { operation: "focusEntity", entityId },
      });
    },
    async openMap(input) {
      await client.call("service.call", {
        name: MAPS_NAVIGATION,
        major: 1,
        payload: { operation: "openMap", mapEntityId: input.mapEntityId, linkId: input.linkId },
      });
    },
  };
}
