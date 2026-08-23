import assert from "node:assert/strict";
import {
  IPA_SECTIONS,
  IPA_SYMBOLS,
  insertIpaAtSelection,
  searchIpaSymbols,
} from "../packages/modules/language/src/ipa.ts";
import {
  countPhonemeReferences,
  mappingFromPhoneme,
  normalizeOrthography,
  orthographyCoverage,
  representedPhonemeIds,
  validateOrthography,
} from "../packages/modules/language/src/orthography.ts";
import { consonantChart, normalizePhoneme, vowelChart } from "../packages/modules/language/src/phonology.ts";

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

assert.deepEqual(new Set(IPA_SYMBOLS.map((entry) => entry.section)), new Set(IPA_SECTIONS));
assert.equal(searchIpaSymbols("ʃ")[0].name, "Voiceless postalveolar fricative");
assert.equal(
  searchIpaSymbols("velar nasal").some((entry) => entry.symbol === "ŋ"),
  true,
);
assert.equal(
  searchIpaSymbols("long").some((entry) => entry.symbol === "ː"),
  true,
);
assert.deepEqual(insertIpaAtSelection("ta", "ʃ", 1, 1), { value: "tʃa", cursor: 2 });
assert.deepEqual(insertIpaAtSelection("ta", "ʃ", 0, 1), { value: "ʃa", cursor: 1 });

const orthography = normalizeOrthography({
  name: " Common Script ",
  direction: "ltr",
  description: " Everyday use ",
  mappings: [
    {
      id: "m1",
      writtenForm: "sh",
      sounds: [{ kind: "phoneme", phonemeId: "sound-sh", symbol: "ʃ" }],
      romanization: "sh",
      group: "consonants",
    },
    {
      id: "m2",
      writtenForm: "x",
      sounds: [
        { kind: "phoneme", phonemeId: "sound-k", symbol: "k" },
        { kind: "ipa", value: "s" },
      ],
      group: "other",
    },
  ],
  samples: [{ id: "sample-1", writtenText: "Shara.", pronunciation: "ʃara", translation: "River." }],
});
assert.equal(orthography.name, "Common Script");
assert.equal(orthography.direction, "ltr");
assert.equal(orthography.mappings[0].writtenForm, "sh");
assert.equal(orthography.mappings[1].sounds[1].kind, "ipa");
assert.equal(orthography.samples[0].pronunciation, "ʃara");
assert.deepEqual([...representedPhonemeIds(orthography)], ["sound-sh", "sound-k"]);
assert.equal(countPhonemeReferences(orthography, "sound-sh"), 1);
assert.equal(countPhonemeReferences(orthography, "missing"), 0);
assert.deepEqual(orthographyCoverage(orthography, ["sound-sh", "sound-k", "sound-ng"]), {
  represented: 2,
  total: 3,
  unmapped: ["sound-ng"],
});
assert.equal(validateOrthography(orthography), null);
assert.equal(
  validateOrthography({ ...orthography, mappings: [{ ...orthography.mappings[0], writtenForm: "" }] }),
  "Every character mapping needs a written form.",
);

const vowelMapping = mappingFromPhoneme({ id: "sound-a", symbol: "a", ipa: "ɑ", kind: "vowel" });
assert.equal(vowelMapping.group, "vowels");
assert.deepEqual(vowelMapping.sounds, [{ kind: "phoneme", phonemeId: "sound-a", symbol: "ɑ" }]);

console.log("language phonology, IPA, and writing helpers ok");
