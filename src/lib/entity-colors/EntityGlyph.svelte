<script lang="ts">
import type { EntityTypeColor, IconRef } from "../../../packages/plugin-sdk/src/generated";
import EntityIcon from "$lib/entity-icons/EntityIcon.svelte";
import { DEFAULT_TYPE_COLOR, resolveEntityTypeColor } from "$lib/entity-colors/presets";
import type { ResolvedTheme } from "$lib/theme";

let {
  icon,
  iconColor = DEFAULT_TYPE_COLOR,
  pluginId = null,
  size = 14,
  box = null,
  theme: forcedTheme = null,
  class: className = "",
}: {
  icon: IconRef;
  iconColor?: EntityTypeColor;
  pluginId?: string | null;
  size?: number;
  box?: number | null;
  theme?: ResolvedTheme | null;
  class?: string;
} = $props();

let liveTheme = $state<ResolvedTheme>("light");

$effect(() => {
  const root = document.documentElement;
  const sync = () => {
    liveTheme = root.dataset.theme === "dark" ? "dark" : "light";
  };
  sync();
  const observer = new MutationObserver(sync);
  observer.observe(root, { attributes: true, attributeFilter: ["data-theme"] });
  return () => observer.disconnect();
});

const theme = $derived(forcedTheme ?? liveTheme);
const resolved = $derived(resolveEntityTypeColor(iconColor, theme));
const boxSize = $derived(box ?? (size <= 16 ? size + 8 : size + 16));
</script>

<span
  class="entity-glyph {className}"
  style={`--glyph-fg:${resolved.fg};--glyph-bg:${resolved.bg};width:${boxSize}px;height:${boxSize}px;`}>
  <EntityIcon {icon} {pluginId} {size} />
</span>

<style>
.entity-glyph {
  display: grid;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 50%;
  background: var(--glyph-bg);
  color: var(--glyph-fg);
  line-height: 1;
}
</style>
