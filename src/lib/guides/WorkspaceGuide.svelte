<script lang="ts">
import { tick } from "svelte";
import type { GuideStep } from "./types.ts";

let {
  open = false,
  steps,
  stepIndex = 0,
  onStepIndex,
  onDismiss,
  onComplete,
  onPrimary,
}: {
  open?: boolean;
  steps: GuideStep[];
  stepIndex?: number;
  onStepIndex: (index: number) => void;
  onDismiss: () => void;
  onComplete: () => void;
  onPrimary?: (step: GuideStep) => void | Promise<void>;
} = $props();

const step = $derived(steps[Math.min(stepIndex, Math.max(steps.length - 1, 0))]);
const isFirst = $derived(stepIndex <= 0);
const isLast = $derived(stepIndex >= steps.length - 1);
const primaryLabel = $derived(
  step?.waitForTarget ? (step.primaryLabel ?? "Skip this step") : (step?.primaryLabel ?? (isLast ? "Done" : "Next")),
);

let highlight = $state<{ top: number; left: number; width: number; height: number } | null>(null);
let card = $state<{ top: number; left: number } | null>(null);

function targetEl(): HTMLElement | null {
  if (!step?.target || typeof document === "undefined") return null;
  const el = document.querySelector<HTMLElement>(step.target);
  if (!el) return null;
  const rect = el.getBoundingClientRect();
  if (rect.width < 2 || rect.height < 2) return null;
  return el;
}

function measure() {
  const el = targetEl();
  if (!el) {
    highlight = null;
    card = { top: Math.max(24, window.innerHeight / 2 - 90), left: Math.max(16, window.innerWidth / 2 - 160) };
    return;
  }
  const rect = el.getBoundingClientRect();
  const pad = 6;
  highlight = {
    top: rect.top - pad,
    left: rect.left - pad,
    width: rect.width + pad * 2,
    height: rect.height + pad * 2,
  };
  const cardWidth = 320;
  const cardHeight = 220;
  let left = rect.left;
  if (left + cardWidth > window.innerWidth - 16) left = window.innerWidth - cardWidth - 16;
  if (left < 16) left = 16;
  let top = rect.bottom + 12;
  if (top + cardHeight > window.innerHeight - 16) top = Math.max(16, rect.top - cardHeight - 12);
  card = { top, left };
}

async function handlePrimary() {
  if (!step) return;
  const pausing = step.action === "pause";
  const finishing = step.action === "complete" || isLast;
  await onPrimary?.(step);
  await tick();
  if (pausing) return;
  if (finishing) {
    onComplete();
    return;
  }
  onStepIndex(Math.min(stepIndex + 1, steps.length - 1));
}

function handleBack() {
  if (!isFirst) onStepIndex(stepIndex - 1);
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    onDismiss();
  }
}

function onDocClick(event: MouseEvent) {
  if (!open || !step?.waitForTarget) return;
  if (event.target instanceof Element && event.target.closest(".guide-card")) return;
  const el = targetEl();
  if (!el || !(event.target instanceof Node) || !el.contains(event.target)) return;
  void handlePrimary();
}

$effect(() => {
  if (!open) return;
  void stepIndex;
  void steps;
  void tick().then(measure);
  const onWin = () => measure();
  window.addEventListener("resize", onWin);
  window.addEventListener("scroll", onWin, true);
  document.addEventListener("click", onDocClick, true);
  return () => {
    window.removeEventListener("resize", onWin);
    window.removeEventListener("scroll", onWin, true);
    document.removeEventListener("click", onDocClick, true);
  };
});
</script>

{#if open && step}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="guide-root" onkeydown={handleKeydown}>
    {#if !highlight}
      <div class="guide-dim"></div>
    {/if}
    {#if highlight}
      <div
        class="guide-spot"
        style={`top:${highlight.top}px;left:${highlight.left}px;width:${highlight.width}px;height:${highlight.height}px`}>
      </div>
    {/if}
    {#if card}
      <div
        class="guide-card"
        role="dialog"
        aria-modal="false"
        aria-labelledby="workspace-guide-title"
        style={`top:${card.top}px;left:${card.left}px`}>
        <p class="guide-kicker">{stepIndex + 1} of {steps.length}</p>
        <h2 id="workspace-guide-title">{step.title}</h2>
        <p>{step.body}</p>
        <div class="guide-actions">
          <button type="button" class="guide-text" onclick={onDismiss}>Skip guide</button>
          <span class="guide-actions-end">
            {#if !isFirst}
              <button type="button" class="guide-secondary" onclick={handleBack}>Back</button>
            {/if}
            <button type="button" class="guide-primary" onclick={() => void handlePrimary()}>{primaryLabel}</button>
          </span>
        </div>
      </div>
    {/if}
  </div>
{/if}

<style>
.guide-root {
  position: fixed;
  inset: 0;
  z-index: 60;
  pointer-events: none;
}
.guide-dim {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--ink) 28%, transparent);
  pointer-events: none;
}
.guide-spot {
  position: fixed;
  border: 2px solid var(--accent);
  border-radius: 10px;
  box-shadow: 0 0 0 9999px color-mix(in srgb, var(--ink) 42%, transparent);
  pointer-events: none;
}
.guide-card {
  position: fixed;
  z-index: 61;
  width: min(320px, calc(100vw - 32px));
  padding: 16px 16px 12px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  color: var(--ink);
  box-shadow: 0 12px 32px color-mix(in srgb, var(--ink) 18%, transparent);
  pointer-events: auto;
}
.guide-kicker {
  margin: 0 0 6px;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}
.guide-card h2 {
  margin: 0 0 8px;
  font: 600 16px/1.3 var(--font-display, inherit);
}
.guide-card p {
  margin: 0 0 14px;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.45;
}
.guide-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.guide-actions-end {
  display: flex;
  gap: 6px;
}
.guide-card button {
  min-height: 32px;
  padding: 0 10px;
  border-radius: 8px;
  font: 600 12px/1 var(--font-body, system-ui, sans-serif);
  cursor: pointer;
}
.guide-text {
  border: 0;
  background: transparent;
  color: var(--ink-faint);
}
.guide-secondary {
  border: 1px solid var(--line);
  background: transparent;
  color: var(--ink-soft);
}
.guide-primary {
  border: 1px solid var(--accent);
  background: var(--accent);
  color: var(--brass-ink, #2f2619);
}
</style>
