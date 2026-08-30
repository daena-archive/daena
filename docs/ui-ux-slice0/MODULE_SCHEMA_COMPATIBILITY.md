# Module schema compatibility (Slice 7)

Policy authority: `docs/TEMP_UI_UX_ENTITY_SCHEMA_PLAN.md` §5.8.
Runtime helpers: `src/lib/schema-workbench/module-compatibility.ts`.

## Overlay vs managed

| Module | `schema.overlay` | Fields & Types card | Notes |
| --- | --- | --- | --- |
| Lore | Yes | Workbench | Full Types, Fields, Templates, relationship metadata, Timeline options. |
| Timeline | Yes | Workbench | Full Types, Fields, Templates; calendar/date invariants via preview. |
| Writing | Yes | Workbench | Full Types, Fields, Templates. |
| Houses | Yes | Workbench | House authoring fields/templates and custom Types. Tree stays contract-limited (below). |
| Language | No* | Managed by extension | Specialized Overview still reads **packaged** field definitions, not merged schema. Overlay stays off until that workspace renders custom fields consistently (`LANGUAGE_SCHEMA_OVERLAY_READY = false`; shell latch via `schemaOverlayWorkbenchAllowed`). |
| Maps | No | Managed by extension | Provider/internal fields stay extension-managed. Do not expose them as author schema. A future author map-metadata namespace would be separate. |

Do not grant `schema.overlay` only to make plugin cards look the same.

## Houses: Tree-compatible vs collection-only

Tree hydrates only:

- `daena.lore:person` (Person nodes)
- `daena.houses:house` (House roots / membership)

In the **Houses** type editor, only the contract House type is labeled Tree-compatible.
Every other type id under Houses — including custom types and any accidental
`daena.lore:person` on a Houses overlay — is labeled **collection-only**.

- collection-only types appear in the Houses collection and generic editor;
- they do **not** become Tree nodes;
- Open tree remains available only for the contract House type.

Projection labels (single source: `projectionLabelsForModuleType`):

- builtin House → `Houses collection`, `Tree`
- all other Houses types → `Houses collection only`

Lore / Timeline / Writing types get module projections (`Library`/`Wiki`/`Graph`,
`Timeline`, `Writing Studio`; Lore Person also lists `Tree`).

## Language latch

`LANGUAGE_SCHEMA_OVERLAY_READY` is a shell safety latch. Until it is `true`,
Language stays on the Managed card even if the manifest declares
`schema.overlay` (`schemaOverlayWorkbenchAllowed`).

## Exit gate

Every enabled module either opens a consistent schema workbench or shows a
Managed-by-extension card with an explicit reason.
