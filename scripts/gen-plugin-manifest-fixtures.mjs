import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
// Overridable so a drift check can regenerate into a temp directory.
const outDir = process.env.DAENA_FIXTURES_OUT_DIR ?? join(root, "schemas", "fixtures", "manifest");
const lore = JSON.parse(readFileSync(join(root, "packages/modules/lore/manifest.json"), "utf8"));

const fixtures = [
  {
    rule: "valid",
    expected: "accepted",
    mutate: (m) => m,
  },
  {
    rule: "unknown-key",
    expected: "rejected",
    mutate: (m) => {
      m.unexpected = true;
      return m;
    },
  },
  {
    rule: "bad-version",
    expected: "rejected",
    mutate: (m) => {
      m.version = "1.0";
      return m;
    },
  },
  {
    rule: "bad-host-api",
    expected: "rejected",
    mutate: (m) => {
      m.hostApi = "banana";
      return m;
    },
  },
  {
    rule: "bad-package-path",
    expected: "rejected",
    mutate: (m) => {
      m.entrypoints.ui = "dist/./index.html";
      return m;
    },
  },
  {
    rule: "unknown-capability",
    expected: "rejected",
    mutate: (m) => {
      m.capabilities.push("fly.to");
      return m;
    },
  },
  {
    rule: "unowned-namespace",
    expected: "rejected",
    mutate: (m) => {
      m.namespaces = ["lore2"];
      return m;
    },
  },
  {
    rule: "field-unknown-entity-type",
    expected: "rejected",
    mutate: (m) => {
      m.schemas[0].fields[2].entityTypes = ["ghost"];
      return m;
    },
  },
  {
    rule: "duplicate-field-key",
    expected: "rejected",
    mutate: (m) => {
      m.schemas[0].fields.push(m.schemas[0].fields[0]);
      return m;
    },
  },
  {
    rule: "relationship-incomplete",
    expected: "rejected",
    mutate: (m) => {
      delete m.schemas[0].fields[3].targetEntityTypes;
      return m;
    },
  },
  {
    rule: "relationship-duplicate-target",
    expected: "rejected",
    mutate: (m) => {
      m.schemas[0].fields[3].targetEntityTypes = ["place", "place"];
      return m;
    },
  },
  {
    rule: "template-undeclared-field",
    expected: "rejected",
    mutate: (m) => {
      m.templates[0].fields.mystery = "";
      return m;
    },
  },
  {
    rule: "template-inapplicable-field",
    expected: "rejected",
    mutate: (m) => {
      m.templates[0].fields.region = "";
      return m;
    },
  },
  {
    rule: "template-bad-preset",
    expected: "rejected",
    mutate: (m) => {
      m.templates[0].fields.summary = 42;
      return m;
    },
  },
  {
    rule: "template-bad-required",
    expected: "rejected",
    mutate: (m) => {
      m.templates[0].requiredFields.push("mystery");
      return m;
    },
  },
  {
    rule: "duplicate-template-id",
    expected: "rejected",
    mutate: (m) => {
      m.templates.push(m.templates[0]);
      return m;
    },
  },
  {
    rule: "view-empty-title",
    expected: "rejected",
    mutate: (m) => {
      m.views.push({ id: "empty", title: "", components: [] });
      return m;
    },
  },
  {
    rule: "command-empty-title",
    expected: "rejected",
    mutate: (m) => {
      m.commands[0].title = "";
      return m;
    },
  },
];

mkdirSync(outDir, { recursive: true });
const index = [];
for (const fixture of fixtures) {
  const manifest = fixture.mutate(structuredClone(lore));
  const fileName = `${fixture.rule}.json`;
  writeFileSync(join(outDir, fileName), `${JSON.stringify(manifest, null, 2)}\n`);
  index.push({ rule: fixture.rule, file: fileName, expected: fixture.expected });
}
writeFileSync(join(outDir, "index.json"), `${JSON.stringify({ manifestVersion: 1, fixtures: index }, null, 2)}\n`);
console.log(`wrote ${fixtures.length} fixtures to ${outDir}`);
