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
.grammar-template-hint {
  color: var(--ink-faint);
  font-size: 11px;
}
</style>
