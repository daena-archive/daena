<script lang="ts">
let { children }: { children?: import("svelte").Snippet } = $props();
</script>

<div class="language-inline">{@render children?.()}</div>
