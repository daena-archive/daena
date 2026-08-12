import assert from "node:assert/strict";
import {
  lexiconExport,
  normalizeLexeme,
  parseLexiconImport,
  serializeLexeme,
} from "../packages/modules/language/src/lexeme.ts";

const legacy = normalizeLexeme({
  lemma: " sol ",
  meanings: ["sun", "day"],
  pronunciation: "sol",
  example: { text: "sol oritur", translation: "the sun rises" },
});
assert.equal(legacy.lemma, "sol");
assert.equal(legacy.senses.length, 2);
assert.equal(legacy.senses[0].gloss, "sun");
assert.equal(legacy.senses[0].examples[0].text, "sol oritur");
assert.equal(legacy.pronunciations[0].value, "sol");
assert.deepEqual(legacy.meanings, ["sun", "day"]);

const exported = lexiconExport("Asteri", [legacy]);
const imported = parseLexiconImport(exported);
assert.equal(imported.length, 1);
assert.equal(imported[0].senses[1].gloss, "day");
assert.equal(serializeLexeme(imported[0]).lemma, "sol");

const wrapped = parseLexiconImport(JSON.stringify({ records: [{ value: { lemma: "luna", meanings: ["moon"] } }] }));
assert.equal(wrapped[0].senses[0].gloss, "moon");

console.log("language lexeme helpers ok");
