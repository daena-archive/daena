import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "svelte/compiler";

const root = fileURLToPath(new URL("../", import.meta.url));
const copy = JSON.parse(readFileSync(join(root, "src/lib/i18n/ui-ja.json"), "utf8"));
const messages = [];
const attrs = /^(?:title|placeholder|alt|aria-label|aria-description|description|subtitle|hint|label|message|caption|tooltip|heading|[\w]+(?:Label|Placeholder|Message|Description|Hint))$/;
const hasWords = (value) => /[a-zA-Z]{2}/.test(value);

function checkMarkup(node, file) {
  if (!node || typeof node !== "object") return;
  // Inline code documents literal API and HTML names.
  if (node.type === "RegularElement" && node.name === "code") return;
  if (node.type === "Fragment") {
    for (const child of node.nodes) {
      if (child.type === "Text" && hasWords(child.data)) messages.push([file, child.data.trim()]);
      else checkMarkup(child, file);
    }
    return;
  }
  if (node.type === "Attribute") {
    if (attrs.test(node.name) && Array.isArray(node.value)) {
      const text = node.value.filter((part) => part.type === "Text").map((part) => part.data).join("");
      if (hasWords(text)) messages.push([file, node.name, text]);
    }
    return;
  }
  for (const [key, value] of Object.entries(node)) {
    if (["expression", "loc", "metadata", "name_loc", "test"].includes(key)) continue;
    if (Array.isArray(value)) value.forEach((child) => checkMarkup(child, file));
    else if (value && typeof value === "object") checkMarkup(value, file);
  }
}

function checkDirectory(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) checkDirectory(path);
    else if (/\.(?:ts|svelte)$/.test(path)) {
      const src = readFileSync(path, "utf8");
      const file = relative(root, path);
      if (path.endsWith(".svelte")) checkMarkup(parse(src, { modern: true }).fragment, file);
      for (const match of src.matchAll(/(?:\$ui|localizeUi)\(\s*("(?:\\.|[^"\\])*")/g)) {
        const source = JSON.parse(match[1]);
        if (!Object.hasOwn(copy, source)) messages.push([file, "missing translation", source]);
      }
    }
  }
}

checkDirectory(join(root, "src"));
checkDirectory(join(root, "packages/modules"));
assert.deepEqual(messages, [], "Application copy should use the active language catalog");
const placeholders = (text) => (text.match(/\{\d+\}/g) ?? []).sort();
for (const [english, japanese] of Object.entries(copy)) {
  assert.deepEqual(placeholders(japanese), placeholders(english), english);
}
console.log("Application UI copy and interpolation checks passed");
