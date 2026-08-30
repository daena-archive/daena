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
| `A11Y-TREE-01` | High | Canvas declares `role="application"` while arrow-key selection updates React/Svelte state without moving DOM focus to the newly selected Person card. Visual selection and keyboard focus desync. | `FamilyTreeCanvas.svelte` `onCanvasKeydown` + `nearestPerson` |
| `A11Y-TREE-02` | High | No documented `aria-describedby` keyboard help for the application canvas. | Canvas root lacks described-by |
| `A11Y-TREE-03` | Medium | Enter opens/selects and Shift+Enter re-roots, but focus does not move into the Person dock; Escape dock/popover lifecycle is incomplete for View settings. | `FamilyTreeCanvas`, `FamilyTreeSurface` settings panel |
| `A11Y-TREE-04` | Medium | View settings popover is `role="dialog"` toggled in place; no initial focus move, no focus trap, outside-pointer close only. | `FamilyTreeSurface.svelte` settings panel |
| `A11Y-TREE-05` | Medium | Root / landing listboxes lack full combobox keyboard patterns (active descendant, typeahead announcements). | `FamilyTreeLanding`, `FamilyRootPicker` |
| `A11Y-TREE-06` | Low | Relationship edge captions live on SVG `<g aria-label>` without a guaranteed keyboard path to open the relationship dock. | `FamilyRelationshipEdge.svelte` |
| `A11Y-TREE-07` | Medium | Branch chips are tabbable, but there is no announced model for moving between nodes vs activating branch controls. | `FamilyPersonNode.svelte` |
| `A11Y-TREE-08` | Low | Subbar legend is `aria-hidden`, so scope/emphasis meaning is visual-only once scope lands. | `FamilyTreeSurface.svelte` |

## Tree information architecture (affects SR understanding)

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| `A11Y-TREE-09` | Medium | Duplicate **Secondary field** control in subbar and View settings increases tab stops without new information. | `FamilyTreeSurface.svelte` |
| `A11Y-TREE-10` | Medium | **New house** sits inside View settings, mixing domain mutation with rendering limits. | settings panel |
| `A11Y-TREE-11` | Medium | Malformed/skipped edges surface mainly as counts in settings, hard to investigate with SR. | Tree warnings / settings footnote patterns |
| `A11Y-TREE-12` | Low | Partnership titles use directional arrows unsuitable for undirected relationships. | relationship dock / edge captions |

## Fields & Types

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| `A11Y-SCHEMA-01` | Medium | Very long Types/Fields/Templates lists have no search/filter; keyboard traversal cost grows with overlay size. | `ModuleSchemaPanel.svelte` |
| `A11Y-SCHEMA-02` | Medium | Validation errors often appear only after Save as unstructured text, not as field-linked announcements. | save path in schema panel |
| `A11Y-SCHEMA-03` | Low | Builtin enable chips duplicate detailed rows, producing redundant focus stops for the same field. | Fields tab |
| `A11Y-SCHEMA-04` | Medium | No live-data impact summary before destructive overlay changes; SR users only learn counts after failure or surprise confirmations. | plan §3.7; Slice 6 target |

## Minimum interactive target / motion

| ID | Severity | Issue | Evidence |
| --- | --- | --- | --- |
| `A11Y-CHROME-01` | Low | Shell `controls.css` defines coarse-pointer and reduced-motion rules; Tree settings icon button and some chips still need audit against the shared minimum target size. | `src/lib/shell/controls.css`, Tree icon controls |
| `A11Y-CHROME-02` | Low | Canvas fit animation respects reduced motion in places; confirm all ELK/viewport transitions after Slice 4. | `FamilyTreeCanvas` fitViewOptions |

## Acceptance for clearing an issue

An issue may be marked resolved only when:

1. the target behavior in `INTERACTION_SPEC.md` is implemented;
2. a focused automated or rendered check covers the path; and
3. the issue ID is struck through here or moved to a “Resolved” section with the slice number.
