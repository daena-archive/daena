<script lang="ts">
import type { Snippet } from "svelte";

let {
  content,
  position = "top",
  children,
}: {
  content: string;
  position?: "top" | "bottom" | "left" | "right";
  children: Snippet;
} = $props();

let showTooltip = $state(false);
let tooltipEl: HTMLDivElement | undefined = $state();

function handleMouseEnter() {
  showTooltip = true;
}

function handleMouseLeave() {
  showTooltip = false;
}

function handleFocus() {
  showTooltip = true;
}

function handleBlur() {
  showTooltip = false;
}
</script>

<span
  class="help-trigger"
  role="tooltip"
  aria-describedby={showTooltip ? "help-tooltip" : undefined}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  onfocus={handleFocus}
  onblur={handleBlur}
>
  {@render children()}
  {#if showTooltip}
    <div
      bind:this={tooltipEl}
      id="help-tooltip"
      class="help-tooltip"
      class:top={position === "top"}
      class:bottom={position === "bottom"}
      class:left={position === "left"}
      class:right={position === "right"}
      role="tooltip"
    >
      {content}
      <div class="help-tooltip-arrow"></div>
    </div>
  {/if}
</span>

<style>
.help-trigger {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.help-tooltip {
  position: absolute;
  z-index: 100;
  padding: 8px 12px;
  background: var(--ink);
  color: var(--surface);
  font-size: 12px;
  line-height: 1.5;
  border-radius: 8px;
  white-space: nowrap;
  max-width: 280px;
  white-space: normal;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  pointer-events: none;
}

.help-tooltip.top {
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
}

.help-tooltip.bottom {
  top: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
}

.help-tooltip.left {
  right: calc(100% + 8px);
  top: 50%;
  transform: translateY(-50%);
}

.help-tooltip.right {
  left: calc(100% + 8px);
  top: 50%;
  transform: translateY(-50%);
}

.help-tooltip-arrow {
  position: absolute;
  width: 8px;
  height: 8px;
  background: var(--ink);
  transform: rotate(45deg);
}

.help-tooltip.top .help-tooltip-arrow {
  bottom: -4px;
  left: 50%;
  margin-left: -4px;
}

.help-tooltip.bottom .help-tooltip-arrow {
  top: -4px;
  left: 50%;
  margin-left: -4px;
}

.help-tooltip.left .help-tooltip-arrow {
  right: -4px;
  top: 50%;
  margin-top: -4px;
}

.help-tooltip.right .help-tooltip-arrow {
  left: -4px;
  top: 50%;
  margin-top: -4px;
}
</style>
