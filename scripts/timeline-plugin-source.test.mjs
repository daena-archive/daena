import assert from "node:assert/strict";
import { createTimelineProjectionSource } from "../packages/modules/timeline/src/projection-source.ts";

const manifests = [
  {
    id: "daena.maps",
    services: { provides: [{ name: "daena.maps/navigation", major: 1 }] },
  },
  { id: "daena.lore", schemas: [], services: { provides: [] } },
  {
    id: "daena.timeline",
    schemas: [{ namespace: "timeline", entityTypes: [{ id: "daena.timeline:event", name: "Timeline event" }] }],
    services: { provides: [] },
  },
];
const bundledTimeline = {
  id: "daena.timeline",
  schemas: [{ namespace: "timeline", entityTypes: [{ id: "event", name: "Timeline event" }] }],
};

function fakeClient() {
  const calls = [];
  const client = {
    calls,
    async call(method, payload) {
      calls.push({ method, payload });
      if (method === "schema.list") return manifests;
      if (method === "entity.query") {
        const offset = payload.offset ?? 0;
        if (offset === 0) {
          return {
            items: [{ id: "person-1", name: "Aven", entityType: "daena.lore:person" }],
            hasMore: true,
          };
        }
        return { items: [{ id: "event-1", name: "Founding", entityType: "daena.timeline:event" }], hasMore: false };
      }
      if (method === "field.list" && payload.namespace === "timeline") {
        return [{ key: "startsAt", value: "42" }];
      }
      if (method === "field.list" && payload.sharedOnly === true) {
        return [
          {
            entity_id: payload.entityId,
            namespace: payload.namespace,
            key: "birth",
            value: "42",
            revision: "1",
          },
        ];
      }
      if (method === "relationship.list") {
        return [{ relationship_type: "during", target_id: "era-1" }];
      }
      if (method === "entity.get") return { id: payload.id, name: "Era", entity_type: "daena.timeline:era" };
      if (method === "record.list") return [{ value: { schemaVersion: 1 } }];
      if (method === "service.call") return [{ id: "link-1", mapEntityId: "map-1" }];
      throw new Error(`unexpected ${method}`);
    },
  };
  return client;
}

const client = fakeClient();
const source = await createTimelineProjectionSource(client, bundledTimeline);
assert.equal(
  source.module.schemas[0].entityTypes[0].id,
  "daena.timeline:event",
  "chronology matches stored entities against the effective, qualified manifest",
);

assert.deepEqual(
  client.calls.filter((call) => call.method === "schema.list").map((call) => call.payload),
  [{}],
);
assert.equal(source.mapsNavigationEnabled(), true);
assert.deepEqual(await source.listEnabledManifests(), manifests);

const entities = await source.listEntities();
assert.deepEqual(
  client.calls.filter((call) => call.method === "entity.query").map((call) => call.payload),
  [
    { offset: 0, limit: 200 },
    { offset: 1, limit: 200 },
  ],
);
assert.deepEqual(entities, [
  { id: "person-1", name: "Aven", type: "daena.lore:person" },
  { id: "event-1", name: "Founding", type: "daena.timeline:event" },
]);

assert.deepEqual(await source.listOwnFields("event-1"), { startsAt: "42" });
assert.deepEqual(client.calls.at(-1), {
  method: "field.list",
  payload: { entityId: "event-1", namespace: "timeline" },
});

assert.deepEqual(await source.listSharedFields("person-1", "lore"), [
  { entityId: "person-1", namespace: "lore", key: "birth", value: "42", revision: "1" },
]);
assert.equal(client.calls.at(-1).payload.sharedOnly, true);
assert.equal(client.calls.at(-1).payload.namespace, "lore");

assert.deepEqual(await source.listRelationships("event-1"), [{ type: "during", targetId: "era-1" }]);
assert.deepEqual(await source.getEntity("era-1"), { id: "era-1", name: "Era", type: "daena.timeline:era" });
assert.deepEqual(await source.listCalendarDefinition("cal-1"), { schemaVersion: 1 });
assert.deepEqual(client.calls.at(-1).payload, {
  collection: "calendar-definition",
  ownerEntityId: "cal-1",
  limit: 1,
});

await source.listLocations("event-1");
await source.focusEntity("event-1");
await source.openMap({ mapEntityId: "map-1", linkId: "link-1" });
assert.deepEqual(
  client.calls.filter((call) => call.method === "service.call").map((call) => call.payload),
  [
    { name: "daena.maps/navigation", major: 1, payload: { operation: "listLocations", entityId: "event-1" } },
    { name: "daena.maps/navigation", major: 1, payload: { operation: "focusEntity", entityId: "event-1" } },
    {
      name: "daena.maps/navigation",
      major: 1,
      payload: { operation: "openMap", mapEntityId: "map-1", linkId: "link-1" },
    },
  ],
);

const withoutMaps = fakeClient();
withoutMaps.call = async (method, payload) => {
  withoutMaps.calls.push({ method, payload });
  if (method === "schema.list") return [{ id: "daena.lore", services: { provides: [] } }];
  return { items: [], hasMore: false };
};
const closed = await createTimelineProjectionSource(withoutMaps);
assert.equal(closed.mapsNavigationEnabled(), false);

console.log("timeline plugin source passed");
