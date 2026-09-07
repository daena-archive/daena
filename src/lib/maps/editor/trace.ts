export type PathHit = {
  index: number;
  t: number;
  point: [number, number];
  distanceSq: number;
};

function positionsEqual(left: number[], right: number[]): boolean {
  return left[0] === right[0] && left[1] === right[1];
}

function distinctPath(points: number[][]): number[][] {
  const unique: number[][] = [];
  for (const point of points) {
    const previous = unique[unique.length - 1];
    if (!previous || !positionsEqual(previous, point)) unique.push(point);
  }
  return unique;
}

function pathLength(points: number[][]): number {
  let length = 0;
  for (let index = 0; index < points.length - 1; index += 1) {
    const dx = points[index + 1][0] - points[index][0];
    const dy = points[index + 1][1] - points[index][1];
    length += Math.hypot(dx, dy);
  }
  return length;
}

export function closestHitOnLine(line: number[][], point: readonly number[]): PathHit | null {
  if (line.length < 2) return null;
  let best: PathHit | null = null;
  for (let index = 0; index < line.length - 1; index += 1) {
    const start = line[index];
    const end = line[index + 1];
    const dx = end[0] - start[0];
    const dy = end[1] - start[1];
    const lengthSq = dx * dx + dy * dy;
    const t =
      lengthSq === 0
        ? 0
        : Math.max(0, Math.min(1, ((point[0] - start[0]) * dx + (point[1] - start[1]) * dy) / lengthSq));
    const hx = start[0] + t * dx;
    const hy = start[1] + t * dy;
    const ox = hx - point[0];
    const oy = hy - point[1];
    const distanceSq = ox * ox + oy * oy;
    if (!best || distanceSq < best.distanceSq) {
      best = { index, t, point: [hx, hy], distanceSq };
    }
  }
  return best;
}

function walkPath(line: number[][], from: PathHit, to: PathHit, edgeCount: number): number[][] {
  const points: number[][] = [from.point];
  if (from.index === to.index && from.t <= to.t) {
    if (!positionsEqual(from.point, to.point)) points.push(to.point);
    return distinctPath(points);
  }
  let edge = from.index + 1;
  for (let step = 0; step <= edgeCount; step += 1) {
    const vertex = line[edge % edgeCount];
    if (!positionsEqual(points[points.length - 1], vertex)) points.push(vertex);
    if (edge % edgeCount === to.index) break;
    edge += 1;
  }
  if (!positionsEqual(points[points.length - 1], to.point)) points.push(to.point);
  return distinctPath(points);
}

function walkOpen(line: number[][], from: PathHit, to: PathHit): number[][] {
  const start = from.index + from.t <= to.index + to.t ? from : to;
  const end = start === from ? to : from;
  const points: number[][] = [start.point];
  for (let index = start.index + 1; index <= end.index; index += 1) {
    const vertex = line[index];
    if (!positionsEqual(points[points.length - 1], vertex)) points.push(vertex);
  }
  if (!positionsEqual(points[points.length - 1], end.point)) points.push(end.point);
  return distinctPath(points);
}

export function traceSubpath(path: number[][], from: PathHit, to: PathHit, closed: boolean): number[][] {
  if (path.length < 2) return [];
  if (!closed) {
    const ordered = walkOpen(path, from, to);
    return ordered.length >= 2 ? ordered : [];
  }
  const ring = positionsEqual(path[0], path[path.length - 1]) ? path : [...path, path[0]];
  const edgeCount = ring.length - 1;
  const forward = walkPath(ring, from, to, edgeCount);
  const backward = walkPath(ring, to, from, edgeCount);
  const chosen = pathLength(forward) <= pathLength(backward) ? forward : backward;
  return chosen.length >= 2 ? chosen : [];
}

export function isClosedPath(path: number[][]): boolean {
  return path.length >= 4 && positionsEqual(path[0], path[path.length - 1]);
}
