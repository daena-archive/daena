<script lang="ts">
import type { Entity } from "$lib/project/client";
import { headingOutlineFromTree, parseMarkdown } from "./index.ts";
import MarkdownNode from "./MarkdownNode.svelte";

export let markdown = "";
export let entities: Entity[] = [];
export let onOpenEntity: (id: string) => void = () => {};

$: tree = parseMarkdown(markdown);
$: outline = headingOutlineFromTree(tree);
$: entityIds = new Set(entities.filter((entity) => !entity.deleted).map((entity) => entity.id));
</script>

<div class="markdown-article">
  {#if outline.length > 1}
    <nav class="markdown-toc" aria-label="Table of contents">
      <strong>On this page</strong>
      <ol>
        {#each outline as item}
          <li class={`depth-${item.depth}`}><a href={`#${item.id}`}>{item.text}</a></li>
        {/each}
      </ol>
    </nav>
  {/if}
  <div class="markdown-body">
    {#each tree.children as node}
      <MarkdownNode node={node as never} {entityIds} {entities} {onOpenEntity} />
    {/each}
  </div>
</div>

<style>
.markdown-article {
  display: grid;
  gap: 18px;
}
.markdown-toc {
  padding: 12px 14px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 8px;
  background: var(--surface-soft, #f7f4ee);
}
.markdown-toc strong {
  display: block;
  margin-bottom: 8px;
  color: var(--ink-soft, #77766d);
  font-size: 10px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.markdown-toc ol {
  margin: 0;
  padding: 0;
  list-style: none;
}
.markdown-toc li {
  margin: 4px 0;
}
.markdown-toc li.depth-2 {
  padding-left: 10px;
}
.markdown-toc li.depth-3,
.markdown-toc li.depth-4,
.markdown-toc li.depth-5,
.markdown-toc li.depth-6 {
  padding-left: 20px;
}
.markdown-toc a {
  color: var(--accent-dark, #365342);
  font-size: 12px;
  text-decoration: none;
}
.markdown-toc a:hover,
.markdown-toc a:focus-visible {
  text-decoration: underline;
}
.markdown-body :global(p) {
  margin: 0 0 1em;
}
.markdown-body :global(h1),
.markdown-body :global(h2),
.markdown-body :global(h3) {
  color: var(--ink, #25251f);
  font-family: var(--font-display, Georgia, serif);
  line-height: 1.25;
}
.markdown-body :global(a.entity-reference) {
  border-bottom: 1px solid #b4773f;
  color: var(--accent-dark, #365342);
  cursor: pointer;
  text-decoration: none;
}
.markdown-body :global(a.entity-reference-missing) {
  border-bottom-style: dashed;
  color: var(--danger, #8a3b2a);
}
.markdown-body :global(blockquote) {
  margin: 0 0 1em;
  padding-left: 12px;
  border-left: 3px solid var(--line, #e4e1d8);
  color: var(--ink-soft, #77766d);
}
.markdown-body :global(span.spoiler) {
  background: #2b2b2b;
  color: transparent;
  border-radius: 3px;
  padding: 0 4px;
  cursor: pointer;
  user-select: none;
  transition:
    color 0.15s ease,
    background 0.15s ease;
}
.markdown-body :global(span.spoiler.revealed) {
  background: #3a3a3a;
  color: var(--canvas, #f7f6f2);
}
.markdown-body :global(span.spoiler:focus-visible) {
  outline: 2px solid var(--accent, #b4773f);
  outline-offset: 2px;
}
.markdown-body :global(pre) {
  overflow: auto;
  padding: 10px 12px;
  border-radius: 6px;
  background: #f4efe6;
}
.markdown-body :global(table) {
  width: 100%;
  margin-bottom: 1em;
  border-collapse: collapse;
  font-size: 13px;
}
.markdown-body :global(td),
.markdown-body :global(th) {
  padding: 6px 8px;
  border: 1px solid var(--line, #e4e1d8);
}
</style>
