import { derived, fromStore } from "svelte/store";
import { displayLanguage, type DisplayLanguage } from "./index.ts";
import { uiMessage } from "../../../packages/modules/ui-catalogs.ts";

/** Format only application-owned copy. Interpolated names and documents are opaque. */
export function formatUi(language: DisplayLanguage, source: string, ...values: unknown[]): string {
  const message = uiMessage(language, source);
  return message.replace(/\{(\d+)\}/g, (token, index) => index < values.length ? String(values[index] ?? "") : token);
}

export const ui = derived(displayLanguage, (language) => (source: string, ...values: unknown[]) =>
  formatUi(language, source, ...values),
);

const active = fromStore(ui);

export function localizeUi(source: string, ...values: unknown[]): string {
  return active.current(source, ...values);
}
