import assert from "node:assert/strict";
import {
  defaultHiddenGraphRelations,
  isHouseGraphRelation,
  relationTypeLabel,
} from "../packages/modules/lore/src/graphFilters.ts";

assert.equal(isHouseGraphRelation("family_parent_of"), true);
assert.equal(isHouseGraphRelation("family_partner_with"), true);
assert.equal(isHouseGraphRelation("family_member_of"), true);
assert.equal(isHouseGraphRelation("family_godparent_of"), true);
assert.equal(isHouseGraphRelation("located_in"), false);
assert.equal(isHouseGraphRelation("member_of"), false);

const hidden = defaultHiddenGraphRelations(["located_in", "family_parent_of", "ally_of", "family_partner_with"]);
assert.deepEqual([...hidden].sort(), ["family_parent_of", "family_partner_with"]);
assert.equal(relationTypeLabel("family_parent_of"), "Parent");
assert.equal(relationTypeLabel("located_in"), "located in");

console.log("lore graph relation filters passed");
