import assert from "node:assert/strict";
import {
  groupSamples,
  normalizeSample,
  samplePreviewHtml,
  sampleTitle,
  tokenizeSample,
} from "../packages/modules/language/src/samples.ts";

const sample = normalizeSample({
  title: " Sunrise ",
  kind: "sentence",
  text: "sol oritur",
  translation: "the sun rises",
  transliteration: "sol oritur",
  tokens: [
    { text: "sol", gloss: "sun", grammar: "NOM", lexemeId: "f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11" },
    { text: "oritur", gloss: "rise.3sg" },
  ],
});
assert.equal(sample.title, "Sunrise");
assert.equal(sample.tokens[0].gloss, "sun");
assert.equal(normalizeSample({ text: "x", kind: "mystery" }).kind, "sentence");
assert.equal(sampleTitle({ title: "", kind: "sentence", text: "luna oritur\nnext", tokens: [] }), "luna oritur");

const original = sample.tokens.map((token) => ({ ...token }));
const retokenized = tokenizeSample("oritur sol luna", sample.tokens);
assert.equal(retokenized[0].gloss, "rise.3sg");
assert.equal(retokenized[1].gloss, "sun");
assert.equal(retokenized[1].lexemeId, "f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11");
assert.equal(retokenized[2].text, "luna");
assert.equal(retokenized[2].gloss, undefined);
assert.deepEqual(sample.tokens, original);

const html = samplePreviewHtml(sample);
assert.match(html, /sol oritur/);
assert.match(html, /data-lexeme-id="f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11"/);
assert.match(html, /the sun rises/);
assert.match(html, /rise\.3sg/);

const unsafe = samplePreviewHtml(
  normalizeSample({
    text: "<script>alert(1)</script>",
    tokens: [{ text: "<img>", gloss: "<b>x</b>" }],
  }),
);
assert.match(unsafe, /&lt;script&gt;/);
assert.equal(unsafe.includes("<script>"), false);
assert.equal(unsafe.includes("<img>"), false);

const grouped = groupSamples([
  { value: sample },
  { value: normalizeSample({ title: "Story", kind: "paragraph", text: "Longer text." }) },
]);
assert.equal(grouped[0].id, "sentence");
assert.equal(grouped[0].samples.length, 1);
assert.equal(grouped[1].samples[0].value.title, "Story");

console.log("language sample helpers ok");
