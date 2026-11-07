/** Drop index after removing `fromIndex`: before the first other tab whose centre is right of `x`. */
export function tabDropIndex(rects: Array<{ left: number; width: number }>, x: number, fromIndex: number): number {
  if (rects.length === 0) return 0;
  let insertAt = 0;
  for (let index = 0; index < rects.length; index += 1) {
    if (index === fromIndex) continue;
    const centre = rects[index].left + rects[index].width / 2;
    if (centre > x) break;
    insertAt += 1;
  }
  if (insertAt < 0) return 0;
  const last = rects.length - 1;
  return insertAt > last ? last : insertAt;
}
