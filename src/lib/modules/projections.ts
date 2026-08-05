import type { DaenaModule } from "../../../packages/module-api/src/index";
import { lore } from "../../../packages/modules/lore/src/index";
import { timeline } from "../../../packages/modules/timeline/src/index";

export type ProjectionModuleId = "lore" | "timeline";

const projectionModules: Record<ProjectionModuleId, { title: string; module: DaenaModule }> = {
  lore: { title: "World graph", module: lore },
  timeline: { title: "Chronology", module: timeline },
};

export function projectionModule(id: ProjectionModuleId) {
  return projectionModules[id];
}
