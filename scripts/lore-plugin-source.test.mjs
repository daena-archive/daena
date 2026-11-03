import assert from "node:assert/strict";
import { createLoreGraphSource } from "../packages/modules/lore/src/projection-source.ts";

const mapsManifest = {
  id: "daena.maps",
  services: { provides: [{ name: "daena.maps/navigation", major: 1 }] },
};
const loreManifest = {
  id: "daena.lore",
  schemas: [
    {
      namespace: "lore",
      entityTypes: [
        { id: "daena.lore:person", name: "Person" },
        { id: "daena.lore:place", name: "Place" },
      ],
    },
  ],
  services: { provides: [] },
};
const bundledLore = {
  id: "daena.lore",
  schemas: [
    {
      namespace: "lore",
      entityTypes: [
        { id: "person", name: "Person" },
        { id: "place", name: "Place" },
      ],
    },
  ],
};

function fakeClient({ manifests = [mapsManifest, loreManifest], focus, openMap } = {}) {
  const calls = [];
  return {
    calls,
    async call(method, payload) {
      calls.push({ method, payload });
      if (method === "schema.list") return manifests;
      if (method === "entity.query") {
        if (payload.offset === 0) {
          return {
            items: [{ id: "person-1", name: "Aven", entityType: "daena.lore:person", deleted: false }],
            hasMore: true,
          };
        }
        return {
          items: [{ id: "place-1", name: "Vale", entityType: "daena.lore:place", deleted: false }],
          hasMore: false,
        };
      }
      if (method === "relationship.list") {
        return [
          { id: "rel-1", source_id: "person-1", target_id: "place-1", relationship_type: "lives_in", metadata: "{}" },
          { id: "rel-2", sourceId: "place-1", targetId: "person-1", relationshipType: "rules", metadata: "{}" },
        ];
      }
      if (method === "service.call") {
        if (payload.payload.operation === "focusEntity") return focus ?? { mapEntityId: "map-1", linkId: "link-1" };
        return openMap ?? null;
      }
      throw new Error(`unexpected ${method}`);
    },
  };
}

// schema.list is called exactly once and the effective, qualified manifest wins.
{
  const client = fakeClient();
  const source = await createLoreGraphSource(client, bundledLore);
  assert.deepEqual(
    client.calls.filter((call) => call.method === "schema.list").map((call) => call.payload),
    [{}],
  );
  assert.equal(source.module.schemas[0].entityTypes[0].id, "daena.lore:person");
  assert.equal(client.calls.filter((call) => call.method === "schema.list").length, 1);
}

// Without a lore entry in schema.list, the bundled manifest is the fallback.
{
  const client = fakeClient({ manifests: [mapsManifest] });
  const source = await createLoreGraphSource(client, bundledLore);
  assert.equal(source.module, bundledLore);
}

// entity.query is paged with the type filter, and deleted entities are left to
// the host default (no archived flag is sent).
{
  const client = fakeClient();
  const source = await createLoreGraphSource(client, bundledLore);
  const entities = await source.listEntities(["daena.lore:person", "daena.lore:place"]);
  assert.deepEqual(
    client.calls.filter((call) => call.method === "entity.query").map((call) => call.payload),
    [
      { entityTypes: ["daena.lore:person", "daena.lore:place"], excludedEntityTypes: [], offset: 0, limit: 200 },
      { entityTypes: ["daena.lore:person", "daena.lore:place"], excludedEntityTypes: [], offset: 1, limit: 200 },
    ],
  );
  for (const call of client.calls.filter((entry) => entry.method === "entity.query")) {
    assert.equal("archived" in call.payload, false);
    assert.equal("deleted" in call.payload, false);
  }
  assert.deepEqual(entities, [
    { id: "person-1", name: "Aven", type: "daena.lore:person" },
    { id: "place-1", name: "Vale", type: "daena.lore:place" },
  ]);
}

// An empty page that still claims more results cannot loop forever.
{
  const client = fakeClient();
  client.call = async (method, payload) => {
    client.calls.push({ method, payload });
    if (method === "schema.list") return [loreManifest];
    return { items: [], hasMore: true };
  };
  const source = await createLoreGraphSource(client, bundledLore);
  assert.deepEqual(await source.listEntities(["daena.lore:person"]), []);
}

// relationship.list maps both wire casings onto the graph's shape.
{
  const client = fakeClient();
  const source = await createLoreGraphSource(client, bundledLore);
  assert.deepEqual(await source.listRelationships("person-1"), [
    { id: "rel-1", sourceId: "person-1", targetId: "place-1", type: "lives_in" },
    { id: "rel-2", sourceId: "place-1", targetId: "person-1", type: "rules" },
  ]);
  assert.deepEqual(client.calls.at(-1), { method: "relationship.list", payload: { entityId: "person-1" } });
}

// Maps availability follows the enabled daena.maps manifest providing navigation@1.
{
  assert.equal((await createLoreGraphSource(fakeClient(), bundledLore)).mapsNavigationEnabled(), true);
  assert.equal(
    (await createLoreGraphSource(fakeClient({ manifests: [loreManifest] }), bundledLore)).mapsNavigationEnabled(),
    false,
  );
  const wrongMajor = {
    id: "daena.maps",
    services: { provides: [{ name: "daena.maps/navigation", major: 2 }] },
  };
  assert.equal(
    (
      await createLoreGraphSource(fakeClient({ manifests: [wrongMajor, loreManifest] }), bundledLore)
    ).mapsNavigationEnabled(),
    false,
  );
}

// Map actions go through service.call with the navigation payloads.
{
  const client = fakeClient();
  const source = await createLoreGraphSource(client, bundledLore);
  assert.deepEqual(await source.focusEntity("person-1"), { mapEntityId: "map-1", linkId: "link-1" });
  await source.openMap({ mapEntityId: "map-1", linkId: "link-1" });
  assert.deepEqual(
    client.calls.filter((call) => call.method === "service.call").map((call) => call.payload),
    [
      { name: "daena.maps/navigation", major: 1, payload: { operation: "focusEntity", entityId: "person-1" } },
      {
        name: "daena.maps/navigation",
        major: 1,
        payload: { operation: "openMap", mapEntityId: "map-1", linkId: "link-1" },
      },
    ],
  );
}

// Multiple links come back as a choice list for the view to resolve.
{
  const locations = [
    { id: "link-a", mapEntityId: "map-1" },
    { id: "link-b", mapEntityId: "map-2" },
  ];
  const client = fakeClient({ focus: { status: "multiple-links", locations } });
  const source = await createLoreGraphSource(client, bundledLore);
  assert.deepEqual(await source.focusEntity("person-1"), { status: "multiple-links", locations });
}

console.log("lore plugin source passed");
