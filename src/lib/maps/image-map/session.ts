export type ImageMapSessionApi = {
  save: () => Promise<void>;
  isDirty: () => boolean;
};

let api: ImageMapSessionApi | null = null;

export function registerImageMapSession(next: ImageMapSessionApi | null) {
  api = next;
}

export function imageMapSession(): ImageMapSessionApi | null {
  return api;
}
