# Serverless plugin registry

## Status and purpose

Phase 9. Host refresh, catalog install, and `daena-plugin keygen` / `sign`
live in this repository. The identity index is
[daena-archive/plugin-registry](https://github.com/daena-archive/plugin-registry).
Local `.daenaplugin` installation does not depend on GitHub. The default fetch
URLs are `https://daena-archive.github.io/plugin-registry/trust.json` and
`catalog.json`.

Standing policy is [ADR 0001](adr/0001-plugin-platform-boundary.md): a registry
or publisher signature may inform installation trust and cannot become an
authorization dependency. Phase 8 in
[`PLUGIN_PLATFORM_PLAN.md`](PLUGIN_PLATFORM_PLAN.md) already ships host-owned
`TrustSnapshot` (`plugins/trust.json`), signature rotation, revocation, and
source-independent verify. Phase 9 is this document: a GitHub-hosted index
that feeds that snapshot without a Daena server.

Authoring remains [`PLUGIN_SDK.md`](PLUGIN_SDK.md). This file does not change
the canonical project data model in [`STORAGE.md`](STORAGE.md).

## Goal

Publishers register identity with a pull request. Users discover plugins from
a static GitHub Pages catalog. The host downloads the same `.daenaplugin`
bytes it would open from disk and verifies them with the same policy. Offline
and local install keep working when GitHub is unreachable.

## Decisions

### 1. The registry is an identity index, not a package store

The GitHub repository holds publisher records, keys, revocations, and pointers
to publisher-owned source repositories. It does not store `.daenaplugin`
archives. v1 `sourceRepo` is an `https://github.com/<owner>/<repo>` URL.
Packages ship as GitHub Release assets on that repo.

A listing is discovery. Listing never grants capabilities, never skips digest
or signature checks, and never replaces user consent.

### 2. Three artifacts, one trust boundary

| Artifact                | Where                                | Role                                                                  |
| ----------------------- | ------------------------------------ | --------------------------------------------------------------------- |
| Identity index          | Registry git repo, PR-gated          | Publisher ids, keys, revocations, allowed plugin ids, source repo URL |
| Compiled trust snapshot | Generated `trust.json` (Pages + git) | Exact `TrustSnapshot` schema the host already loads                   |
| Discovery catalog       | Generated `catalog.json` on Pages    | Names, versions, artifact URLs, digests as advertised metadata        |

The host may fetch the catalog to show a browse UI. Installation always
downloads package bytes and runs `verify_archive_bytes` with the compiled
snapshot. Catalog fields are not inputs to verification.

### 3. Releases stay on the publisher repo

Register identity once. Do not require a central PR for every plugin version.
The publisher cuts a GitHub Release whose asset is the `.daenaplugin` and
whose metadata can include the package digest. Registry CI may crawl
registered release feeds to refresh `catalog.json`. A failed or stale crawl
degrades discovery, not verification.

### 4. Publisher ids are immutable reverse-domain names

Same rule as the manifest `publisher` field. First accepted PR owns the id.
Later PRs may rotate keys, add plugin ids, or revoke; they may not rename or
reassign the publisher id.

A registered plugin id must be the publisher id or a suffix of it
(`publisher` or `publisher.*`). That prevents two publishers from claiming the
same package identity in the index. CI enforces it. The host does not yet
require that prefix in `validate_manifest`; package identity remains the
signed manifest. Do not treat the allow-list as a capability grant.

### 5. Keys and revocation use the host snapshot model

Each publisher lists one or more Ed25519 keys (`keyId`, `publicKey`). Rotation
adds a key; old keys remain valid until revoked. Key revocation matches the
signing **public key**, not `keyId` alone. The index may also list revoked
package digests and plugin identities. Publisher-wide key revocation is an
entry with publisher and no public key.

Unsigned local packages with explicit consent remain allowed. A registry
listing does not forbid local unsigned install.

### 6. GitHub Pages is a mirror, not an authority

The app fetches HTTPS resources from the Pages origin as it would from any
other host. Compromise or downtime of Pages must not grant extra authority and
must not block local packages. Prefer fetching `trust.json` from a tagged
release or a commit-pinned URL when the host implements refresh; a floating
`main` Pages URL is convenience.

Protect `main`, require reviews, and disable force-push. Git history is the
transparency log for identity and revocation.

### 7. Review is human and host-owned

Registry maintainers review PRs for identity, keys, and source repo
ownership. They do not review plugin capabilities as an authorization step;
the host still computes `review_manifest` (publisher, digest, signature, trust
status, capabilities) from verified bytes. When marketplace install ships, show
that disclosure before writing the version store. Enablement still requires
project-scoped grants.

A plugin cannot draw or phrase that disclosure.

### 8. The CLI produces keys and `signature.json`

Authoring uses `@daena-archive/plugin-cli`, not a Daena server:

- `daena-plugin keygen` writes an Ed25519 key pair and prints the Base64
  verifying key plus a `keyId` for `publisher.json`.
- `daena-plugin sign <archive> --key <path> [--key-id <id>]` adds
  `signature.json` (`algorithm: ed25519`, `publisher` from the manifest,
  `publicKey`, `keyId` if given, `digest`, `signature`) using the same
  digest rules as the host. The private key never leaves the author's
  machine.

Unsigned local install with consent stays allowed. Registry CI omits
unsigned releases from `catalog.json`.

## Registry repository layout

Proposed paths in the identity-index repo:

```text
publishers/<publisher-id>/publisher.json
revocations.json
.github/workflows/validate.yml
.github/workflows/publish.yml
```

CI on `main` generates:

```text
dist/trust.json
dist/catalog.json
```

GitHub Pages publishes `dist/`. Unknown files under `publishers/` are ignored.
`publisher.json` is a registry-only schema (identity, source repo, plugin
allow-list). Unknown keys in it fail closed in CI. It is not a host type.

### `publishers/<id>/publisher.json`

```json
{
  "schemaVersion": 1,
  "id": "com.example",
  "keys": [{ "keyId": "2026-02", "publicKey": "<base64-ed25519-32>" }],
  "sourceRepo": "https://github.com/example/daena-plugins",
  "plugins": ["com.example.genealogy"]
}
```

Rules:

- `id` must match the directory name and the manifest `publisher` those
  packages will use.
- `keys` must be non-empty at registration. Each `publicKey` is a 32-byte
  Ed25519 verifying key, standard Base64.
- `sourceRepo` is an https GitHub repository URL the publisher controls.
- `plugins` is the allow-list of package ids this publisher may list. It is
  not a capability list.

Display names, homepages, and blurbs belong in `catalog.json` if at all. They
are not trust inputs.

### `revocations.json`

```json
{
  "schemaVersion": 1,
  "keys": [
    {
      "publisher": "com.example",
      "keyId": "2026-01",
      "publicKey": "<base64-ed25519-32>"
    }
  ],
  "digests": [],
  "packages": []
}
```

Same fields as host `RevocationList`. A key revocation without `publicKey` is
rejected unless it is publisher-wide (no `keyId` either).

### Compiled `trust.json`

CI maps the index into the host `TrustSnapshot` shape (`deny_unknown_fields`):

```json
{
  "schemaVersion": 1,
  "publishers": {
    "com.example": {
      "keys": [{ "keyId": "2026-02", "publicKey": "<base64-ed25519-32>" }]
    }
  },
  "revocations": { "keys": [], "digests": [], "packages": [] }
}
```

Only `id` → `publishers[id].keys` is copied from each `publisher.json`.
`sourceRepo` and `plugins` stay in the index and catalog; they must not appear
in `trust.json`. Copy `revocations.json` into `revocations`. The compiled file
must stay under the host size limit (256 KiB). Registry refresh downloads this
file over HTTPS, replaces `plugins/trust.json` atomically, then uses the same
verify path.

## Publisher workflow

1. `daena-plugin keygen`. Keep the signing key offline. Publish only the
   verifying key.
2. Open a PR adding `publishers/<id>/publisher.json`.
3. Prove control of the reverse-domain or of the GitHub source repo (see
   **Identity proof**).
4. After merge, `daena-plugin package` then `daena-plugin sign`, and attach
   the `.daenaplugin` to a GitHub Release on `sourceRepo`.
5. To rotate, PR an additional key; keep the previous key until clients have
   moved, then PR a revocation of the old **public key**.
6. To stop a package, PR its digest and/or plugin id into `revocations.json`.

CI on the PR must reject: unknown fields, empty ids, invalid keys, plugin ids
that are not `id` or `id.*`, `sourceRepo` that is not https GitHub, colliding
publisher directories, and schema versions other than 1.

## Identity proof

Accept one of:

- GitHub: `sourceRepo` is under the PR author's user or org, and the PR is
  opened from that account; or
- Domain: for a reverse-domain id `com.example`, a file at
  `https://example.com/.well-known/daena-plugin-publisher` whose body is the
  publisher id and verifying key fingerprint. GitHub-only ids (`com.github.*`)
  use the GitHub proof, not DNS.

Do not treat email, social bios, or unsigned README claims as proof. Squatted
ids are not reassigned; contact is a revocation and a new id.

## Host behavior

When registry refresh exists:

1. Fetch `trust.json` over HTTPS.
2. Parse with `TrustSnapshot::load` (size limit, schema 1, fail closed).
3. Replace `plugins/trust.json` atomically.
4. Optionally fetch `catalog.json` for the Plugins panel browse list.
5. On install from the catalog, the artifact URL must already be listed in the
   local `catalog.json`. Fetch it over HTTPS from GitHub (including
   `githubusercontent.com` redirects for Release assets), require the
   `.daenaplugin` extension, compare the catalog digest if present, and call
   the same installer used for a local file, with
   `VerificationPolicy::from_install_root`.

If fetch fails, keep the last good snapshot and local install. An empty or
missing snapshot is today’s default: signatures still verify cryptographically,
unsigned still needs consent, pinned-publisher checks are off until a snapshot
pins keys.

Refresh is host chrome. It is not plugin `network:<origin>`, does not run
plugin code, does not grant capabilities, and must not use a plugin webview.

## Discovery catalog

`catalog.json` is generated, not hand-edited:

```json
{
  "schemaVersion": 1,
  "plugins": [
    {
      "id": "com.example.genealogy",
      "publisher": "com.example",
      "name": "Genealogy",
      "version": "1.2.0",
      "digest": "<hex>",
      "artifactUrl": "https://github.com/example/daena-plugins/releases/download/genealogy-1.2.0/genealogy.daenaplugin"
    }
  ]
}
```

`artifactUrl` chooses which bytes to fetch. `digest` is optional fetch
integrity: if present, the host compares it to the digest computed from those
bytes and fails closed on mismatch. It is not a trust input. Signature and
`TrustSnapshot` still decide signed/trusted/revoked. Catalog `digest` must
never skip verify.

CI may omit a plugin from the catalog when the release is missing, unsigned,
or not signed by a current key. Omission is not revocation; revocation lives
in `trust.json`.

## Transparency

- Every identity and revocation change is a reviewed git commit.
- Pages publishes the compiled snapshot from `main` (or a release tag).
- The Plugins panel shows publisher, digest, signed/unsigned, and trust
  status from `review_manifest`.
- The registry does not attest code quality, security of plugin logic, or
  fitness for a project.

## Rejected alternatives

- **Daena-operated API or account service.** A server would become an
  availability and authorization dependency. Deferred and unnecessary.
- **Central PRs for every version (Homebrew formula bumps).** Too much review
  load; identity and artifacts would stay coupled.
- **Git LFS / repo-stored zips in the index.** Hits size limits and mixes
  identity history with large binaries.
- **Trusting Pages or GitHub Release “latest” as authorization.** Mutable
  pointers. Verify bytes.
- **Single key per publisher.** Host already supports rotation.
- **Revocation by `keyId` only.** A package can omit `keyId`; public key is
  the key identity.
- **Plugin-supplied marketplace UI.** Host chrome only.

## Implementation remainder

Shipped here: `daena-plugin keygen` / `sign`, host snapshot refresh, catalog
prepare/install (same `verify_archive_bytes` path), pre-install
`review_manifest`, and the identity-index compiler used by tests
(`scripts/plugin-registry-compile.mjs`). The live index, PR templates, and
Pages workflows live in `daena-archive/plugin-registry`.

Still outside this repository:

1. Protect `main` on `plugin-registry` and enable Pages.
2. Optional CI crawl of publisher GitHub Releases to fill `catalog.json`
   (unsigned assets omitted). Empty catalog is valid.
