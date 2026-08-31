import { getContext, setContext, type Snippet } from "svelte";
import type { BranchDirection } from "./model.ts";

const KEY = Symbol("tree-canvas-host");

export type TreeCanvasHost = {
  avatar?: Snippet<[string, string]>;
  onSelectPerson: (id: string | null) => void;
  onMakeRoot: (id: string) => void;
  onToggleBranch: (id: string, direction: BranchDirection) => void;
  onAddUnionChild: (memberIds: string[]) => void;
  onSelectRelationship: (id: string | null) => void;
};

export function setTreeCanvasHost(host: TreeCanvasHost) {
  setContext(KEY, host);
}

export function getTreeCanvasHost(): TreeCanvasHost {
  return getContext(KEY);
}
