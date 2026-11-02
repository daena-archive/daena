import type { FieldRecord, ModuleManifest } from "../../../module-api/src/index";
import { CALENDAR_DEFINITION_COLLECTION } from "./calendar.ts";

export type TimelineEntityRef = { id: string; name: string; type: string | null };

export type TimelineMapLocation = { id: string; mapEntityId: string };

export type TimelineFocusResult =
  | { status?: undefined; mapEntityId: string; linkId: string }
  | { status: "multiple-links"; locations: readonly TimelineMapLocation[] };

export interface TimelineProjectionSource {
  module: ModuleManifest;
  listEntities(): Promise<TimelineEntityRef[]>;
  listEnabledManifests(): Promise<ModuleManifest[]>;
  listOwnFields(entityId: string): Promise<Record<string, unknown>>;
  listSharedFields(entityId: string, namespace: string): Promise<FieldRecord[]>;
  listRelationships(entityId: string): Promise<Array<{ type: string; targetId: string }>>;
  getEntity(id: string): Promise<TimelineEntityRef | null>;
  listCalendarDefinition(ownerEntityId: string): Promise<unknown | null>;
  mapsNavigationEnabled(): boolean;
  listLocations(entityId: string): Promise<readonly TimelineMapLocation[]>;
  focusEntity(entityId: string): Promise<TimelineFocusResult>;
  openMap(input: { mapEntityId: string; linkId?: string }): Promise<void>;
  reportSurfaceMeta?(meta: { subtitle: string }): void;
}

export interface TimelineRpc {
  call<T>(method: string, payload: unknown): Promise<T>;
}

type WireEntity = {
  id: string;
  name: string;
  type?: string | null;
  entityType?: string | null;
  entity_type?: string | null;
};

type WireField = {
  entityId?: string;
  entity_id?: string;
  namespace: string;
  key: string;
  value: unknown;
  revision: string;
};

type WireRelationship = {
  relationshipType?: string;
  relationship_type?: string;
  targetId?: string;
  target_id?: string;
};

const MAPS_NAVIGATION = "daena.maps/navigation";

function entityFromWire(entity: WireEntity): TimelineEntityRef {
  return {
    id: entity.id,
    name: entity.name,
    type: entity.type ?? entity.entityType ?? entity.entity_type ?? null,
  };
}

function mapsProviderEnabled(manifests: readonly ModuleManifest[]): boolean {
  return manifests.some(
    (manifest) =>
      manifest.id === "daena.maps" &&
      (manifest.services?.provides ?? []).some((service) => service.name === MAPS_NAVIGATION && service.major === 1),
  );
}

export async function createTimelineProjectionSource(
  client: TimelineRpc,
  moduleManifest: ModuleManifest = { id: "daena.timeline" } as ModuleManifest,
): Promise<TimelineProjectionSource> {
  const manifests = await client.call<ModuleManifest[]>("schema.list", {});
  const mapsEnabled = mapsProviderEnabled(manifests);
  // The effective manifest carries qualified entity type ids
  // (`daena.timeline:event`) and the project's schema overlay; the bundled
  // manifest.json only has local ids, which never match stored entities.
  const effectiveModule = manifests.find((manifest) => manifest.id === moduleManifest.id) ?? moduleManifest;
  return {
    module: effectiveModule,
    listEnabledManifests: async () => manifests,
    mapsNavigationEnabled: () => mapsEnabled,
    async listEntities() {
      const entities: TimelineEntityRef[] = [];
      let offset = 0;
      while (true) {
        const page = await client.call<{ items: WireEntity[]; hasMore: boolean }>("entity.query", {
          offset,
          limit: 200,
        });
        entities.push(...page.items.map(entityFromWire));
        if (!page.hasMore) return entities;
        offset += page.items.length;
      }
    },
    async listOwnFields(entityId) {
      const fields = await client.call<WireField[]>("field.list", { entityId, namespace: "timeline" });
      return Object.fromEntries(fields.map((field) => [field.key, field.value]));
    },
    async listSharedFields(entityId, namespace) {
      const fields = await client.call<WireField[]>("field.list", { entityId, namespace, sharedOnly: true });
      return fields.map((field) => ({
        entityId: field.entityId ?? field.entity_id ?? entityId,
        namespace: field.namespace,
        key: field.key,
        value: field.value,
        revision: field.revision,
      }));
    },
    async listRelationships(entityId) {
      const relationships = await client.call<WireRelationship[]>("relationship.list", { entityId });
      return relationships.map((relationship) => ({
        type: relationship.relationshipType ?? relationship.relationship_type ?? "",
        targetId: relationship.targetId ?? relationship.target_id ?? "",
      }));
    },
    async getEntity(id) {
      const entity = await client.call<WireEntity | null>("entity.get", { id });
      return entity == null ? null : entityFromWire(entity);
    },
    async listCalendarDefinition(ownerEntityId) {
      const records = await client.call<Array<{ value?: unknown }>>("record.list", {
        collection: CALENDAR_DEFINITION_COLLECTION,
        ownerEntityId,
        limit: 1,
      });
      return records[0]?.value ?? null;
    },
    async listLocations(entityId) {
      return client.call<TimelineMapLocation[]>("service.call", {
        name: MAPS_NAVIGATION,
        major: 1,
        payload: { operation: "listLocations", entityId },
      });
    },
    async focusEntity(entityId) {
      return client.call<TimelineFocusResult>("service.call", {
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
