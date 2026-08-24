<script lang="ts">
import {
  ChevronDown,
  DatabaseZap,
  Download,
  FlaskConical,
  FolderOpen,
  Import as ImportIcon,
  LogOut,
  X,
} from "@lucide/svelte";

export interface ProjectSwitcherRecent {
  name: string;
  root: string;
}

interface Props {
  ready: boolean;
  collapsed: boolean;
  menuOpen: boolean;
  projectName: string;
  recentProjects: ProjectSwitcherRecent[];
  onOpenProject: () => void;
  onOpenRecent: (root: string) => void;
  onRemoveRecent: (root: string) => void;
  onMenuChange: (open: boolean) => void;
  onExportMarkdown: () => void;
  onImportExternal: () => void;
  onRebuildIndex: () => void;
  onSeedExample: () => void;
  onCloseProject: () => void;
}

let {
  ready,
  collapsed,
  menuOpen,
  projectName,
  recentProjects,
  onOpenProject,
  onOpenRecent,
  onRemoveRecent,
  onMenuChange,
  onExportMarkdown,
  onImportExternal,
  onRebuildIndex,
  onSeedExample,
  onCloseProject,
}: Props = $props();
</script>

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
      aria-expanded={menuOpen}
      aria-haspopup="menu"
      class:active={menuOpen}
      class="project-card"
      onclick={() => onMenuChange(!menuOpen)}>
      <span class:online={ready} class="project-dot"></span>
      <span class="project-copy"><strong>{projectName}</strong></span>
      <span class="project-chevron" aria-hidden="true"
        ><ChevronDown size={14} strokeWidth={1.8} aria-hidden="true" /></span>
    </button>
    {#if menuOpen}
      {#if collapsed}<button class="rail-backdrop" aria-label="Close menu" onclick={() => onMenuChange(false)}></button
        >{/if}
      <div class="project-menu" role="menu">
        <button class="rail-button" role="menuitem" onclick={onOpenProject}
          ><span class="rail-icon"><FolderOpen size={16} strokeWidth={1.8} /></span><span>Open another folder</span
          ></button>
        <button class="rail-button" role="menuitem" onclick={onExportMarkdown}
          ><span class="rail-icon"><Download size={16} strokeWidth={1.8} /></span><span>Export Markdown</span></button>
        <button class="rail-button" role="menuitem" onclick={onImportExternal}
          ><span class="rail-icon"><ImportIcon size={16} strokeWidth={1.8} /></span><span>Import external material</span
          ></button>
        <button class="rail-button" role="menuitem" onclick={onRebuildIndex}
          ><span class="rail-icon"><DatabaseZap size={16} strokeWidth={1.8} /></span><span>Rebuild index</span></button>
        <button class="rail-button" role="menuitem" onclick={onSeedExample}
          ><span class="rail-icon"><FlaskConical size={16} strokeWidth={1.8} /></span><span>Seed example</span></button>
        <button class="rail-button" role="menuitem" onclick={onCloseProject}
          ><span class="rail-icon"><LogOut size={16} strokeWidth={1.8} /></span><span>Close project</span></button>
      </div>
    {/if}
  </div>
{/if}
