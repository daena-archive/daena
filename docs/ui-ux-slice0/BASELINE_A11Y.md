# Baseline keyboard and screen-reader issues (Slice 0)

Recorded against the August 2026 codebase before shared lifecycle and Tree
accessibility work. Use this list to measure later-slice progress; do not treat
open rows as accepted permanent behavior.

## Shell and collections

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| ~~`A11Y-SHELL-01`~~ | Medium | **Resolved in Slice 1.** Collection rows expose Open / Edit identity / Archive / Open tree via `EntityRowActions`. | `EntityRowActions.svelte`, shell collection |
| ~~`A11Y-SHELL-02`~~ | Medium | **Resolved in Slice 1.** Post-archive offers **View Archive** follow-up with focus return. | `+page.svelte` archive feedback |
| ~~`A11Y-SHELL-03`~~ | Low | **Resolved in Slice 1.** Global New uses **New** / contextual labels (`ENTITY_ACTIONS`). | sidebar / workspace headers |
| ~~`A11Y-SHELL-04`~~ | Medium | **Resolved in Slice 1.** Identity dialog uses **Edit identity**. | `EntityIdentityDialog.svelte` |
| ~~`A11Y-SHELL-05`~~ | Medium | **Resolved in Slice 1.** Language Overview uses shared `archiveConfirmOptions` / `archivePendingLabel`. | `Overview.svelte` |

## Tree canvas and focus

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| ~~`A11Y-TREE-01`~~ | High | **Resolved in Slice 4.** Arrow selection moves DOM focus onto the Person card (`focusPersonCard`). | `FamilyTreeCanvas.svelte` |
| ~~`A11Y-TREE-02`~~ | High | **Resolved in Slice 4.** Hidden help via `aria-describedby={TREE_KEYBOARD.canvasDescribedById}`. | Canvas + `TREE_KEYBOARD` |
| ~~`A11Y-TREE-03`~~ | Medium | **Resolved in Slice 4.** Dock focuses Close on open; Escape returns focus to the origin card. | `FamilyPersonPanel`, `closeDock` |
| ~~`A11Y-TREE-04`~~ | Medium | **Resolved in Slice 4.** More menu: Escape, initial focus, focus return to trigger. | `FamilyTreeSurface` settings |
| ~~`A11Y-TREE-05`~~ | Medium | **Resolved in wrap-up.** Root picker / AsyncEntityPicker and Tree landing searches use combobox + `aria-activedescendant` arrow/Enter patterns. | `AsyncEntityPicker`, `FamilyTreeLanding`, `FamilyRootPicker` |
| ~~`A11Y-TREE-06`~~ | Low | **Resolved in wrap-up.** `R` cycles relationships of the focused person; selected edges are keyboard-activatable; person dock offers **Edit relationship**. | `FamilyTreeCanvas`, `FamilyRelationshipEdge`, `FamilyPersonPanel` |
| ~~`A11Y-TREE-07`~~ | Medium | **Resolved in wrap-up.** Person cards announce Tab-for-branches; branch chips are only tabbable on the focused person; help text documents Arrow vs Tab. | `FamilyPersonNode`, `TREE_KEYBOARD` |
| ~~`A11Y-TREE-08`~~ | Low | **Resolved in Slice 4.** Legend is no longer `aria-hidden`; scope vocabulary is readable. | `FamilyTreeSurface.svelte` |

## Tree information architecture (affects SR understanding)

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| ~~`A11Y-TREE-09`~~ | Medium | **Resolved in Slice 4.** Duplicate Secondary field removed. | Toolbar View group |
| ~~`A11Y-TREE-10`~~ | Medium | **Resolved in Slice 4.** New house lives in Create group, not More. | Toolbar |
| ~~`A11Y-TREE-11`~~ | Medium | **Resolved in Slice 4.** Expandable warning list in More menu. | settings panel |
| ~~`A11Y-TREE-12`~~ | Low | **Resolved in Slice 4.** Partnerships use “A and B”; parents keep “A → B”. | `formatRelationshipTitle` |

## Fields & Types

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| ~~`A11Y-SCHEMA-01`~~ | Medium | **Resolved in Slice 5.** Search + status filters on Types/Fields/Templates lists. | workbench toolbar |
| ~~`A11Y-SCHEMA-02`~~ | Medium | **Resolved in Slice 6.** Preview returns item-level errors; impact dialog lists kind/id/property before Save. | `SchemaOverlayPreviewResult` / `SchemaImpactReview` |
| ~~`A11Y-SCHEMA-03`~~ | Low | **Resolved in Slice 5.** Builtin fields use one row with status + Enable (no duplicate chips). | `SchemaFieldsPane` |
| ~~`A11Y-SCHEMA-04`~~ | Medium | **Resolved in Slice 6.** Live entity/field counts and impact review gate risky overlay saves. | `preview_module_schema_overlay` / `SchemaImpactReview` |

## Minimum interactive target / motion

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| ~~`A11Y-CHROME-01`~~ | Low | **Resolved in Slice 4.** Tree icon controls use 34px minimum target. | canvas controls / topbar |
| ~~`A11Y-CHROME-02`~~ | Low | **Resolved in Slice 4.** Fit / layout transitions honor `prefers-reduced-motion`. | `FamilyTreeCanvas` |

## Status

All recorded Slice 0 baseline IDs are resolved. Remaining product deferrals (Language
overlay readiness, Maps author schema, EntityCreateDialog extract, warm-cache
mentions migration) are tracked in
[`TEMP_UI_UX_ENTITY_SCHEMA_PLAN.md`](../TEMP_UI_UX_ENTITY_SCHEMA_PLAN.md) §7
deferred notes and [`MODULE_SCHEMA_COMPATIBILITY.md`](./MODULE_SCHEMA_COMPATIBILITY.md),
not as open a11y baseline rows.
