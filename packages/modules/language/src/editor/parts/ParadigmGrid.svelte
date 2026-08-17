<script lang="ts">
import { CELL_STATE_OPTIONS, cartesianCoordinates, coordKey } from "../../grammar/paradigm";
import type { ParadigmCell, ParadigmCellState, ParadigmAxis } from "../../grammar/types";

let {
  axes,
  cells,
  locked = false,
  oncell,
}: {
  axes: ParadigmAxis[];
  cells: ParadigmCell[];
  locked?: boolean;
  oncell: (cellId: string, patch: Partial<Omit<ParadigmCell, "id" | "coordinates">>) => void;
} = $props();

const rowAxes = $derived(axes.length > 1 ? axes.slice(0, -1) : axes);
const colAxis = $derived(axes.length > 1 ? axes[axes.length - 1] : undefined);
const columns = $derived(colAxis ? colAxis.values : [{ id: "", label: "" }]);
const rowCombos = $derived(cartesianCoordinates(rowAxes));
</script>

{#if axes.length === 0}
  <p class="language-empty" role="status">Select at least one distinction to generate the paradigm.</p>
{:else}
  <div class="grammar-paradigm" role="group" aria-label="Paradigm">
    <table class="grammar-paradigm-table">
      <caption class="visually-hidden">Paradigm</caption>
      <thead>
        <tr>
          <th scope="col">{rowAxes.map((axis) => axis.label).join(" · ")}</th>
          {#each columns as column (column.id)}
            <th scope="col">{column.label || colAxis?.label || "Form"}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rowCombos as row, rowIndex (rowIndex)}
          <tr>
            <th scope="row">
              {rowAxes.map((axis) => axis.values.find((value) => value.id === row[axis.id])?.label ?? "").join(" · ")}
            </th>
            {#each columns as column (column.id)}
              {@const coordinates = colAxis ? { ...row, [colAxis.id]: column.id } : row}
              {@const cell = cells.find((item) => coordKey(item.coordinates) === coordKey(coordinates))}
              <td>
                {#if cell}
                  <div class="grammar-paradigm-cell">
                    <select
                      aria-label="Cell state"
                      disabled={locked}
                      onchange={(event) => oncell(cell.id, { state: event.currentTarget.value as ParadigmCellState })}>
                      {#each CELL_STATE_OPTIONS as option (option.value)}
                        <option value={option.value} selected={option.value === cell.state}>{option.label}</option>
                      {/each}
                    </select>
                    {#if cell.state === "form"}
                      <input aria-label="Form" type="text" bind:value={cell.form} disabled={locked} />
                    {:else if cell.state === "same-as"}
                      <select
                        aria-label="Same as"
                        value={cell.sameAsCellId ?? ""}
                        disabled={locked}
                        onchange={(event) => oncell(cell.id, { sameAsCellId: event.currentTarget.value || undefined })}>
                        <option value="">Choose a form…</option>
                        {#each cells.filter((other) => other.id !== cell.id) as other (other.id)}
                          <option value={other.id}>
                            {Object.values(other.coordinates).join(" · ")}{other.form ? ` (${other.form})` : ""}
                          </option>
                        {/each}
                      </select>
                    {/if}
                    <input
                      aria-label="Notes"
                      type="text"
                      placeholder="Notes"
                      bind:value={cell.notes}
                      disabled={locked} />
                  </div>
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
.grammar-paradigm {
  overflow: auto;
  max-width: 100%;
  max-height: min(70vh, 36rem);
}
.grammar-paradigm-table {
  border-collapse: collapse;
  min-width: 100%;
}
.grammar-paradigm-table caption.visually-hidden,
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
.grammar-paradigm-table th,
.grammar-paradigm-table td {
  border: 1px solid var(--line);
  padding: 8px;
  vertical-align: top;
  text-align: left;
  background: var(--surface);
}
.grammar-paradigm-table thead th {
  position: sticky;
  top: 0;
  z-index: 2;
  background: var(--surface-muted);
}
.grammar-paradigm-table th[scope="row"] {
  position: sticky;
  left: 0;
  z-index: 1;
}
.grammar-paradigm-table thead th:first-child {
  z-index: 3;
  left: 0;
}
.grammar-paradigm-cell {
  display: grid;
  gap: 6px;
  min-width: 8rem;
}
</style>
