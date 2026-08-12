import assert from "node:assert/strict";
import { consonantChart, normalizePhoneme, vowelChart } from "../packages/modules/language/src/phonology.ts";
import { normalizeOrthography } from "../packages/modules/language/src/orthography.ts";

const p = normalizePhoneme({ symbol: " t ", kind: "consonant", place: "Alveolar", manner: "plosive" });
assert.equal(p.symbol, "t");
assert.equal(p.kind, "consonant");
const consonants = consonantChart([
  p,
  normalizePhoneme({ symbol: "d", kind: "consonant", place: "alveolar", manner: "plosive", voicing: "voiced" }),
  normalizePhoneme({ symbol: "ʔ", kind: "consonant" }),
]);
assert.deepEqual(consonants.columns, ["alveolar"]);
assert.equal(consonants.cells[0].items.length, 2);
assert.equal(consonants.unplaced[0].symbol, "ʔ");

const vowels = vowelChart([
  normalizePhoneme({ symbol: "i", kind: "vowel", height: "close", backness: "front" }),
  normalizePhoneme({ symbol: "ə", kind: "vowel" }),
]);
assert.equal(vowels.cells[0].items[0].symbol, "i");
assert.equal(vowels.unplaced[0].symbol, "ə");

const orthography = normalizeOrthography({
  name: " High ",
  mappings: [{ grapheme: "zh", sounds: "ʒ, dʒ" }, { grapheme: "  " }],
});
assert.equal(orthography.name, "High");
assert.deepEqual(orthography.mappings[0].sounds, ["ʒ", "dʒ"]);
assert.equal(orthography.mappings.length, 1);

console.log("language phonology helpers ok");
