export const ENTITY_FIELDS_TAB_IDS = ["relationships", "profile", "assets", "maps", "backlinks"] as const;

export type EntityFieldsTabId = (typeof ENTITY_FIELDS_TAB_IDS)[number];

export type EntityFieldsTab = {
  id: EntityFieldsTabId;
  label: string;
  count?: number;
};
