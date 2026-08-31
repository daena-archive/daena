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
import { onDestroy, untrack } from "svelte";
import { formatDiffLineForDisplay } from "$lib/git/diff-display";
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
  aiEnabled = false,
  onError,
  onBusyMessage,
  beforeWrite,
  onStatusChange,
}: {
  projectOpen: boolean;
  projectId: string;
  aiEnabled?: boolean;
  onError: (message: string) => void;
  onBusyMessage?: (message: string) => void;
  beforeWrite?: () => Promise<boolean>;
  onStatusChange?: (status: GitStatus | null) => void;
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
let messageExpanded = $state(true);
let changesExpanded = $state(true);
let repositoryExpanded = $state(false);
let expandedHistoryHash = $state<string | null>(null);
let historyMessages = $state<Record<string, string>>({});
let historyMessageLoading = $state<string | null>(null);
let busy = $state(false);
let loading = $state(false);
let loadError = $state("");
let remoteModalOpen = $state(false);
let remoteModalMode = $state<RemoteModalMode>("add");
let remoteName = $state("");
let remoteUrl = $state("");
let editingRemoteName = $state<string | null>(null);
let selectedCommit = $state<string | null>(null);
let selectedCommitMessage = $state("");
let snapshotChanges = $state<GitChange[]>([]);
let recoveryUpstream = $state<GitUpstream | null>(null);
let refreshToken = 0;
let snapshotLoadToken = 0;
let diffRequestToken = 0;
let historyMessageToken = 0;
let busyDepth = 0;
let selectionProjectId: string | null = null;
let remoteDialog = $state<HTMLElement | null>(null);
let confirmationDialog = $state<HTMLElement | null>(null);
let snapshotDialog = $state<HTMLElement | null>(null);
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
let commitMessageTitle = $derived(snapshotMessageTitle(commitMessage));
let commitMessageBody = $derived(snapshotMessageBody(commitMessage));
let diffLines = $derived(
  changeDiff
    .split("\n")
    .filter((line) => !isDiffMetadata(line))
    .map((line) => formatDiffLineForDisplay(selectedChangePath, line)),
);
type GitConfirmation = {
  title: string;
  message: string;
  confirmLabel: string;
  run: () => Promise<boolean>;
  squash?: boolean;
};
let confirmation = $state<GitConfirmation | null>(null);
let confirmationBusy = $state(false);
let squashMessage = $state("Consolidate snapshot history");

function friendly(cause: unknown) {
  return cause instanceof Error ? cause.message : String(cause);
}

function snapshotMessageTitle(value: string) {
  return value.replaceAll("\r\n", "\n").split("\n")[0]?.trim() ?? "";
}

function snapshotMessageBody(value: string) {
  return value.replaceAll("\r\n", "\n").split("\n").slice(1).join("\n").trim();
}

function notifyStatus(nextStatus: GitStatus | null) {
  status = nextStatus;
  onStatusChange?.(nextStatus);
}

function trapModalFocus(event: KeyboardEvent, dialog: HTMLElement | null) {
  if (event.key !== "Tab" || !dialog) return;
  const focusable = [
    ...dialog.querySelectorAll<HTMLElement>(
      'button:not(:disabled), input:not(:disabled), textarea:not(:disabled), select:not(:disabled), [href], [tabindex]:not([tabindex="-1"])',
    ),
  ].filter((element) => !element.hasAttribute("hidden"));
  if (focusable.length === 0) {
    event.preventDefault();
    dialog.focus();
    return;
  }
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
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
  if (!confirmation) return;
  const frame = window.requestAnimationFrame(() => document.getElementById("git-confirm-cancel")?.focus());
  return () => window.cancelAnimationFrame(frame);
});

$effect(() => {
  if (!selectedCommit) return;
  const frame = window.requestAnimationFrame(() => document.getElementById("git-snapshot-close")?.focus());
  return () => window.cancelAnimationFrame(frame);
});

