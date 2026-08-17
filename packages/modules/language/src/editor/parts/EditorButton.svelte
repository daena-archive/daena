<script lang="ts">
let {
  secondary = false,
  danger = false,
  disabled = false,
  ariaLabel,
  onclick,
  children,
}: {
  secondary?: boolean;
  danger?: boolean;
  disabled?: boolean;
  ariaLabel?: string;
  onclick?: (event: MouseEvent) => void;
  children?: import("svelte").Snippet;
} = $props();
let classes = $derived("language-button" + (secondary ? " secondary" : "") + (danger ? " language-danger" : ""));
</script>

<button type="button" class={classes} {disabled} aria-label={ariaLabel} {onclick}>{@render children?.()}</button>
