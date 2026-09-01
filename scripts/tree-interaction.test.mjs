import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { compile } from "svelte/compiler";

const root = resolve(import.meta.dirname, "..");
const read = (path) => readFile(resolve(root, path), "utf8");

const { formatRelationshipTitle, formatRelationshipTypeLabel } = await import("../src/lib/houses/model.ts");
const { TREE_KEYBOARD, TREE_LEGEND, TREE_SCOPES } = await import("../src/lib/entity-lifecycle/vocabulary.ts");

const componentPaths = [
  "src/lib/houses/TreeCanvas.svelte",
  "src/lib/houses/FamilyPersonPanel.svelte",
  "src/lib/houses/FamilyRelationshipPanel.svelte",
  "src/lib/houses/TreeSurface.svelte",
];

for (const path of componentPaths) {
  const source = await read(path);
  compile(source, { filename: resolve(root, path), css: "injected" });
}

assert.equal(formatRelationshipTitle("partner", "Ada", "Bea"), "Ada and Bea");
assert.equal(formatRelationshipTitle("parent", "Ada", "Bea"), "Ada → Bea");
assert.equal(formatRelationshipTypeLabel({ kind: "partner", partnerKind: "marriage" }), "Marriage");
assert.equal(formatRelationshipTypeLabel({ kind: "parent", parentKind: "adoptive" }), "Adoptive parent");

assert.equal(TREE_SCOPES.membersOnly.id, "members-only");
assert.equal(TREE_SCOPES.membersPlusImmediateFamily.id, "members-plus-immediate-family");
assert.equal(TREE_KEYBOARD.canvasDescribedById, "tree-keyboard-help");
assert.match(TREE_KEYBOARD.helpText, /Arrow keys/);
assert.equal(TREE_LEGEND.outsider, "Muted = relative outside the house");

const surface = await read("src/lib/houses/TreeSurface.svelte");
const canvas = await read("src/lib/houses/TreeCanvas.svelte");
const personPanel = await read("src/lib/houses/FamilyPersonPanel.svelte");
const relationshipPanel = await read("src/lib/houses/FamilyRelationshipPanel.svelte");
const shell = await read("src/routes/+page.svelte");

assert.match(surface, /TREE_SCOPES\.membersPlusImmediateFamily/);
assert.match(surface, /aria-label="House tree scope"/);
assert.match(surface, /aria-label="Navigation"/);
assert.match(surface, /aria-label="View"/);
assert.match(surface, /aria-label="Expansion"/);
assert.match(surface, /aria-label="More view options"/);
assert.match(surface, /Show minimap/);
assert.match(surface, /Reduced detail/);
assert.match(surface, /TREE_LEGEND\.member/);
assert.match(surface, /warnings-list/);
assert.match(surface, /onEditPersonIdentity/);
assert.match(surface, /closeDock/);
assert.match(surface, /onSurfaceKeydown/);
assert.doesNotMatch(surface, /aria-label="View settings"/);
assert.equal((surface.match(/aria-label="Secondary field"/g) ?? []).length, 1, "secondary field appears once");

assert.match(surface, /scope-cap-banner/);
assert.match(surface, /queueFocusPerson/);
assert.match(surface, /selectCanvasRelationship/);
assert.match(surface, /focusCanvasOrigin/);
assert.match(surface, /houseMemberIds\.length/);
assert.doesNotMatch(surface, /\.toolbar-field span \{\s*display:\s*none/);

assert.match(canvas, /const nextNodes = flowNodes\(\)/);
assert.match(canvas, /tabIndex: personId === activePersonId/);
assert.match(canvas, /prefers-reduced-motion/);
assert.match(canvas, /media\.addEventListener/);
assert.match(canvas, /TREE_KEYBOARD\.canvasDescribedById/);
assert.match(canvas, /role="group"/);
assert.match(canvas, /openRelationshipAround|key === \"r\"/);
assert.match(canvas, /focusPersonCard/);
assert.doesNotMatch(canvas, /role="application"/);

assert.match(personPanel, /ENTITY_ACTIONS\.editIdentity/);
assert.match(personPanel, /ENTITY_ACTIONS\.archive/);
assert.match(personPanel, /data-dock-focus/);
assert.match(personPanel, /Edit relationship/);

const personNode = await read("src/lib/houses/FamilyPersonNode.svelte");
assert.match(personNode, /tabindex=\{cardTabIndex\}/);
assert.match(personNode, /branchTabIndex/);
assert.match(personNode, /Tab for branch controls/);
assert.match(personNode, /Hide \$\{units\}/);
assert.match(personNode, /Show \$\{chipText/);
assert.doesNotMatch(personNode, /--control-min-height/);

assert.match(relationshipPanel, /formatRelationshipTitle/);
assert.match(relationshipPanel, /formatRelationshipTypeLabel/);

const relationshipEdge = await read("src/lib/houses/FamilyRelationshipEdge.svelte");
assert.match(relationshipEdge, /data-relationship-id/);
assert.match(relationshipEdge, /onActivate|onkeydown=\{activateEdge\}/);
assert.match(relationshipEdge, /edgeTabIndex/);

const landing = await read("src/lib/houses/TreeLanding.svelte");
assert.match(landing, /aria-activedescendant/);
assert.match(landing, /role="combobox"/);

assert.match(TREE_KEYBOARD.helpText, /R opens a relationship|Tab moves to branch/);
assert.equal(TREE_KEYBOARD.keys.openRelationship, "r");

assert.match(shell, /onEditPersonIdentity=/);
assert.match(shell, /onArchivePerson=/);
assert.match(shell, /kinshipRefreshEpoch/);
assert.match(shell, /bumpKinshipRefresh/);
assert.match(shell, /isKinshipRelationshipType/);
assert.match(shell, /ENTITY_ACTIONS/);

assert.match(surface, /kinshipRefreshEpoch/);
assert.match(surface, /refreshKinshipData/);

console.log("tree interaction checks passed");
