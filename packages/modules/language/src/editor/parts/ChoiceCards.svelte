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

<fieldset class="grammar-choices">
  <legend>{legend}</legend>
  <div class="grammar-choice-grid">
    {#each options as option (option.value)}
      <label class="grammar-choice" class:is-selected={option.value === value}>
        <input
          type="radio"
          {name}
          value={option.value}
          checked={option.value === value}
          disabled={locked}
          onchange={() => onselect(option.value)} />
        <strong>{option.label}</strong>
        {#if option.expansion}<span>{option.expansion}</span>{/if}
        {#if option.example}<em>{option.example}</em>{/if}
      </label>
    {/each}
  </div>
</fieldset>

<style>
.grammar-choices {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  border: 0;
}
.grammar-choices legend {
  padding: 0;
  padding-bottom: 6px;
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.grammar-choice-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(148px, 1fr));
  gap: 8px;
}
.grammar-choice {
  display: grid;
  gap: 4px;
  align-content: start;
  padding: 12px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 10px;
  background: var(--theme-surface-bg, var(--surface));
  cursor: pointer;
}
.grammar-choice.is-selected {
  border-color: var(--theme-success-border, var(--accent));
  background: color-mix(in srgb, var(--theme-success-bg, var(--surface-muted)) 70%, var(--surface));
}
.grammar-choice input {
  margin: 0;
  accent-color: var(--accent-dark, var(--accent));
}
.grammar-choice span,
.grammar-choice em {
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.45;
}
.grammar-choice em {
  font-style: italic;
}
.grammar-choice:focus-within {
  outline: 3px solid var(--focus-ring, rgba(180, 119, 63, 0.24));
  outline-offset: 2px;
}
</style>
