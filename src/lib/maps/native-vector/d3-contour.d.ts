declare module "d3-contour" {
  interface Contours {
    size(size: [number, number]): Contours;
    smooth(enabled: boolean): Contours;
    thresholds(values: number[]): Contours;
    (values: ArrayLike<number>): Array<{
      type: string;
      value: number;
      coordinates: number[][][][];
    }>;
  }
  export function contours(): Contours;
}
