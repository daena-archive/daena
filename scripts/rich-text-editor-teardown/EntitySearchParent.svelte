<script lang="ts">
import { untrack } from "svelte";
import RichTextEditor from "$lib/editor/RichTextEditor.svelte";
import type { AsyncEntitySearchFn } from "$lib/entity-lifecycle/asyncEntityQuery.ts";

// Mounted like the Language panes: no host prop, and no searchEntities prop unless the test passes one.
// initialValue seeds the document once, for the entity-reference scenarios. entities is passed only when a test
// sets it; the Language panes pass none.
let {
  entityId = null,
  entities = [],
  explicitSearch = null,
  initialValue = "",
}: {
  entityId?: string | null;
  entities?: any[];
  explicitSearch?: AsyncEntitySearchFn | null;
  initialValue?: string;
} = $props();
let value = $state(untrack(() => initialValue));
</script>

<RichTextEditor {value} {entityId} {entities} searchEntities={explicitSearch} onChange={(next) => (value = next)} />
