# Daena Git Integration Architecture and Delivery Plan

## Status, authority, and purpose

This document is the definitive architecture and implementation plan for
built-in Git in Daena Archive. It governs system-git probing, Settings → Git UX,
canonical preflight and selective staging, commit messages, history and snapshot
browsing, hard-reset restore, remotes, and post-reset remote recovery.

It supplements, and does not override, these authorities:

- [`ARCHITECTURE.md`](./ARCHITECTURE.md) defines the product and shared
  entity/document model.
- [`PLAIN_TEXT_STORAGE_PLAN.md`](./PLAIN_TEXT_STORAGE_PLAN.md) defines canonical
  project files, disposable indexes, and the Phase 5 Git preflight boundary.
- [`ARCHITECTURE.md`](./ARCHITECTURE.md) defines shell-owned helpers vs plugin
  isolation.
- [`PLUGIN_PLATFORM_PLAN.md`](./PLUGIN_PLATFORM_PLAN.md) forbids raw Git/Tauri
  commands in plugin webviews.

If this document conflicts with those boundaries, the stricter authority and
storage rule wins. Git must remain an optional, user-controlled helper around
canonical files. It must not become a collaboration protocol, automatic sync
engine, semantic merge resolver, or plugin-reachable filesystem API.

Status as of 2026-08-08: **architecture approved by this document; core APIs,
Tauri commands, client bindings, and Settings → Git UI are implemented in
source**. Agents must still verify the worktree and run focused tests before
treating any behavior as complete.

The governing rule is:

> Canonical project files are authoritative; Git records optional snapshots of
> those files; users explicitly choose every init, stage, commit, reset, push,
> and pull; `.daena/` stays disposable and gitignored.

---

## 1. Product intent

### 1.1 Goals

- Make Git availability and version visible in Settings, with a path to install
  when missing.
- Let authors review exactly which **canonical** paths will enter a commit and
  choose a subset to stage.
- Support custom commit messages and deterministic generated messages (no AI
  required for this surface).
- Expose commit history in Settings, including read-only browsing of files in a
  chosen snapshot.
- Allow restoring an earlier snapshot via **hard reset**, with explicit danger
  warnings about losing later commits and uncommitted work.
- Allow adding and managing remotes.
- After a hard reset that diverges from an upstream remote, offer a clear
  recovery choice: force-push-with-lease to rewrite the remote, or restore from
  remote (regret path).

### 1.2 Non-goals

- Embedding libgit2/gitoxide or replacing the system `git` binary.
- Auto-commit on autosave, background push/pull, or always-on sync.
- Automatically resolving semantic Git merge conflicts.
- Staging noncanonical paths (anything outside the canonical allowlist).
- Bare `git push --force` (lease-protected force only).
- Exposing Git commands to plugins or sandboxed webviews.
- AI-authored commit messages as a requirement of this plan (may appear later
  under [`AI_INTEGRATION.md`](./AI_INTEGRATION.md) without changing this
  contract).

### 1.3 Relationship to prior Phase 5 Git boundary

[`PLAIN_TEXT_STORAGE_PLAN.md`](./PLAIN_TEXT_STORAGE_PLAN.md) § Git Integration
already requires:

1. flush pending autosaves;
2. finish or recover journaled transactions;
3. validate changed canonical files;
4. ensure the index represents their current hashes;
5. reject unresolved Git conflicts and invalid records; and
6. show exactly which canonical files and assets will be staged.

This document **extends** that boundary with selective staging among the
preflight set, Settings UX, history/snapshot browsing, hard reset, remotes, and
remote recovery. It does not weaken preflight rejection rules.

---

## 2. Current implementation baseline

Agents must extend the implementation that exists. Line numbers drift; symbol
and filename are authoritative.

