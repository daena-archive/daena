/** A rectangle in viewport coordinates, as returned by getBoundingClientRect. */
export interface ViewportRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

function hasArea(rect: ViewportRect): boolean {
  return rect.right > rect.left && rect.bottom > rect.top;
}

/**
 * True when the two rectangles share some area. Rectangles that only touch along an edge do not overlap,
 * and a rectangle with no area (an element that is not laid out) overlaps nothing.
 */
export function rectsOverlap(a: ViewportRect, b: ViewportRect): boolean {
  if (!hasArea(a) || !hasArea(b)) return false;
  return a.left < b.right && b.left < a.right && a.top < b.bottom && b.top < a.bottom;
}

/** True when any popover rectangle overlaps any placeholder rectangle. */
export function anyRectOverlaps(popovers: readonly ViewportRect[], placeholders: readonly ViewportRect[]): boolean {
  return popovers.some((popover) => placeholders.some((placeholder) => rectsOverlap(popover, placeholder)));
}
