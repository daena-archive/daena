import assert from "node:assert/strict";
import {
  applyOperation,
  clearOverride,
  generatedForm,
  normalizeParadigm,
  pinOverride,
  previewParadigm,
} from "../packages/modules/language/src/morphology.ts";

const paradigm = normalizeParadigm({
  name: " Regular verb ",
  kind: "inflection",
  slots: [
    { id: "s1", label: "1sg" },
    { id: "s2", label: "3sg" },
    { id: "s3", label: "past" },
  ],
  rules: [
    {
      id: "r-default",
      name: "default",
      operations: [
        { id: "o1", slotId: "s1", op: "suffix", value: "o" },
        { id: "o2", slotId: "s2", op: "suffix", value: "a" },
        { id: "o3", slotId: "s3", op: "suffix", value: "is" },
      ],
    },
    {
      id: "r-ar",
      name: "-ar verbs",
      match: "ar",
      operations: [
        { id: "o4", slotId: "s1", op: "replace-suffix", from: "ar", value: "o" },
        { id: "o5", slotId: "s2", op: "replace-suffix", from: "ar", value: "a" },
        { id: "o6", slotId: "s3", op: "replace-suffix", from: "ar", value: "is" },
      ],
    },
  ],
});
assert.equal(paradigm.name, "Regular verb");
assert.equal(normalizeParadigm({ name: "X", kind: "mystery" }).kind, "inflection");
assert.equal(applyOperation("sol", { id: "x", slotId: "s1", op: "prefix", value: "re" }), "resol");
assert.equal(applyOperation("sol", { id: "x", slotId: "s1", op: "identity" }), "sol");

const regular = generatedForm(paradigm, "sol", "s1");
assert.equal(regular?.form, "solo");
assert.equal(regular?.rule.name, "default");
const ar = generatedForm(paradigm, "cantar", "s1");
assert.equal(ar?.form, "canto");
assert.equal(ar?.rule.name, "-ar verbs");

const authored = [{ id: "f1", form: "fui", kind: "past", paradigmId: "p1", slotId: "s3", provenance: "override" }];
const before = structuredClone(authored);
const preview = previewParadigm(paradigm, "cantar", authored, "p1");
assert.equal(preview[0].provenance, "generated");
assert.equal(preview[0].form, "canto");
assert.equal(preview[2].provenance, "authored");
assert.equal(preview[2].form, "fui");
assert.equal(preview[2].generated, "cantis");

paradigm.rules[1].operations[2].value = "ió";
const afterRuleChange = previewParadigm(paradigm, "cantar", authored, "p1");
assert.equal(afterRuleChange[2].form, "fui");
assert.equal(afterRuleChange[2].generated, "cantió");
assert.deepEqual(authored, before);

const pinned = pinOverride([], "p1", paradigm.slots[0], "canto");
assert.equal(pinned[0].provenance, "override");
assert.equal(pinned[0].slotId, "s1");
assert.equal(clearOverride(pinned, "p1", paradigm.slots[0]).length, 0);

console.log("language morphology helpers ok");
