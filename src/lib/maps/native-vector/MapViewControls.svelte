<script lang="ts">
let {
  zoom,
  min = 0,
  max = 8,
  onzoom,
  onreset,
}: {
  zoom: number;
  min?: number;
  max?: number;
  onzoom: (zoom: number) => void;
  onreset: () => void;
} = $props();
</script>

<div class="map-view-controls" aria-label="Map zoom">
  <label>
    Zoom
    <input
      type="range"
      {min}
      {max}
      step="0.05"
      value={zoom}
      aria-label="Zoom"
      oninput={(event) => onzoom(Number(event.currentTarget.value))} />
  </label>
  <button type="button" onclick={onreset}>Reset view</button>
</div>

<style>
.map-view-controls {
  position: absolute;
  z-index: 2;
  right: 10px;
  bottom: 10px;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border: 1px solid #405047;
  border-radius: 8px;
  background: rgb(27 40 34 / 92%);
  color: #d8e3d9;
  font-size: 12px;
}
label {
  display: flex;
  align-items: center;
  gap: 6px;
}
input[type="range"] {
  width: 120px;
  accent-color: #d5ab6c;
}
button {
  border: 0;
  border-radius: 6px;
  padding: 4px 8px;
  background: #31443b;
  color: #edf2ec;
  font: 700 12px system-ui;
  cursor: pointer;
}
button:focus-visible,
input:focus-visible {
  outline: 2px solid #f3d39a;
  outline-offset: 2px;
}
</style>
