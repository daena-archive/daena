# ADR 0004: Plugin interoperability and data ownership

- Status: Accepted
- Date: 2026-08-03

## Decision

The core owns entities, documents, relationships, assets, search, and project
lifecycle. Plugins own schemas, namespaces, templates, and presentation. Each
namespace has one owning plugin, and plugin data remains in the core project
database rather than a plugin-owned database.

Plugins interact through stable core entity IDs, explicitly shared fields,
versioned asynchronous events, and versioned request/response services. They
do not import one another's runtime code or exchange direct object references.
Entity, document, field, relationship, and asset reads include an opaque
canonical revision; mutable calls must echo that revision as
`expectedRevision`, while the broker envelope `requestId` makes retries
idempotent.

Events are session-local and at-most-once. Services have one active provider per
major version, explicit schemas, deadlines, cancellation, and cycle detection.
Marketplace, durable event queues, cloud execution, and unrestricted network
access are deferred.
