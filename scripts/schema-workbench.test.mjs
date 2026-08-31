import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compile } from "svelte/compiler";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const {
  normalizeOverlay,
  fingerprint,
  flattenPackageSchemas,
  overlayIsCustomized,
  summarizePackageCounts,
  filterSchemaListItems,
  fieldTypeLabel,
  fieldKindGroupLabel,
  mintTypeId,
  ensureFieldKey,
  FIELD_KIND_GROUPS,
  FIELD_TYPES,
  applyTypeRemovalPlan,
  pruneOverlayForRemovedType,
  typeRemovalPlanIsComplete,
  validateFieldForm,
} = await import("../src/lib/schema-workbench/model.ts");

const {
  HOUSES_PLUGIN_ID,
  LANGUAGE_PLUGIN_ID,
  LANGUAGE_SCHEMA_OVERLAY_READY,
  LORE_PLUGIN_ID,
  MANAGED_EXPECTED_PLUGIN_IDS,
  MAPS_PLUGIN_ID,
  OVERLAY_EXPECTED_PLUGIN_IDS,
  TIMELINE_PLUGIN_ID,
  TREE_HOUSE_TYPE,
  TREE_PERSON_TYPE,
  WRITING_PLUGIN_ID,
  housesTypeTreeRole,
  isTreeCompatibleHouseType,
  managedSchemaPluginReason,
  projectionLabelsForModuleType,
  schemaOverlayWorkbenchAllowed,
} = await import("../src/lib/schema-workbench/module-compatibility.ts");

const empty = { version: 1 };
const once = normalizeOverlay(empty, { pluginId: "daena.lore" });
const twice = normalizeOverlay(once, { pluginId: "daena.lore" });
assert.equal(JSON.stringify(once), JSON.stringify(twice), "normalize must be idempotent");
assert.equal(fingerprint(empty, { pluginId: "daena.lore" }), fingerprint(once, { pluginId: "daena.lore" }));

const customized = normalizeOverlay(
  {
    version: 1,
    customEntityTypes: [{ id: "relic", name: "Relic" }],
    customFields: [{ key: "Weight", label: "Weight", type: "number", entityTypes: ["relic"] }],
    customTemplates: [
      {
        id: "New Relic",
        name: "New Relic",
        entityType: "relic",
        fields: { weight: "" },
        requiredFields: [],
      },
    ],
    disabledFields: ["summary"],
  },
  { pluginId: "daena.lore" },
);

assert.equal(customized.customEntityTypes?.[0]?.id, "daena.lore:relic");
assert.equal(customized.customFields?.[0]?.key, "weight");
assert.equal(customized.customFields?.[0]?.entityTypes?.[0], "daena.lore:relic");
assert.equal(customized.customTemplates?.[0]?.id, "new-relic");
assert.equal(customized.customTemplates?.[0]?.entityType, "daena.lore:relic");
assert.deepEqual(customized.disabledFields, ["summary"]);
assert.ok(overlayIsCustomized(customized));
assert.equal(JSON.stringify(customized), JSON.stringify(normalizeOverlay(customized, { pluginId: "daena.lore" })));

const multi = flattenPackageSchemas({
  schemas: [
    {
      namespace: "alpha",
      entityTypes: [
        { id: "person", name: "Person" },
        { id: "place", name: "Place" },
      ],
      fields: [{ key: "summary", label: "Summary", type: "text" }],
    },
    {
      namespace: "beta",
      entityTypes: [
        { id: "person", name: "Person Duplicate" },
        { id: "era", name: "Era" },
      ],
      fields: [
        { key: "summary", label: "Summary Dup", type: "text" },
        { key: "startsAt", label: "Starts", type: "date" },
      ],
    },
  ],
  templates: [{ id: "person", name: "Person", entityType: "person", fields: {}, requiredFields: [] }],
});

assert.deepEqual(multi.namespaces, ["alpha", "beta"]);
assert.equal(multi.entityTypes.length, 3);
assert.equal(multi.fields.length, 2);
assert.equal(multi.typeNamespace.person, "alpha");
assert.equal(multi.typeNamespace.era, "beta");
assert.equal(multi.fieldNamespace.summary, "alpha");
assert.equal(multi.fieldNamespace.startsAt, "beta");

