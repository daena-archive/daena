<script lang="ts">
import { onMount } from "svelte";
import { project, type Entity } from "$lib/project/client";
import { htmlToMarkdown, markdownToPlainText } from "$lib/markdown";

export let entities: Entity[] = [];
export let onOpen: (entity: Entity) => void = () => {};

let activeEntity: Entity | null = null;
let position = { top: 0, left: 0 };
let previewById: Record<string, string> = {};
let hideTimer: number | null = null;

function previewText(body: string, format: string): string {
  const markdown = format === "rich-text" ? htmlToMarkdown(body) : body;
  const plainText = markdownToPlainText(markdown);
  return plainText.length > 240 ? `${plainText.slice(0, 237).trimEnd()}...` : plainText;
}

function cancelHide() {
  if (hideTimer !== null) window.clearTimeout(hideTimer);
  hideTimer = null;
}

function scheduleHide() {
  cancelHide();
  hideTimer = window.setTimeout(() => {
    activeEntity = null;
    hideTimer = null;
  }, 150);
}

async function showFor(target: HTMLElement) {
  const id = target.dataset.entityId;
  const entity = entities.find((candidate) => candidate.id === id && !candidate.deleted);
  if (!entity) return;
  cancelHide();
  activeEntity = entity;
  const bounds = target.getBoundingClientRect();
  position = {
    top: Math.min(window.innerHeight - 12, bounds.bottom + 8),
    left: Math.min(window.innerWidth - 12, Math.max(12, bounds.left)),
  };
  if (Object.hasOwn(previewById, entity.id)) return;
  previewById = { ...previewById, [entity.id]: "" };
  try {
    const [fields, document] = await Promise.all([project.listFields(entity.id), project.listDocuments(entity.id)]);
    const summary = fields.find(
      (field) => field.key === "summary" && typeof field.value === "string" && field.value.trim(),
    );
    const source = summary
      ? String(summary.value)
      : document[0]
        ? previewText(document[0].body, document[0].format)
        : "";
    previewById = { ...previewById, [entity.id]: previewText(source, "markdown") };
  } catch {
    previewById = { ...previewById, [entity.id]: "" };
  }
}

onMount(() => {
  const relatedTargetIsCard = (target: EventTarget | null) =>
    target instanceof Node && document.querySelector(".entity-hover-card")?.contains(target);
  const resolveTarget = (target: EventTarget | null) =>
    target instanceof Element
      ? target.closest<HTMLElement>("a[data-entity-id], .relationship-chip[data-entity-id]")
      : null;
  const onMouseOver = (event: MouseEvent) => {
    const target = resolveTarget(event.target);
    if (target) void showFor(target);
  };
  const onMouseOut = (event: MouseEvent) => {
    const target = resolveTarget(event.target);
    if (target && !target.contains(event.relatedTarget as Node | null) && !relatedTargetIsCard(event.relatedTarget))
      scheduleHide();
  };
  const onFocusIn = (event: FocusEvent) => {
    const target = resolveTarget(event.target);
    if (target) void showFor(target);
  };
  document.addEventListener("mouseover", onMouseOver);
  document.addEventListener("mouseout", onMouseOut);
  document.addEventListener("focusin", onFocusIn);
  return () => {
    document.removeEventListener("mouseover", onMouseOver);
    document.removeEventListener("mouseout", onMouseOut);
    document.removeEventListener("focusin", onFocusIn);
    cancelHide();
  };
});
</script>

{#if activeEntity}
  <section
    class="entity-hover-card"
    style={`top: ${position.top}px; left: ${position.left}px;`}
    aria-label={`${activeEntity.name} preview`}
    onmouseenter={cancelHide}
    onmouseleave={scheduleHide}>
    <span>{activeEntity.entity_type ?? "Uncategorized"}</span>
    <strong>{activeEntity.name}</strong>
    {#if previewById[activeEntity.id]}
      <p>{previewById[activeEntity.id]}</p>
    {:else}
      <p class="entity-hover-empty">No document preview available.</p>
    {/if}
    <button type="button" onclick={() => onOpen(activeEntity!)}>Open entity</button>
  </section>
{/if}

<style>
.entity-hover-card {
  position: fixed;
  z-index: 70;
  width: min(300px, calc(100vw - 24px));
  padding: 12px;
  border: 1px solid var(--line, #e4e1d8);
  border-radius: 9px;
  background: var(--surface, #fffefa);
  box-shadow: 0 14px 34px rgba(38, 42, 33, 0.16);
}
.entity-hover-card > span {
  display: block;
  margin-bottom: 4px;
  color: var(--accent, #b4773f);
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.entity-hover-card strong {
  display: block;
  color: var(--ink, #25251f);
  font: 700 15px/1.2 var(--font-display, Georgia, serif);
}
.entity-hover-card p {
  display: -webkit-box;
  margin: 7px 0 10px;
  overflow: hidden;
  color: var(--ink-soft, #77766d);
  font-size: 11px;
  line-height: 1.45;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
  line-clamp: 3;
}
.entity-hover-card p.entity-hover-empty {
  color: var(--ink-faint, #aaa79d);
}
.entity-hover-card button {
  border: 0;
  padding: 0;
  background: transparent;
  color: var(--accent-dark, #365342);
  font: 700 11px/1.2 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.entity-hover-card button:hover,
.entity-hover-card button:focus-visible {
  color: var(--accent, #b4773f);
  outline: 0;
  text-decoration: underline;
}
</style>
