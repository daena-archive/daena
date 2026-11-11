// The page state a LocationContext reads that the store's effects must react to. `location-store.test.mjs` loads this
// file through the same Svelte hook as the store, so a change here re-runs the store's effects like it does on the page.
export class ReactiveContextState {
  ready = $state(false);
  projectInfo = $state<{ root: string } | null>(null);
  moduleCount = $state(0);
}
