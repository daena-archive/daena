import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../src/lib/GitSettingsPanel.svelte", import.meta.url), "utf8");

const createSnapshot = source.indexOf("<h3>Create snapshot</h3>");
const snapshotHistory = source.indexOf("<h3>Snapshot history</h3>");
const repositoryTools = source.indexOf("<strong>Sync & repository</strong>");

assert.ok(createSnapshot >= 0, "the primary Create snapshot workflow is present");
assert.ok(snapshotHistory > createSnapshot, "snapshot history follows snapshot creation");
assert.ok(repositoryTools > snapshotHistory, "sync and repository tools remain secondary to history");

assert.match(source, /aria-expanded=\{changesExpanded\}/, "the change selection can collapse");
assert.match(source, /aria-expanded=\{messageExpanded\}/, "the snapshot message editor can collapse");
assert.match(source, /commitMessageTitle \|\| "Add a message"/, "a collapsed message keeps its title visible");
assert.match(source, /snapshotBlockReason/, "the primary action exposes a concrete block reason");
assert.match(source, /Suggest message/, "deterministic message suggestions remain available");
assert.match(source, /Write with AI/, "the optional AI action has an explicit label");

assert.match(source, /toggleHistoryMessage\(entry\.hash\)/, "history rows expand on demand");
assert.match(source, /Review changes/, "expanded history exposes snapshot review");
assert.match(source, /Restore this snapshot…/, "restore remains explicit and destructive");
assert.match(source, /await selectSnapshotChange\(firstChangedPath\)/, "snapshot review selects the first change");
assert.match(source, /<summary>Technical details<\/summary>/, "commit hashes stay behind a disclosure");
assert.match(source, /<summary>Show stored path<\/summary>/, "stored file paths stay behind a disclosure");

assert.match(source, /aria-expanded=\{repositoryExpanded\}/, "sync and repository tools can collapse");
assert.match(source, /Condense history…/, "history maintenance remains available in repository tools");

console.log("snapshot settings UX contracts passed");
