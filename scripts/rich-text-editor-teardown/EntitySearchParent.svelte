<script lang="ts">
import RichTextEditor from "$lib/editor/RichTextEditor.svelte";
import type { AsyncEntitySearchFn } from "$lib/entity-lifecycle/asyncEntityQuery.ts";

// Mounted like the Language panes: no host prop, and no searchEntities prop unless the test passes one.
let {
  entityId = null,
  explicitSearch = null,
}: { entityId?: string | null; explicitSearch?: AsyncEntitySearchFn | null } = $props();
let value = $state("");
</script>

<RichTextEditor {value} {entityId} searchEntities={explicitSearch} onChange={(next) => (value = next)} />
