export type NativeVectorSessionApi = {
  save: () => Promise<void>;
  isDirty: () => boolean;
  teardown: () => void;
};

let api: NativeVectorSessionApi | null = null;

export function registerNativeVectorSession(next: NativeVectorSessionApi | null) {
  api = next;
}

export function nativeVectorSession(): NativeVectorSessionApi | null {
  return api;
}
