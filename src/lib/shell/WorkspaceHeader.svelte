<script lang="ts">
import type { Snippet } from "svelte";

let {
  kicker,
  title,
  description,
  actions,
}: {
  kicker: string;
  title: string;
  description: string;
  actions?: Snippet;
} = $props();
</script>

<header class="workspace-heading">
  <div>
    <span class="overline">{kicker}</span>
    <h1>{title}</h1>
    <p>{description}</p>
  </div>
  {#if actions}
    <div class="heading-actions">{@render actions()}</div>
  {/if}
</header>

<style>
.workspace-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
  padding: 32px 40px 25px;
}
.overline {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
.workspace-heading h1 {
  margin: 8px 0 4px;
  color: var(--ink);
  font: 500 38px/1 var(--font-display);
}
.workspace-heading p {
  margin: 0;
  color: var(--ink-soft);
  font-size: 13px;
}
.heading-actions {
  display: flex;
  gap: 7px;
}

@media (max-width: 1040px) {
  .workspace-heading {
    padding: 28px 28px 16px;
  }
}

@media (max-width: 760px) {
  .workspace-heading {
    align-items: flex-start;
    flex-direction: column;
    padding: 20px 17px 12px;
  }
  .workspace-heading h1 {
    font-size: clamp(31px, 10vw, 38px);
  }
  .workspace-heading p {
    max-width: 38ch;
    line-height: 1.5;
  }
  .heading-actions {
    width: 100%;
  }
}
</style>
