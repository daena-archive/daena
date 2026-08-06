# ADR 0002: Rust owns plugin identity and authorization

- Status: Accepted
- Date: 2026-08-03

## Decision

Rust is the only authority boundary for plugin operations. The host creates a
session bound to the installed plugin ID, package digest, version, API version,
project, originating runtime, grants, activation generation, expiry, and
revocation state. Plugin requests do not provide an authoritative caller ID.

Every broker method validates the session, payload, capability, and resource
scope before calling a core service. TypeScript checks are developer feedback,
never enforcement. Canonical reads carry an opaque revision. Rust requires that
revision for every mutable broker operation and binds retryable operations to
the envelope request ID before entering the repository-first core boundary.

## Consequences

The frontend `ModuleContext` and public SDK are broker-backed clients and must
not gain direct access to the trusted project client. Session revocation is required for disablement,
upgrade, uninstall, project close, and application restart.
