import type { LayoutGraph, PositionedGraph, PositionedNode } from "./model.ts";

export const ELK_LAYOUT_OPTIONS = {
  "elk.algorithm": "layered",
  "elk.direction": "DOWN",
  "elk.layered.spacing.nodeNodeBetweenLayers": "90",
  "elk.spacing.nodeNode": "42",
  "elk.spacing.edgeNode": "24",
  "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
  "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
  "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
} as const;

export interface ElkNodeInput {
  id: string;
  width: number;
  height: number;
  children?: ElkNodeInput[];
  edges?: ElkEdgeInput[];
  layoutOptions?: Record<string, string>;
  x?: number;
  y?: number;
}

export interface ElkEdgeInput {
  id: string;
  sources: string[];
  targets: string[];
}

export interface LayoutRequest {
  generation: number;
  graph: ElkNodeInput;
}

export interface LayoutResponse {
  generation: number;
  ok: boolean;
  message?: string;
  graph?: ElkNodeInput;
}

export function isCurrentGeneration(current: number, result: number): boolean {
  return current === result && current > 0;
}

export function nextGeneration(current: number): number {
  return current + 1;
}

export function buildElkGraph(layout: LayoutGraph, previousOrder: string[] = []): ElkNodeInput {
  const order = new Map(previousOrder.map((id, index) => [id, index]));
  const nodes = [...layout.nodes].sort((left, right) => {
    const leftOrder = order.get(left.id);
    const rightOrder = order.get(right.id);
    if (leftOrder !== undefined || rightOrder !== undefined) {
      return (leftOrder ?? Number.MAX_SAFE_INTEGER) - (rightOrder ?? Number.MAX_SAFE_INTEGER);
    }
    return left.id.localeCompare(right.id);
  });
  const edges = [...layout.edges].sort((left, right) => left.id.localeCompare(right.id));
  return {
    id: "family-tree",
    width: 0,
    height: 0,
    layoutOptions: { ...ELK_LAYOUT_OPTIONS },
    children: nodes.map((node) => ({
      id: node.id,
      width: node.width,
      height: node.height,
    })),
    edges: edges.map((edge) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  };
}

export function positionedFromElk(generation: number, layout: LayoutGraph, elk: ElkNodeInput): PositionedGraph {
  const positions = new Map<string, { x: number; y: number }>();
  for (const child of elk.children ?? []) {
    positions.set(child.id, { x: child.x ?? 0, y: child.y ?? 0 });
  }
  const nodes: PositionedNode[] = layout.nodes.map((node) => ({
    ...node,
    x: positions.get(node.id)?.x ?? 0,
    y: positions.get(node.id)?.y ?? 0,
  }));
  return { generation, nodes, edges: layout.edges };
}

export class LayoutGeneration {
  private current = 0;

  get value() {
    return this.current;
  }

  start(): number {
    this.current += 1;
    return this.current;
  }

  accept(generation: number): boolean {
    return isCurrentGeneration(this.current, generation);
  }
}
