# Architecture decision records

This directory contains the architectural decisions that still constrain
Daena. Product behavior belongs in the focused project documentation, and
exact schemas, constants, error strings, fixtures, benchmarks, and test
instructions belong with the implementation.

## Current records

| ADR                                                        | Decision                                                       | Primary authority                                                                            |
| ---------------------------------------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| [0001](./0001-plugin-platform-boundary.md)                 | Plugin isolation, authority, package trust, and data ownership | [`PLUGIN_PLATFORM_PLAN.md`](../PLUGIN_PLATFORM_PLAN.md)                                      |
| [0002](./0002-rust-owned-public-contracts.md)              | Rust-owned public contracts and project schema overlays        | [`PLUGIN_PLATFORM_PLAN.md`](../PLUGIN_PLATFORM_PLAN.md), [`PLUGIN_SDK.md`](../PLUGIN_SDK.md) |
| [0003](./0003-ai-trust-privacy-and-proposals.md)           | AI trust, privacy, retrieval, and proposal lifecycle           | [`AI_INTEGRATION.md`](../AI_INTEGRATION.md)                                                  |
| [0004](./0004-physical-world-authority-and-determinism.md) | Physical-world authority, derivation, history, and determinism | [`MAPS.md`](../MAPS.md)                                                                      |
| [0005](./0005-atlas-derived-rendering-and-studio.md)       | Atlas-derived geography, Studio, and static export             | [`MAPS.md`](../MAPS.md)                                                                      |

## Maintenance policy

An ADR records a consequential choice, the alternatives it excludes, and the
constraints that future work must preserve. It does not duplicate a feature
specification or serve as an implementation diary.

Update an existing ADR when a decision is clarified without changing its
meaning. Add a new ADR when an accepted change reverses or materially replaces
a current decision. Mark the replaced record as superseded and link both
directions. Do not create an ADR for each delivery phase, algorithm tuning,
fixture hash, or version bump.

The previous set of plugin, AI, Physical World, and Atlas iteration records was
consolidated into these five records on 2026-08-26. Superseded implementation
details were removed; the dated history sections retain the rationale that is
still useful.
