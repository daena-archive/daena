import type { DaenaModule } from "../../../packages/module-api/src/index";
import { lore } from "../../../packages/modules/lore/src/index";

export type ProjectionModuleId = "lore";
export type ProjectionKind = "graph" | "timeline";

const projectionModules: Record<
  ProjectionModuleId,
  { title: string; subtitle: string; kind: ProjectionKind; module: DaenaModule }
> = {
  lore: { title: "World graph", subtitle: "Explore entities and their connections", kind: "graph", module: lore },
};

export function projectionModule(id: ProjectionModuleId) {
  return projectionModules[id];
}
