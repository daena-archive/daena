import type { SettingsSection, TimelineView, WorkspaceSection, WritingView } from "$lib/modules/workspace";

export type WorkspaceLocationView =
  "library" | "wiki" | "graph" | "timeline" | "events" | "eras" | "calendars" | "manuscripts" | "reference" | "default";

export type ShellLocation =
  | { kind: "home" }
  | { kind: "settings"; section: SettingsSection }
  | {
      kind: "workspace";
      section: WorkspaceSection;
      view: WorkspaceLocationView;
      entityId: string | null;
      writingView: WritingView;
      timelineView: TimelineView;
    }
  | {
      kind: "plugin";
      key: string;
      section: WorkspaceSection;
      entityId: string | null;
    };

export interface ShellNavigationHistory {
  back: ShellLocation[];
  forward: ShellLocation[];
}

export interface ShellNavigationTransition {
  target: ShellLocation;
  history: ShellNavigationHistory;
}

export const SHELL_HISTORY_LIMIT = 40;

export function emptyShellNavigationHistory(): ShellNavigationHistory {
  return { back: [], forward: [] };
}

export function shellLocationKey(location: ShellLocation): string {
  if (location.kind === "home") return "home";
  if (location.kind === "settings") return `settings:${location.section}`;
  if (location.kind === "plugin") return `plugin:${location.key}:${location.section}:${location.entityId ?? ""}`;
  return [
    "workspace",
    location.section,
    location.view,
    location.entityId ?? "",
    location.writingView,
    location.timelineView,
  ].join(":");
}

export function sameShellLocation(left: ShellLocation, right: ShellLocation): boolean {
  return shellLocationKey(left) === shellLocationKey(right);
}

export function recordShellLocation(history: ShellNavigationHistory, current: ShellLocation): ShellNavigationHistory {
  if (history.back.at(-1) && sameShellLocation(history.back.at(-1)!, current)) {
    return history.forward.length === 0 ? history : { ...history, forward: [] };
  }
  return {
    back: [...history.back, current].slice(-SHELL_HISTORY_LIMIT),
    forward: [],
  };
}

export function shellHistoryBack(
  history: ShellNavigationHistory,
  current: ShellLocation,
): ShellNavigationTransition | null {
  const target = history.back.at(-1);
  if (!target) return null;
  return {
    target,
    history: {
      back: history.back.slice(0, -1),
      forward: [current, ...history.forward].slice(0, SHELL_HISTORY_LIMIT),
    },
  };
}

export function shellHistoryForward(
  history: ShellNavigationHistory,
  current: ShellLocation,
): ShellNavigationTransition | null {
  const target = history.forward[0];
  if (!target) return null;
  return {
    target,
    history: {
      back: [...history.back, current].slice(-SHELL_HISTORY_LIMIT),
      forward: history.forward.slice(1),
    },
  };
}
