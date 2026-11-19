import { localizeUi } from "../i18n/ui.ts";
/**
 * Helpers for shared entity archive confirmation copy.
 */

import { en } from "../i18n/en.ts";
import { ENTITY_ACTIONS, MUTATION_STATUS } from "./vocabulary.ts";

export type ArchiveConfirmOptions = {
  title: string;
  message: string;
  confirmLabel: string;
  danger: true;
};

export function archiveConfirmOptions(
  name: string,
  options?: { dirtyNote?: string; copy?: typeof en.inspector },
): ArchiveConfirmOptions {
  const copy = options?.copy ?? en.inspector;
  const message = options?.dirtyNote ? `${copy.archiveMessage(name)} ${options.dirtyNote}` : copy.archiveMessage(name);
  return {
    title: copy.archiveTitle(name),
    message,
    confirmLabel: copy.archive,
    danger: true,
  };
}

export function archivePendingLabel(busy: boolean) {
  return busy ? MUTATION_STATUS.working : ENTITY_ACTIONS.archive;
}

export function archivedToastMessage(name: string) {
  return localizeUi("\"{0}\" archived.", name);
}