const lore = JSON.parse(await read("packages/modules/lore/manifest.json"));
const loreFlat = flattenPackageSchemas({ schemas: lore.schemas, templates: lore.templates });
assert.ok(loreFlat.entityTypes.length >= 6);
assert.ok(loreFlat.fields.length >= 20);
assert.equal(loreFlat.namespaces[0], "lore");

const counts = summarizePackageCounts(
  { schemas: lore.schemas, templates: lore.templates },
  { version: 1, disabledEntityTypes: [lore.schemas[0].entityTypes[0].id] },
);
assert.equal(counts.types, loreFlat.entityTypes.length - 1);
assert.equal(counts.customized, true);

const filtered = filterSchemaListItems(
  [
    {
      id: "person",
      kind: "type",
      name: "Person",
      origin: "builtin",
      enabled: true,
      searchText: "person people",
    },
    {
      id: "custom",
      kind: "type",
      name: "Relic",
      origin: "custom",
      enabled: true,
      searchText: "relic custom",
    },
    {
      id: "hidden",
      kind: "field",
      name: "Summary",
      origin: "builtin",
      enabled: false,
      searchText: "summary text",
    },
  ],
  "relic",
  "custom",
);
assert.equal(filtered.length, 1);
assert.equal(filtered[0].id, "custom");

assert.equal(fieldTypeLabel("oneof"), "One of");
assert.equal(fieldTypeLabel("boolean"), "Yes/No");
assert.equal(fieldKindGroupLabel("relationship"), "Linking");
assert.equal(mintTypeId("My Type", "daena.writing"), "daena.writing:my-type");
assert.equal(ensureFieldKey("WordCount"), "word_count");
assert.ok(FIELD_TYPES.includes("relationship"));
assert.equal(FIELD_KIND_GROUPS.basic.label, "Basic");

const removalOverlay = {
  version: 1,
  customEntityTypes: [
    { id: "alpha", name: "Alpha" },
    { id: "beta", name: "Beta" },
  ],
  customFields: [
    { key: "remove_me", label: "Remove me", type: "text", entityTypes: ["alpha"] },
    { key: "disable_me", label: "Disable me", type: "text", entityTypes: ["alpha"] },
    { key: "move_me", label: "Move me", type: "text", entityTypes: ["alpha"] },
    { key: "shared", label: "Shared", type: "text", entityTypes: ["alpha", "beta"] },
  ],
};
const dispositions = {
  remove_me: { action: "remove" },
  disable_me: { action: "disable" },
  move_me: { action: "reassign", toTypeId: "beta" },
};
assert.equal(typeRemovalPlanIsComplete(["remove_me", "disable_me", "move_me"], dispositions, ["beta"]), true);
assert.equal(
  typeRemovalPlanIsComplete(["remove_me", "disable_me", "move_me"], dispositions, ["beta"], {
    entityCount: 3,
  }),
  false,
);
assert.equal(
  typeRemovalPlanIsComplete(["remove_me", "disable_me", "move_me"], dispositions, ["beta"], {
    entityCount: 3,
    entityDisposition: { action: "reassign", toTypeId: "beta" },
  }),
  true,
);
assert.equal(
  typeRemovalPlanIsComplete([], {}, ["beta"], {
    entityCount: null,
    entityDisposition: { action: "none" },
  }),
  true,
);
const removedType = applyTypeRemovalPlan(removalOverlay, {
  typeId: "alpha",
  exclusiveDispositions: dispositions,
  removeSharedFieldKeys: [],
  entityDisposition: { action: "reassign", toTypeId: "beta" },
  entityCount: 2,
});
assert.equal(removedType.customFields?.find((field) => field.key === "move_me")?.entityTypes?.[0], "beta");
assert.deepEqual(removedType.customFields?.find((field) => field.key === "shared")?.entityTypes, ["beta"]);
assert.ok(removedType.customFields?.every((field) => (field.entityTypes?.length ?? 0) > 0));
const directlyPruned = pruneOverlayForRemovedType(removalOverlay, "alpha");
assert.ok(directlyPruned.customFields?.every((field) => (field.entityTypes?.length ?? 0) > 0));

