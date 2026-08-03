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
never enforcement.

## Consequences

The existing frontend `ModuleContext` is transitional. It must become a
broker-backed SDK client during later phases and must not gain direct access to
the trusted project client. Session revocation is required for disablement,
upgrade, uninstall, project close, and application restart.

