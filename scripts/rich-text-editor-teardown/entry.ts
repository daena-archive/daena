import { flushSync, mount, unmount } from "svelte";
import { insideReactiveUpdate } from "$lib/editor/reactive-context.svelte.ts";
import type { AsyncEntitySearchFn } from "$lib/entity-lifecycle/asyncEntityQuery.ts";
import { setDefaultEditorHost } from "$lib/editor/host";
import { markdownFromEditorHtml } from "$lib/editor/html-convert";
import EntitySearchParent from "./EntitySearchParent.svelte";
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

export function mountEntitySearchParent(
  target: HTMLElement,
  props: {
    entityId?: string | null;
    entities?: unknown[];
    explicitSearch?: AsyncEntitySearchFn | null;
    initialValue?: string;
  } = {},
) {
  return mount(EntitySearchParent, { target, props });
}

export { flushSync, insideReactiveUpdate, markdownFromEditorHtml, setDefaultEditorHost, unmount };
