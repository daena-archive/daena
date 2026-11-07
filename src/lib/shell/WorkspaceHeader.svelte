<script lang="ts">
import type { Snippet } from "svelte";

let {
  kicker,
  title,
  description,
  nav,
  layout,
  actions,
  onGuide,
  guideLabel = "Show guide",
}: {
  kicker: string;
  title: string;
  description: string;
  nav?: Snippet;
  layout?: Snippet;
  actions?: Snippet;
  onGuide?: () => void;
  guideLabel?: string;
} = $props();

const descriptionId = "workspace-heading-description";
</script>

<header class="workspace-heading" aria-describedby={description ? descriptionId : undefined}>
  <div class="heading-title">
    <span class="overline">{kicker}</span>
    <h1 title={description || undefined}>{title}</h1>
    {#if description}<p id={descriptionId} class="visually-hidden">{description}</p>{/if}
  </div>
  {#if nav}
    <div class="heading-nav">{@render nav()}</div>
  {/if}
  <div class="heading-spacer"></div>
  {#if layout || onGuide || actions}
    <div class="heading-actions">
      {#if layout}{@render layout()}{/if}
      {#if onGuide}
        <button type="button" class="workspace-guide-button" aria-label={guideLabel} onclick={onGuide}>?</button>
      {/if}
      {#if actions}{@render actions()}{/if}
    </div>
  {/if}
</header>

<style>
.workspace-heading {
  position: relative;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px 16px;
  padding: 14px 40px 12px;
}
.heading-title {
  display: flex;
  min-width: 0;
  flex: 0 1 auto;
  flex-direction: column;
}
.heading-nav {
  display: flex;
  min-width: 0;
  flex: 0 1 auto;
  align-items: center;
}
.heading-spacer {
  flex: 1 1 auto;
}
.overline {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
.workspace-heading h1 {
  margin: 3px 0 0;
  overflow: hidden;
  color: var(--ink);
  font: 500 24px/1.15 var(--font-display);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.heading-actions {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-left: auto;
}
.workspace-guide-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  border: 1px solid var(--line);
  border-radius: 50%;
  background: transparent;
  color: var(--ink-soft);
  font-size: 14px;
  font-weight: 700;
  cursor: pointer;
}
.workspace-guide-button:hover {
  border-color: var(--accent);
  background: var(--surface-muted);
  color: var(--ink);
}
.heading-actions :global(.heading-create-group) {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
}
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

@media (max-width: 1040px) {
  .workspace-heading {
    padding-inline: 28px;
  }
}

@media (max-width: 760px) {
  .workspace-heading {
    padding-inline: 17px;
  }
  .heading-title {
    flex: 1 0 100%;
  }
  .workspace-heading h1 {
    white-space: normal;
  }
}
</style>
