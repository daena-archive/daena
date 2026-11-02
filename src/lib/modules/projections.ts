import type { DaenaModule } from "../../../packages/module-api/src/index";
import { lore } from "../../../packages/modules/lore/src/index";
import { language } from "../../../packages/modules/language/src/index";

export type ProjectionModuleId = "lore" | "language";
export type ProjectionKind = "graph" | "timeline" | "language";

const projectionModules: Record<
  ProjectionModuleId,
  { title: string; subtitle: string; kind: ProjectionKind; module: DaenaModule }
> = {
  lore: { title: "World graph", subtitle: "Explore entities and their connections", kind: "graph", module: lore },
  language: { title: "Language", subtitle: "Explore language systems and samples", kind: "language", module: language },
};

export function projectionModule(id: ProjectionModuleId) {
  return projectionModules[id];
}
