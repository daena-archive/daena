export type AtlasRenderCompletionTracker = {
  watch: (
    subscribe: (complete: () => void) => void,
    render: () => void,
    isCurrent: () => boolean,
    onComplete: () => void,
  ) => void;
  invalidate: () => void;
};

export function createAtlasRenderCompletionTracker(): AtlasRenderCompletionTracker {
  let generation = 0;
  return {
    watch(subscribe, render, isCurrent, onComplete) {
      const current = ++generation;
      subscribe(() => {
        if (current !== generation || !isCurrent()) return;
        onComplete();
      });
      render();
    },
    invalidate() {
      generation += 1;
    },
  };
}
