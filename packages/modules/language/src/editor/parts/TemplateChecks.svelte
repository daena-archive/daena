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
</script>

<fieldset class="grammar-checks">
  <legend>{legend}</legend>
  {#each options as option (option.id)}
    <label>
      <input
        type="checkbox"
        value={option.id}
        checked={selected instanceof Set ? selected.has(option.id) : selected.includes(option.id)}
        disabled={locked}
        onchange={() => ontoggle(option.id)} />
      {option.label}
      {#if option.meaning}<span class="grammar-template-hint">{option.meaning}</span>{/if}
      {#if option.example}<em class="grammar-template-hint">{option.example}</em>{/if}
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
.grammar-template-hint {
  color: var(--ink-faint);
  font-size: 11px;
}
</style>
