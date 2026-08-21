<script lang="ts">
import {
  X,
  Sparkles,
  ChevronDown,
  ChevronRight,
  GitBranch,
  DatabaseZap,
  ShieldCheck,
  History,
  Cable,
  FileText,
} from "@lucide/svelte";
import { listen } from "@tauri-apps/api/event";
import {
  project,
  type Entity,
  type GitChange,
  type GitLogEntry,
  type GitPreflight,
  type GitRemote,
  type GitStatus,
  type GitToolInfo,
  type GitUpstream,
  type AiStreamEvent,
} from "$lib/project/client";

let {
  projectOpen,
  projectId,
  onError,
  onBusyMessage,
  beforeWrite,
}: {
  projectOpen: boolean;
  projectId: string;
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
type SnapshotChangeGroup = { label: string; changes: GitChange[]; kind: "added" | "modified" | "deleted" | "other" };

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
let selectedCommitMessage = $state("");
let snapshotChanges = $state<GitChange[]>([]);
let recoveryUpstream = $state<GitUpstream | null>(null);
let snapshotLoadToken = 0;
let diffRequestToken = 0;
let selectedChangePath = $state<string | null>(null);
let changeDiff = $state("");
let diffLoading = $state(false);
let aiMessageBusy = $state(false);
let aiMessageRequestId = $state<string | null>(null);
let aiMessageBase = $state("");
let aiMessageStream = $state("");
let aiMessageLastSequence = $state(-1);
let aiMessageUnlisten: (() => void) | null = null;
let wrapDiffLines = $state(false);
let expandedSnapshotGroups = $state<string[]>([]);
let snapshotChangeGroups = $derived(groupSnapshotChanges(snapshotChanges, entities));
let diffLines = $derived(changeDiff.split("\n").filter((line) => !isDiffMetadata(line)));
type GitConfirmation = {
  title: string;
  message: string;
  confirmLabel: string;
  run: () => Promise<boolean | void>;
  squash?: boolean;
};
let confirmation = $state<GitConfirmation | null>(null);
let confirmationBusy = $state(false);
let squashMessage = $state("Consolidate snapshot history");

function friendly(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

$effect(() => {
  if (!remoteModalOpen) return;
  const frame = window.requestAnimationFrame(() => {
    const nameInput = document.getElementById("git-remote-name");
    const urlInput = document.getElementById("git-remote-url");
    if (nameInput) nameInput.focus();
    else urlInput?.focus();
  });
  return () => window.cancelAnimationFrame(frame);
});

$effect(() => {
  const anyOpen = remoteModalOpen || confirmation !== null || selectedCommit !== null || aiMessageBusy;
  if (!anyOpen) return;
  const onKey = (event: KeyboardEvent) => {
    if (event.key !== "Escape") return;
    event.preventDefault();
    if (confirmation && !confirmationBusy) closeConfirmation();
    else if (remoteModalOpen) closeRemoteModal();
    else if (selectedCommit) closeSnapshotModal();
  };
  window.addEventListener("keydown", onKey, true);
  return () => window.removeEventListener("keydown", onKey, true);
});

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

function clearAiMessageListener() {
  aiMessageUnlisten?.();
  aiMessageUnlisten = null;
}

function handleAiMessageEvent(event: AiStreamEvent) {
  // The same buffered event can also arrive through the live listener. The
  // sequence guard prevents polling from duplicating streamed text or
  // finalizing the request twice.
  if (event.sequence <= aiMessageLastSequence) return;
  aiMessageLastSequence = event.sequence;

  if (event.phase === "delta" && event.delta) {
    aiMessageStream += event.delta;
    commitMessage = appendAiMessage(aiMessageBase, aiMessageStream);
  }
  if (event.phase === "completed") {
    // Normally the terminal output and deltas contain the same response. If
    // a provider's terminal event is truncated, retain the fuller streamed
    // response so comment paragraphs are not lost at completion.
    const terminalText = event.output ?? "";
    const finalText = terminalText.length >= aiMessageStream.length ? terminalText : aiMessageStream;
    commitMessage = appendAiMessage(aiMessageBase, formatSnapshotMessage(finalText));
    aiMessageBusy = false;
    aiMessageRequestId = null;
    clearAiMessageListener();
  } else if (event.phase === "failed" || event.phase === "cancelled" || event.phase === "deadline_exceeded") {
    commitMessage = aiMessageBase;
    aiMessageBusy = false;
    aiMessageRequestId = null;
    clearAiMessageListener();
    if (event.phase !== "cancelled") onError(event.error ?? "Could not generate a snapshot message.");
  }
}

function appendAiMessage(base: string, generated: string) {
  const cleanBase = formatSnapshotMessage(base);
  const cleanGenerated = formatSnapshotMessage(generated);
  return cleanGenerated ? (cleanBase ? `${cleanBase}\n\n${cleanGenerated}` : cleanGenerated) : cleanBase;
}

function formatSnapshotMessage(value: string) {
  const lines = value.replaceAll("\r\n", "\n").split("\n");
  const title = lines.shift()?.trim() ?? "";
  const comments = lines.join("\n").trim();
  return comments ? `${title}\n\n${comments}` : title;
}

async function generateAiMessage() {
  if (!projectId || aiMessageBusy || !preflight?.ready || selectedPaths.length === 0) return;
  aiMessageBusy = true;
  aiMessageBase = commitMessage;
  aiMessageStream = "";
  aiMessageLastSequence = -1;
  try {
    const selectedGroups = changeGroups.filter((group) => selectedGroupIds.includes(group.id));
    const diff = await project.gitWorktreeDiff(selectedPaths);
    const readableDiff = diff
      .split("\n")
      .filter((line) => !isDiffMetadata(line))
      .join("\n")
      .trim();
    const changeLabels = selectedGroups
      .map(
        (group) =>
          `${group.title} (${group.subtitle})\n${group.paths.map((path) => `Changed: ${snapshotChangeLabel(path)}`).join("\n")}`,
      )
      .join("\n\n");
    const previousMessages = log
      .map((entry) => entry.subject.trim())
      .filter(Boolean)
      .slice(0, 5);
    const context = [
      "CONFIRMED SNAPSHOT CHANGES",
      "The labels below identify the affected entities and file roles. The diff is the actual selected project change.",
      changeLabels,
      "ACTUAL DIFF\n" + readableDiff,
      ...(previousMessages.length > 0
        ? [
            "PREVIOUS SNAPSHOT MESSAGES",
            "Use these project-authored messages as style examples only; do not copy their facts:",
            previousMessages.map((message) => `- ${message}`).join("\n"),
          ]
        : []),
    ].join("\n\n");
    const instruction =
      "Write a concise snapshot message from the confirmed changes. Put a title of 72 characters or fewer on the first line. Optionally add the comment body after a newline; never combine them on one line. Use plain text without labels, Markdown, bullets, paths, UUIDs, hashes, or internal identifiers.";
    console.info("[Snapshots AI] sending message-generation request", {
      projectId,
      instruction,
      selection: context,
      selectedPaths,
    });
    const requestId = await project.aiGenerateText(projectId, instruction, context, undefined, context, 1);
    aiMessageRequestId = requestId;
    aiMessageUnlisten = await listen<AiStreamEvent>(`ai-stream:${requestId}`, (event) => {
      handleAiMessageEvent(event.payload);
    });
    const buffered = await project.aiPollText(requestId);
    for (const event of buffered) handleAiMessageEvent(event);
  } catch (cause) {
    clearAiMessageListener();
    commitMessage = aiMessageBase;
    aiMessageBusy = false;
    aiMessageRequestId = null;
    onError(friendly(cause));
  }
}

async function cancelAiMessage() {
  if (!aiMessageRequestId) return;
  try {
    await project.aiCancelText(aiMessageRequestId);
  } catch (cause) {
    onError(friendly(cause));
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

function changeStatus(status: string) {
  return status.slice(0, 1).toUpperCase();
}

function changeKind(status: string): "added" | "modified" | "deleted" {
  const code = changeStatus(status);
  return code === "A" ? "added" : code === "D" ? "deleted" : "modified";
}

function groupSnapshotChanges(changes: GitChange[], entityList: Entity[]): SnapshotChangeGroup[] {
  const groups = new Map<string, GitChange[]>();
  const entityNames = new Map(entityList.map((entity) => [entity.id, entity.name]));
  for (const change of changes) {
    const entityId = change.path.startsWith("entities/") ? change.path.split("/")[1] : null;
    const label = entityId
      ? (entityNames.get(entityId) ?? `Deleted entity (${entityId.slice(0, 8)})`)
      : change.path.startsWith("plugins/")
        ? "Plugins"
        : change.path.startsWith("assets/")
          ? "Assets"
          : "Project";
    groups.set(label, [...(groups.get(label) ?? []), change]);
  }
  return [...groups.entries()].map(([label, groupedChanges]) => ({
    label,
    changes: groupedChanges,
    kind: label.startsWith("Deleted entity")
      ? "deleted"
      : groupedChanges.some((change) => change.path.endsWith("/entity.json") && changeStatus(change.status) === "A")
        ? "added"
        : groupedChanges.some((change) => change.path.endsWith("/entity.json") && changeStatus(change.status) === "D")
          ? "deleted"
          : label === "Project" || label === "Plugins" || label === "Assets"
            ? "other"
            : "modified",
  }));
}

function snapshotChangeLabel(path: string) {
  if (!path.startsWith("entities/")) return path;
  const [, , ...parts] = path.split("/");
  const relative = parts.join("/");
  const fileLabel =
    relative === "document.md"
      ? "Document"
      : relative === "relationships.json"
        ? "Relationships"
        : relative === "assets.json"
          ? "Asset links"
          : relative === "entity.json"
            ? "Identity"
            : relative.startsWith("fields/")
              ? `Field · ${relative.slice("fields/".length)}`
              : relative;
  return fileLabel;
}

function snapshotGroupExpanded(label: string) {
  return expandedSnapshotGroups.includes(label);
}

function toggleSnapshotGroup(label: string) {
  expandedSnapshotGroups = snapshotGroupExpanded(label)
    ? expandedSnapshotGroups.filter((item) => item !== label)
    : [...expandedSnapshotGroups, label];
}

function diffLineClass(line: string) {
  return line.startsWith("+++") || line.startsWith("---")
    ? "diff-file-header"
    : line.startsWith("+")
      ? "diff-added"
      : line.startsWith("-")
        ? "diff-removed"
        : line.startsWith("@@")
          ? "diff-hunk"
          : "diff-context";
}

function isDiffMetadata(line: string) {
  return (
    line.startsWith("diff --git ") ||
    line.startsWith("new file mode ") ||
    line.startsWith("deleted file mode ") ||
    line.startsWith("old mode ") ||
    line.startsWith("new mode ") ||
    line.startsWith("similarity index ") ||
    line.startsWith("rename from ") ||
    line.startsWith("rename to ") ||
    line.startsWith("index ") ||
    line.startsWith("--- ") ||
    line.startsWith("+++ ") ||
    line.startsWith("Binary files ")
  );
}

function shortId(id: string) {
  return id.length > 8 ? id.slice(0, 8) : id;
}

function snapshotDateLabel(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString([], {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
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
let selectedFileCount = $derived(selectedPaths.length);
let totalChangeCount = $derived(preflight?.staging_paths.length ?? 0);

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

function askConfirmation(
  title: string,
  message: string,
  confirmLabel: string,
  run: () => Promise<boolean | void>,
  squash = false,
) {
  confirmation = { title, message, confirmLabel, run, squash };
}

function closeConfirmation() {
  if (confirmationBusy) return;
  confirmation = null;
}

async function runConfirmation() {
  const action = confirmation;
  if (!action) return;
  confirmationBusy = true;
  try {
    const completed = await action.run();
    if (completed !== false) confirmation = null;
  } finally {
    confirmationBusy = false;
  }
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
  await withBusy("Enabling snapshots…", async () => {
    status = await project.gitInit();
    await refresh();
  });
}

async function commitSelected() {
  if (!commitMessage.trim() || selectedPaths.length === 0) return;
  if (beforeWrite && !(await beforeWrite())) return;
  await withBusy("Creating snapshot…", async () => {
    status = await project.gitCommit(formatSnapshotMessage(commitMessage), selectedPaths);
    commitMessage = "";
    await refresh();
  });
}

function askSuperSquash() {
  if (!preflight?.ready || preflight.staging_paths.length > 0 || log.length < 2) return;
  squashMessage = "Consolidate snapshot history";
  askConfirmation(
    "Keep only the latest snapshot?",
    "This permanently replaces the snapshot history with one snapshot representing the latest committed state. All earlier snapshots will be pruned. Remote history may diverge and require an explicit force-push with lease.",
    "Keep latest snapshot",
    async () => {
      if (beforeWrite && !(await beforeWrite())) return false;
      await withBusy("Squashing snapshots…", async () => {
        status = await project.gitSuperSquash(squashMessage.trim() || "Consolidate snapshot history");
        closeSnapshotModal();
        await refresh();
      });
    },
    true,
  );
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

function removeRemote(name: string) {
  askConfirmation(
    `Remove ${name}?`,
    "This removes the remote from this repository. It does not delete anything from the remote server.",
    "Remove remote",
    async () => {
      await withBusy("Removing remote…", async () => {
        remotes = await project.gitRemoteRemove(name);
      });
    },
  );
}

async function openDownload() {
  try {
    await project.openExternalUrl("https://git-scm.com/downloads");
  } catch (cause) {
    onError(friendly(cause));
  }
}

async function selectCommit(hash: string) {
  if (selectedCommit === hash) return;
  const loadToken = ++snapshotLoadToken;
  selectedCommit = hash;
  selectedCommitMessage = "";
  selectedChangePath = null;
  changeDiff = "";
  snapshotChanges = [];
  await withBusy("Loading snapshot…", async () => {
    const [message, changes] = await Promise.all([project.gitShowMessage(hash), project.gitShowChanges(hash)]);
    if (loadToken === snapshotLoadToken && selectedCommit === hash) {
      selectedCommitMessage = message;
      snapshotChanges = changes;
      expandedSnapshotGroups = groupSnapshotChanges(changes, entities).map((group) => group.label);
    }
  });
}

function closeSnapshotModal() {
  snapshotLoadToken += 1;
  diffRequestToken += 1;
  selectedCommit = null;
  selectedCommitMessage = "";
  selectedChangePath = null;
  changeDiff = "";
  snapshotChanges = [];
  expandedSnapshotGroups = [];
}

async function selectSnapshotChange(path: string) {
  if (!selectedCommit) return;
  const token = ++diffRequestToken;
  const commit = selectedCommit;
  const selectedPath = path;
  selectedChangePath = path;
  diffLoading = true;
  try {
    const diff = await project.gitShowDiff(commit, selectedPath);
    if (token === diffRequestToken && selectedCommit === commit && selectedChangePath === selectedPath) {
      changeDiff = diff;
    }
  } catch (cause) {
    if (token === diffRequestToken && selectedCommit === commit && selectedChangePath === selectedPath) {
      onError(friendly(cause));
      changeDiff = "";
    }
  } finally {
    if (token === diffRequestToken) diffLoading = false;
  }
}

function askReset(hash: string) {
  askConfirmation(
    `Hard-reset to ${hash}?`,
    "This discards later commits and all uncommitted changes. Remotes may diverge and need recovery afterward.",
    "Hard-reset to snapshot",
    async () => {
      if (beforeWrite && !(await beforeWrite())) return false;
      await withBusy("Restoring snapshot…", async () => {
        const result = await project.gitResetHard(hash);
        status = result.status;
        recoveryUpstream = result.divergedFromUpstream ? result.upstream : null;
        selectedCommit = null;
        await refresh();
        if (result.divergedFromUpstream) recoveryUpstream = result.upstream;
      });
    },
  );
}

function forcePushRecovery() {
  if (!recoveryUpstream) return;
  const upstream = recoveryUpstream;
  askConfirmation(
    "Rewrite the remote history?",
    `Force-push local history to ${upstream.remote}/${upstream.branch} with lease. This may rewrite the remote history, but the lease protects newer remote work.`,
    "Force-push with lease",
    async () => {
      await withBusy("Force-pushing with lease…", async () => {
        status = await project.gitPush(upstream.remote, upstream.branch, true);
        recoveryUpstream = null;
        await refresh();
      });
    },
  );
}

function restoreFromRemote() {
  askConfirmation(
    "Restore from the remote?",
    "This discards the local hard-reset state and rebuilds the project index from the upstream remote.",
    "Restore from remote",
    async () => {
      await withBusy("Restoring from remote…", async () => {
        const result = await project.gitRestoreFromUpstream();
        status = result.status;
        recoveryUpstream = null;
        await refresh();
      });
    },
  );
}
</script>

<div class="git-settings">
  <div class="panel-hero">
    <div class="hero-icon">
      <GitBranch size={18} strokeWidth={1.8} aria-hidden="true" />
    </div>
    <div class="hero-copy">
      <span class="kicker">VERSIONING</span>
      <strong>Snapshots</strong>
      <p>Save named versions of this project’s canonical files. Every snapshot is a Git commit you can restore.</p>
    </div>
    <div class="hero-stats" aria-label="Snapshot summary">
      <span class="stat-pill"
        ><GitBranch size={12} strokeWidth={1.8} aria-hidden="true" /> {status?.branch || "No repo"}</span>
      <span class="stat-pill"><History size={12} strokeWidth={1.8} aria-hidden="true" /> {log.length} snapshots</span>
      <span class="stat-pill"><Cable size={12} strokeWidth={1.8} aria-hidden="true" /> {remotes.length} remotes</span>
    </div>
  </div>

  <section class="git-block elevated">
    <div class="block-heading">
      <div class="heading-left">
        <span class="heading-icon"><DatabaseZap size={14} strokeWidth={1.8} aria-hidden="true" /></span>
        <h3>Version control</h3>
      </div>
      <span class="block-hint">Git availability</span>
    </div>
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
    <div class="empty-inline">
      <GitBranch size={16} strokeWidth={1.7} aria-hidden="true" />
      <div>
        <strong>Open a project to manage snapshots</strong><span>Snapshots, remotes, and history are per-project.</span>
      </div>
    </div>
  {:else if tool && !tool.available}
    <div class="empty-inline">
      <DatabaseZap size={16} strokeWidth={1.7} aria-hidden="true" />
      <div><strong>Git not available</strong><span>Install Git to save snapshots for this project.</span></div>
    </div>
  {:else if status && !status.repository}
    <section class="git-block elevated">
      <div class="block-heading">
        <div class="heading-left">
          <span class="heading-icon"><ShieldCheck size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h3>Repository</h3>
        </div>
      </div>
      <div class="empty-inline">
        <ShieldCheck size={16} strokeWidth={1.7} aria-hidden="true" />
        <div>
          <strong>Snapshots not enabled yet</strong><span>Enable snapshots to start versioning this project.</span>
        </div>
      </div>
      <button type="button" class="primary-button" disabled={busy} onclick={() => void initializeGit()}
        >Enable snapshots</button>
    </section>
  {:else if status}
    <section class="git-overview elevated" aria-label="Repository status">
      <div>
        <span class="panel-kicker">REPOSITORY</span>
        <strong>{status.branch || "Detached HEAD"}</strong>
        <small
          >{totalChangeCount === 0 ? "No snapshot-ready changes" : `${totalChangeCount} snapshot-ready changes`}</small>
      </div>
      <div class:git-overview-warn={!preflight?.ready || totalChangeCount > 0} class="git-overview-stat">
        <strong>{totalChangeCount}</strong>
        <small>{totalChangeCount === 1 ? "snapshot-ready change" : "snapshot-ready changes"}</small>
      </div>
      <button type="button" class="quiet-button" disabled={busy} onclick={() => void refresh()}>Refresh</button>
    </section>
    {#if recoveryUpstream}
      <section class="git-recovery elevated" role="status">
        <strong>Remote history diverged after restore</strong>
        <p>
          Local HEAD no longer matches {recoveryUpstream.remote}/{recoveryUpstream.branch}. Force-push with lease
          rewrites the remote to match this snapshot. Restore from remote undoes the local hard reset using the remote
          tip.
        </p>
        <div class="git-actions">
          <button type="button" class="danger-button" disabled={busy} onclick={() => void forcePushRecovery()}
            >Force-push with lease</button>
          <button type="button" class="primary-button" disabled={busy} onclick={() => void restoreFromRemote()}
            >Restore from remote</button>
        </div>
      </section>
    {/if}

    <section class="git-block elevated">
      <div class="block-heading">
        <div class="heading-left">
          <span class="heading-icon"><Cable size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h3>Remotes</h3>
          <span class="count-badge">{remotes.length}</span>
        </div>
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
                <button type="button" class="quiet-button" disabled={busy} onclick={() => openEditRemoteModal(remote)}
                  >Edit URL</button>
                <button
                  type="button"
                  class="quiet-button"
                  disabled={busy}
                  onclick={() => void removeRemote(remote.name)}>Remove</button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="git-block elevated">
      <div class="block-heading">
        <div class="heading-left">
          <span class="heading-icon"><FileText size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h3>Changes</h3>
          <span class="count-badge">{changeGroups.length}</span>
        </div>
        <span class="block-hint">Canonical files only</span>
      </div>
      <p class="git-section-copy">Choose the canonical project changes to include in the next snapshot.</p>
      {#if preflight && !preflight.ready}
        <p class="plugin-warning">{preflight.diagnostics[0] ?? "Commit preflight blocked."}</p>
      {/if}
      {#if changeGroups.length === 0}
        <p class="settings-empty">Working tree has no canonical changes to commit.</p>
      {:else}
        <div class="git-change-toolbar">
          <div>
            <strong>{selectedFileCount} of {totalChangeCount} files selected</strong><small
              >Selection is limited to canonical project files.</small>
          </div>
          <div class="git-actions">
            <button type="button" class="quiet-button" onclick={selectAllGroups}>Select all</button>
            <button type="button" class="quiet-button" onclick={clearGroups}>Select none</button>
          </div>
        </div>
        <ul class="git-change-list">
          {#each changeGroups as group}
            <li class:selected={groupIsSelected(group.id)}>
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
        <div class="git-commit-card">
          <label class="create-input-field git-message-field" for="git-commit-message">
            <span>Snapshot message</span>
            <textarea
              id="git-commit-message"
              rows="4"
              bind:value={commitMessage}
              placeholder="Describe this snapshot"
              disabled={busy || aiMessageBusy}></textarea>
            <button
              type="button"
              class="git-ai-message-button"
              aria-label={aiMessageBusy ? "Cancel AI message generation" : "Generate snapshot message with AI"}
              title={aiMessageBusy ? "Cancel generation" : "Generate snapshot message with AI"}
              disabled={busy || (!aiMessageBusy && (!preflight?.ready || selectedPaths.length === 0))}
              onclick={() => void (aiMessageBusy ? cancelAiMessage() : generateAiMessage())}
              >{#if aiMessageBusy}<X size={14} strokeWidth={1.8} aria-hidden="true" />{:else}<Sparkles
                  size={14}
                  strokeWidth={1.8}
                  aria-hidden="true" />{/if}</button>
          </label>
          <div class="git-commit-actions">
            <button type="button" class="quiet-button" onclick={generateMessage}>Generate message</button>
            <button
              type="button"
              class="primary-button"
              disabled={busy || !commitMessage.trim() || selectedPaths.length === 0 || !preflight?.ready}
              onclick={() => void commitSelected()}
              >Create snapshot · {selectedFileCount} {selectedFileCount === 1 ? "file" : "files"}</button>
          </div>
        </div>
      {/if}
    </section>

    <section class="git-block elevated">
      <div class="block-heading">
        <div class="heading-left">
          <span class="heading-icon"><History size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h3>Snapshot history</h3>
          <span class="count-badge">{log.length}</span>
        </div>
        {#if preflight?.staging_paths.length}<small class="git-section-note"
            >Commit pending changes before squashing history.</small
          >{/if}
        {#if log.length > 1}<button
            type="button"
            class="quiet-button"
            disabled={busy || !preflight?.ready || preflight.staging_paths.length > 0}
            onclick={askSuperSquash}>Keep latest</button
          >{/if}
      </div>
      {#if log.length === 0}
        <p class="settings-empty">No snapshots yet.</p>
      {:else}
        <ul class="git-log-list">
          {#each log as entry}
            <li class:active={selectedCommit === entry.hash}>
              <button type="button" class="git-log-button" onclick={() => void selectCommit(entry.hash)}>
                <strong>{entry.subject}</strong>
                <small>{entry.hash} · {snapshotDateLabel(entry.date)}</small>
              </button>
              <button type="button" class="quiet-button" disabled={busy} onclick={() => askReset(entry.hash)}
                >Restore</button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if selectedCommit}
        {@const snapshotEntry = log.find((entry) => entry.hash === selectedCommit)}
        <div class="modal-backdrop">
          <div class="dialog git-snapshot-dialog" role="dialog" aria-modal="true" aria-labelledby="snapshot-title">
            <div class="new-form-heading">
              <div>
                <span class="panel-kicker">SNAPSHOT DETAILS</span>
                <strong id="snapshot-title">{snapshotEntry?.subject ?? "Snapshot details"}</strong>
                <small class="git-snapshot-meta"
                  >{snapshotEntry ? snapshotDateLabel(snapshotEntry.date) : selectedCommit} · {selectedCommit}</small>
                {#if selectedCommitMessage}
                  {@const messageParts = selectedCommitMessage.replaceAll("\r\n", "\n").split("\n")}
                  {@const messageBody = messageParts.slice(1).join("\n").trim()}
                  {#if messageBody}
                    <p class="git-snapshot-comment">{messageBody}</p>
                  {/if}
                {/if}
              </div>
              <button
                type="button"
                class="new-form-close"
                aria-label="Close snapshot details"
                onclick={closeSnapshotModal}><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
            </div>
            <div class="git-snapshot-summary">
              <strong>{snapshotChanges.length} changed {snapshotChanges.length === 1 ? "file" : "files"}</strong>
              <span>Select a file to inspect its diff.</span>
            </div>
            {#if snapshotChanges.length === 0}
              <p class="settings-empty">{busy ? "Loading changes…" : "No canonical file changes in this snapshot."}</p>
            {:else}
              <div class="git-snapshot-layout">
                <div class="git-snapshot-files">
                  {#each snapshotChangeGroups as group}
                    <section
                      class:change-added={group.kind === "added"}
                      class:change-modified={group.kind === "modified"}
                      class:change-deleted={group.kind === "deleted"}
                      class="git-change-category">
                      <button
                        type="button"
                        class="git-change-category-toggle"
                        onclick={() => toggleSnapshotGroup(group.label)}>
                        <span class="git-tree-chevron" aria-hidden="true"
                          >{#if snapshotGroupExpanded(group.label)}<ChevronDown
                              size={14}
                              strokeWidth={1.8}
                              aria-hidden="true" />{:else}<ChevronRight
                              size={14}
                              strokeWidth={1.8}
                              aria-hidden="true" />{/if}</span>
                        <span class="git-change-kind">{group.kind === "other" ? "Changes" : group.kind}</span>
                        <span>{group.label}</span>
                        <small>{group.changes.length}</small>
                      </button>
                      {#if snapshotGroupExpanded(group.label)}<ul class="git-path-list compact">
                          {#each group.changes as change (change.path)}
                            <li>
                              <button
                                type="button"
                                class:active={selectedChangePath === change.path}
                                class="git-file-button"
                                onclick={() => void selectSnapshotChange(change.path)}
                                ><span
                                  class:change-added={changeStatus(change.status) === "A"}
                                  class:change-deleted={changeStatus(change.status) === "D"}
                                  class="git-change-status">{changeStatus(change.status)}</span
                                ><span>{snapshotChangeLabel(change.path)}</span></button>
                            </li>
                          {/each}
                        </ul>{/if}
                    </section>
                  {/each}
                </div>
                <div class="git-diff-panel">
                  {#if diffLoading}
                    <p class="settings-empty">Loading diff…</p>
                  {:else if selectedChangePath}
                    <div class="git-diff-heading">
                      <strong>{selectedChangePath}</strong>
                      <label class="diff-wrap-toggle">
                        <input type="checkbox" bind:checked={wrapDiffLines} />
                        <span>Wrap lines</span>
                      </label>
                    </div>
                    {#if changeDiff}
                      <pre class:diff-wrap={wrapDiffLines} class="git-diff-view">{#each diffLines as line}<span
                            class={diffLineClass(line)}
                            >{line}
</span>{/each}</pre>
                    {:else}
                      <p class="settings-empty">No textual diff available for this file.</p>
                    {/if}
                  {:else}
                    <p class="settings-empty">Choose a changed file to view its diff.</p>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
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
          <span class="panel-kicker">SNAPSHOT REMOTE</span>
          <strong>{remoteModalMode === "add" ? "Add remote" : `Edit ${editingRemoteName}`}</strong>
        </div>
        <button type="button" class="new-form-close" onclick={closeRemoteModal}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </div>
      <div class="git-remote-form">
        {#if remoteModalMode === "add"}
          <label class="create-input-field" for="git-remote-name">
            <span>Name</span>
            <input
              id="git-remote-name"
              bind:value={remoteName}
              placeholder={remotes.length === 0 ? "origin" : "upstream"} />
          </label>
        {:else}
          <p class="dialog-body-copy">
            Remote name stays <code>{editingRemoteName}</code>. Update the fetch/push URL below.
          </p>
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
          onclick={() => void submitRemoteModal()}>{remoteModalMode === "add" ? "Add remote" : "Save URL"}</button>
      </div>
    </div>
  </div>
{/if}

{#if confirmation}
  <div class="modal-backdrop">
    <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="git-confirm-title">
      <div class="new-form-heading">
        <div>
          <span class="panel-kicker">CONFIRM SNAPSHOT ACTION</span>
          <strong id="git-confirm-title">{confirmation.title}</strong>
        </div>
        <button type="button" class="new-form-close" disabled={confirmationBusy} onclick={closeConfirmation}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </div>
      <p class="dialog-body-copy">{confirmation.message}</p>
      {#if confirmation.squash}
        <label class="create-input-field squash-message-field" for="squash-message">
          <span>Snapshot message <small>(optional)</small></span>
          <input id="squash-message" bind:value={squashMessage} placeholder="Consolidate snapshot history" />
        </label>
      {/if}
      <div class="new-form-actions">
        <button type="button" class="quiet-button" disabled={confirmationBusy} onclick={closeConfirmation}
          >Cancel</button>
        <button
          type="button"
          class="primary-button danger-button"
          disabled={busy || confirmationBusy}
          onclick={() => void runConfirmation()}>{confirmation.confirmLabel}</button>
      </div>
    </div>
  </div>
{/if}

<style>
.git-settings {
  display: grid;
  gap: 16px;
}
.git-overview,
.git-block {
  display: grid;
  gap: 12px;
  padding: 17px 18px;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.git-overview {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 18px;
}
.git-overview strong,
.git-overview small {
  display: block;
}
.git-overview > div:first-child strong {
  font-size: 15px;
}
.git-overview small {
  margin-top: 4px;
  color: var(--ink-soft);
  font-size: 11px;
}
.git-overview-stat {
  min-width: 92px;
  padding-left: 18px;
  border-left: 1px solid var(--line);
}
.git-overview-stat strong {
  color: var(--accent-dark);
  font-size: 19px;
}
.git-overview-stat.git-overview-warn strong {
  color: var(--accent);
}
.git-section-copy {
  margin: -4px 0 13px;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.45;
}
.git-section-note {
  display: block;
  margin-top: 4px;
  color: var(--accent);
  font-size: 11px;
  font-weight: 500;
}
.git-block h3 {
  margin: 0 0 10px;
  font-size: 14px;
}
.git-block-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.git-tool-ok {
  margin: 0 0 10px;
  color: var(--ink-soft);
  font-size: 12px;
}
.git-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 12px;
}
.git-change-toolbar {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.git-change-toolbar strong,
.git-change-toolbar small {
  display: block;
}
.git-change-toolbar strong {
  font-size: 12px;
}
.git-change-toolbar small {
  margin-top: 3px;
  color: var(--ink-soft);
  font-size: 11px;
}
.git-change-toolbar .git-actions {
  margin: 0;
}
.git-commit-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}
.git-commit-card {
  display: grid;
  gap: 10px;
  padding: 14px;
  border: 1px solid #e5d8c6;
  border-radius: 10px;
  background: #fcf8f1;
}
.git-remote-list,
.git-path-list,
.git-log-list,
.git-change-list {
  list-style: none;
  margin: 0 0 14px;
  padding: 0;
  display: grid;
  gap: 8px;
}
.git-remote-list li,
.git-log-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.git-remote-list strong,
.git-remote-list small,
.git-log-button strong,
.git-log-button small,
.git-change-list strong,
.git-change-list small {
  display: block;
}
.git-remote-list small,
.git-log-button small,
.git-change-list small {
  margin-top: 3px;
  color: var(--ink-soft);
  font-size: 11px;
}
.git-remote-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.git-remote-form {
  display: grid;
  gap: 10px;
  margin-bottom: 14px;
}
.git-change-list label {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  font-size: 12px;
}
.git-change-list li {
  padding: 9px 10px;
  border: 1px solid transparent;
  border-radius: 8px;
  background: var(--canvas, #f7f4ee);
}
.git-change-list li.selected {
  border-color: #d8c3a5;
  background: #fffaf2;
}
.git-change-list strong {
  font-size: 13px;
}
.git-path-list.compact {
  max-height: 220px;
  overflow: auto;
}
.git-log-button,
.git-file-button {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 9px;
  border: 0;
  background: transparent;
  text-align: left;
  cursor: pointer;
  color: inherit;
  font-size: 12px;
  line-height: 1.35;
  padding: 8px 0;
}
.git-change-status {
  flex: 0 0 22px;
  color: var(--accent);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 10px;
  font-weight: 800;
  text-align: center;
}
.git-change-status.change-added {
  color: #3f8b4d;
}
.git-change-status.change-deleted {
  color: #a44d42;
}
.git-log-list li.active,
.git-file-button.active {
  color: var(--accent-dark, #365342);
}
.git-snapshot-dialog {
  width: calc(100vw - 64px);
  max-width: 1400px;
  min-width: 960px;
  max-height: calc(100vh - 48px);
  overflow: auto;
}
.git-snapshot-meta {
  display: block;
  min-width: 0;
  overflow-wrap: anywhere;
  margin-top: 4px;
  color: var(--ink-soft);
  font-size: 11px;
}
.git-snapshot-comment {
  margin: 12px 0 0;
  max-width: 78ch;
  color: var(--ink-soft);
  white-space: pre-wrap;
  line-height: 1.55;
}
.git-snapshot-summary {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--line);
}
.git-snapshot-summary span {
  color: var(--ink-soft);
  font-size: 11px;
}
.git-snapshot-layout {
  display: grid;
  grid-template-columns: minmax(280px, 0.72fr) minmax(0, 1.8fr);
  gap: 16px;
}
.git-snapshot-files {
  min-width: 0;
  max-height: 460px;
  overflow: auto;
}
.git-change-category + .git-change-category {
  margin-top: 15px;
}
.git-change-category-toggle {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 6px;
  padding: 7px 9px;
  border-top: 0;
  border-right: 0;
  border-bottom: 0;
  border-left: 3px solid var(--line);
  border-radius: 5px;
  background: var(--canvas);
  color: var(--ink-soft);
  font-size: 11px;
  font-family: inherit;
  font-weight: 800;
  letter-spacing: 0.04em;
  text-align: left;
  cursor: pointer;
}
.git-change-category.change-added .git-change-category-toggle {
  border-left-color: #5b9b68;
  background: #edf7ec;
  color: #3f7449;
}
.git-change-category.change-modified .git-change-category-toggle {
  border-left-color: #c9973e;
  background: #fff7e5;
  color: #946c24;
}
.git-change-category.change-deleted .git-change-category-toggle {
  border-left-color: #b85b4e;
  background: #fbecea;
  color: #98443a;
}
.git-change-kind {
  flex: 0 0 auto;
  font-size: 9px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}
.git-tree-chevron {
  flex: 0 0 10px;
  font-size: 14px;
  line-height: 1;
}
.git-change-category-toggle > span:nth-last-of-type(1) {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.git-change-category-toggle small {
  margin-left: auto;
  color: inherit;
  font-size: 10px;
}
.git-change-category .git-path-list {
  margin-bottom: 0;
}
.git-diff-panel {
  min-width: 0;
  min-height: 240px;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--canvas);
}
.git-diff-heading {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 9px;
  font-size: 11px;
}
.git-diff-heading strong {
  min-width: 0;
  overflow-wrap: anywhere;
}
.diff-wrap-toggle {
  flex: 0 0 auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--ink-soft);
  font-size: 10px;
}
.diff-wrap-toggle input {
  width: 14px;
  height: 14px;
  margin: 0;
  accent-color: var(--accent-dark);
}
.git-file-preview {
  margin: 0;
  max-height: 420px;
  overflow: auto;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #f7f4ee;
  font-size: 11px;
  white-space: pre-wrap;
}
.git-diff-view {
  max-height: 420px;
  margin: 0;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: #f7f4ee;
  color: var(--ink);
  font:
    11px/1.55 ui-monospace,
    SFMono-Regular,
    Menlo,
    monospace;
  white-space: pre;
}
.git-diff-view.diff-wrap {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.git-diff-view span {
  display: block;
  min-height: 1.55em;
  padding: 0 10px;
}
.git-diff-view .diff-added {
  background: #e7f3e5;
  color: #2f6b3b;
}
.git-diff-view .diff-removed {
  background: #f7e5e1;
  color: #9b4438;
}
.git-diff-view .diff-hunk {
  background: #e8edf6;
  color: #4a638c;
}
.git-diff-view .diff-file-header {
  color: var(--ink-soft);
  font-weight: 700;
}
@media (max-width: 700px) {
  .git-snapshot-dialog {
    width: calc(100vw - 24px);
    min-width: 0;
  }
  .git-snapshot-layout {
    grid-template-columns: 1fr;
  }
  .git-diff-panel {
    min-height: 180px;
  }
}
.git-recovery {
  padding: 14px;
  border: 1px solid #e5c4b4;
  border-radius: 10px;
  background: #fff4ee;
}
.git-recovery p {
  margin: 8px 0 12px;
  color: #7a4a36;
  font-size: 12px;
  line-height: 1.5;
}
.settings-empty {
  margin: 0 0 12px;
  color: var(--ink-soft);
  font-size: 13px;
}
.primary-button,
.quiet-button,
.danger-button {
  border: 0;
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
  padding: 9px 12px;
}
.primary-button {
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: var(--accent-dark, #365342);
  color: #fff;
  box-shadow:
    0 2px 0 #263d30,
    0 7px 16px rgba(42, 68, 51, 0.16);
  transition:
    background 0.16s ease,
    box-shadow 0.16s ease,
    transform 0.16s ease;
}
.primary-button:hover {
  background: #2b4535;
  box-shadow:
    0 2px 0 #263d30,
    0 10px 20px rgba(42, 68, 51, 0.2);
  transform: translateY(-1px);
}
.primary-button:active {
  box-shadow:
    0 1px 0 #263d30,
    0 3px 8px rgba(42, 68, 51, 0.14);
  transform: translateY(1px);
}
.primary-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.32);
  outline-offset: 2px;
}
.quiet-button {
  border: 1px solid #ded8cd;
  background: var(--surface, #fffefa);
  color: var(--ink-soft, #6f6a60);
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}
.quiet-button:hover {
  border-color: #cbbda9;
  background: #f7f3eb;
  color: var(--ink, #2b2a24);
  box-shadow: 0 3px 8px rgba(48, 45, 38, 0.08);
  transform: translateY(-1px);
}
.quiet-button:active {
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transform: translateY(1px);
}
.quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.24);
  outline-offset: 2px;
}
.danger-button,
:global(.danger-button) {
  background: #a1482f;
  color: #fff;
}
.primary-button:disabled,
.quiet-button:disabled,
.danger-button:disabled {
  opacity: 0.55;
  cursor: default;
}
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
.git-message-field {
  position: relative;
}
.git-message-field > textarea {
  padding-bottom: 30px;
}
.git-ai-message-button {
  position: absolute;
  bottom: 9px;
  left: 9px;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: #f2e4d2;
  color: var(--accent);
  font-size: 13px;
  cursor: pointer;
}
.git-ai-message-button:hover {
  background: #ead7bc;
}
.git-ai-message-button:disabled {
  opacity: 0.55;
  cursor: wait;
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
.new-form-heading {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.new-form-close {
  border: 0;
  background: transparent;
  font-size: 20px;
  cursor: pointer;
}
.panel-kicker {
  display: block;
  margin-bottom: 4px;
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.14em;
}
.dialog-body-copy {
  margin: 0 0 12px;
  color: var(--ink-soft);
  font-size: 13px;
  line-height: 1.55;
}
.squash-message-field {
  margin: 16px 0 4px;
}
.squash-message-field > span small {
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 500;
}
.new-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.plugin-warning {
  margin: 0 0 10px;
  color: #a1482f;
  font-size: 12px;
}
@media (max-width: 620px) {
  .git-overview {
    grid-template-columns: 1fr auto;
  }
  .git-overview-stat {
    padding-left: 0;
    border-left: 0;
    text-align: right;
  }
  .git-overview > .quiet-button {
    grid-column: 1 / -1;
    justify-self: start;
  }
  .git-change-toolbar {
    align-items: stretch;
    flex-direction: column;
  }
  .git-change-toolbar .git-actions {
    margin-top: 2px;
  }
  .git-remote-list li,
  .git-log-list li {
    align-items: flex-start;
    flex-direction: column;
  }
}

.panel-hero {
  display: grid;
  grid-template-columns: 40px 1fr;
  gap: 14px;
  padding: 16px 16px 14px;
  border: 1px solid var(--line, #e9e1d4);
  border-radius: 14px;
  background: var(--surface, #fffefa);
}
.hero-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: #fffefa;
}
.hero-copy .kicker {
  color: #b4773f;
  font:
    700 10px/1 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.hero-copy strong {
  display: block;
  margin-top: 3px;
  color: var(--ink);
  font: 600 16px/1.15 var(--font-display, Georgia, serif);
}
.hero-copy p {
  margin: 6px 0 0;
  max-width: 640px;
  color: var(--ink-soft, #8f897e);
  font:
    400 12.5px/1.5 Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.hero-stats {
  grid-column: 2;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 2px;
}
.stat-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 9px;
  border-radius: 999px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
  font:
    600 11px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.block-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid #f0e8d9;
}
.heading-left {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}
.heading-icon {
  width: 28px;
  height: 28px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
}
.count-badge {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  padding: 0 7px;
  border-radius: 999px;
  background: #f4eee3;
  border: 1px solid #e9e1d4;
  color: #62594e;
  font:
    700 11px Inter,
    sans-serif;
}
.block-hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--ink-faint, #b0a89c);
  font:
    500 11.5px Inter,
    ui-sans-serif,
    system-ui,
    sans-serif;
}
.empty-inline {
  display: flex;
  gap: 12px;
  align-items: flex-start;
  padding: 14px 14px;
  border: 1px dashed #d9cdbd;
  border-radius: 11px;
  background: #fffcf7;
  color: #8f897e;
}
.empty-inline strong {
  display: block;
  color: var(--ink);
  font:
    600 13px Inter,
    sans-serif;
  margin-bottom: 3px;
}
.empty-inline span {
  font:
    400 12px/1.5 Inter,
    sans-serif;
}
.git-block.elevated,
.git-overview.elevated,
.git-recovery.elevated {
  box-shadow:
    0 1px 0 rgba(48, 44, 38, 0.03),
    0 8px 24px rgba(48, 44, 38, 0.04);
}
</style>
