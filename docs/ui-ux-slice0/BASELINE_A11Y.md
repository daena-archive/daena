# Baseline keyboard and screen-reader issues (Slice 0)

Recorded against the August 2026 codebase before shared lifecycle and Tree
accessibility work. Use this list to measure Slice 1 / Slice 4 progress; do not
treat these as accepted permanent behavior.

## Shell and collections

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| `A11Y-SHELL-01` | Medium | Collection rows expose selection only; common actions require opening the entity first, so keyboard users cannot Archive / Edit identity from the list. | `CollectionPane` / shell list rendering; plan §3.1 |
| `A11Y-SHELL-02` | Medium | Post-archive focus clears selection without a **View Archive** follow-up, leaving keyboard users without a recovery path. | `archiveSelected` in `+page.svelte` |
| `A11Y-SHELL-03` | Low | Global New and mobile FAB use **New entry**; plan vocabulary standardizes on **New** / contextual labels. | `AppSidebar`, mobile create button |
| `A11Y-SHELL-04` | Medium | Identity dialog kicker/title still say **EDIT ENTRY** / **Edit {name}** rather than **Edit identity**. | entity edit dialog in `+page.svelte` |
| `A11Y-SHELL-05` | Medium | Language Overview archive uses different confirmation copy and pending text (`Archiving…`) than the shell Archive confirm. | `packages/modules/language/src/panes/Overview.svelte` |

## Tree canvas and focus

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| ~~`A11Y-TREE-01`~~ | High | **Resolved in Slice 4.** Arrow selection moves DOM focus onto the Person card (`focusPersonCard`). | `FamilyTreeCanvas.svelte` |
| ~~`A11Y-TREE-02`~~ | High | **Resolved in Slice 4.** Hidden help via `aria-describedby={TREE_KEYBOARD.canvasDescribedById}`. | Canvas + `TREE_KEYBOARD` |
| ~~`A11Y-TREE-03`~~ | Medium | **Resolved in Slice 4.** Dock focuses Close on open; Escape returns focus to the origin card. | `FamilyPersonPanel`, `closeDock` |
| ~~`A11Y-TREE-04`~~ | Medium | **Resolved in Slice 4.** More menu: Escape, initial focus, focus return to trigger. | `FamilyTreeSurface` settings |
| `A11Y-TREE-05` | Medium | Root / landing listboxes lack full combobox keyboard patterns (active descendant, typeahead announcements). | `FamilyTreeLanding`, `FamilyRootPicker` |
| `A11Y-TREE-06` | Low | Relationship edge captions live on SVG `<g aria-label>` without a guaranteed keyboard path to open the relationship dock. | `FamilyRelationshipEdge.svelte` |
| `A11Y-TREE-07` | Medium | Branch chips are tabbable, but there is no announced model for moving between nodes vs activating branch controls. | `FamilyPersonNode.svelte` |
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

## Resolved in Slice 4

- `A11Y-TREE-01` … `A11Y-TREE-04`, `A11Y-TREE-08` … `A11Y-TREE-12`, `A11Y-CHROME-01`, `A11Y-CHROME-02`.
- Remaining Tree gaps: root picker combobox patterns (`A11Y-TREE-05`), edge keyboard path (`A11Y-TREE-06`), branch vs node announcement (`A11Y-TREE-07`).