$effect(() => {
  const anyOpen = remoteModalOpen || confirmation !== null || selectedCommit !== null;
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
  busyDepth += 1;
  busy = true;
  onBusyMessage?.(label);
  try {
    return await run();
  } catch (cause) {
    onError(friendly(cause));
    return undefined;
  } finally {
    busyDepth = Math.max(0, busyDepth - 1);
    busy = busyDepth > 0;
    if (!busy) onBusyMessage?.("");
  }
}

async function prepareWrite() {
  if (!beforeWrite) return true;
  try {
    return await beforeWrite();
  } catch (cause) {
    onError(friendly(cause));
    return false;
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
    if (commitMessage.trim()) messageExpanded = false;
    aiMessageBusy = false;
    aiMessageRequestId = null;
    clearAiMessageListener();
  } else if (event.phase === "failed" || event.phase === "cancelled" || event.phase === "deadline_exceeded") {
    const partialText = event.output ?? aiMessageStream;
    commitMessage =
      event.phase === "deadline_exceeded" && partialText
        ? appendAiMessage(aiMessageBase, formatSnapshotMessage(partialText))
        : aiMessageBase;
    if (event.phase === "deadline_exceeded" && commitMessage.trim()) messageExpanded = false;
    aiMessageBusy = false;
    aiMessageRequestId = null;
    clearAiMessageListener();
    if (event.phase === "deadline_exceeded" && partialText) {
      onError("AI generation reached its time limit. The partial snapshot message is preserved.");
    } else if (event.phase !== "cancelled") {
      onError(event.error ?? "Could not generate a snapshot message.");
    }
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
  if (!aiEnabled || !projectId || aiMessageBusy || !preflight?.ready || selectedPaths.length === 0) return;
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
let snapshotBlockReason = $derived(
  !preflight?.ready
    ? "Resolve the snapshot diagnostics before continuing."
    : selectedPaths.length === 0
      ? "Select at least one change."
      : !commitMessage.trim()
        ? "Add a snapshot message."
        : "",
);

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

function toggleMessageEditor() {
  if (!commitMessage.trim()) {
    messageExpanded = true;
    return;
  }
  messageExpanded = !messageExpanded;
}

async function toggleHistoryMessage(hash: string) {
  if (expandedHistoryHash === hash) {
    expandedHistoryHash = null;
    return;
  }
  expandedHistoryHash = hash;
  if (historyMessages[hash] !== undefined) return;

  const token = ++historyMessageToken;
  historyMessageLoading = hash;
  try {
    const message = await project.gitShowMessage(hash);
    if (token === historyMessageToken && expandedHistoryHash === hash) {
      historyMessages = { ...historyMessages, [hash]: message };
    }
  } catch (cause) {
    if (token === historyMessageToken) onError(friendly(cause));
  } finally {
    if (token === historyMessageToken) historyMessageLoading = null;
  }
}

function askConfirmation(
  title: string,
  message: string,
  confirmLabel: string,
  run: () => Promise<boolean>,
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
    if (completed) confirmation = null;
  } finally {
    confirmationBusy = false;
  }
}

function syncSelectedGroups(groups: ChangeGroup[], previousSelected: string[]) {
  const allowed = new Set(groups.map((group) => group.id));
  return previousSelected.filter((id) => allowed.has(id));
}

function resetProjectState() {
  snapshotLoadToken += 1;
  diffRequestToken += 1;
  historyMessageToken += 1;
  if (aiMessageRequestId) void project.aiCancelText(aiMessageRequestId).catch(() => undefined);
  clearAiMessageListener();
  aiMessageBusy = false;
  aiMessageRequestId = null;
  notifyStatus(null);
  preflight = null;
  remotes = [];
  entities = [];
  log = [];
  selectedGroupIds = [];
  selectionProjectId = null;
  commitMessage = "";
  messageExpanded = true;
  changesExpanded = true;
  repositoryExpanded = false;
  expandedHistoryHash = null;
  historyMessages = {};
  historyMessageLoading = null;
  recoveryUpstream = null;
  selectedCommit = null;
  selectedCommitMessage = "";
  selectedChangePath = null;
  changeDiff = "";
  snapshotChanges = [];
  expandedSnapshotGroups = [];
  remoteModalOpen = false;
  confirmation = null;
}

async function refresh(resetSelection = false, expectedProjectId = projectId, expectedOpen = projectOpen) {
  const token = ++refreshToken;
  loading = true;
  loadError = "";
  try {
    const nextTool = await project.gitToolInfo();
    if (token !== refreshToken || expectedProjectId !== projectId) return;
    tool = nextTool;
    if (!expectedOpen || !nextTool.available) return;

    const nextStatus = await project.gitStatus(true);
    if (token !== refreshToken || expectedProjectId !== projectId) return;
    notifyStatus(nextStatus);
    if (nextStatus.repository) {
      const [nextPreflight, nextRemotes, nextLog, nextEntities] = await Promise.all([
        project.gitStagingPreview(),
        project.gitRemoteList(),
        project.gitLog(),
        project.listEntities(),
      ]);
      if (token !== refreshToken || expectedProjectId !== projectId) return;
      preflight = nextPreflight;
      remotes = nextRemotes;
      log = nextLog;
      entities = nextEntities;
      const groups = buildChangeGroups(nextPreflight.staging_paths, nextEntities);
      if (resetSelection || selectionProjectId !== expectedProjectId) {
        selectedGroupIds = groups.map((group) => group.id);
        selectionProjectId = expectedProjectId;
      } else {
        selectedGroupIds = syncSelectedGroups(groups, selectedGroupIds);
      }
    } else {
      preflight = null;
      remotes = [];
      entities = [];
      log = [];
      selectedGroupIds = [];
      selectionProjectId = expectedProjectId;
    }
  } catch (cause) {
    if (token === refreshToken && expectedProjectId === projectId) loadError = friendly(cause);
  } finally {
    if (token === refreshToken) loading = false;
  }
}

$effect(() => {
  const expectedOpen = projectOpen;
  const expectedProjectId = projectId;
  refreshToken += 1;
  loadError = "";
  untrack(resetProjectState);
  void refresh(true, expectedProjectId, expectedOpen);
});

function generateMessage() {
  const selected = changeGroups.filter((group) => selectedGroupIds.includes(group.id));
  if (selected.length === 0) {
    commitMessage = "";
    messageExpanded = true;
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
  messageExpanded = false;
}

async function initializeGit() {
  await withBusy("Enabling snapshots…", async () => {
    notifyStatus(await project.gitInit());
    await refresh();
  });
}

async function commitSelected() {
  if (!commitMessage.trim() || selectedPaths.length === 0) return;
  if (!(await prepareWrite())) return;
  await withBusy("Creating snapshot…", async () => {
    notifyStatus(await project.gitCommit(formatSnapshotMessage(commitMessage), selectedPaths));
    commitMessage = "";
    messageExpanded = true;
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
      if (!(await prepareWrite())) return false;
      const completed = await withBusy("Squashing snapshots…", async () => {
        const result = await project.gitSuperSquash(squashMessage.trim() || "Consolidate snapshot history");
        notifyStatus(result.status);
        recoveryUpstream = result.divergedFromUpstream ? result.upstream : null;
        closeSnapshotModal();
        await refresh();
        if (result.divergedFromUpstream) recoveryUpstream = result.upstream;
        return true;
      });
      return completed === true;
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
      const completed = await withBusy("Removing remote…", async () => {
        remotes = await project.gitRemoteRemove(name);
        return true;
      });
      return completed === true;
    },
  );
}

async function pushRemote(remote: GitRemote) {
  const branch = status?.branch?.trim();
  if (!branch) return;
  await withBusy(`Pushing ${branch} to ${remote.name}…`, async () => {
    notifyStatus(await project.gitPush(remote.name, branch, false));
    await refresh();
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
  if (selectedCommit === hash) return;
  const loadToken = ++snapshotLoadToken;
  let firstChangedPath: string | null = null;
  selectedCommit = hash;
  selectedCommitMessage = "";
  selectedChangePath = null;
  changeDiff = "";
  snapshotChanges = [];
  await withBusy("Loading snapshot…", async () => {
    const [message, changes] = await Promise.all([
      historyMessages[hash] !== undefined ? Promise.resolve(historyMessages[hash]) : project.gitShowMessage(hash),
      project.gitShowChanges(hash),
    ]);
    if (loadToken === snapshotLoadToken && selectedCommit === hash) {
      selectedCommitMessage = message;
      historyMessages = { ...historyMessages, [hash]: message };
      snapshotChanges = changes;
      const groups = groupSnapshotChanges(changes, entities);
      expandedSnapshotGroups = groups[0] ? [groups[0].label] : [];
      firstChangedPath = changes[0]?.path ?? null;
    }
  });
  if (firstChangedPath && loadToken === snapshotLoadToken && selectedCommit === hash) {
    await selectSnapshotChange(firstChangedPath);
  }
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
      if (!(await prepareWrite())) return false;
      const completed = await withBusy("Restoring snapshot…", async () => {
        const result = await project.gitResetHard(hash);
        notifyStatus(result.status);
        recoveryUpstream = result.divergedFromUpstream ? result.upstream : null;
        selectedCommit = null;
        await refresh();
        if (result.divergedFromUpstream) recoveryUpstream = result.upstream;
        return true;
      });
      return completed === true;
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
      const completed = await withBusy("Force-pushing with lease…", async () => {
        notifyStatus(await project.gitPush(upstream.remote, upstream.branch, true));
        recoveryUpstream = null;
        await refresh();
        return true;
      });
      return completed === true;
    },
  );
}

function restoreFromRemote() {
  askConfirmation(
    "Restore from the remote?",
    "This discards the local hard-reset state and rebuilds the project index from the upstream remote.",
    "Restore from remote",
    async () => {
      const completed = await withBusy("Restoring from remote…", async () => {
        const result = await project.gitRestoreFromUpstream();
        notifyStatus(result.status);
        recoveryUpstream = null;
        await refresh();
        return true;
      });
      return completed === true;
    },
  );
}

onDestroy(() => {
  refreshToken += 1;
  snapshotLoadToken += 1;
  diffRequestToken += 1;
  clearAiMessageListener();
  if (aiMessageRequestId) void project.aiCancelText(aiMessageRequestId).catch(() => undefined);
});
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

  {#if !projectOpen || tool === null || !tool.available}
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
  {/if}

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
  {:else if loading && !status}
    <div class="empty-inline" role="status">
      <GitBranch size={16} strokeWidth={1.7} aria-hidden="true" />
      <div><strong>Loading snapshots…</strong><span>Reading repository status and canonical changes.</span></div>
    </div>
  {:else if loadError && !status}
    <div class="empty-inline load-error" role="alert">
      <GitBranch size={16} strokeWidth={1.7} aria-hidden="true" />
      <div>
        <strong>Could not load snapshots</strong><span>{loadError}</span>
        <button type="button" class="quiet-button inline-retry" onclick={() => void refresh()}>Try again</button>
      </div>
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
    {#if loadError}
      <div class="inline-warning" role="alert">
        <span>{loadError}</span>
        <button type="button" class="quiet-button" disabled={loading} onclick={() => void refresh()}>Try again</button>
      </div>
    {/if}
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
      <button type="button" class="quiet-button" disabled={busy || loading} onclick={() => void refresh()}
        >{loading ? "Refreshing…" : "Refresh"}</button>
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

    <section class="git-block git-create-block elevated">
      <div class="block-heading">
        <div class="heading-left">
          <span class="heading-icon"><FileText size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <h3>Create snapshot</h3>
        </div>
        <span class="block-hint">Canonical files only</span>
      </div>
      <p class="git-section-copy">Choose what to preserve, add a short message, then create the snapshot.</p>
      {#if preflight && !preflight.ready}
        <div class="git-diagnostics" role="alert">
          <strong>Snapshot blocked</strong>
          <ul>
            {#each preflight.diagnostics as diagnostic}
              <li>{diagnostic}</li>
            {/each}
          </ul>
        </div>
      {/if}
      {#if changeGroups.length === 0}
        <p class="settings-empty">Working tree has no canonical changes to commit.</p>
      {:else}
        <div class="git-disclosure">
          <button
            type="button"
            class="git-disclosure-toggle"
            aria-expanded={changesExpanded}
            onclick={() => (changesExpanded = !changesExpanded)}>
            <span class="git-disclosure-copy">
              <strong>Changes</strong>
              <small>{selectedFileCount} of {totalChangeCount} files selected</small>
            </span>
            <span class="git-disclosure-chevron" aria-hidden="true">
              {#if changesExpanded}<ChevronDown size={16} strokeWidth={1.8} />{:else}<ChevronRight
                  size={16}
                  strokeWidth={1.8} />{/if}
            </span>
          </button>
          {#if changesExpanded}
            <div class="git-disclosure-body">
              <div class="git-change-toolbar">
                <small>Selection is limited to canonical project files.</small>
                <div class="git-actions">
                  <button type="button" class="quiet-button" onclick={selectAllGroups}>Select all</button>
                  <button type="button" class="quiet-button" onclick={clearGroups}>Select none</button>
                </div>
              </div>
              <ul class="git-change-list">
                {#each changeGroups as group}
                  <li class:selected={groupIsSelected(group.id)}>
                    <label>
                      <input
                        type="checkbox"
                        checked={groupIsSelected(group.id)}
                        onchange={() => toggleGroup(group.id)} />
                      <span>
                        <strong>{group.title}</strong>
                        <small>{group.subtitle}</small>
                      </span>
                    </label>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
        <div class="git-commit-card">
          <button
            type="button"
            class="git-message-toggle"
            aria-expanded={messageExpanded}
            onclick={toggleMessageEditor}>
            <span class="git-disclosure-copy">
              <span>Snapshot message</span>
              <strong>{commitMessageTitle || "Add a message"}</strong>
              <small
                >{commitMessageTitle
                  ? commitMessageBody
                    ? "Title and notes"
                    : "Title only"
                  : "A short title is required"}</small>
            </span>
            <span class="git-disclosure-chevron" aria-hidden="true">
              {#if messageExpanded}<ChevronDown size={16} strokeWidth={1.8} />{:else}<ChevronRight
                  size={16}
                  strokeWidth={1.8} />{/if}
            </span>
          </button>
          {#if messageExpanded}
            <label class="create-input-field git-message-field" for="git-commit-message">
              <span>Title on the first line; optional notes below</span>
              <textarea
                id="git-commit-message"
                rows="4"
                bind:value={commitMessage}
                placeholder="Describe this snapshot"
                disabled={busy || aiMessageBusy}></textarea>
            </label>
            <div class="git-message-actions">
              <button type="button" class="quiet-button" disabled={busy || aiMessageBusy} onclick={generateMessage}
                >Suggest message</button>
              {#if aiEnabled && (aiMessageBusy || !commitMessage.trim())}
                <button
                  type="button"
                  class="quiet-button git-ai-action"
                  disabled={busy || (!aiMessageBusy && (!preflight?.ready || selectedPaths.length === 0))}
                  onclick={() => void (aiMessageBusy ? cancelAiMessage() : generateAiMessage())}>
                  {#if aiMessageBusy}<X size={14} strokeWidth={1.8} aria-hidden="true" /> Cancel AI{:else}<Sparkles
                      size={14}
                      strokeWidth={1.8}
                      aria-hidden="true" /> Write with AI{/if}
                </button>
              {/if}
            </div>
          {/if}
          <div class="git-commit-footer">
            <span class:ready={!snapshotBlockReason} role="status">
              {snapshotBlockReason || `${selectedFileCount} ${selectedFileCount === 1 ? "file" : "files"} ready`}
            </span>
            <button
              type="button"
              class="primary-button"
              disabled={busy || Boolean(snapshotBlockReason)}
              onclick={() => void commitSelected()}>Create snapshot</button>
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
        <span class="block-hint">Newest first</span>
      </div>
      {#if log.length === 0}
        <p class="settings-empty">No snapshots yet.</p>
      {:else}
        <ul class="git-log-list">
          {#each log as entry}
            <li class:expanded={expandedHistoryHash === entry.hash}>
              <button
                type="button"
                class="git-history-toggle"
                aria-expanded={expandedHistoryHash === entry.hash}
                onclick={() => void toggleHistoryMessage(entry.hash)}>
                <span class="git-disclosure-chevron" aria-hidden="true">
                  {#if expandedHistoryHash === entry.hash}<ChevronDown
                      size={16}
                      strokeWidth={1.8} />{:else}<ChevronRight size={16} strokeWidth={1.8} />{/if}
                </span>
                <span class="git-history-copy">
                  <strong>{entry.subject || "Untitled snapshot"}</strong>
                  <small>{snapshotDateLabel(entry.date)}</small>
                </span>
              </button>
              {#if expandedHistoryHash === entry.hash}
                <div class="git-history-details">
                  {#if historyMessageLoading === entry.hash}
                    <p>Loading message…</p>
                  {:else}
                    {@const historyBody = snapshotMessageBody(historyMessages[entry.hash] ?? "")}
                    <p class:empty={!historyBody}>{historyBody || "No additional notes for this snapshot."}</p>
                  {/if}
                  <details class="git-technical-details">
                    <summary>Technical details</summary>
                    <code>{entry.hash}</code>
                  </details>
                  <div class="git-history-actions">
                    <button
                      type="button"
                      class="quiet-button"
                      disabled={busy}
                      onclick={() => void selectCommit(entry.hash)}>Review changes</button>
                    <button type="button" class="danger-button" disabled={busy} onclick={() => askReset(entry.hash)}
                      >Restore this snapshot…</button>
                  </div>
                </div>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      {#if selectedCommit}
        {@const snapshotEntry = log.find((entry) => entry.hash === selectedCommit)}
        <div class="modal-backdrop">
          <div
            bind:this={snapshotDialog}
            class="dialog git-snapshot-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="snapshot-title"
            tabindex="-1"
            onkeydown={(event) => trapModalFocus(event, snapshotDialog)}>
            <div class="new-form-heading">
              <div>
                <span class="panel-kicker">SNAPSHOT DETAILS</span>
                <strong id="snapshot-title">{snapshotEntry?.subject ?? "Snapshot details"}</strong>
                <small class="git-snapshot-meta"
                  >{snapshotEntry ? snapshotDateLabel(snapshotEntry.date) : "Snapshot"}</small>
                {#if selectedCommitMessage}
                  {@const messageBody = snapshotMessageBody(selectedCommitMessage)}
                  {#if messageBody}
                    <p class="git-snapshot-comment">{messageBody}</p>
                  {/if}
                {/if}
                <details class="git-technical-details snapshot-technical-details">
                  <summary>Technical details</summary>
                  <code>{selectedCommit}</code>
                </details>
              </div>
              <button
                type="button"
                id="git-snapshot-close"
                class="new-form-close"
                aria-label="Close snapshot details"
                onclick={closeSnapshotModal}><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
            </div>
            <div class="git-snapshot-summary">
              <div>
                <strong>{snapshotChanges.length} changed {snapshotChanges.length === 1 ? "file" : "files"}</strong>
                <span>The first change is selected automatically.</span>
              </div>
              <button type="button" class="danger-button" disabled={busy} onclick={() => askReset(selectedCommit!)}
                >Restore this snapshot…</button>
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
                      <div class="git-diff-file-copy">
                        <strong>{snapshotChangeLabel(selectedChangePath)}</strong>
                        <details class="git-technical-details">
                          <summary>Show stored path</summary>
                          <code>{selectedChangePath}</code>
                        </details>
                      </div>
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

    <section class="git-block git-repository-block elevated">
      <button
        type="button"
        class="git-section-toggle"
        aria-expanded={repositoryExpanded}
        onclick={() => (repositoryExpanded = !repositoryExpanded)}>
        <span class="heading-left">
          <span class="heading-icon"><Cable size={14} strokeWidth={1.8} aria-hidden="true" /></span>
          <span>
            <strong>Sync & repository</strong>
            <small>{remotes.length} {remotes.length === 1 ? "remote" : "remotes"} · {tool?.version}</small>
          </span>
        </span>
        <span class="git-disclosure-chevron" aria-hidden="true">
          {#if repositoryExpanded}<ChevronDown size={16} strokeWidth={1.8} />{:else}<ChevronRight
              size={16}
              strokeWidth={1.8} />{/if}
        </span>
      </button>
      {#if repositoryExpanded}
        <div class="git-repository-content">
          <div class="git-repository-group">
            <div class="git-subsection-heading">
              <div>
                <strong>Remotes</strong>
                <small>Push snapshots or connect another repository.</small>
              </div>
              <button type="button" class="primary-button" disabled={busy} onclick={openAddRemoteModal}
                >Add remote</button>
            </div>
            {#if remotes.length === 0}
              <p class="settings-empty">No remotes configured. Local snapshots still work without one.</p>
            {:else}
              <ul class="git-remote-list">
                {#each remotes as remote}
                  <li>
                    <div>
                      <strong>{remote.name}</strong>
                      <small>{remote.fetchUrl}</small>
                      {#if remote.pushUrl !== remote.fetchUrl}<small>Push: {remote.pushUrl}</small>{/if}
                    </div>
                    <div class="git-remote-actions">
                      <button
                        type="button"
                        class="primary-button compact-button"
                        disabled={busy || loading || !status.branch || log.length === 0}
                        title={!status.branch
                          ? "Switch to a branch before pushing"
                          : log.length === 0
                            ? "Create a snapshot before pushing"
                            : `Push ${status.branch} to ${remote.name}`}
                        onclick={() => void pushRemote(remote)}>Push</button>
                      <button
                        type="button"
                        class="quiet-button"
                        disabled={busy}
                        onclick={() => openEditRemoteModal(remote)}>Edit URL</button>
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
          </div>
          <div class="git-repository-group">
            <div class="git-subsection-heading">
              <div>
                <strong>History maintenance</strong>
                <small>Git: {tool?.version}. Condensing permanently removes earlier local snapshots.</small>
              </div>
              {#if log.length > 1}
                <button
                  type="button"
                  class="quiet-button"
                  disabled={busy || !preflight?.ready || preflight.staging_paths.length > 0}
                  title={preflight?.staging_paths.length
                    ? "Create a snapshot for pending changes first"
                    : "Replace history with one snapshot of the latest committed state"}
                  onclick={askSuperSquash}>Condense history…</button>
              {/if}
            </div>
          </div>
        </div>
      {/if}
    </section>
  {/if}
</div>

{#if remoteModalOpen}
  <div class="modal-backdrop">
    <div
      bind:this={remoteDialog}
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="git-remote-title"
      tabindex="-1"
      onkeydown={(event) => trapModalFocus(event, remoteDialog)}>
      <div class="new-form-heading">
        <div>
          <span class="panel-kicker">SNAPSHOT REMOTE</span>
          <strong id="git-remote-title"
            >{remoteModalMode === "add" ? "Add remote" : `Edit ${editingRemoteName}`}</strong>
        </div>
        <button type="button" class="new-form-close" aria-label="Close remote dialog" onclick={closeRemoteModal}
          ><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </div>
      <form onsubmit={(event) => (event.preventDefault(), void submitRemoteModal())}>
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
            <input
              id="git-remote-url"
              type="text"
              inputmode="url"
              autocomplete="url"
              bind:value={remoteUrl}
              placeholder="https://…" />
          </label>
        </div>
        <div class="new-form-actions">
          <button type="button" class="quiet-button" onclick={closeRemoteModal}>Cancel</button>
          <button
            type="submit"
            class="primary-button"
            disabled={busy || !remoteUrl.trim() || (remoteModalMode === "add" && !remoteName.trim())}
            >{remoteModalMode === "add" ? "Add remote" : "Save URL"}</button>
        </div>
      </form>
    </div>
  </div>
{/if}

{#if confirmation}
  <div class="modal-backdrop">
    <div
      bind:this={confirmationDialog}
      class="dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="git-confirm-title"
      aria-describedby="git-confirm-description"
      tabindex="-1"
      onkeydown={(event) => trapModalFocus(event, confirmationDialog)}>
      <div class="new-form-heading">
        <div>
          <span class="panel-kicker">CONFIRM SNAPSHOT ACTION</span>
          <strong id="git-confirm-title">{confirmation.title}</strong>
        </div>
        <button
          type="button"
          class="new-form-close"
          aria-label="Close confirmation"
          disabled={confirmationBusy}
          onclick={closeConfirmation}><X size={16} strokeWidth={1.8} aria-hidden="true" /></button>
      </div>
      <p id="git-confirm-description" class="dialog-body-copy">{confirmation.message}</p>
      {#if confirmation.squash}
        <label class="create-input-field squash-message-field" for="squash-message">
          <span>Snapshot message <small>(optional)</small></span>
          <input id="squash-message" bind:value={squashMessage} placeholder="Consolidate snapshot history" />
        </label>
      {/if}
      <div class="new-form-actions">
        <button
          id="git-confirm-cancel"
          type="button"
          class="quiet-button"
          disabled={confirmationBusy}
          onclick={closeConfirmation}>Cancel</button>
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
.git-change-toolbar small {
  display: block;
}
.git-change-toolbar small {
  margin-top: 3px;
  color: var(--ink-soft);
  font-size: 11px;
}
.git-change-toolbar .git-actions {
  margin: 0;
}
.git-commit-card {
  display: grid;
  gap: 12px;
  padding: 12px;
  border: 1px solid var(--theme-warning-border, #e5d8c6);
  border-radius: 10px;
  background: var(--theme-warning-bg, #fcf8f1);
}
.git-disclosure {
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: var(--canvas);
}
.git-disclosure-toggle,
.git-message-toggle,
.git-section-toggle,
.git-history-toggle {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 11px 12px;
  border: 0;
  background: transparent;
  color: var(--ink);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.git-disclosure-toggle:hover,
.git-message-toggle:hover,
.git-section-toggle:hover,
.git-history-toggle:hover {
  background: color-mix(in srgb, var(--surface-muted) 70%, transparent);
}
.git-disclosure-copy,
.git-history-copy {
  min-width: 0;
  display: grid;
  gap: 3px;
}
.git-disclosure-copy > span {
  color: var(--ink-faint);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.git-disclosure-copy strong,
.git-history-copy strong {
  min-width: 0;
  overflow: hidden;
  color: var(--ink);
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.git-disclosure-copy small,
.git-history-copy small {
  color: var(--ink-soft);
  font-size: 11px;
}
.git-disclosure-chevron {
  flex: 0 0 auto;
  display: grid;
  place-items: center;
  color: var(--ink-faint);
}
.git-disclosure-body {
  padding: 12px 12px 0;
  border-top: 1px solid var(--line);
}
.git-message-toggle {
  padding: 2px 2px 10px;
  border-bottom: 1px solid var(--theme-warning-border, #e5d8c6);
}
.git-message-toggle[aria-expanded="false"] {
  padding-bottom: 2px;
  border-bottom: 0;
}
.git-message-actions,
.git-commit-footer,
.git-history-actions,
.git-subsection-heading {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
}
.git-message-actions {
  justify-content: flex-start;
}
.git-ai-action {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.git-commit-footer {
  padding-top: 10px;
  border-top: 1px solid var(--theme-warning-border, #e5d8c6);
}
.git-commit-footer > span {
  color: var(--theme-danger-text, #9b4438);
  font-size: 11px;
}
.git-commit-footer > span.ready {
  color: var(--theme-success-text, #3f7449);
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
.git-remote-list li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.git-remote-list strong,
.git-remote-list small,
.git-change-list strong,
.git-change-list small {
  display: block;
}
.git-remote-list small,
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
.compact-button {
  padding: 7px 10px;
}
.inline-warning {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 11px 13px;
  border: 1px solid var(--theme-danger-border, #e5c4b4);
  border-radius: 9px;
  background: var(--theme-danger-bg, #fff4ee);
  color: var(--theme-danger-text, #7a4a36);
  font-size: 12px;
}
.inline-retry {
  display: block;
  margin-top: 10px;
}
.load-error {
  border-color: var(--theme-danger-border, #e5c4b4);
  background: var(--theme-danger-bg, #fff4ee);
}
.git-diagnostics {
  padding: 11px 13px;
  border: 1px solid var(--theme-danger-border, #e5c4b4);
  border-radius: 9px;
  background: var(--theme-danger-bg, #fff4ee);
  color: var(--theme-danger-text, #7a4a36);
  font-size: 12px;
}
.git-diagnostics strong {
  display: block;
  margin-bottom: 5px;
}
.git-diagnostics ul {
  margin: 0;
  padding-left: 18px;
}
.git-diagnostics li + li {
  margin-top: 3px;
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
  background: var(--canvas);
}
.git-change-list li.selected {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--theme-warning-bg, #fffaf2);
}
.git-change-list strong {
  font-size: 13px;
}
.git-path-list.compact {
  max-height: 220px;
  overflow: auto;
}
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
  color: var(--theme-success-text, #3f8b4d);
}
.git-change-status.change-deleted {
  color: var(--theme-danger-text, #a44d42);
}
.git-file-button.active {
  color: var(--accent-dark);
}
.git-log-list {
  margin-bottom: 0;
}
.git-log-list li {
  display: grid;
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--canvas);
}
.git-log-list li.expanded {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--surface-muted);
}
.git-history-toggle {
  justify-content: flex-start;
}
.git-history-copy {
  flex: 1;
}
.git-history-details {
  display: grid;
  gap: 10px;
  padding: 0 12px 12px 40px;
  border-top: 1px solid var(--line);
}
.git-history-details > p {
  max-width: 78ch;
  margin: 10px 0 0;
  color: var(--ink-soft);
  font-size: 12px;
  line-height: 1.55;
  white-space: pre-wrap;
}
.git-history-details > p.empty {
  color: var(--ink-faint);
  font-style: italic;
}
.git-history-actions {
  justify-content: flex-start;
}
.git-technical-details {
  min-width: 0;
  color: var(--ink-faint);
  font-size: 10px;
}
.git-technical-details summary {
  width: fit-content;
  cursor: pointer;
  font-weight: 700;
}
.git-technical-details code {
  display: block;
  margin-top: 6px;
  color: var(--ink-soft);
  font-size: 10px;
  overflow-wrap: anywhere;
  white-space: normal;
}
.snapshot-technical-details {
  margin-top: 8px;
}
.git-repository-block {
  gap: 0;
  overflow: hidden;
  padding: 0;
}
.git-section-toggle {
  padding: 15px 17px;
}
.git-section-toggle .heading-left {
  min-width: 0;
}
.git-section-toggle .heading-left > span:last-child {
  min-width: 0;
  display: grid;
  gap: 3px;
}
.git-section-toggle strong {
  font-size: 14px;
}
.git-section-toggle small,
.git-subsection-heading small {
  color: var(--ink-soft);
  font-size: 11px;
}
.git-repository-content {
  display: grid;
  border-top: 1px solid var(--line);
}
.git-repository-group {
  padding: 15px 17px;
}
.git-repository-group + .git-repository-group {
  border-top: 1px solid var(--line);
}
.git-subsection-heading {
  margin-bottom: 12px;
}
.git-subsection-heading strong,
.git-subsection-heading small {
  display: block;
}
.git-subsection-heading small {
  margin-top: 3px;
}
.git-repository-group .git-remote-list {
  margin-bottom: 0;
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
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 14px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--line);
}
.git-snapshot-summary > div {
  display: grid;
  gap: 3px;
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
  border-left-color: var(--theme-success-border, #5b9b68);
  background: var(--theme-success-bg, #edf7ec);
  color: var(--theme-success-text, #3f7449);
}
.git-change-category.change-modified .git-change-category-toggle {
  border-left-color: var(--theme-warning-border, #c9973e);
  background: var(--theme-warning-bg, #fff7e5);
  color: var(--theme-warning-text, #946c24);
}
.git-change-category.change-deleted .git-change-category-toggle {
  border-left-color: var(--theme-danger-border, #b85b4e);
  background: var(--theme-danger-bg, #fbecea);
  color: var(--theme-danger-text, #98443a);
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
.git-diff-file-copy {
  min-width: 0;
  display: grid;
  gap: 4px;
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
  background: var(--theme-warning-bg, #f7f4ee);
  font-size: 11px;
  white-space: pre-wrap;
}
.git-diff-view {
  max-height: 420px;
  margin: 0;
  overflow: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--canvas);
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
  width: max-content;
  min-width: 100%;
  min-height: 1.55em;
  padding: 0 10px;
}
.git-diff-view.diff-wrap span {
  width: auto;
}
.git-diff-view .diff-added {
  background: var(--theme-success-bg, #e7f3e5);
  color: var(--theme-success-text, #2f6b3b);
}
.git-diff-view .diff-removed {
  background: var(--theme-danger-bg, #f7e5e1);
  color: var(--theme-danger-text, #9b4438);
}
.git-diff-view .diff-hunk {
  background: var(--theme-info-bg, #e8edf6);
  color: var(--theme-info-text, #4a638c);
}
.git-diff-view .diff-file-header {
  color: var(--ink-soft);
  font-weight: 700;
}
@media (max-width: 700px) {
  .git-overview {
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .git-overview-stat {
    display: none;
  }
  .git-change-toolbar,
  .git-commit-footer,
  .git-subsection-heading,
  .git-remote-list li,
  .git-snapshot-summary,
  .git-diff-heading {
    align-items: stretch;
    flex-direction: column;
  }
  .git-change-toolbar .git-actions,
  .git-remote-actions {
    width: 100%;
  }
  .git-commit-footer .primary-button,
  .git-snapshot-summary .danger-button {
    width: 100%;
  }
  .git-history-details {
    padding-left: 12px;
  }
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
  border: 1px solid var(--theme-danger-border, #e5c4b4);
  border-radius: 10px;
  background: var(--theme-danger-bg, #fff4ee);
}
.git-recovery p {
  margin: 8px 0 12px;
  color: var(--theme-danger-text, #7a4a36);
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
  background: var(--accent-dark);
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
  border: 1px solid var(--theme-warning-border, #ded8cd);
  background: var(--surface);
  color: var(--ink-soft);
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  transition:
    background 0.16s ease,
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    color 0.16s ease,
    transform 0.16s ease;
}
.quiet-button:hover {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--theme-warning-bg, #f7f3eb);
  color: var(--ink);
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
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 700;
}
.create-input-field > input,
.create-input-field > textarea {
  width: 100%;
  box-sizing: border-box;
  padding: 10px 11px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  outline: 0;
  background: var(--canvas);
  color: var(--ink);
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
  border-color: var(--accent-soft);
  box-shadow: 0 0 0 3px rgba(180, 119, 63, 0.1);
}
.git-message-field {
  position: relative;
}
.git-message-field > textarea {
  min-height: 104px;
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
  background: var(--surface);
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
  color: var(--theme-danger-text, #a1482f);
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
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.hero-icon {
  width: 40px;
  height: 40px;
  display: grid;
  place-items: center;
  border-radius: 11px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.hero-copy .kicker {
  color: var(--accent);
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
  color: var(--ink-soft);
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
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
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
  border-bottom: 1px solid var(--theme-warning-border, #f0e8d9);
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
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
}
.count-badge {
  display: inline-grid;
  place-items: center;
  min-width: 22px;
  height: 20px;
  padding: 0 7px;
  border-radius: 999px;
  background: var(--surface-warm);
  border: 1px solid var(--line-soft);
  color: var(--ink-muted);
  font:
    700 11px Inter,
    sans-serif;
}
.block-hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--ink-faint);
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
  border: 1px dashed var(--line-strong);
  border-radius: 11px;
  background: var(--surface-quiet);
  color: var(--ink-muted);
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
