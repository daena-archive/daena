<script lang="ts">
import type { WorkspaceSection } from "$lib/modules/workspace";
import ProjectSwitcher from "./ProjectSwitcher.svelte";
import {
  CalendarRange,
  Boxes,
  Home,
  Languages,
  Library,
  Map as MapIcon,
  PanelLeftClose,
  PanelLeftOpen,
  Pencil,
  Plus,
  Puzzle,
  Settings as SettingsIcon,
} from "@lucide/svelte";
import "./AppSidebar.css";

export interface SidebarRecentProject {
  name: string;
  root: string;
}

export interface SidebarWorkspaceItem {
  key: string;
  section: WorkspaceSection;
  title: string;
  beta: boolean;
  active: boolean;
}

export interface SidebarToolItem {
  key: string;
  title: string;
  ariaLabel: string;
  active: boolean;
}

let {
  ready,
  collapsed,
  projectMenuOpen,
  projectName,
  recentProjects,
  workspaces,
  tools,
  homeActive,
  createOpen,
  snapshotsTitle,
  snapshotChangeCount,
  projectCenterActive,
  settingsActive,
  version,
  onOpenProject,
  onOpenRecent,
  onRemoveRecent,
  onProjectMenuChange,
  onOpenProjectCenter,
  onCloseProject,
  onOpenHome,
  onCreate,
  onOpenWorkspace,
  onOpenTool,
  onOpenSettings,
  onCollapsedChange,
}: {
  ready: boolean;
  collapsed: boolean;
  projectMenuOpen: boolean;
  projectName: string;
  recentProjects: SidebarRecentProject[];
  workspaces: SidebarWorkspaceItem[];
  tools: SidebarToolItem[];
  homeActive: boolean;
  createOpen: boolean;
  snapshotsTitle: string;
  snapshotChangeCount: number;
  projectCenterActive: boolean;
  settingsActive: boolean;
  version: string;
  onOpenProject: () => void;
  onOpenRecent: (root: string) => void;
  onRemoveRecent: (root: string) => void;
  onProjectMenuChange: (open: boolean) => void;
  onOpenProjectCenter: () => void;
  onCloseProject: () => void;
  onOpenHome: () => void;
  onCreate: () => void;
  onOpenWorkspace: (key: string) => void;
  onOpenTool: (key: string) => void;
  onOpenSettings: () => void;
  onCollapsedChange: (collapsed: boolean) => void;
} = $props();

function workspaceIcon(section: WorkspaceSection) {
  if (section === "lore") return Library;
  if (section === "timeline") return CalendarRange;
  if (section === "writing") return Pencil;
  if (section === "language") return Languages;
  return MapIcon;
}
</script>

<aside
  class:startup-rail={!ready}
  class:rail-collapsed={collapsed && ready}
  class:menu-open={collapsed && ready && projectMenuOpen}
  class="rail">
  <div class="brand">
    {#if collapsed && ready}
      <img class="brand-icon" src="/branding/icon.png" alt="Daena" />
    {:else}
      <img class="brand-logo" src="/branding/logo.png" alt="Daena Archive" />
    {/if}
  </div>

  <ProjectSwitcher
    {ready}
    {collapsed}
    menuOpen={projectMenuOpen}
    {projectName}
    {recentProjects}
    {onOpenProject}
    {onOpenRecent}
    {onRemoveRecent}
    onMenuChange={onProjectMenuChange}
    {onOpenProjectCenter}
    {onCloseProject} />

  {#if ready}
    <button
      type="button"
      aria-current={homeActive ? "page" : undefined}
      class:active={homeActive}
      class="rail-button rail-home-button"
      onclick={onOpenHome}>
      <span class="rail-icon"><Home size={16} strokeWidth={1.8} aria-hidden="true" /></span><span>Home</span>
    </button>

    {#if workspaces.length > 0}
      <button aria-expanded={createOpen} class="rail-create-button" title="New entry" onclick={onCreate}
        ><span class="rail-icon"><Plus size={16} strokeWidth={1.8} /></span><span>New entry</span></button>
      <div class="rail-label">WORKSPACES</div>
      <nav class="workspace-nav" aria-label="Workspace sections">
        {#each workspaces as item (item.key)}
          {@const Icon = workspaceIcon(item.section)}
          <button
            title={item.beta ? `${item.title} · Beta plugin — may be unstable` : item.title}
            aria-current={item.active ? "page" : undefined}
            class:active={item.active}
            class="rail-button"
            onclick={() => onOpenWorkspace(item.key)}>
            <span class="rail-icon"><Icon size={16} strokeWidth={1.8} /></span>
            <span
              >{item.title}{#if item.beta}<em class="workspace-beta">Beta</em>{/if}</span>
          </button>
        {/each}
      </nav>
    {/if}

    {#if tools.length > 0}
      <div class="rail-label plugin-views-label">TOOLS</div>
      <nav class="workspace-nav" aria-label="Plugin views">
        {#each tools as item (item.key)}
          <div class="plugin-nav-row">
            <button
              class:active={item.active}
              class="rail-button"
              title={item.title}
              aria-current={item.active ? "page" : undefined}
              aria-label={item.ariaLabel}
              onclick={() => onOpenTool(item.key)}>
              <span class="rail-icon"><Puzzle size={16} strokeWidth={1.8} /></span>
              <span class="plugin-nav-title">{item.title}</span>
            </button>
          </div>
        {/each}
      </nav>
    {/if}
  {/if}

  <div class="rail-spacer"></div>
  {#if ready}
    <button
      aria-current={projectCenterActive ? "page" : undefined}
      class:active={projectCenterActive}
      class="rail-button muted-button rail-git-button"
      title={snapshotsTitle || "Project"}
      onclick={onOpenProjectCenter}>
      <span class="rail-icon"><Boxes size={16} strokeWidth={1.8} /></span><span>Project</span>
      {#if snapshotChangeCount > 0}<small class="rail-git-count">{snapshotChangeCount}</small>{/if}
    </button>
  {/if}
  <button
    aria-expanded={settingsActive}
    class:active={settingsActive}
    class="rail-button muted-button"
    title="Settings"
    onclick={onOpenSettings}>
    <span class="rail-icon"><SettingsIcon size={16} strokeWidth={1.8} /></span><span>Settings</span>
  </button>
  {#if ready}
    <button
      class="rail-button muted-button rail-collapse-toggle"
      aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
      title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
      onclick={() => onCollapsedChange(!collapsed)}>
      <span class="rail-icon">
        {#if collapsed}<PanelLeftOpen size={16} strokeWidth={1.8} />{:else}<PanelLeftClose
            size={16}
            strokeWidth={1.8} />{/if}
      </span>
      <span>{collapsed ? "Expand" : "Collapse"}</span>
    </button>
  {/if}
  <div class="rail-footer">v{version}</div>
</aside>
