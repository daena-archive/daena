import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compile } from "svelte/compiler";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const { countKinshipFamilyGroups, formatHouseMemberSummary, houseMemberSummaries } =
  await import("../src/lib/family-tree/fetch.ts");
const { formatMembershipRole, isLeadershipRole, MEMBERSHIP_RELATIONSHIP, PARENT_RELATIONSHIP, PARTNER_RELATIONSHIP } =
  await import("../src/lib/family-tree/model.ts");
const { membershipMetadataFields, defaultMembershipDraft } = await import("../src/lib/family-tree/membershipFields.ts");
const { ENTITY_ACTION_CONFIRM, TREE_LEGEND } = await import("../src/lib/entity-lifecycle/vocabulary.ts");

const componentPaths = [
  "src/lib/family-tree/FamilyHousePanel.svelte",
  "src/lib/family-tree/FamilyMembershipDialog.svelte",
];

for (const path of componentPaths) {
  const source = await read(path);
  compile(source, { filename: resolve(root, path), css: "injected" });
}

const shell = await read("src/routes/+page.svelte");
const surface = await read("src/lib/family-tree/FamilyTreeSurface.svelte");
const housePanel = await read("src/lib/family-tree/FamilyHousePanel.svelte");
const membershipDialog = await read("src/lib/family-tree/FamilyMembershipDialog.svelte");
const personNode = await read("src/lib/family-tree/FamilyPersonNode.svelte");
const landing = await read("src/lib/family-tree/FamilyTreeLanding.svelte");
const rowActions = await read("src/lib/entity-lifecycle/EntityRowActions.svelte");

assert.equal(formatMembershipRole("head"), "Head");
assert.equal(formatMembershipRole("custom", "Steward"), "Steward");
assert.equal(formatMembershipRole("member"), "Member");
assert.equal(isLeadershipRole("heir"), true);
assert.equal(isLeadershipRole("member"), false);

assert.equal(
  formatHouseMemberSummary({
    houseId: "h1",
    memberCount: 3,
    headName: "Aria",
    heirName: "Cela",
  }),
  "3 members · Head Aria · Heir Cela",
);

assert.equal(countKinshipFamilyGroups(["a", "b", "c"], []), 3, "disconnected members each form a family group");
assert.equal(
  countKinshipFamilyGroups(["a", "b", "c"], [{ type: PARTNER_RELATIONSHIP, sourceId: "a", targetId: "b" }]),
  2,
);
assert.equal(
  countKinshipFamilyGroups(
    ["a", "b", "c"],
    [
      { type: PARENT_RELATIONSHIP, sourceId: "a", targetId: "b" },
      { type: PARENT_RELATIONSHIP, sourceId: "b", targetId: "c" },
    ],
  ),
  1,
);
assert.equal(TREE_LEGEND.disconnectedGroups(3), "3 family groups");

assert.match(ENTITY_ACTION_CONFIRM.removeMembershipMessage, /remains in Lore/);

assert.match(shell, /formatHouseMemberSummary\(houseCollectionSummaries/);
assert.match(shell, /ENTITY_ACTIONS\.openTree/);
assert.match(shell, /onOpenHouseEntry=/);
assert.match(shell, /onArchiveHouse=/);
assert.match(shell, /onRenameHouse=/);
assert.doesNotMatch(shell, /createHouseFromToolbar/);

assert.match(surface, /FamilyHousePanel/);
assert.match(surface, /FamilyMembershipDialog/);
assert.match(surface, /TREE_LEGEND\.disconnectedGroups/);
assert.match(surface, /rolesByPerson/);
assert.doesNotMatch(surface, /promptDialog/);
assert.doesNotMatch(surface, /Add people from a person neighborhood/);

assert.match(housePanel, /Add existing/);
assert.match(housePanel, /Create person/);
assert.match(housePanel, /Open full entry/);
assert.match(housePanel, /Edit membership for|aria-label=\{`Edit membership/);
assert.match(housePanel, /ENTITY_ACTION_CONFIRM\.removeMembershipMessage/);
assert.match(membershipDialog, /membershipMetadataFields/);
assert.match(membershipDialog, /createMembership/);
assert.match(membershipDialog, /Remove from House/);

assert.match(personNode, /role-badge|roleBadge/);
assert.match(landing, /formatHouseMemberSummary|houseMemberSummaries/);

const fields = membershipMetadataFields({
  module: {
    schemas: [
      {
        fields: [
          {
            relationshipType: MEMBERSHIP_RELATIONSHIP,
            metadataFields: [
              { key: "role", label: "Role", type: "enum", options: ["member", "head"] },
              { key: "notes", label: "Notes", type: "text" },
            ],
          },
        ],
      },
    ],
  },
});
assert.equal(fields[0]?.key, "role");
assert.deepEqual(defaultMembershipDraft({ role: "heir" }).role, "heir");

assert.equal(formatHouseMemberSummary(null, { pending: true }), "Loading…");
assert.match(shell, /onMembershipChanged=\{\(\) => bumpCollectionRefresh\(\)\}/);
assert.match(shell, /onBack=\{/);
assert.match(shell, /fromTree/);
assert.match(shell, /houseSummariesPending/);
assert.match(shell, /InspectorSection title="House"/);
assert.match(surface, /onMembershipChanged/);
assert.match(surface, /onBack/);
assert.doesNotMatch(landing, /createMinimalPerson/);
assert.doesNotMatch(landing, /createHouse\(/);
assert.match(landing, /listHouses\(context,/);
assert.match(membershipDialog, /MUTATION_STATUS_MESSAGES\.conflictReload/);
const menuOpenTree = rowActions.indexOf("{ENTITY_ACTIONS.openTree}");
const menuOpenIn = rowActions.indexOf("{openInText}");
assert.ok(menuOpenTree > 0 && menuOpenIn > menuOpenTree, "Open tree precedes Open in… in the menu");

console.log("houses-management checks passed");
