<script lang="ts">
import type { IconRef } from "../../../packages/plugin-sdk/src/generated";
import { catalogIcon, FALLBACK_ICON, pluginIconUrl, userIconUrl } from "./catalog";

let {
  icon = FALLBACK_ICON,
  pluginId = null,
  size = 18,
  strokeWidth = 1.8,
  label = null,
}: {
  icon?: IconRef | null;
  pluginId?: string | null;
  size?: number;
  strokeWidth?: number;
  label?: string | null;
} = $props();

let failed = $state(false);
const effectiveIcon = $derived(icon ?? FALLBACK_ICON);
const catalogComponent = $derived.by(() => {
  if (failed) return catalogIcon("unknown");
  return effectiveIcon.kind === "catalog" ? catalogIcon(effectiveIcon.id) : null;
});
const svgUrl = $derived(
  !failed && effectiveIcon.kind === "plugin-svg" && pluginId
    ? pluginIconUrl(pluginId, effectiveIcon.path)
    : !failed && effectiveIcon.kind === "user-svg"
      ? userIconUrl(effectiveIcon.svg)
      : null,
);

$effect(() => {
  void icon;
  void pluginId;
  failed = false;
});
</script>

{#if catalogComponent}
  {@const Icon = catalogComponent}
  <Icon
    {size}
    {strokeWidth}
    aria-hidden={label ? undefined : "true"}
    aria-label={label ?? undefined}
    role={label ? "img" : undefined} />
{:else if svgUrl}
  <span
    class="plugin-svg-icon"
    style={`width:${size}px;height:${size}px;--entity-icon-url:url("${svgUrl}")`}
    aria-hidden={label ? undefined : "true"}
    aria-label={label ?? undefined}
    role={label ? "img" : undefined}>
    <img src={svgUrl} alt="" onerror={() => (failed = true)} />
  </span>
{:else}
  {@const Icon = catalogIcon("unknown")}
  <Icon {size} {strokeWidth} aria-hidden={label ? undefined : "true"} aria-label={label ?? undefined} />
{/if}

<style>
.plugin-svg-icon {
  position: relative;
  display: inline-block;
  flex: 0 0 auto;
  background: currentColor;
  mask: var(--entity-icon-url) center / contain no-repeat;
  -webkit-mask: var(--entity-icon-url) center / contain no-repeat;
}
.plugin-svg-icon img {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  opacity: 0;
  pointer-events: none;
}
</style>
