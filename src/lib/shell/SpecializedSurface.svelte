<script lang="ts">
import { tick, type Snippet } from "svelte";

interface Props {
  restoreKey: string;
  restoreScrollTop: number;
  children: Snippet;
  element?: HTMLElement | null;
  onScroll: (scrollTop: number) => void;
}

let { restoreKey, restoreScrollTop, children, element = $bindable(null), onScroll }: Props = $props();

$effect(() => {
  void restoreKey;
  const target = restoreScrollTop;
  void tick().then(() => {
    if (element) element.scrollTop = target;
  });
});
</script>

<section
  bind:this={element}
  class="specialized-surface"
  data-surface-key={restoreKey}
  onscroll={() => onScroll(element?.scrollTop ?? 0)}>
  {@render children()}
</section>

<style>
.specialized-surface {
  display: flex;
  width: 100%;
  height: calc(100vh - 58px);
  min-width: 0;
  min-height: 0;
  flex: 1 1 auto;
  flex-direction: column;
  overflow: auto;
}
@media (max-width: 760px) {
  .specialized-surface {
    height: auto;
    min-height: calc(100vh - 104px);
    overflow: visible;
  }
}
</style>