| Boundary | Current source | Consequence |
| --- | --- | --- |
| System git CLI wrapper | `crates/daena-core/src/project.rs` (`run_git`, `git_status`, `git_init`, `git_log`, `git_preflight`, `git_commit`) | Keep CLI-based integration; do not introduce a second VCS stack. |
| Canonical path allowlist | `ProjectStore::is_canonical_git_path` | Only `project.json`, `.gitignore`, `entities/`, `plugins/`, `assets/`. |
| Preflight / staging preview | `git_preflight` / `git_staging_preview` | Commit must re-run preflight; block unmerged / noncanonical-staged / stale index. |
| Tauri commands | `src-tauri/src/lib.rs` (`project_git_*`) | Shell-owned; never on the plugin broker catalog. |
| Frontend client | `src/lib/project/client.ts` | Typed invoke wrappers for Git status/preflight/init/log/commit. |
| Rail + commit modal | `src/routes/+page.svelte` | Thin status today; detailed Git UX moves to Settings → Git. |
| Settings shell | `src/lib/SettingsView.svelte` | Add a `git` section beside General / Plugins. |
| Open external URLs | `tauri-plugin-opener` already initialized in `src-tauri` | Use for `https://git-scm.com/downloads`. |
| App settings file | `src-tauri/src/settings.rs` (`settings.json`) | Git tool prefs are not required in v1; remotes live in the repo’s `.git/`. |

**Baseline gaps this plan fills:** no `git --version` probe UI, no selective
path commit API, unused `gitLog` in the UI, no snapshot tree/file show, no hard
reset, no remotes UI, no post-reset remote recovery.

---

## 3. Locked decisions

These decisions are binding for agents unless this document is explicitly
revised.

1. **Settings home.** Git’s primary surface is **Settings → Git**. The rail Git
   control may show a short status and deep-link with `openSettings("git")`, but
   must not remain the only place for staging, history, remotes, or reset.
2. **Revert means hard reset.** Restoring an earlier snapshot runs
   `git reset --hard <hash>` after a danger confirmation. It is not
   `git revert` (compensating commit) and not a silent working-tree checkout of
   individual files.
3. **Danger copy must be explicit.** The confirm dialog must state that later
   commits are discarded, uncommitted changes are discarded, and remotes may
   diverge. Prefer the existing host `alertdialog` / danger-button pattern used
   for destructive plugin actions.
4. **Selective staging stays canonical-only.** The UI may only offer paths from
   the current preflight `staging_paths`. The core must reject any path outside
   that set.
5. **Generated messages are deterministic.** Build from the selected path set
   (counts and/or top-level buckets such as entities / plugins / assets). Do not
   block commit on AI availability.
6. **Force push is lease-only.** Use `git push --force-with-lease`. Never bare
   `--force` from Daena.
7. **Regret path uses the remote.** If the user hard-resets and then regrets,
   and an upstream still has the discarded tip, offer restore-from-remote
   (fetch + hard reset to upstream, or equivalent dedicated helper). Ordinary
   `git pull --ff-only` alone is insufficient after a non-ff divergence.
8. **Index rebuild after history moves.** After hard reset or restore-from-
   upstream, rebuild/reconcile so `.daena/index.sqlite` matches canonical files.
9. **Snapshot browsing is read-only.** Listing and previewing files at a commit
   must not write the working tree. Only hard reset mutates the tree to match a
   snapshot.
10. **Plugins never get Git.** No broker methods, no webview invoke of
    `project_git_*`.

```text
Rail Git  -->  Settings → Git
                 ├─ Tool status (version / download)
                 ├─ Remotes (add / edit / remove)
                 ├─ Changes (select paths → message → commit)
                 ├─ History (log → tree → file preview → hard reset)
                 └─ Remote recovery banner (force-with-lease | restore)
```

---

## 4. Canonical staging model

### 4.1 Allowlist

A path is canonical for Git helpers only when it is one of:

- `project.json`
- `.gitignore`
- anything under `entities/`
- anything under `plugins/`
- anything under `assets/`

Everything under `.daena/` remains gitignored and must never appear in built-in
staging previews.

### 4.2 Preflight (unchanged requirements)

Before any built-in commit:

1. Flush pending autosaves in the shell.
2. Recover journaled transactions in core.
3. Reject unresolved/unmerged Git paths.
4. Surface reconcile / index-staleness diagnostics.
5. Reject already-staged noncanonical paths.
6. Compute `staging_paths` as the canonical change set eligible for commit.

If `ready` is false, commit is blocked. Selective staging may only shrink
`staging_paths`, never expand them.

### 4.3 Selective commit

```text
user_selected_paths ⊆ preflight.staging_paths
git add --all -- <user_selected_paths>
git commit -m <message>
```

Empty selection and empty message are errors. Re-run preflight inside
`git_commit` so the UI cannot commit a stale preview.

### 4.4 Generated commit message

Deterministic template examples (exact wording may improve, structure must stay
local and path-derived):

