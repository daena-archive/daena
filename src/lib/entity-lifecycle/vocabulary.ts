/**
 * Shared entity-lifecycle vocabulary for Houses / Tree / Fields & Types work.
 */

export const ENTITY_ACTIONS = {
  new: "New",
  open: "Open",
  openIn: "Open in…",
  editIdentity: "Edit identity",
  archive: "Archive",
  viewArchive: "View Archive",
  restore: "Restore",
  deletePermanently: "Delete permanently",
  openTree: "Open tree",
  openInLore: "Open in Lore",
  makeRoot: "Make root",
  newPerson: "New person",
  newHouse: "New house",
} as const;

export const ENTITY_ACTION_CONFIRM = {
  archiveTitle: (name: string) => `Archive ${name}?`,
  archiveMessage: (name: string) => `${name} will be archived and hidden from the workspace.`,
  archiveConfirm: "Archive",
  restoreTitle: (name: string) => `Restore "${name}"?`,
  restoreMessage: "Returns the entry to the workspace and search.",
  restoreConfirm: "Restore",
  deletePermanentlyTitle: (name: string) => `Delete "${name}" permanently?`,
  deletePermanentlyMessage: "Removes the entry, its content, and relationships. This cannot be undone.",
  deletePermanentlyConfirm: "Delete permanently",
  removeMembershipMessage:
    "Removes this person from the house. The person remains in Lore and is not archived or deleted.",
} as const;

/** Mutation-status vocabulary used by shared status surfaces. */
export const MUTATION_STATUS = {
  idle: "idle",
  saving: "Saving…",
  saved: "Saved",
  conflict: "conflict",
  failed: "failed",
  retry: "Retry",
  working: "Working…",
} as const;

export const MUTATION_STATUS_MESSAGES = {
  saving: "Saving…",
  saved: "Saved",
  conflictTitle: "This record changed in another view",
  conflictBody: "Reload current values, keep reviewing your draft, or retry after resolving the conflict.",
  conflictReload: "Reload current values",
  conflictReviewDraft: "Review draft",
  conflictCompare: "Compare current vs draft",
  conflictReapply: "Reapply draft onto current",
  failedTitle: "The change could not be saved",
  retry: "Retry",
  revisionConflictCode: "revision-conflict",
} as const;

/** Tree keyboard contract. Author-facing model: docs/UI_UX.md. */
export const TREE_KEYBOARD = {
  canvasRolePreference: "grouped-buttons-or-application-after-focus-model",
  canvasAriaLabel: "Tree canvas",
  canvasDescribedById: "tree-keyboard-help",
  keys: {
    tabIntoCanvas: "Tab",
    moveSelection: ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"],
    openPersonDock: "Enter",
    makeRoot: "Shift+Enter",
    openRelationship: "r",
    closeDockOrPopover: "Escape",
  },
  helpText:
    "Arrow keys move between people. Enter opens details. Shift+Enter makes the selected person the root. R opens a relationship of the selected person. Tab moves to branch controls on the focused person. Escape closes the dock or popover.",
} as const;

export const TREE_SCOPES = {
  membersOnly: {
    id: "members-only",
    label: "Members only",
    description: "Show all house members and only relationships between them.",
  },
  membersPlusImmediateFamily: {
    id: "members-plus-immediate-family",
    label: "Members + immediate family",
    description:
      "Add parents, partners, and children one hop outside the house, visually de-emphasized and bounded by the visible-person cap.",
  },
} as const;

export const TREE_LEGEND = {
  member: "Full emphasis = house member",
  outsider: "Muted = relative outside the house",
  roleBadge: "Role badge = head, heir, founder, and so on",
  disconnectedGroups: (count: number) => `${count} family groups`,
} as const;

export type EntityActionId = keyof typeof ENTITY_ACTIONS;
export type MutationStatusId = keyof typeof MUTATION_STATUS;
