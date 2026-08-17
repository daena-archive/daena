export type PartOption<T extends string = string> = {
  value: T;
  label: string;
  expansion?: string;
  example?: string;
};
