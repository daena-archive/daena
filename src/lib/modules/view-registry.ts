import type { ModuleView, ModuleContext, WorldbuilderModule, ModuleId } from "../../../packages/module-api/src/index";
import { mount, type SvelteComponent } from "svelte";

interface MountedView {
  moduleId: ModuleId;
  viewId: string;
  cleanup: () => void;
}

class ViewRegistry {
  private mountedViews = new Map<string, MountedView>();

  mountView(
    moduleId: ModuleId,
    view: ModuleView,
    element: HTMLElement,
    context: ModuleContext
  ): () => void {
    const key = `${moduleId}:${view.id}`;
    this.unmountView(key);
    const cleanup = view.mount(element, context);
    this.mountedViews.set(key, { moduleId, viewId: view.id, cleanup });
    return cleanup;
  }

  unmountView(key: string): void {
    const entry = this.mountedViews.get(key);
    if (entry) {
      entry.cleanup();
      this.mountedViews.delete(key);
    }
  }

  unmountAllForModule(moduleId: ModuleId): void {
    for (const [key, entry] of this.mountedViews) {
      if (entry.moduleId === moduleId) {
        entry.cleanup();
        this.mountedViews.delete(key);
      }
    }
  }

  isMounted(moduleId: ModuleId, viewId: string): boolean {
    return this.mountedViews.has(`${moduleId}:${viewId}`);
  }

  listMounted(): Array<{ moduleId: ModuleId; viewId: string }> {
    return [...this.mountedViews.values()].map((v) => ({
      moduleId: v.moduleId,
      viewId: v.viewId,
    }));
  }
}

export const viewRegistry = new ViewRegistry();