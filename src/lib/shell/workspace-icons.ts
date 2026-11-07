import type { WorkspaceSection } from "$lib/modules/workspace";
import { CalendarRange, Castle, Languages, Library, Map as MapIcon, Pencil } from "@lucide/svelte";

export function workspaceIcon(section: WorkspaceSection) {
  if (section === "lore") return Library;
  if (section === "timeline") return CalendarRange;
  if (section === "writing") return Pencil;
  if (section === "language") return Languages;
  if (section === "houses") return Castle;
  return MapIcon;
}
