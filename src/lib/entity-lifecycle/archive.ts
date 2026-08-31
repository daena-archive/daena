/**
 * Helpers for shared entity archive confirmation copy.
 */

import { ENTITY_ACTION_CONFIRM, ENTITY_ACTIONS, MUTATION_STATUS } from "./vocabulary.ts";

export type ArchiveConfirmOptions = {
  title: string;
  message: string;
  confirmLabel: string;
  danger: true;
};

export function archiveConfirmOptions(name: string, options?: { dirtyNote?: string }): ArchiveConfirmOptions {
  const message = options?.dirtyNote
    ? `${ENTITY_ACTION_CONFIRM.archiveMessage(name)} ${options.dirtyNote}`
    : ENTITY_ACTION_CONFIRM.archiveMessage(name);
  return {
    title: ENTITY_ACTION_CONFIRM.archiveTitle(name),
    message,
    confirmLabel: ENTITY_ACTION_CONFIRM.archiveConfirm,
    danger: true,
  };
}

export function archivePendingLabel(busy: boolean) {
  return busy ? MUTATION_STATUS.working : ENTITY_ACTIONS.archive;
}

export function archivedToastMessage(name: string) {
  return `"${name}" archived.`;
}
