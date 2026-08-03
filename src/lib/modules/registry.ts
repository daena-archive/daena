import type { ModuleContext, ModuleId, WorldbuilderModule } from "../../../packages/module-api/src/index";
import { lore, timeline } from "../../../packages/modules/index";
import { viewRegistry } from "./view-registry";

export const bundledModules: WorldbuilderModule[] = [lore, timeline];

export function entityTypesFor(moduleId: ModuleId): string[] {
  const module = bundledModules.find((candidate) => candidate.manifest.id === moduleId);
  return module?.manifest.schemas.flatMap((schema) => schema.entityTypes) ?? [];
}

export class ModuleRegistry {
  private readonly modules = new Map<ModuleId, WorldbuilderModule>();
  private readonly enabled = new Set<ModuleId>();

  register(module: WorldbuilderModule): void {
    if (this.modules.has(module.manifest.id)) {
      throw new Error(`Duplicate module: ${module.manifest.id}`);
    }
    this.modules.set(module.manifest.id, module);
  }

  list(): WorldbuilderModule[] {
    return [...this.modules.values()];
  }

  isEnabled(id: ModuleId): boolean {
    return this.enabled.has(id);
  }

  async enable(id: ModuleId, context: ModuleContext): Promise<void> {
    const module = this.modules.get(id);
    if (!module) throw new Error(`Unknown module: ${id}`);
    if (this.enabled.has(id)) return;
    await module.register?.(context);
    this.enabled.add(id);
  }

  disable(id: ModuleId): void {
    if (!this.modules.has(id)) throw new Error(`Unknown module: ${id}`);
    viewRegistry.unmountAllForModule(id);
    this.enabled.delete(id);
  }
}
