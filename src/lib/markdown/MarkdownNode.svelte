<script lang="ts">
import MarkdownNode from "./MarkdownNode.svelte";
import { nodeText } from "./text.ts";
import { safeHref, safeSrc } from "./urls.ts";

type MdNode = {
  type: string;
  children?: MdNode[];
  value?: string;
  depth?: number;
  align?: string;
  lang?: string | null;
  url?: string;
  alt?: string;
  entityId?: string;
  ordered?: boolean;
  start?: number | null;
  checked?: boolean | null;
  data?: { hProperties?: { id?: string } };
};

export let node: MdNode;
export let entityIds: Set<string> = new Set();
export let onOpenEntity: (id: string) => void = () => {};

function headingId(): string {
  return node.data?.hProperties?.id ?? "";
}

function headingTag(): "h1" | "h2" | "h3" | "h4" | "h5" | "h6" {
  const depth = Number(node.depth ?? 1);
  return `h${Math.min(6, Math.max(1, depth))}` as "h1" | "h2" | "h3" | "h4" | "h5" | "h6";
}

function children(): MdNode[] {
  return node.children ?? [];
}

function alignStyle(): string {
  return node.align === "center" || node.align === "right" ? `text-align: ${node.align}` : "";
}

function language(): string {
  return node.lang ?? "";
}

function value(): string {
  return node.value ?? "";
}

function url(): string {
  return node.url ?? "";
}

function alt(): string {
  return node.alt ?? "";
}

function entityId(): string {
  return node.entityId ?? "";
}

function ordered(): boolean {
  return Boolean(node.ordered);
}

function start(): number | undefined {
  return node.start == null ? undefined : node.start;
}

function checked(): boolean | null {
  return node.checked ?? null;
}

function openEntity(event: MouseEvent) {
  event.preventDefault();
  const id = entityId();
  if (id) onOpenEntity(id);
}
</script>

{#if node.type === "heading"}
  <svelte:element this={headingTag()} id={headingId()}>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </svelte:element>
{:else if node.type === "paragraph" || node.type === "alignedParagraph"}
  <p style={alignStyle()}>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </p>
{:else if node.type === "blockquote"}
  <blockquote>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </blockquote>
{:else if node.type === "list"}
  {#if ordered()}
    <ol start={start()}>
      {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
    </ol>
  {:else}
    <ul>
      {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
    </ul>
  {/if}
{:else if node.type === "listItem"}
  <li>
    {#if checked() !== null}<input type="checkbox" checked={checked() === true} disabled />{/if}
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </li>
{:else if node.type === "thematicBreak"}
  <hr />
{:else if node.type === "code"}
  <pre><code class={language() ? `language-${language()}` : ""}>{value()}</code></pre>
{:else if node.type === "table"}
  <table>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </table>
{:else if node.type === "tableRow"}
  <tr>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </tr>
{:else if node.type === "tableCell"}
  <td>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </td>
{:else if node.type === "text"}
  {value()}
{:else if node.type === "break"}
  <br />
{:else if node.type === "strong"}
  <strong>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </strong>
{:else if node.type === "emphasis"}
  <em>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </em>
{:else if node.type === "delete"}
  <s>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </s>
{:else if node.type === "underline"}
  <u>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </u>
{:else if node.type === "inlineCode"}
  <code>{value()}</code>
{:else if node.type === "link"}
  {@const href = safeHref(url())}
  {#if href}
    <a {href} rel="noreferrer">
      {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
    </a>
  {:else}
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  {/if}
{:else if node.type === "entityReference"}
  {@const id = entityId()}
  {@const missing = !entityIds.has(id)}
  <a
    href={`daena://entity/${encodeURIComponent(id)}`}
    class="entity-reference"
    class:entity-reference-missing={missing}
    data-entity-id={id}
    title={missing ? "Missing entity" : undefined}
    onclick={openEntity}>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  </a>
{:else if node.type === "image"}
  {@const src = safeSrc(url())}
  {#if src}<img {src} alt={alt()} />{:else}{alt()}{/if}
{:else}
  {#each children() as child}<MarkdownNode node={child} {entityIds} {onOpenEntity} />{/each}
  {#if !children().length}{nodeText(node)}{/if}
{/if}
