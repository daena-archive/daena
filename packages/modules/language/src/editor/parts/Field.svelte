<script lang="ts">
let { label, children }: { label: string; children?: import("svelte").Snippet } = $props();
</script>

<label class="language-field language-field-prompt">
  <span>{label}</span>
  {@render children?.()}
</label>
