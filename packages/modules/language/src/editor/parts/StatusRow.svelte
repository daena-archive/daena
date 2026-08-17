<script lang="ts">
import type { PartOption } from "./option";

let {
  name,
  legend,
  value,
  locked = false,
  options,
  onselect,
}: {
  name: string;
  legend: string;
  value: string | undefined;
  locked?: boolean;
  options: PartOption[];
  onselect: (value: string) => void;
} = $props();
</script>

<fieldset class="grammar-status">
  <legend>{legend}</legend>
  {#each options as option (option.value)}
    <label>
      <input
        type="radio"
        {name}
        value={option.value}
        checked={option.value === value}
        disabled={locked}
        onchange={() => onselect(option.value)} />
      {option.label}
    </label>
  {/each}
</fieldset>

<style>
.grammar-status {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
  border: 0;
  margin: 0;
  padding: 0;
}
.grammar-status legend {
  padding: 0;
  color: var(--ink-soft);
  font-size: 11px;
}
.grammar-status input:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
</style>
