<script lang="ts">
interface Props {
  label: string;
  value: number;
  min: number;
  max: number;
  direction?: 1 | -1;
  onResize: (delta: number) => void;
  onReset: () => void;
}

let { label, value, min, max, direction = 1, onResize, onReset }: Props = $props();
let dragging = $state(false);

function startResize(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  dragging = true;
  let previousX = event.clientX;
  const move = (next: PointerEvent) => {
    const delta = (next.clientX - previousX) * direction;
    previousX = next.clientX;
    if (delta) onResize(delta);
  };
  const stop = () => {
    dragging = false;
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", stop);
    window.removeEventListener("pointercancel", stop);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", stop, { once: true });
  window.addEventListener("pointercancel", stop, { once: true });
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  event.preventDefault();
  const physicalDelta = event.key === "ArrowRight" ? 16 : -16;
  onResize(physicalDelta * direction);
}

function handleDoubleClick(event: MouseEvent) {
  event.preventDefault();
  dragging = false;
  onReset();
}
</script>

<button
  type="button"
  class:dragging
  class="pane-resize-handle"
  aria-label={`${label}. Current width ${Math.round(value)} pixels. Double-click to reset.`}
  title={`${label} (${Math.round(value)} px) · Double-click to reset`}
  tabindex="0"
  onpointerdown={startResize}
  ondblclick={handleDoubleClick}
  onkeydown={handleKeydown}>
  <span aria-hidden="true"></span>
</button>

<style>
.pane-resize-handle {
  display: grid;
  width: 10px;
  min-height: 120px;
  padding: 0;
  place-items: center;
  border: 0;
  background: transparent;
  cursor: col-resize;
  touch-action: none;
}
.pane-resize-handle span {
  width: 2px;
  height: 42px;
  border-radius: 2px;
  background: var(--line-strong);
  opacity: 0.55;
  transition:
    height 0.16s ease,
    opacity 0.16s ease,
    background 0.16s ease;
}
.pane-resize-handle:hover span,
.pane-resize-handle:focus-visible span,
.pane-resize-handle.dragging span {
  height: 64px;
  background: var(--accent-soft);
  opacity: 1;
}
.pane-resize-handle:focus-visible {
  border-radius: 6px;
  outline: 2px solid color-mix(in srgb, var(--accent-soft) 45%, transparent);
  outline-offset: -2px;
}
@media (max-width: 1180px) {
  .pane-resize-handle {
    display: none;
  }
}
</style>
