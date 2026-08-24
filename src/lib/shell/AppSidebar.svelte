<script lang="ts">
import type { WorkspaceSection } from "$lib/modules/workspace";
import {
  CalendarRange,
  ChevronDown,
  DatabaseZap,
  Download,
  FlaskConical,
  FolderOpen,
  GitBranch,
  Home,
  Import as ImportIcon,
  Languages,
  Library,
  LogOut,
  Map as MapIcon,
  PanelLeftClose,
  PanelLeftOpen,
  Pencil,
  Plus,
  Puzzle,
  Settings as SettingsIcon,
  X,
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
  settingsActive,
  version,
  onOpenProject,
  onOpenRecent,
  onRemoveRecent,
  onProjectMenuChange,
  onExportMarkdown,
  onImportExternal,
  onRebuildIndex,
  onSeedExample,
  onCloseProject,
  onOpenHome,
  onCreate,
  onOpenWorkspace,
  onOpenTool,
  onOpenSnapshots,
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
  settingsActive: boolean;
  version: string;
  onOpenProject: () => void;
  onOpenRecent: (root: string) => void;
  onRemoveRecent: (root: string) => void;
  onProjectMenuChange: (open: boolean) => void;
  onExportMarkdown: () => void;
  onImportExternal: () => void;
  onRebuildIndex: () => void;
  onSeedExample: () => void;
  onCloseProject: () => void;
  onOpenHome: () => void;
  onCreate: () => void;
  onOpenWorkspace: (key: string) => void;
  onOpenTool: (key: string) => void;
  onOpenSnapshots: () => void;
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

  {#if !ready}
    <div class="startup-actions">
      <button class="rail-button startup-primary" onclick={onOpenProject}
        ><span class="rail-icon"><FolderOpen size={16} strokeWidth={1.8} /></span><span>Open project folder</span
        ></button>
    </div>
    {#if recentProjects.length > 0}
      <div class="rail-label recent-label">RECENT PROJECTS</div>
      <div class="recent-projects">
        {#each recentProjects as recent (recent.root)}
          <div class="recent-project">
            <button class="recent-project-open" onclick={() => onOpenRecent(recent.root)}>
              <span class="project-dot"></span>
              <span><strong>{recent.name}</strong><small>{recent.root}</small></span>
            </button>
            <button
              class="recent-project-remove"
              aria-label={`Remove ${recent.name} from recent projects`}
              title="Remove from recent projects"
              onclick={() => onRemoveRecent(recent.root)}><X size={12} strokeWidth={1.8} aria-hidden="true" /></button>
          </div>
        {/each}
      </div>
    {/if}
  {:else}
    <div class="project-switcher">
      <button
        type="button"
        aria-expanded={projectMenuOpen}
        aria-haspopup="menu"
        class:active={projectMenuOpen}
        class="project-card"
        onclick={() => onProjectMenuChange(!projectMenuOpen)}>
        <span class:online={ready} class="project-dot"></span>
        <span class="project-copy"><strong>{projectName}</strong></span>
        <span class="project-chevron" aria-hidden="true"
          ><ChevronDown size={14} strokeWidth={1.8} aria-hidden="true" /></span>
      </button>
      {#if projectMenuOpen}
        {#if collapsed}<button class="rail-backdrop" aria-label="Close menu" onclick={() => onProjectMenuChange(false)}
          ></button>
        {/if}
        <div class="project-menu" role="menu">
          <button class="rail-button" role="menuitem" onclick={onOpenProject}
            ><span class="rail-icon"><FolderOpen size={16} strokeWidth={1.8} /></span><span>Open another folder</span
            ></button>
          <button class="rail-button" role="menuitem" onclick={onExportMarkdown}
            ><span class="rail-icon"><Download size={16} strokeWidth={1.8} /></span><span>Export Markdown</span
            ></button>
          <button class="rail-button" role="menuitem" onclick={onImportExternal}
            ><span class="rail-icon"><ImportIcon size={16} strokeWidth={1.8} /></span><span
              >Import external material</span
            ></button>
          <button class="rail-button" role="menuitem" onclick={onRebuildIndex}
            ><span class="rail-icon"><DatabaseZap size={16} strokeWidth={1.8} /></span><span>Rebuild index</span
            ></button>
          <button class="rail-button" role="menuitem" onclick={onSeedExample}
            ><span class="rail-icon"><FlaskConical size={16} strokeWidth={1.8} /></span><span>Seed example</span
            ></button>
          <button class="rail-button" role="menuitem" onclick={onCloseProject}
            ><span class="rail-icon"><LogOut size={16} strokeWidth={1.8} /></span><span>Close project</span></button>
        </div>
      {/if}
    </div>

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
    <button class="rail-button muted-button rail-git-button" title={snapshotsTitle} onclick={onOpenSnapshots}>
      <span class="rail-icon"><GitBranch size={16} strokeWidth={1.8} /></span><span>Snapshots</span>
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
