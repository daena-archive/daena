# Slice 0: interaction specification and fixtures

Status: complete for review. Later slices implement against these artifacts.

Authority: [`TEMP_UI_UX_ENTITY_SCHEMA_PLAN.md`](../TEMP_UI_UX_ENTITY_SCHEMA_PLAN.md) §7 Slice 0.

## Deliverables

| Artifact | Path | Role |
| --- | --- | --- |
| Interaction spec | [`INTERACTION_SPEC.md`](./INTERACTION_SPEC.md) | Target labels, mutation vocabulary, Tree keyboard model, contextual New |
| Surface inventory | [`SURFACES.md`](./SURFACES.md) | Current workspace / Tree / schema surfaces (fixture stand-in for screenshots) |
| Baseline a11y | [`BASELINE_A11Y.md`](./BASELINE_A11Y.md) | Known keyboard and screen-reader gaps before refactor |
| Machine fixtures | [`src/lib/ui-ux/fixtures.ts`](../../src/lib/ui-ux/fixtures.ts) | Empty, large, disconnected House, memberships, malformed edge, custom schema, conflict |
| Vocabulary constants | [`src/lib/ui-ux/vocabulary.ts`](../../src/lib/ui-ux/vocabulary.ts) | Locked strings for later shared components |
| Locking test | [`scripts/ui-ux-slice0.test.mjs`](../../scripts/ui-ux-slice0.test.mjs) | Validates fixtures and vocabulary remain complete |

## Exit gate

Reviewers can evaluate later slices against stable scenarios rather than visual
memory when:

1. every surface in `SURFACES.md` is named and linked to a scenario or baseline note;
2. entity-action and mutation vocabulary are defined once in `vocabulary.ts`;
3. the Tree keyboard model is explicit in `INTERACTION_SPEC.md`;
4. all seven representative fixtures exist and pass `npm run test:ui-ux-slice0`; and
5. baseline keyboard/SR issues are recorded so regressions and fixes are measurable.

Native Tauri screenshots are optional follow-ups. Until captured, the surface
inventory plus fixtures are the review authority for layout and copy.

## How to use in later slices

- Slice 1+: import `ENTITY_ACTIONS` / `MUTATION_STATUS` instead of inventing labels.
- Slice 2+: use `LARGE_PROJECT.scale` and `synthesizeLargeProjectPeople`.
- Slice 3–4: drive House/Tree tests from `DISCONNECTED_HOUSE`, `MULTIPLE_MEMBERSHIPS`, `MALFORMED_EDGE`.
- Slice 5–6: drive schema impact/conflict tests from `CUSTOM_SCHEMA_LIVE_DATA` and `REVISION_CONFLICT`.
- Slice 7: module overlay vs managed policy and Houses Tree vs collection-only types in [`MODULE_SCHEMA_COMPATIBILITY.md`](./MODULE_SCHEMA_COMPATIBILITY.md).