```text
Update 3 canonical files

- entities/…/document.md
- entities/…/fields/….json
- project.json
```

or bucketed:

```text
Update lore entities and project manifest (4 files)
```

Do not call network services to generate messages.

---

## 5. Tool availability

### 5.1 Probe

`git_tool_info` (or equivalent) must run `git --version` via the same spawn path
as other Git helpers:

- **Available:** parse a human-readable version string for Settings.
- **Unavailable:** spawn failure maps to a clear “Git is not installed or not
  on PATH” state (existing `git is unavailable: …` errors remain valid for
  other calls).

### 5.2 Install affordance

When unavailable, Settings → Git shows a button that opens

`https://git-scm.com/downloads`

through `tauri-plugin-opener`. Daena does not bundle or silently download Git.

---

## 6. History, snapshot browse, and hard reset

### 6.1 History

- Load commit history through `git_log` (extend to about 50 entries if needed).
- Each entry exposes at least `hash`, `date`, and `subject`.
- History is rendered in Settings → Git (the rail must not leave `gitLog`
  fetched-but-unused).

### 6.2 Snapshot file list

For a selected commit hash:

- `git ls-tree -r --name-only <hash>`
- Filter to the canonical allowlist before returning to the UI.

### 6.3 Snapshot file preview

For a selected `(hash, path)`:

- Reject noncanonical paths.
- `git show <hash>:<path>`
- Return text for preview; reject or clearly refuse binary / oversized blobs.
- Preview is read-only.

### 6.4 Hard reset

Flow:

1. Flush autosave.
2. Danger `alertdialog` with irreversible consequences (later commits,
   uncommitted work, possible remote divergence).
3. `git reset --hard <hash>`.
4. Rebuild/reconcile disposable index from canonical files.
5. Refresh status, preflight, log, and remotes/upstream divergence.
6. If an upstream is configured and local/remote history diverged, show the
   **remote recovery** banner.

Daena may capture enough pre-reset metadata (previous HEAD, upstream name/url,
branch) to drive the recovery banner; it must not invent a second backup store
outside Git/`ORIG_HEAD`/remotes unless a later revision of this document says
so.

---

## 7. Remotes and post-reset recovery

### 7.1 Remote management

Settings → Git must support:

| Action | Behavior |
| --- | --- |
| List | `git remote -v` → structured name + fetch/push URLs |
| Add | `git remote add <name> <url>` after basic URL validation |
| Edit URL | `git remote set-url <name> <url>` |
| Remove | `git remote remove <name>` after confirm |

Remotes are repository state under `.git/`, not app-profile `settings.json`, and
not canonical project files.

### 7.2 Push and pull helpers

- Default push: `git push <remote> <branch>` when the user asks.
- Force path after reset: `git push --force-with-lease <remote> <branch>` only,
  with UI copy that the remote history will be rewritten if the lease allows.
- Restore-from-remote (regret): dedicated helper that fetches and hard-resets to
  the upstream tip (or equivalent), then rebuilds the Daena index. Label the
  button clearly as restoring discarded local history from the remote.

### 7.3 Recovery banner rules

Show the banner only when:

- a hard reset just completed (or divergence remains unresolved), and
- an upstream remote/branch is configured, and
- local and upstream tips differ in a way that needs an explicit choice.

Dismiss or clear the banner after a successful force-with-lease or
restore-from-remote, then refresh Git state.

If there is no upstream, still complete the hard reset and index rebuild; tell
the user that discarded commits may only be recoverable through Git reflog
outside Daena (optional future enhancement: expose `ORIG_HEAD` undo—out of
scope unless this document is updated).

---

## 8. API contract (core → Tauri → client)

All APIs are shell-owned. Names may be adjusted for Rust style, but semantics
must match.

| Capability | Core responsibility | Notes |
| --- | --- | --- |
| `git_tool_info` | `git --version` | `{ available, version, error? }` |
| `git_status` / `git_init` / `git_preflight` / `git_log` | Existing | Keep; log length may increase |
| `git_commit(message, paths?)` | Preflight + subset check + add + commit | `paths` default = all staging paths only if UI sends full set; prefer explicit paths from UI |
| `git_show_tree(hash)` | Canonical-filtered tree | Read-only |
| `git_show_file(hash, path)` | Canonical + size/binary guard | Read-only |
| `git_reset_hard(hash)` | Hard reset + divergence hints | Shell rebuilds index after |
| `git_remote_list` / `add` / `set_url` / `remove` | Remote CRUD | Validate URL; confirm remove in UI |
| `git_push(remote, branch, forceWithLease)` | Push / lease force-push | No bare `--force` |
| `git_restore_from_upstream` | Fetch + reset to upstream | Regret path; rebuild index after |

