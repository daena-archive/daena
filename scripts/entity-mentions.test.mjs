import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compile } from "svelte/compiler";
import {
  findMentionTrigger,
  mentionLabelForInsert,
  mentionRangeStillSelected,
  mentionTriggerDocRange,
} from "../src/lib/editor/entity-mentions.ts";
import { looksLikeWebUrl, normalizeHref } from "../src/lib/editor/link-href.ts";

const root = resolve(import.meta.dirname, "..");

assert.deepEqual(findMentionTrigger("@Jo"), { query: "Jo", length: 3 });
assert.deepEqual(findMentionTrigger("Hello @Jo"), { query: "Jo", length: 3 });
assert.equal(findMentionTrigger("email@Jo"), null);
assert.equal(findMentionTrigger("see Jo"), null);
assert.deepEqual(findMentionTrigger("(@Jo"), { query: "Jo", length: 3 });
assert.deepEqual(findMentionTrigger("@"), { query: "", length: 1 });
assert.deepEqual(mentionTriggerDocRange(10, { query: "Jo", length: 3 }), { from: 7, to: 10 });
assert.deepEqual(mentionTriggerDocRange(4, { query: "", length: 1 }), { from: 3, to: 4 });

assert.deepEqual(mentionLabelForInsert({ entityName: "Jon Doe", selectedText: "@Jo", keepLabel: false }), {
  text: "Jon Doe",
  isCustom: false,
});
assert.deepEqual(mentionLabelForInsert({ entityName: "Jon Doe", selectedText: "the king", keepLabel: true }), {
  text: "the king",
  isCustom: true,
});
assert.deepEqual(mentionLabelForInsert({ entityName: "Jon Doe", selectedText: "Jon Doe", keepLabel: true }), {
  text: "Jon Doe",
  isCustom: false,
});
assert.deepEqual(
  mentionLabelForInsert({
    entityName: "Jon Doe",
    selectedText: "the king",
    keepLabel: true,
    requestedCustom: false,
  }),
  { text: "Jon Doe", isCustom: false },
);
assert.deepEqual(
  mentionLabelForInsert({
    entityName: "Jon Doe",
    selectedText: "@Jo",
    keepLabel: false,
    requestedCustom: true,
    requestedLabel: "Jo",
  }),
  { text: "Jo", isCustom: true },
);

assert.equal(mentionRangeStillSelected({ from: 2, to: 8 }, { from: 2, to: 8 }, true), true);
assert.equal(mentionRangeStillSelected({ from: 2, to: 2 }, { from: 2, to: 8 }, true), false);
assert.equal(mentionRangeStillSelected({ from: 2, to: 8 }, { from: 2, to: 8 }, false), false);

assert.equal(looksLikeWebUrl("example.com"), true);
assert.equal(looksLikeWebUrl("https://example.com"), true);
assert.equal(looksLikeWebUrl("the king"), false);
assert.equal(normalizeHref("example.com"), "https://example.com");
assert.equal(normalizeHref("https://example.com/a"), "https://example.com/a");
assert.equal(normalizeHref("javascript:alert(1)"), null);
assert.equal(normalizeHref("assets/file.pdf"), "assets/file.pdf");

const shell = await readFile(resolve(root, "src/lib/editor/RichTextEditor.svelte"), "utf8");
assert.match(shell, /EntityMentionMenu/, "inline mention menu is wired");
assert.match(shell, /openEntityMentionForRange/, "selection opens mention suggestions");
assert.match(shell, /mentionLabelForInsert/, "insert preserves selected labels");
assert.match(shell, /entityMentionMenu\?\.confirmActive/, "Enter confirms an inline suggestion");
assert.doesNotMatch(
  shell,
  /entityReferenceMenuOpen && \(event\.key === "Enter" \|\| event\.key === "Escape"\)[\s\S]*openEntityReferenceDialog/,
  "Enter no longer always opens the extra modal",
);
assert.match(shell, /normalizeHref/, "web URLs are normalized before apply");
assert.match(shell, /inclusive\(\)\s*\{\s*return false/, "typing after a URL leaves the link");

const menuSource = await readFile(resolve(root, "src/lib/editor/EntityMentionMenu.svelte"), "utf8");
compile(menuSource, { filename: resolve(root, "src/lib/editor/EntityMentionMenu.svelte"), css: "injected" });

console.log("entity mention linking passed");
