<script lang="ts">
import type { WorkspaceSection } from "$lib/modules/workspace";
import ProjectSwitcher from "./ProjectSwitcher.svelte";
import {
  CalendarRange,
  Boxes,
  Castle,
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
  projectCenterActive,
  settingsActive,
  version,
  onCreateProject,
  onOpenProject,
  onOpenRecent,
  onRemoveRecent,
  onProjectMenuChange,
  onOpenProjectCenter,
  onOpenSettings,
  onCloseProject,
  onOpenHome,
  onCreate,
  onOpenWorkspace,
  onOpenTool,
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
  projectCenterActive: boolean;
  settingsActive: boolean;
  version: string;
  onCreateProject: () => void;
  onOpenProject: () => void;
  onOpenRecent: (root: string) => void;
  onRemoveRecent: (root: string) => void;
  onProjectMenuChange: (open: boolean) => void;
  onOpenProjectCenter: () => void;
  onOpenSettings: () => void;
  onCloseProject: () => void;
  onOpenHome: () => void;
  onCreate: () => void;
  onOpenWorkspace: (key: string) => void;
  onOpenTool: (key: string) => void;
  onCollapsedChange: (collapsed: boolean) => void;
} = $props();

let tooltip = $state<{ text: string; x: number; y: number } | null>(null);

function showTooltip(event: Event, title: string) {
  if (!collapsed || !ready || !title) return;
  const target = event.currentTarget as HTMLElement | null;
  if (!target) return;
  const rect = target.getBoundingClientRect();
  const x = rect.right + 8;
  let y = rect.top + rect.height / 2;
  if (typeof window !== "undefined") {
    const margin = 12;
    const maxY = window.innerHeight - 40;
    y = Math.max(margin, Math.min(y, maxY));
  }
  tooltip = { text: title, x, y };
}

function hideTooltip() {
  tooltip = null;
}

$effect(() => {
  if (!collapsed || !ready) tooltip = null;
});

$effect(() => {
  if (typeof window === "undefined") return;
  const onWindowChange = () => hideTooltip();
  window.addEventListener("resize", onWindowChange);
  window.addEventListener("scroll", onWindowChange, true);
  return () => {
    window.removeEventListener("resize", onWindowChange);
    window.removeEventListener("scroll", onWindowChange, true);
  };
});

function workspaceIcon(section: WorkspaceSection) {
  if (section === "lore") return Library;
  if (section === "timeline") return CalendarRange;
  if (section === "writing") return Pencil;
  if (section === "language") return Languages;
  if (section === "houses") return Castle;
  return MapIcon;
}
</script>

<aside
  class:startup-rail={!ready}
  class:rail-collapsed={collapsed && ready}
  class:menu-open={collapsed && ready && projectMenuOpen}
  class="rail">
  <div class="rail-scroll" class:menu-open={collapsed && ready && projectMenuOpen} onscroll={hideTooltip}>
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
      {onCreateProject}
      {onOpenProject}
      {onOpenRecent}
      {onRemoveRecent}
      onMenuChange={onProjectMenuChange}
      {onCloseProject} />

    {#if ready}
      <button
        type="button"
        aria-current={homeActive ? "page" : undefined}
        class:active={homeActive}
        class="rail-button rail-home-button"
        title="Home"
        onclick={onOpenHome}
        onmouseenter={(e) => showTooltip(e, "Home")}
        onmouseleave={hideTooltip}
        onfocus={(e) => showTooltip(e, "Home")}
        onblur={hideTooltip}>
        <span class="rail-icon"><Home size={16} strokeWidth={1.8} aria-hidden="true" /></span><span>Home</span>
      </button>

      {#if workspaces.length > 0}
        <button
          aria-expanded={createOpen}
          class="rail-create-button"
          title="New"
          onclick={onCreate}
          onmouseenter={(e) => showTooltip(e, "New")}
          onmouseleave={hideTooltip}
          onfocus={(e) => showTooltip(e, "New")}
          onblur={hideTooltip}>
          <span class="rail-icon"><Plus size={16} strokeWidth={1.8} /></span><span>New</span></button>
        <div class="rail-label">WORKSPACES</div>
        <nav class="workspace-nav" aria-label="Workspace sections">
          {#each workspaces as item (item.key)}
            {@const Icon = workspaceIcon(item.section)}
            {@const wsTitle = item.beta ? `${item.title} · Beta plugin — may be unstable` : item.title}
            <button
              title={wsTitle}
              aria-current={item.active ? "page" : undefined}
              class:active={item.active}
              class="rail-button"
              onclick={() => onOpenWorkspace(item.key)}
              onmouseenter={(e) => showTooltip(e, wsTitle)}
              onmouseleave={hideTooltip}
              onfocus={(e) => showTooltip(e, wsTitle)}
              onblur={hideTooltip}>
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
                onclick={() => onOpenTool(item.key)}
                onmouseenter={(e) => showTooltip(e, item.title)}
                onmouseleave={hideTooltip}
                onfocus={(e) => showTooltip(e, item.title)}
                onblur={hideTooltip}>
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
        class="rail-button muted-button"
        title="Project"
        onclick={onOpenProjectCenter}
        onmouseenter={(e) => showTooltip(e, "Project")}
        onmouseleave={hideTooltip}
        onfocus={(e) => showTooltip(e, "Project")}
        onblur={hideTooltip}>
        <span class="rail-icon"><Boxes size={16} strokeWidth={1.8} /></span><span>Project</span>
      </button>
    {/if}
    <button
      aria-expanded={settingsActive}
      class:active={settingsActive}
      class="rail-button muted-button"
      title="Settings"
      onclick={onOpenSettings}
      onmouseenter={(e) => showTooltip(e, "Settings")}
      onmouseleave={hideTooltip}
      onfocus={(e) => showTooltip(e, "Settings")}
      onblur={hideTooltip}>
      <span class="rail-icon"><SettingsIcon size={16} strokeWidth={1.8} /></span><span>Settings</span>
    </button>
    {#if ready}
      {@const collapseTitle = collapsed ? "Expand sidebar" : "Collapse sidebar"}
      <button
        class="rail-button muted-button rail-collapse-toggle"
        aria-label={collapseTitle}
        title={collapseTitle}
        onclick={() => onCollapsedChange(!collapsed)}
        onmouseenter={(e) => showTooltip(e, collapseTitle)}
        onmouseleave={hideTooltip}
        onfocus={(e) => showTooltip(e, collapseTitle)}
        onblur={hideTooltip}>
        <span class="rail-icon">
          {#if collapsed}<PanelLeftOpen size={16} strokeWidth={1.8} />{:else}<PanelLeftClose
              size={16}
              strokeWidth={1.8} />{/if}
        </span>
        <span>{collapsed ? "Expand" : "Collapse"}</span>
      </button>
    {/if}
    <div class="rail-footer">v{version}</div>
  </div>
  {#if tooltip}
    <div class="rail-tooltip" style="left: {tooltip.x}px; top: {tooltip.y}px;">{tooltip.text}</div>
  {/if}
</aside>
