import { flushSync, mount } from "svelte";
import { insideReactiveUpdate } from "$lib/editor/reactive-context.svelte.ts";
import LegacyParent from "./LegacyParent.svelte";
import RunesParent from "./RunesParent.svelte";

export type EntityChange = (entityId: string, value: string) => void;

export interface ParentHandle {
  setShown(next: boolean): void;
  setEntity(next: string): void;
  flushEditor(): void;
}

export function mountRunesParent(target: HTMLElement, onChange: EntityChange): ParentHandle {
  return mount(RunesParent, { target, props: { onChange } }) as ParentHandle;
}

export function mountLegacyParent(target: HTMLElement, onChange: EntityChange): ParentHandle {
  return mount(LegacyParent, { target, props: { onChange } }) as ParentHandle;
}

export { flushSync, insideReactiveUpdate };
