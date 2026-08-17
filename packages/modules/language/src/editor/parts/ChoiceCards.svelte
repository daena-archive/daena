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
</fieldset>

<style>
.grammar-choices {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(168px, 1fr));
  gap: 8px;
  margin: 0;
  padding: 0;
  border: 0;
}
.grammar-choices legend {
  padding: 0;
  color: var(--ink-soft);
  font-size: 11px;
}
.grammar-choice {
  display: grid;
  gap: 4px;
  align-content: start;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--surface);
  cursor: pointer;
}
.grammar-choice.is-selected {
  border-color: var(--accent-dark);
  background: var(--surface-muted);
}
.grammar-choice input {
  margin: 0;
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
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
</style>
