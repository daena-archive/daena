<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  fullscreen: boolean;
  mapEditorActive: boolean;
  children: Snippet;
  element?: HTMLElement | null;
  onScroll?: () => void;
}

let { fullscreen, mapEditorActive, children, element = $bindable(null), onScroll = () => {} }: Props = $props();
</script>

<article
  bind:this={element}
  class:editor-fullscreen={fullscreen}
  class:map-editor-active={mapEditorActive}
  class="editor-panel"
  onscroll={onScroll}>
  {@render children()}
</article>

<style>
.editor-panel {
  min-width: 0;
  min-height: 650px;
  overflow: clip;
  padding: 24px 25px 18px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.map-editor-active {
  display: flex;
  min-height: 0;
  height: 100%;
  flex-direction: column;
  overflow: hidden;
  padding: 0;
}
.editor-fullscreen {
  position: fixed;
  inset: 0 0 0 248px;
  z-index: 30;
  display: flex;
  min-height: 100vh;
  flex-direction: column;
  overflow: auto;
  padding: 28px clamp(22px, 5vw, 72px) 24px;
  border: 0;
  border-radius: 0;
  background: var(--canvas);
  box-shadow: 0 24px 80px rgba(37, 37, 31, 0.18);
}
.editor-fullscreen.map-editor-active {
  inset: 0;
  padding: 0;
}
.editor-fullscreen :global(.editor-header) {
  width: min(1120px, 100%);
  flex: 0 0 auto;
  align-self: center;
}
.editor-fullscreen :global(.editor-shell) {
  display: grid;
  width: min(1120px, 100%);
  min-height: 0;
  flex: 1 1 auto;
  align-self: center;
  grid-template-rows: auto auto minmax(0, 1fr) auto;
}
.editor-fullscreen :global(.editor-content) {
  overflow: auto;
}
.editor-fullscreen :global(.editor-footer) {
  width: min(1120px, 100%);
  flex: 0 0 auto;
  align-self: center;
  padding-top: 12px;
}
.editor-fullscreen :global(.editor-header h2) {
  font-size: 32px;
}
.editor-fullscreen :global(.map-editor-shell) {
  width: 100%;
  align-self: center;
}
@media (max-width: 760px) {
  .editor-panel {
    width: 100%;
    min-height: auto;
    padding: 18px 14px 14px;
    border-radius: 11px;
  }
  .editor-fullscreen {
    inset: 0;
    padding: 16px 14px 12px;
  }
  .editor-fullscreen :global(.editor-header) {
    min-height: 58px;
  }
  .editor-fullscreen :global(.editor-header h2) {
    font-size: 24px;
  }
}
</style>
