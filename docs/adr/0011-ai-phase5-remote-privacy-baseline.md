# ADR 0011: Phase 5 remote-provider privacy baseline

## Decision

Remote generation is an explicit host-owned path. It is never selected by
falling back from LM Studio, and plugins do not receive provider endpoints or
credentials. A remote request must match a remembered `(project, provider,
endpoint)` consent record and must find its provider key in OS-backed storage.

The first adapter is OpenAI-compatible JSON chat completion over HTTPS. The
client disables redirects and rejects embedded credentials, query/fragment
components, localhost, and private or local literal IP addresses before a
credential-bearing request is made. Hostname resolution is checked and pinned
for the request, so a DNS name resolving to a local address is rejected and a
subsequent DNS rebinding cannot redirect the request. Provider status is normalized to the
existing `AiError` categories; provider response text is not exposed as the
public error.

## Credential boundary

The host uses the platform-native backend supplied by the `keyring` library,
under a service derived from the provider ID: macOS Keychain, Windows
Credential Manager, or Linux Secret Service/libsecret. The import command reads
`DAENA_REMOTE_API_KEY` only in the host process and writes it to that native
secret store;
the secret is not a Tauri command argument and is not persisted in settings,
project files, logs, or plugin state. If the platform keyring is unavailable,
credential lookup and remote generation fail closed.

Remote endpoint, model, provider, and consent metadata are application
settings. Consent is exact-match, so changing the endpoint or provider
requires a new explicit confirmation. Revoking consent removes the matching
record without touching canonical project data. Source tests cover endpoint
replacement, revocation, redirect-status rejection, and secret redaction;
the remote command also fails before transport when consent or the OS key is
missing. The host policy defaults to `localOnly`; `disabled` and `localOnly`
deny remote requests in Rust, while `ask` and `approvedPairs` require an exact
consent pair and `remoteAllowed` permits the explicitly selected remote
provider. Cancellation and deadline flags gate remote terminal events, with
deadlines normalized to `deadline_exceeded`.

## Scope and remaining evidence

The host command and settings panel provide configuration, credential-status,
consent, and revoke controls. Remote generation returns a normalized
non-streaming completion through the existing bounded proposal lifecycle and
surfaces provider-reported token usage when available. Live provider calls,
keychain prompts, redirect behavior against a real server, and rendered consent
evidence are manual Phase 5 exit-gate checks.
