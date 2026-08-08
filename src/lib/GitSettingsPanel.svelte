<script lang="ts">
  import {
    project,
    type Entity,
    type GitLogEntry,
    type GitPreflight,
    type GitRemote,
    type GitStatus,
    type GitToolInfo,
    type GitUpstream,
  } from "$lib/project/client";

  let {
    projectOpen,
    onError,
    onBusyMessage,
    beforeWrite,
  }: {
    projectOpen: boolean;
    onError: (message: string) => void;
    onBusyMessage?: (message: string) => void;
    beforeWrite?: () => Promise<boolean>;
  } = $props();

  type ChangeGroup = {
    id: string;
    kind: "entity" | "project" | "plugin" | "asset";
    title: string;
    subtitle: string;
    paths: string[];
  };

  type RemoteModalMode = "add" | "edit";

  let tool = $state<GitToolInfo | null>(null);
  let status = $state<GitStatus | null>(null);
  let preflight = $state<GitPreflight | null>(null);
  let remotes = $state<GitRemote[]>([]);
  let entities = $state<Entity[]>([]);
  let log = $state<GitLogEntry[]>([]);
  let selectedGroupIds = $state<string[]>([]);
  let commitMessage = $state("");
  let busy = $state(false);
  let remoteModalOpen = $state(false);
  let remoteModalMode = $state<RemoteModalMode>("add");
  let remoteName = $state("");
  let remoteUrl = $state("");
  let editingRemoteName = $state<string | null>(null);
  let selectedCommit = $state<string | null>(null);
  let snapshotPaths = $state<string[]>([]);
  let selectedSnapshotPath = $state<string | null>(null);
  let snapshotBody = $state("");
  let recoveryUpstream = $state<GitUpstream | null>(null);
  let showResetConfirm = $state(false);
  let pendingResetHash = $state<string | null>(null);

  function friendly(cause: unknown) {
    return cause instanceof Error ? cause.message : String(cause);
  }

  async function withBusy<T>(label: string, run: () => Promise<T>) {
    busy = true;
    onBusyMessage?.(label);
    try {
      return await run();
    } catch (cause) {
      onError(friendly(cause));
      throw cause;
    } finally {
      busy = false;
      onBusyMessage?.("");
    }
  }

  function entityFileRole(path: string): string | null {
    const relative = path.replace(/^entities\/[^/]+\//, "");
    if (relative === "entity.json") return "Identity";
    if (relative === "document.md") return "Document";
    if (relative === "relationships.json") return "Relationships";
    if (relative === "assets.json") return "Asset index";
    if (relative.startsWith("fields/")) return "Fields";
    return null;
  }

  function summarizeRoles(paths: string[]): string {
    const roles = [...new Set(paths.map(entityFileRole).filter((role): role is string => Boolean(role)))];
    return roles.length > 0 ? roles.join(" · ") : `${paths.length} file${paths.length === 1 ? "" : "s"}`;
  }

  function shortId(id: string) {
    return id.length > 8 ? id.slice(0, 8) : id;
  }

  function buildChangeGroups(paths: string[], entityList: Entity[]): ChangeGroup[] {
    const byId = new Map(entityList.map((entity) => [entity.id, entity]));
    const entityBuckets = new Map<string, string[]>();
    const pluginBuckets = new Map<string, string[]>();
    const assetBuckets = new Map<string, string[]>();
    const projectPaths: string[] = [];

    for (const path of paths) {
      if (path.startsWith("entities/")) {
        const entityId = path.split("/")[1];
        if (!entityId) continue;
        const bucket = entityBuckets.get(entityId) ?? [];
        bucket.push(path);
        entityBuckets.set(entityId, bucket);
        continue;
      }
      if (path.startsWith("plugins/") && path.endsWith(".json")) {
        const pluginId = path.slice("plugins/".length, -".json".length);
        const bucket = pluginBuckets.get(pluginId) ?? [];
        bucket.push(path);
        pluginBuckets.set(pluginId, bucket);
        continue;
      }
      if (path.startsWith("assets/")) {
        assetBuckets.set(path, [path]);
        continue;
      }
      projectPaths.push(path);
    }

    const groups: ChangeGroup[] = [];

    if (projectPaths.length > 0) {
      groups.push({
        id: "project",
        kind: "project",
        title: "Project settings",
        subtitle: projectPaths
          .map((path) => (path === "project.json" ? "Project manifest" : path === ".gitignore" ? "Ignore rules" : path))
          .join(" · "),
        paths: projectPaths,
      });
    }

    for (const [entityId, entityPaths] of [...entityBuckets.entries()].sort(([a], [b]) => a.localeCompare(b))) {
      const entity = byId.get(entityId);
      groups.push({
        id: `entity:${entityId}`,
        kind: "entity",
        title: entity?.name ?? `Deleted entity (${shortId(entityId)})`,
        subtitle: [
          entity?.entity_type ?? (entity ? "Uncategorized" : "Unknown entity"),
          summarizeRoles(entityPaths),
        ].join(" · "),
        paths: entityPaths,
      });
    }

    for (const [pluginId, pluginPaths] of [...pluginBuckets.entries()].sort(([a], [b]) => a.localeCompare(b))) {
      groups.push({
        id: `plugin:${pluginId}`,
        kind: "plugin",
        title: pluginId,
        subtitle: "Plugin config",
        paths: pluginPaths,
      });
    }

    for (const [assetPath, assetPaths] of [...assetBuckets.entries()].sort(([a], [b]) => a.localeCompare(b))) {
      const parts = assetPath.split("/");
      const filename = parts[parts.length - 1] ?? assetPath;
      const folder = parts[1] ?? "files";
      groups.push({
        id: `asset:${assetPath}`,
        kind: "asset",
        title: filename,
        subtitle: `Asset · ${folder}`,
        paths: assetPaths,
      });
    }

    return groups;
  }

  let changeGroups = $derived(buildChangeGroups(preflight?.staging_paths ?? [], entities));

  function selectedPathsFromGroups(groupIds: string[], groups: ChangeGroup[]) {
    const selected = new Set(groupIds);
    return groups.filter((group) => selected.has(group.id)).flatMap((group) => group.paths);
  }

  let selectedPaths = $derived(selectedPathsFromGroups(selectedGroupIds, changeGroups));

  function groupIsSelected(groupId: string) {
    return selectedGroupIds.includes(groupId);
  }

  function toggleGroup(groupId: string) {
    selectedGroupIds = selectedGroupIds.includes(groupId)
      ? selectedGroupIds.filter((id) => id !== groupId)
      : [...selectedGroupIds, groupId];
  }

  function selectAllGroups() {
    selectedGroupIds = changeGroups.map((group) => group.id);
  }

  function clearGroups() {
    selectedGroupIds = [];
  }

  function syncSelectedGroups(groups: ChangeGroup[], previousSelected: string[]) {
    const allowed = new Set(groups.map((group) => group.id));
    const kept = previousSelected.filter((id) => allowed.has(id));
    return kept.length > 0 ? kept : groups.map((group) => group.id);
  }

  async function refresh() {
    tool = await project.gitToolInfo();
    if (!projectOpen) {
      status = null;
      preflight = null;
      remotes = [];
      entities = [];
      log = [];
      selectedGroupIds = [];
      return;
    }
    try {
      status = await project.gitStatus();
      if (status.repository) {
        const [nextPreflight, nextRemotes, nextLog, nextEntities] = await Promise.all([
          project.gitStagingPreview(),
          project.gitRemoteList(),
          project.gitLog(),
          project.listEntities(),
        ]);
        preflight = nextPreflight;
        remotes = nextRemotes;
        log = nextLog;
        entities = nextEntities;
        const groups = buildChangeGroups(nextPreflight.staging_paths, nextEntities);
        selectedGroupIds = syncSelectedGroups(groups, selectedGroupIds);
      } else {
        preflight = null;
        remotes = [];
        entities = [];
        log = [];
        selectedGroupIds = [];
      }
    } catch (cause) {
      onError(friendly(cause));
    }
  }

  $effect(() => {
    void projectOpen;
    void refresh();
  });

  function generateMessage() {
    const selected = changeGroups.filter((group) => selectedGroupIds.includes(group.id));
    if (selected.length === 0) {
      commitMessage = "";
      return;
    }
    const buckets = {
      entities: selected.filter((group) => group.kind === "entity").length,
      plugins: selected.filter((group) => group.kind === "plugin").length,
      assets: selected.filter((group) => group.kind === "asset").length,
      other: selected.filter((group) => group.kind === "project").length,
    };
    const parts = [
      buckets.entities ? `${buckets.entities} entit${buckets.entities === 1 ? "y" : "ies"}` : "",
      buckets.plugins ? `${buckets.plugins} plugin${buckets.plugins === 1 ? "" : "s"}` : "",
      buckets.assets ? `${buckets.assets} asset${buckets.assets === 1 ? "" : "s"}` : "",
      buckets.other ? "project settings" : "",
    ].filter(Boolean);
    const headline = `Update ${parts.join(", ") || `${selected.length} change${selected.length === 1 ? "" : "s"}`}`;
    const list = selected
      .slice(0, 12)
      .map((group) => `- ${group.title}${group.subtitle ? ` (${group.subtitle})` : ""}`)
      .join("\n");
    const more = selected.length > 12 ? `\n- …and ${selected.length - 12} more` : "";
    commitMessage = `${headline}\n\n${list}${more}\n`;
  }

  async function initializeGit() {
    await withBusy("Initializing Git…", async () => {
      status = await project.gitInit();
      await refresh();
    });
  }

  async function commitSelected() {
    if (!commitMessage.trim() || selectedPaths.length === 0) return;
    if (beforeWrite && !(await beforeWrite())) return;
    await withBusy("Committing…", async () => {
      status = await project.gitCommit(commitMessage.trim(), selectedPaths);
      commitMessage = "";
      await refresh();
    });
  }

  function openAddRemoteModal() {
    remoteModalMode = "add";
    editingRemoteName = null;
    remoteName = remotes.length === 0 ? "origin" : "";
    remoteUrl = "";
    remoteModalOpen = true;
  }

  function openEditRemoteModal(remote: GitRemote) {
    remoteModalMode = "edit";
    editingRemoteName = remote.name;
    remoteName = remote.name;
    remoteUrl = remote.fetchUrl;
    remoteModalOpen = true;
  }

  function closeRemoteModal() {
    remoteModalOpen = false;
    editingRemoteName = null;
    remoteName = "";
    remoteUrl = "";
  }

  async function submitRemoteModal() {
    if (!remoteUrl.trim()) return;
    if (remoteModalMode === "add") {
      if (!remoteName.trim()) return;
      await withBusy("Adding remote…", async () => {
        remotes = await project.gitRemoteAdd(remoteName.trim(), remoteUrl.trim());
        closeRemoteModal();
      });
      return;
    }
    if (!editingRemoteName) return;
    await withBusy("Updating remote…", async () => {
      remotes = await project.gitRemoteSetUrl(editingRemoteName!, remoteUrl.trim());
      closeRemoteModal();
    });
  }

  async function removeRemote(name: string) {
    await withBusy("Removing remote…", async () => {
      remotes = await project.gitRemoteRemove(name);
    });
  }

  async function openDownload() {
    try {
      await project.openExternalUrl("https://git-scm.com/downloads");
    } catch (cause) {
      onError(friendly(cause));
    }
  }

  async function selectCommit(hash: string) {
    selectedCommit = hash;
    selectedSnapshotPath = null;
    snapshotBody = "";
    await withBusy("Loading snapshot…", async () => {
      snapshotPaths = await project.gitShowTree(hash);
    });
  }

  async function selectSnapshotPath(path: string) {
    if (!selectedCommit) return;
    selectedSnapshotPath = path;
    await withBusy("Loading file…", async () => {
      snapshotBody = await project.gitShowFile(selectedCommit!, path);
    });
  }

  function askReset(hash: string) {
    pendingResetHash = hash;
    showResetConfirm = true;
  }

  async function confirmReset() {
    const hash = pendingResetHash;
    if (!hash) return;
    if (beforeWrite && !(await beforeWrite())) return;
    showResetConfirm = false;
    pendingResetHash = null;
    await withBusy("Restoring snapshot…", async () => {
      const result = await project.gitResetHard(hash);
      status = result.status;
      recoveryUpstream = result.divergedFromUpstream ? result.upstream : null;
      selectedCommit = null;
      snapshotPaths = [];
      snapshotBody = "";
      await refresh();
      if (result.divergedFromUpstream) recoveryUpstream = result.upstream;
    });
  }

  async function forcePushRecovery() {
    if (!recoveryUpstream) return;
    await withBusy("Force-pushing with lease…", async () => {
      status = await project.gitPush(recoveryUpstream!.remote, recoveryUpstream!.branch, true);
      recoveryUpstream = null;
      await refresh();
    });
  }

  async function restoreFromRemote() {
    await withBusy("Restoring from remote…", async () => {
      const result = await project.gitRestoreFromUpstream();
      status = result.status;
      recoveryUpstream = null;
      await refresh();
    });
  }
</script>

<div class="git-settings">
  <div class="settings-section-heading">
    <strong>Git</strong>
    <p>Optional version control for this project's canonical files.</p>
  </div>

  <section class="git-block">
    <h3>Git tool</h3>
    {#if tool === null}
      <p class="settings-empty">Checking Git…</p>
    {:else if tool.available}
      <p class="git-tool-ok">{tool.version}</p>
    {:else}
      <p class="settings-empty">{tool.error ?? "Git was not found on this computer."}</p>
      <button type="button" class="primary-button" onclick={() => void openDownload()}>Download Git</button>
    {/if}
  </section>

  {#if !projectOpen}
    <p class="settings-empty">Open a project to manage remotes, commits, and history.</p>
  {:else if tool && !tool.available}
    <p class="settings-empty">Install Git to use version control for this project.</p>
  {:else if status && !status.repository}
    <section class="git-block">
      <h3>Repository</h3>
      <p class="settings-empty">This project folder is not a Git repository yet.</p>
      <button type="button" class="primary-button" disabled={busy} onclick={() => void initializeGit()}>Initialize Git</button>
    </section>
  {:else if status}
    {#if recoveryUpstream}
      <section class="git-recovery" role="status">
        <strong>Remote history diverged after restore</strong>
        <p>
          Local HEAD no longer matches {recoveryUpstream.remote}/{recoveryUpstream.branch}.
          Force-push with lease rewrites the remote to match this snapshot. Restore from remote undoes the local hard reset using the remote tip.
        </p>
        <div class="git-actions">
          <button type="button" class="danger-button" disabled={busy} onclick={() => void forcePushRecovery()}>Force-push with lease</button>
          <button type="button" class="primary-button" disabled={busy} onclick={() => void restoreFromRemote()}>Restore from remote</button>
        </div>
      </section>
    {/if}

    <section class="git-block">
      <div class="git-block-heading">
        <h3>Remotes</h3>
        <button type="button" class="primary-button" disabled={busy} onclick={openAddRemoteModal}>Add remote</button>
      </div>
      {#if remotes.length === 0}
        <p class="settings-empty">No remotes configured. Add one or more remotes to push and restore history.</p>
      {:else}
        <ul class="git-remote-list">
          {#each remotes as remote}
            <li>
              <div>
                <strong>{remote.name}</strong>
                <small>{remote.fetchUrl}</small>
              </div>
              <div class="git-remote-actions">
                <button type="button" class="quiet-button" disabled={busy} onclick={() => openEditRemoteModal(remote)}>Edit URL</button>
                <button type="button" class="quiet-button" disabled={busy} onclick={() => void removeRemote(remote.name)}>Remove</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="git-block">
      <h3>Changes</h3>
      <p class="git-branch">Branch · {status.branch || "detached"}</p>
      {#if preflight && !preflight.ready}
        <p class="plugin-warning">{preflight.diagnostics[0] ?? "Commit preflight blocked."}</p>
      {/if}
      {#if changeGroups.length === 0}
        <p class="settings-empty">Working tree has no canonical changes to commit.</p>
      {:else}
        <div class="git-actions">
          <button type="button" class="quiet-button" onclick={selectAllGroups}>Select all</button>
          <button type="button" class="quiet-button" onclick={clearGroups}>Select none</button>
        </div>
        <ul class="git-change-list">
          {#each changeGroups as group}
            <li>
              <label>
                <input type="checkbox" checked={groupIsSelected(group.id)} onchange={() => toggleGroup(group.id)} />
                <span>
                  <strong>{group.title}</strong>
                  <small>{group.subtitle}</small>
                </span>
              </label>
            </li>
          {/each}
        </ul>
        <label class="create-input-field" for="git-commit-message">
          <span>Commit message</span>
          <textarea id="git-commit-message" rows="4" bind:value={commitMessage} placeholder="Describe the selected changes"></textarea>
        </label>
        <div class="git-commit-actions">
          <button type="button" class="quiet-button" onclick={generateMessage}>Generate message</button>
          <button
            type="button"
            class="primary-button"
            disabled={busy || !commitMessage.trim() || selectedPaths.length === 0 || !preflight?.ready}
            onclick={() => void commitSelected()}
          >Commit selected</button>
        </div>
      {/if}
    </section>

    <section class="git-block">
      <h3>History</h3>
      {#if log.length === 0}
        <p class="settings-empty">No commits yet.</p>
      {:else}
        <ul class="git-log-list">
          {#each log as entry}
            <li class:active={selectedCommit === entry.hash}>
              <button type="button" class="git-log-button" onclick={() => void selectCommit(entry.hash)}>
                <strong>{entry.subject}</strong>
                <small>{entry.hash} · {entry.date}</small>
              </button>
              <button type="button" class="quiet-button" disabled={busy} onclick={() => askReset(entry.hash)}>Restore…</button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if selectedCommit}
        <div class="git-snapshot">
          <strong>Snapshot files · {selectedCommit}</strong>
          {#if snapshotPaths.length === 0}
            <p class="settings-empty">No canonical files in this snapshot.</p>
          {:else}
            <ul class="git-path-list compact">
              {#each snapshotPaths as path}
                <li>
                  <button type="button" class:active={selectedSnapshotPath === path} class="git-file-button" onclick={() => void selectSnapshotPath(path)}>{path}</button>
                </li>
              {/each}
            </ul>
          {/if}
          {#if selectedSnapshotPath}
            <pre class="git-file-preview">{snapshotBody}</pre>
          {/if}
        </div>
      {/if}
    </section>
  {/if}
</div>

{#if remoteModalOpen}
  <div class="modal-backdrop">
    <div class="dialog" role="dialog" aria-modal="true">
      <div class="new-form-heading">
        <div>
          <span class="panel-kicker">GIT REMOTE</span>
          <strong>{remoteModalMode === "add" ? "Add remote" : `Edit ${editingRemoteName}`}</strong>
        </div>
        <button type="button" class="new-form-close" onclick={closeRemoteModal}>×</button>
      </div>
      <div class="git-remote-form">
        {#if remoteModalMode === "add"}
          <label class="create-input-field" for="git-remote-name">
            <span>Name</span>
            <input id="git-remote-name" bind:value={remoteName} placeholder={remotes.length === 0 ? "origin" : "upstream"} />
          </label>
        {:else}
          <p class="dialog-body-copy">Remote name stays <code>{editingRemoteName}</code>. Update the fetch/push URL below.</p>
        {/if}
        <label class="create-input-field" for="git-remote-url">
          <span>URL</span>
          <input id="git-remote-url" bind:value={remoteUrl} placeholder="https://…" />
        </label>
      </div>
      <div class="new-form-actions">
        <button type="button" class="quiet-button" onclick={closeRemoteModal}>Cancel</button>
        <button
          type="button"
          class="primary-button"
          disabled={busy || !remoteUrl.trim() || (remoteModalMode === "add" && !remoteName.trim())}
          onclick={() => void submitRemoteModal()}
        >{remoteModalMode === "add" ? "Add remote" : "Save URL"}</button>
      </div>
    </div>
  </div>
{/if}

{#if showResetConfirm && pendingResetHash}
  <div class="modal-backdrop">
    <div class="dialog" role="alertdialog" aria-modal="true">
      <div class="new-form-heading">
        <div>
          <span class="panel-kicker">DESTRUCTIVE RESTORE</span>
          <strong>Hard-reset to {pendingResetHash}?</strong>
        </div>
        <button type="button" class="new-form-close" onclick={() => { showResetConfirm = false; pendingResetHash = null; }}>×</button>
      </div>
      <p class="dialog-body-copy">This runs <code>git reset --hard</code>. Later commits are discarded. Uncommitted changes are discarded. Remotes may diverge and need a force-push with lease or a restore from remote.</p>
      <div class="new-form-actions">
        <button type="button" class="quiet-button" onclick={() => { showResetConfirm = false; pendingResetHash = null; }}>Cancel</button>
        <button type="button" class="primary-button danger-button" disabled={busy} onclick={() => void confirmReset()}>Hard-reset to snapshot</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .git-settings { display: grid; gap: 22px; }
  .git-block h3 { margin: 0 0 10px; font-size: 14px; }
  .git-block-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; margin-bottom: 10px; }
  .git-block-heading h3 { margin: 0; }
  .git-tool-ok, .git-branch { margin: 0 0 10px; color: var(--ink-soft); font-size: 12px; }
  .git-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 12px; }
  .git-commit-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 12px; }
  .git-remote-list, .git-path-list, .git-log-list, .git-change-list { list-style: none; margin: 0 0 14px; padding: 0; display: grid; gap: 8px; }
  .git-remote-list li, .git-log-list li { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .git-remote-list strong, .git-remote-list small, .git-log-button strong, .git-log-button small, .git-change-list strong, .git-change-list small { display: block; }
  .git-remote-list small, .git-log-button small, .git-change-list small { margin-top: 3px; color: var(--ink-soft); font-size: 11px; }
  .git-remote-actions { display: flex; flex-wrap: wrap; gap: 6px; }
  .git-remote-form { display: grid; gap: 10px; margin-bottom: 14px; }
  .git-change-list label { display: flex; align-items: flex-start; gap: 10px; font-size: 12px; }
  .git-change-list strong { font-size: 13px; }
  .git-path-list.compact { max-height: 220px; overflow: auto; }
  .git-log-button, .git-file-button { width: 100%; border: 0; background: transparent; text-align: left; cursor: pointer; color: inherit; padding: 8px 0; }
  .git-log-list li.active, .git-file-button.active { color: var(--accent-dark, #365342); }
  .git-snapshot { margin-top: 14px; display: grid; gap: 10px; }
  .git-file-preview { margin: 0; max-height: 280px; overflow: auto; padding: 12px; border: 1px solid var(--line); border-radius: 8px; background: #f7f4ee; font-size: 11px; white-space: pre-wrap; }
  .git-recovery { padding: 14px; border: 1px solid #e5c4b4; border-radius: 10px; background: #fff4ee; }
  .git-recovery p { margin: 8px 0 12px; color: #7a4a36; font-size: 12px; line-height: 1.5; }
  .settings-section-heading { margin-bottom: 0; }
  .settings-section-heading strong { display: block; font-size: 16px; }
  .settings-section-heading p { margin: 6px 0 0; color: var(--ink-soft); font-size: 12px; line-height: 1.5; }
  .settings-empty { margin: 0 0 12px; color: var(--ink-soft); font-size: 13px; }
  .primary-button, .quiet-button, .danger-button {
    border: 0;
    border-radius: 8px;
    cursor: pointer;
    font-size: 12px;
    padding: 9px 12px;
  }
  .primary-button { border: 1px solid rgba(255,255,255,.08); background: var(--accent-dark, #365342); color: #fff; box-shadow: 0 2px 0 #263d30, 0 7px 16px rgba(42,68,51,.16); transition: background .16s ease, box-shadow .16s ease, transform .16s ease; }
  .primary-button:hover { background: #2b4535; box-shadow: 0 2px 0 #263d30, 0 10px 20px rgba(42,68,51,.2); transform: translateY(-1px); }
  .primary-button:active { box-shadow: 0 1px 0 #263d30, 0 3px 8px rgba(42,68,51,.14); transform: translateY(1px); }
  .primary-button:focus-visible { outline: 3px solid rgba(180,119,63,.32); outline-offset: 2px; }
  .quiet-button { border: 1px solid #ded8cd; background: var(--surface, #fffefa); color: var(--ink-soft, #6f6a60); box-shadow: 0 1px 2px rgba(48,45,38,.05); transition: background .16s ease, border-color .16s ease, box-shadow .16s ease, color .16s ease, transform .16s ease; }
  .quiet-button:hover { border-color: #cbbda9; background: #f7f3eb; color: var(--ink, #2b2a24); box-shadow: 0 3px 8px rgba(48,45,38,.08); transform: translateY(-1px); }
  .quiet-button:active { box-shadow: 0 1px 2px rgba(48,45,38,.05); transform: translateY(1px); }
  .quiet-button:focus-visible { outline: 3px solid rgba(180,119,63,.24); outline-offset: 2px; }
  .danger-button, :global(.danger-button) { background: #a1482f; color: #fff; }
  .primary-button:disabled, .quiet-button:disabled, .danger-button:disabled { opacity: 0.55; cursor: default; }
  .create-input-field {
    display: grid;
    gap: 6px;
    font-size: 12px;
  }
  .create-input-field > span {
    color: var(--ink-soft, #6f6a60);
    font-size: 10px;
    font-weight: 700;
  }
  .create-input-field > input,
  .create-input-field > textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 11px;
    border: 1px solid #d9cdbd;
    border-radius: 8px;
    outline: 0;
    background: var(--canvas, #f7f4ee);
    color: var(--ink, #2b2a24);
    font: inherit;
    font-size: 12px;
  }
  .create-input-field > textarea {
    min-height: 78px;
    resize: vertical;
    line-height: 1.5;
  }
  .create-input-field > input:focus,
  .create-input-field > textarea:focus {
    border-color: #c99965;
    box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
  }
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgb(30 28 22 / 45%);
  }
  .dialog {
    width: min(520px, 100%);
    padding: 18px;
    border-radius: 12px;
    background: var(--surface, #fffdf8);
    box-shadow: var(--shadow-lg, 0 18px 40px rgb(40 40 20 / 18%));
  }
  .new-form-heading { display: flex; justify-content: space-between; gap: 12px; margin-bottom: 12px; }
  .new-form-close { border: 0; background: transparent; font-size: 20px; cursor: pointer; }
  .panel-kicker { display: block; margin-bottom: 4px; color: var(--ink-soft); font-size: 10px; font-weight: 700; letter-spacing: .14em; }
  .dialog-body-copy { margin: 0 0 12px; color: var(--ink-soft); font-size: 13px; line-height: 1.55; }
  .new-form-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .plugin-warning { margin: 0 0 10px; color: #a1482f; font-size: 12px; }
</style>
