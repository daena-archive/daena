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
  dir?: string;
  lang?: string | null;
  url?: string;
  alt?: string;
  entityId?: string;
  isCustom?: boolean;
  ordered?: boolean;
  start?: number | null;
  checked?: boolean | null;
  data?: { hProperties?: { id?: string; dir?: string; style?: string } };
};

export let node: MdNode;
export let entityIds: Set<string> = new Set();
export let entities: Array<{ id: string; name: string; deleted?: boolean }> = [];
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
  if (node.data?.hProperties?.style && /^text-align\s*:\s*(?:left|center|right)/i.test(node.data.hProperties.style))
    return node.data.hProperties.style;
  return node.align === "center" || node.align === "right" ? `text-align: ${node.align}` : "";
}

function dir(): string {
  const fromData = node.data?.hProperties?.dir;
  if (fromData === "rtl" || fromData === "ltr") return fromData;
  const direct = (node as { dir?: string }).dir;
  if (direct === "rtl" || direct === "ltr") return direct;
  return "";
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

function isCustomEntity(): boolean {
  const flag = (node as any).isCustom;
  if (typeof flag === "boolean") return flag;
  return children().length > 0 && nodeText(node).trim().length > 0;
}

function entityDisplayName(): string {
  const id = entityId();
  const ent = entities.find((e) => e.id === id);
  if (ent) return ent.name;
  const txt = nodeText(node).trim();
  return txt || id;
}

function openEntity(event: MouseEvent) {
  event.preventDefault();
  const id = entityId();
  if (id) onOpenEntity(id);
}
</script>

{#if node.type === "heading"}
  <svelte:element
    this={headingTag()}
    id={headingId()}
    dir={(dir() as any) || undefined}
    style={alignStyle() || (node.data?.hProperties?.style as string) || undefined}>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </svelte:element>
{:else if node.type === "paragraph" || node.type === "alignedParagraph"}
  <p style={alignStyle()} dir={(dir() as any) || undefined}>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </p>
{:else if node.type === "blockquote"}
  <blockquote>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </blockquote>
{:else if node.type === "list"}
  {#if ordered()}
    <ol start={start()}>
      {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
    </ol>
  {:else}
    <ul>
      {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
    </ul>
  {/if}
{:else if node.type === "listItem"}
  <li>
    {#if checked() !== null}<input type="checkbox" checked={checked() === true} disabled />{/if}
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </li>
{:else if node.type === "thematicBreak"}
  <hr />
{:else if node.type === "code"}
  <pre><code class={language() ? `language-${language()}` : ""}>{value()}</code></pre>
{:else if node.type === "table"}
  <table>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </table>
{:else if node.type === "tableRow"}
  <tr>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </tr>
{:else if node.type === "tableCell"}
  <td>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </td>
{:else if node.type === "text"}
  {value()}
{:else if node.type === "break"}
  <br />
{:else if node.type === "strong"}
  <strong>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </strong>
{:else if node.type === "emphasis"}
  <em>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </em>
{:else if node.type === "delete"}
  <s>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </s>
{:else if node.type === "underline"}
  <u>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </u>
{:else if node.type === "spoiler"}
  <span
    class="spoiler"
    data-spoiler
    role="button"
    tabindex="0"
    title="Click to reveal spoiler"
    aria-label="Spoiler, click to reveal"
    onclick={(event: MouseEvent) => (event.currentTarget as HTMLElement).classList.toggle("revealed")}
    onkeydown={(event: KeyboardEvent) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        (event.currentTarget as HTMLElement).classList.toggle("revealed");
      }
    }}>
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  </span>
{:else if node.type === "inlineCode"}
  <code>{value()}</code>
{:else if node.type === "link"}
  {@const href = safeHref(url())}
  {#if href}
    <a {href} rel="noreferrer">
      {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
    </a>
  {:else}
    {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  {/if}
{:else if node.type === "entityReference"}
  {@const id = entityId()}
  {@const missing = !entityIds.has(id)}
  {@const custom = isCustomEntity()}
  <a
    href={`daena://entity/${encodeURIComponent(id)}`}
    class="entity-reference"
    class:entity-reference-missing={missing}
    data-entity-id={id}
    data-is-custom={custom ? "true" : undefined}
    title={missing ? "Missing entity" : undefined}
    onclick={openEntity}>
    {#if custom}
      {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
    {:else}
      {entityDisplayName()}
    {/if}
  </a>
{:else if node.type === "image"}
  {@const src = safeSrc(url())}
  {#if src}<img {src} alt={alt()} />{:else}{alt()}{/if}
{:else}
  {#each children() as child}<MarkdownNode node={child} {entityIds} {entities} {onOpenEntity} />{/each}
  {#if !children().length}{nodeText(node)}{/if}
{/if}