const cardinalityOk = validateFieldForm({
  label: "Parents",
  type: "relationship",
  relationshipType: "parent_of",
  targetEntityTypes: ["person"],
  cardinality: "many",
});
assert.equal(cardinalityOk.cardinality, undefined);
const cardinalityBad = validateFieldForm({
  label: "Parents",
  type: "relationship",
  relationshipType: "parent_of",
  targetEntityTypes: ["person"],
  cardinality: "lots",
});
assert.match(cardinalityBad.cardinality ?? "", /one or many/);

const componentPaths = [
  "src/lib/schema-workbench/SchemaFieldInput.svelte",
  "src/lib/schema-workbench/SchemaTemplatePreview.svelte",
  "src/lib/schema-workbench/SchemaImpactReview.svelte",
  "src/lib/schema-workbench/SchemaTypesPane.svelte",
  "src/lib/schema-workbench/SchemaFieldsPane.svelte",
  "src/lib/schema-workbench/SchemaTemplatesPane.svelte",
  "src/lib/ModuleSchemaPanel.svelte",
  "src/lib/SchemaSettingsPanel.svelte",
];

for (const path of componentPaths) {
  const source = await read(path);
  compile(source, { filename: resolve(root, path), css: "injected" });
}

const panel = await read("src/lib/ModuleSchemaPanel.svelte");
assert.match(panel, /schema-workbench/);
assert.doesNotMatch(panel, /packageManifest\.schemas\[0\]/);
assert.match(panel, /flattenPackageSchemas|flatPackage/);
assert.match(panel, /SchemaTypesPane|SchemaFieldsPane|SchemaTemplatesPane/);
assert.match(panel, /showAdvanced|statusFilter|listQuery/);
assert.doesNotMatch(panel, /mintTypeId\(editTypeValue/);
assert.match(panel, /applyTypeRemovalPlan/);
assert.match(panel, /onReassignEntities|entityDisposition|Reassign entities/);
assert.match(panel, /SchemaImpactReview|impactPreview|onPreview/);
assert.match(panel, /conflictCompare|conflictReapply|onAdoptCurrentRevision|onFetchCurrent/);
assert.match(panel, /acknowledgeImpact|Preview failed|previewError/);
assert.doesNotMatch(panel, /onReviewDraft/);

const typesPane = await read("src/lib/schema-workbench/SchemaTypesPane.svelte");
const fieldsPane = await read("src/lib/schema-workbench/SchemaFieldsPane.svelte");
const templatesPane = await read("src/lib/schema-workbench/SchemaTemplatesPane.svelte");
assert.match(typesPane, /Builtin entity types|builtin/);
assert.match(fieldsPane, /Builtin fields|builtin/);
assert.match(fieldsPane, /cardinality:\s*(new|edit)FieldCardinality/);
assert.match(templatesPane, /SchemaTemplatePreview|template/i);
assert.match(`${typesPane}\n${fieldsPane}\n${templatesPane}`, /workbench-split/);
assert.match(`${typesPane}\n${fieldsPane}\n${templatesPane}`, /workbench-list/);

const settings = await read("src/lib/SchemaSettingsPanel.svelte");
assert.match(settings, /Customized|Managed by extension|typeCount|fieldCount|managedPlugins/);
assert.match(settings, /validationStatus/);
assert.match(settings, /onPreview|onReloadCurrent|conflict|editorRemountKey|contentRevision/);
assert.match(settings, /onFetchCurrent|onAdoptCurrentRevision/);
assert.match(settings, /projectionLabelsForModuleType|projectionLabelsForType/);

const impact = await read("src/lib/schema-workbench/SchemaImpactReview.svelte");
assert.match(impact, /Review schema impact|requiresAcknowledgement|affectedTypes|Confirm save/);

const fieldInput = await read("src/lib/schema-workbench/SchemaFieldInput.svelte");
assert.match(fieldInput, /RelationshipPicker/);
assert.match(fieldInput, /DateEditor/);

const client = await read("src/lib/project/client.ts");
assert.match(client, /SchemaOverlayPreviewResult/);
assert.match(client, /previewModuleSchemaOverlay/);
assert.match(client, /ModuleSchemaOverlayMutationResult/);
assert.match(client, /acknowledgeImpact|acknowledge_impact/);
assert.match(client, /expected_revision.*request_id|setModuleSchemaOverlay/);

const vocabulary = await read("src/lib/entity-lifecycle/vocabulary.ts");
assert.match(vocabulary, /conflictCompare|conflictReapply/);

const page = await read("src/routes/+page.svelte");
assert.match(page, /SchemaFieldInput|schema-workbench\/SchemaFieldInput/);
assert.match(page, /managedSchemaPlugins/);
assert.match(page, /managedSchemaPluginReason/);
assert.match(page, /schemaOverlayWorkbenchAllowed/);
assert.doesNotMatch(page, /schemas\[0\]/);
assert.match(page, /summarizePackageCounts/);
assert.match(page, /overlayValidationStatus/);
assert.match(page, /typeCount:\s*counts\.types/);
assert.match(page, /validationStatus:\s*validation\.status/);
assert.match(page, /reassignSchemaEntities|onReassignEntities/);
assert.match(page, /schemaEntityCountForType/);
assert.match(page, /previewModuleSchemaOverlay|moduleSchemaContentRevision/);
assert.match(page, /moduleSchemaConflict|expectedRevision:\s*moduleSchemaContentRevision/);
assert.match(page, /acknowledgeImpact|onAdoptCurrentRevision|editorRemountKey/);
assert.match(page, /moduleSchemaSaveRequestId = crypto\.randomUUID/);

assert.match(typesPane, /Houses collection only|tree-compat-note/);
assert.doesNotMatch(typesPane, /Custom Houses types appear in the Houses collection only/);

assert.equal(LANGUAGE_SCHEMA_OVERLAY_READY, false);
assert.equal(schemaOverlayWorkbenchAllowed(LANGUAGE_PLUGIN_ID, ["schema.overlay"]), false);
assert.equal(schemaOverlayWorkbenchAllowed(LORE_PLUGIN_ID, ["schema.overlay"]), true);
assert.ok(isTreeCompatibleHouseType(TREE_HOUSE_TYPE));
assert.ok(isTreeCompatibleHouseType("house"));
assert.equal(housesTypeTreeRole("daena.houses:clan"), "collection-only");
assert.equal(housesTypeTreeRole(TREE_PERSON_TYPE), "collection-only");
assert.deepEqual(projectionLabelsForModuleType(HOUSES_PLUGIN_ID, TREE_HOUSE_TYPE), ["Houses collection", "Tree"]);
assert.deepEqual(projectionLabelsForModuleType(HOUSES_PLUGIN_ID, "daena.houses:clan"), ["Houses collection only"]);
assert.deepEqual(projectionLabelsForModuleType(HOUSES_PLUGIN_ID, TREE_PERSON_TYPE), ["Houses collection only"]);
assert.deepEqual(projectionLabelsForModuleType(LORE_PLUGIN_ID, TREE_PERSON_TYPE), ["Library", "Wiki", "Graph", "Tree"]);
assert.deepEqual(projectionLabelsForModuleType(LORE_PLUGIN_ID, "daena.lore:place"), ["Library", "Wiki", "Graph"]);
assert.deepEqual(projectionLabelsForModuleType(TIMELINE_PLUGIN_ID, "event"), ["Timeline"]);
assert.deepEqual(projectionLabelsForModuleType(WRITING_PLUGIN_ID, "manuscript"), ["Writing Studio"]);
assert.match(managedSchemaPluginReason(LANGUAGE_PLUGIN_ID), /packaged fields|merged schema/i);
assert.match(managedSchemaPluginReason(MAPS_PLUGIN_ID), /provider|extension-managed/i);

for (const pluginId of OVERLAY_EXPECTED_PLUGIN_IDS) {
  const manifest = JSON.parse(await read(`packages/modules/${pluginId.replace("daena.", "")}/manifest.json`));
  assert.ok((manifest.capabilities ?? []).includes("schema.overlay"), `${pluginId} must declare schema.overlay`);
}
for (const pluginId of MANAGED_EXPECTED_PLUGIN_IDS) {
  const folder = pluginId.replace("daena.", "");
  const manifest = JSON.parse(await read(`packages/modules/${folder}/manifest.json`));
  assert.ok(
    !(manifest.capabilities ?? []).includes("schema.overlay"),
    `${pluginId} must remain without schema.overlay`,
  );
}

const languageOverview = await read("packages/modules/language/src/panes/Overview.svelte");
assert.match(languageOverview, /manifest\.schemas/);
assert.doesNotMatch(languageOverview, /schema\.overlay|loadModuleSchemaEditor|mergedSchema/);

console.log("schema-workbench tests passed");
