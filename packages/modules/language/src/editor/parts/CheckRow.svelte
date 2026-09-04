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
  <div class="grammar-check-list">
    {#each options as option (option.value)}
      <label class="grammar-check" class:is-selected={selected.includes(option.value)}>
        <input
          type="checkbox"
          {name}
          value={option.value}
          checked={selected.includes(option.value)}
          disabled={locked}
          onchange={() => ontoggle(option.value)} />
        <span>{option.label}</span>
      </label>
    {/each}
  </div>
</fieldset>

<style>
.grammar-checks {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  border: 0;
}
.grammar-checks legend {
  padding: 0;
  padding-bottom: 6px;
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.grammar-check-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.grammar-check {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-height: 34px;
  padding: 0 12px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 8px;
  background: var(--theme-surface-bg, var(--surface));
  color: var(--ink);
  font-size: 12px;
  cursor: pointer;
}
.grammar-check.is-selected {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--surface-muted);
}
.grammar-check input {
  margin: 0;
  accent-color: var(--accent-dark, var(--accent));
}
.grammar-check:focus-within {
  outline: 3px solid var(--focus-ring, rgba(180, 119, 63, 0.24));
  outline-offset: 2px;
}
</style>
