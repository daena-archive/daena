import { localizeUi } from "../i18n/ui.ts";
/**
 * Shared entity-lifecycle vocabulary for Houses / Tree / Fields & Types work.
 */

export const ENTITY_ACTIONS = {
  get new() { return localizeUi("New"); },
  get open() { return localizeUi("Open"); },
  get openInNewTab() { return localizeUi("Open in new tab"); },
  get openIn() { return localizeUi("Open in…"); },
  get editIdentity() { return localizeUi("Edit identity"); },
  get archive() { return localizeUi("Archive"); },
  get viewArchive() { return localizeUi("View Archive"); },
  get restore() { return localizeUi("Restore"); },
  get deletePermanently() { return localizeUi("Delete permanently"); },
  get openTree() { return localizeUi("Open tree"); },
  get openInLore() { return localizeUi("Open in Lore"); },
  get makeRoot() { return localizeUi("Make root"); },
  get newPerson() { return localizeUi("New person"); },
  get newHouse() { return localizeUi("New house"); },
  get newInManuscript() { return localizeUi("New in this manuscript"); },
} as const;

export const ENTITY_ACTION_CONFIRM = {
  archiveTitle: (name: string) => localizeUi("Archive {0}?", name),
  archiveMessage: (name: string) => localizeUi("{0} will be archived and hidden from the workspace.", name),
  get archiveConfirm() { return localizeUi("Archive"); },
  restoreTitle: (name: string) => localizeUi("Restore \"{0}\"?", name),
  get restoreMessage() { return localizeUi("Returns the entry to the workspace and search."); },
  get restoreConfirm() { return localizeUi("Restore"); },
  deletePermanentlyTitle: (name: string) => localizeUi("Delete \"{0}\" permanently?", name),
  get deletePermanentlyMessage() { return localizeUi("Removes the entry, its content, and relationships. This cannot be undone."); },
  get deletePermanentlyConfirm() { return localizeUi("Delete permanently"); },
  get removeMembershipMessage() { return localizeUi("Removes this person from the house. The person remains in Lore and is not archived or deleted."); },
} as const;

/** Mutation-status vocabulary used by shared status surfaces. */
export const MUTATION_STATUS = {
  idle: "idle",
  get saving() { return localizeUi("Saving…"); },
  get saved() { return localizeUi("Saved"); },
  conflict: "conflict",
  failed: "failed",
  get retry() { return localizeUi("Retry"); },
  get working() { return localizeUi("Working…"); },
} as const;

export const MUTATION_STATUS_MESSAGES = {
  get saving() { return localizeUi("Saving…"); },
  get saved() { return localizeUi("Saved"); },
  get conflictTitle() { return localizeUi("This record changed in another view"); },
  get conflictBody() { return localizeUi("Reload current values, keep reviewing your draft, or retry after resolving the conflict."); },
  get conflictReload() { return localizeUi("Reload current values"); },
  get conflictReviewDraft() { return localizeUi("Review draft"); },
  get conflictCompare() { return localizeUi("Compare current vs draft"); },
  get conflictReapply() { return localizeUi("Reapply draft onto current"); },
  get failedTitle() { return localizeUi("The change could not be saved"); },
  get retry() { return localizeUi("Retry"); },
  revisionConflictCode: "revision-conflict",
} as const;

/** Tree keyboard contract. Author-facing model: docs/ui/UI_UX.md. */
export const TREE_KEYBOARD = {
  canvasRolePreference: "grouped-buttons-or-application-after-focus-model",
  get canvasAriaLabel() { return localizeUi("Tree canvas"); },
  canvasDescribedById: "tree-keyboard-help",
  keys: {
    tabIntoCanvas: "Tab",
    moveSelection: ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"],
    openPersonDock: "Enter",
    makeRoot: "Shift+Enter",
    openRelationship: "r",
    closeDockOrPopover: "Escape",
  },
  get helpText() { return localizeUi("Arrow keys move between people. Enter opens details. Shift+Enter makes the selected person the root. R opens a relationship of the selected person. Tab moves to branch controls on the focused person. Escape closes the dock or popover."); },
} as const;

export const TREE_SCOPES = {
  membersOnly: {
    id: "members-only",
    get label() { return localizeUi("Members only"); },
    get description() { return localizeUi("Show all house members and only relationships between them."); },
  },
  membersPlusImmediateFamily: {
    id: "members-plus-immediate-family",
    get label() { return localizeUi("Members + immediate family"); },
    get description() { return localizeUi("Add parents, partners, and children one hop outside the house, visually de-emphasized and bounded by the visible-person cap."); },
  },
} as const;

export const TREE_LEGEND = {
  get member() { return localizeUi("Full emphasis = house member"); },
  get outsider() { return localizeUi("Muted = relative outside the house"); },
  get roleBadge() { return localizeUi("Role badge = head, heir, founder, and so on"); },
  disconnectedGroups: (count: number) => localizeUi("{0} family groups", count),
} as const;

export type EntityActionId = keyof typeof ENTITY_ACTIONS;
export type MutationStatusId = keyof typeof MUTATION_STATUS;