Wire each through `src-tauri/src/lib.rs` `generate_handler!` and
`src/lib/project/client.ts`.

---

## 9. User experience

### 9.1 Settings → Git layout

Single scrollable panel with sections:

1. **Git tool** — version, or missing + Open git-scm.com downloads.
2. **Remotes** — list all remotes; add via modal; edit URL via modal; remove.
3. **Changes** — preflight diagnostics; entity-grouped checkboxes (project /
   entity name / plugin / asset) derived from `staging_paths`; select all/none;
   message field; Generate message uses group titles; Commit selected still
   sends the underlying canonical paths.
4. **History** — commit list; selecting a commit loads snapshot paths; selecting
   a path opens read-only preview; **Restore this snapshot…** triggers the
   danger confirm then hard reset.
5. **Remote recovery** — conditional sticky banner after divergent reset.

Without an open project: show tool status only (version / install). Remotes,
changes, and history require a directory-backed open project.

### 9.2 Rail Git

Keep a compact flyout:

- initialized or not;
- branch / dirty summary;
- Init when needed;
- primary CTA: **Open Git settings**.

Move the detailed commit modal behavior into Settings → Changes so one surface
owns staging, messaging, and history.

### 9.3 Autosave

Flush autosave before commit and before hard reset (reuse the shell
`flushAutoSave` path). Core still re-validates via preflight / reset guards.

---

## 10. Implementation plan for agents

Implement in this order unless a task explicitly narrows scope:

### Phase A — Core APIs and tests

1. Add `git_tool_info`.
2. Extend `git_commit` for explicit path subsets with membership checks.
3. Add `git_show_tree` / `git_show_file`.
4. Add `git_reset_hard` with divergence metadata for the UI.
5. Add remote list/add/set-url/remove.
6. Add `git_push` (including force-with-lease) and `git_restore_from_upstream`.
7. Cover with `crates/daena-core` tests that require system `git`: selective
   commit rejection, reset moves HEAD, remotes round-trip, tree filter is
   canonical-only.

### Phase B — Tauri and TypeScript client

1. Register commands.
2. Extend `project/client.ts` types and methods.
3. Open download URL via opener plugin.

### Phase C — Settings UI

1. Add `git` to `SettingsSection` in `SettingsView.svelte` and `+page.svelte`.
2. Implement the Git panel sections and danger dialogs.
3. Deep-link rail Git → Settings → Git.
4. After reset/restore, call existing reconcile/rebuild paths and refresh Git
   state.

### Phase D — Verification

- `rtk cargo test --manifest-path crates/daena-core/Cargo.toml --locked --offline`
  with Git-focused filters, plus broader core/tauri tests as needed.
- `rtk npm run check`.
- Manual smoke: missing git → download; selective commit; history preview;
  hard-reset warning; force-with-lease vs restore-from-remote.

---

## 11. Testing requirements

Passing unit tests alone do not prove the Settings Git surface. Agents must:

- Exercise preflight rejection (unmerged / noncanonical staged) still blocks
  commit.
- Prove selective commit cannot add paths outside `staging_paths`.
- Prove snapshot tree filtering drops noncanonical paths.
- Prove hard reset updates HEAD and that the shell rebuilds usable project
  state afterward.
- Prove force push invocations include `--force-with-lease` and never bare
  `--force`.
- Prove plugin/webview code paths cannot call new Git commands.

---

## 12. Exit gate

This plan is done when, in the rendered Tauri app:

1. Settings → Git shows git version or a working download affordance.
2. A user can select a subset of canonical changes, generate or type a message,
   and commit only that subset.
3. A user can open history, browse files in a snapshot, and hard-reset only
   after an explicit danger confirmation.
4. A user can add and remove a remote.
5. After a divergent hard reset with upstream configured, the user can either
   force-push-with-lease or restore from remote, and the project index remains
   coherent either way.

Until that gate is met, treat missing APIs or UI as unfinished work under this
document—not as implied product behavior.
