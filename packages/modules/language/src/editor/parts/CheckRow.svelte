<script lang="ts">
import type { PartOption } from "./option";

let {
  name,
  legend,
  selected,
  locked = false,
  options,
  ontoggle,
}: {
  name: string;
  legend: string;
  selected: string[];
  locked?: boolean;
  options: PartOption[];
  ontoggle: (value: string) => void;
} = $props();
</script>

<fieldset class="grammar-checks">
  <legend>{legend}</legend>
  {#each options as option (option.value)}
    <label>
      <input
        type="checkbox"
        {name}
        value={option.value}
        checked={selected.includes(option.value)}
        disabled={locked}
        onchange={() => ontoggle(option.value)} />
      {option.label}
    </label>
  {/each}
</fieldset>

<style>
.grammar-checks {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  margin: 0;
  padding: 0;
  border: 0;
}
.grammar-checks legend {
  padding: 0;
  color: var(--ink-soft);
  font-size: 11px;
}
.grammar-checks label {
  display: grid;
  gap: 2px;
  align-content: start;
}
.grammar-checks input:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
</style>
