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
  <div class="grammar-status-list">
    {#each options as option (option.value)}
      <label class="grammar-status-option" class:is-selected={option.value === value}>
        <input
          type="radio"
          {name}
          value={option.value}
          checked={option.value === value}
          disabled={locked}
          onchange={() => onselect(option.value)} />
        <span>{option.label}</span>
      </label>
    {/each}
  </div>
</fieldset>

<style>
.grammar-status {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  border: 0;
}
.grammar-status legend {
  padding: 0;
  padding-bottom: 6px;
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.grammar-status-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.grammar-status-option {
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
.grammar-status-option.is-selected {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--surface-muted);
}
.grammar-status-option input {
  margin: 0;
  accent-color: var(--accent-dark, var(--accent));
}
.grammar-status-option:focus-within {
  outline: 3px solid var(--focus-ring, rgba(180, 119, 63, 0.24));
  outline-offset: 2px;
}
</style>
