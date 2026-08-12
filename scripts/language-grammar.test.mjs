import assert from "node:assert/strict";
import {
  grammarLinkMarkup,
  grammarMarkdownToHtml,
  groupGrammarTopics,
  normalizeGrammarTopic,
} from "../packages/modules/language/src/grammar.ts";

const topic = normalizeGrammarTopic({
  title: " Word order ",
  section: "word-order",
  body: "# Order\n\nDefault is **SVO** with [[sol]](lexeme:f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11).\n\n- nouns first",
  links: [{ kind: "lexeme", lexemeId: "f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11", label: "sol" }],
});
assert.equal(topic.title, "Word order");
assert.equal(topic.section, "word-order");
assert.equal(topic.links[0].lexemeId, "f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11");
assert.equal(normalizeGrammarTopic({ title: "X", section: "mystery" }).section, "other");

const html = grammarMarkdownToHtml(topic.body);
assert.match(html, /<h1>Order<\/h1>/);
assert.match(html, /<strong>SVO<\/strong>/);
assert.match(html, /data-lexeme-id="f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11"/);
assert.match(html, /<li>nouns first<\/li>/);

const unsafe = grammarMarkdownToHtml("<script>alert(1)</script>\n[click](javascript:alert(1))\n**ok**");
assert.match(unsafe, /&lt;script&gt;/);
assert.equal(unsafe.includes("<script>"), false);
assert.equal(unsafe.includes('href="javascript:'), false);
assert.match(unsafe, /<strong>ok<\/strong>/);

const grouped = groupGrammarTopics([
  { value: topic },
  { value: normalizeGrammarTopic({ title: "Tense", section: "verb", body: "", links: [] }) },
]);
assert.equal(grouped[0].id, "word-order");
assert.equal(grouped[0].topics.length, 1);
assert.equal(grouped[3].id, "verb");
assert.equal(grouped[3].topics[0].value.title, "Tense");

assert.equal(
  grammarLinkMarkup({
    id: "1",
    kind: "example",
    lexemeId: "f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11",
    exampleId: "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    label: "sol oritur",
  }),
  "[[sol oritur]](example:f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11:a1b2c3d4-e5f6-7890-abcd-ef1234567890)",
);

console.log("language grammar helpers ok");
