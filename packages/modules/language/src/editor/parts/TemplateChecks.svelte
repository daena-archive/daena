<script lang="ts">
let {
  legend,
  options,
  selected,
  locked = false,
  ontoggle,
}: {
  legend: string;
  options: { id: string; label: string; meaning?: string; example?: string }[];
  selected: Set<string | undefined> | readonly string[];
  locked?: boolean;
  ontoggle: (id: string) => void;
} = $props();

function isSelected(id: string) {
  return selected instanceof Set ? selected.has(id) : selected.includes(id);
}
</script>

<fieldset class="grammar-checks">
  <legend>{legend}</legend>
  <div class="grammar-check-list">
    {#each options as option (option.id)}
      <label class="grammar-check" class:is-selected={isSelected(option.id)}>
        <input
          type="checkbox"
          value={option.id}
          checked={isSelected(option.id)}
          disabled={locked}
          onchange={() => ontoggle(option.id)} />
        <span>
          <strong>{option.label}</strong>
          {#if option.meaning}<small>{option.meaning}</small>{/if}
          {#if option.example}<em>{option.example}</em>{/if}
        </span>
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
  display: grid;
  gap: 8px;
}
.grammar-check {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  min-height: 34px;
  padding: 8px 12px;
  border: 1px solid var(--theme-neutral-border, var(--line));
  border-radius: 8px;
  background: var(--theme-surface-bg, var(--surface));
  color: var(--ink);
  cursor: pointer;
}
.grammar-check.is-selected {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--surface-muted);
}
.grammar-check input {
  margin: 3px 0 0;
  accent-color: var(--accent-dark, var(--accent));
}
.grammar-check span {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.grammar-check strong {
  font-size: 12px;
  font-weight: 600;
}
.grammar-check small,
.grammar-check em {
  color: var(--ink-muted);
  font-size: 11px;
  line-height: 1.4;
}
.grammar-check em {
  font-style: italic;
}
.grammar-check:focus-within {
  outline: 3px solid var(--focus-ring, rgba(180, 119, 63, 0.24));
  outline-offset: 2px;
}
</style>
