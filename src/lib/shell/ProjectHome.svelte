<script lang="ts">
import type { Entity, EntityTypeColor, IconRef } from "$lib/project/client";
import type { WorkspaceSection } from "$lib/modules/workspace";
import EntityGlyph from "$lib/entity-colors/EntityGlyph.svelte";
import {
  CalendarRange,
  Boxes,
  ChevronRight,
  Clock3,
  GitBranch,
  Languages,
  Library,
  Map as MapIcon,
  Pencil,
  Plus,
} from "@lucide/svelte";

export interface ProjectHomeWorkspace {
  section: WorkspaceSection;
  title: string;
  description: string;
  count: number;
}

export interface ProjectHomeRecent {
  entity: Entity;
  icon: IconRef;
  iconColor: EntityTypeColor;
  pluginId: string | null;
  typeLabel: string;
  updatedLabel: string;
}

let {
  projectName,
  activeEntityCount,
  snapshotChangeCount,
  workspaces,
  recents,
  onNewEntry,
  onSnapshots,
  onProjectCenter,
  onExtensions,
  onOpenWorkspace,
  onOpenEntity,
}: {
  projectName: string;
  activeEntityCount: number;
  snapshotChangeCount: number;
  workspaces: ProjectHomeWorkspace[];
  recents: ProjectHomeRecent[];
  onNewEntry: () => void;
  onSnapshots: () => void;
  onProjectCenter: () => void;
  onExtensions: () => void;
  onOpenWorkspace: (section: WorkspaceSection) => void;
  onOpenEntity: (entity: Entity) => void;
} = $props();

function sectionIcon(section: WorkspaceSection) {
  if (section === "lore") return Library;
  if (section === "timeline") return CalendarRange;
  if (section === "writing") return Pencil;
  if (section === "language") return Languages;
  return MapIcon;
}
</script>

<section class="project-home" aria-labelledby="project-home-title">
  <div class="project-home-hero">
    <div class="project-home-heading">
      <span class="overline">PROJECT HOME</span>
      <h1 id="project-home-title">{projectName}</h1>
      <p>Continue recent work, enter a workspace, or preserve the project with a snapshot.</p>
    </div>
    <div class="project-home-actions">
      {#if workspaces.length > 0}
        <button type="button" class="primary-button" onclick={onNewEntry}
          ><Plus size={14} strokeWidth={1.8} aria-hidden="true" /> New entry</button>
      {/if}
      <button type="button" class="quiet-button" onclick={onSnapshots}
        ><GitBranch size={14} strokeWidth={1.8} aria-hidden="true" /> Snapshots</button>
      <button type="button" class="quiet-button" onclick={onProjectCenter}
        ><Boxes size={14} strokeWidth={1.8} aria-hidden="true" /> Project</button>
    </div>
  </div>

  <div class="project-home-stats" aria-label="Project summary">
    <div>
      <strong>{activeEntityCount}</strong>
      <span>active entries</span>
    </div>
    <div>
      <strong>{workspaces.length}</strong>
      <span>enabled workspaces</span>
    </div>
    <div class:attention={snapshotChangeCount > 0}>
      <strong>{snapshotChangeCount}</strong>
      <span>snapshot-ready changes</span>
    </div>
  </div>

  <section class="project-home-section" aria-labelledby="home-workspaces-title">
    <div class="project-home-section-heading">
      <div>
        <span class="panel-kicker">WORKSPACES</span>
        <h2 id="home-workspaces-title">Choose where to work</h2>
      </div>
      <small>Only enabled workspaces appear here.</small>
    </div>
    {#if workspaces.length === 0}
      <div class="project-home-empty">
        <strong>No workspaces enabled</strong>
        <p>Enable a workspace extension to begin creating project content.</p>
        <button type="button" class="primary-button" onclick={onExtensions}>Open Extensions</button>
      </div>
    {:else}
      <div class="project-home-workspaces">
        {#each workspaces as workspace}
          {@const Icon = sectionIcon(workspace.section)}
          <button type="button" class="project-home-workspace" onclick={() => onOpenWorkspace(workspace.section)}>
            <span class="project-home-workspace-icon"><Icon size={19} strokeWidth={1.8} aria-hidden="true" /></span>
            <span class="project-home-workspace-copy">
              <strong>{workspace.title}</strong>
              <small>{workspace.description}</small>
            </span>
            <span class="project-home-workspace-count">{workspace.count}</span>
            <ChevronRight class="project-home-card-icon" size={15} strokeWidth={1.8} aria-hidden="true" />
          </button>
        {/each}
      </div>
    {/if}
  </section>

  <section class="project-home-section" aria-labelledby="recent-work-title">
    <div class="project-home-section-heading">
      <div>
        <span class="panel-kicker">CONTINUE</span>
        <h2 id="recent-work-title">Recently updated</h2>
      </div>
      <small>Across every enabled workspace.</small>
    </div>
    {#if recents.length === 0}
      <div class="project-home-empty">
        <strong>This project is ready for its first entry</strong>
        <p>Create something new or choose a workspace to see its empty state and guidance.</p>
      </div>
    {:else}
      <div class="project-home-recents">
        {#each recents as recent}
          <button type="button" class="project-home-recent" onclick={() => onOpenEntity(recent.entity)}>
            <EntityGlyph icon={recent.icon} iconColor={recent.iconColor} pluginId={recent.pluginId} size={14} box={30} />
            <span>
              <strong>{recent.entity.name}</strong>
              <small>{recent.typeLabel} · {recent.updatedLabel}</small>
            </span>
            <Clock3 class="project-home-card-icon" size={14} strokeWidth={1.8} aria-hidden="true" />
          </button>
        {/each}
      </div>
    {/if}
  </section>
</section>

<style>
.overline,
.panel-kicker {
  display: block;
  color: var(--accent);
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0.18em;
}
.primary-button,
.quiet-button {
  display: inline-flex;
  align-items: center;
  border-radius: 8px;
  font-size: 12px;
  cursor: pointer;
}
.primary-button {
  gap: 7px;
  padding: 10px 15px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: var(--accent-dark);
  box-shadow:
    0 2px 0 #263d30,
    0 7px 16px rgba(42, 68, 51, 0.16);
  color: #fff;
  font-weight: 700;
}
.primary-button:hover {
  background: #2b4535;
  transform: translateY(-1px);
}
.quiet-button {
  gap: 7px;
  padding: 10px 12px;
  border: 1px solid var(--theme-warning-border, #ded8cd);
  background: var(--surface);
  box-shadow: 0 1px 2px rgba(48, 45, 38, 0.05);
  color: var(--ink-soft);
}
.quiet-button:hover {
  border-color: var(--theme-warning-border, #cbbda9);
  background: var(--surface-muted);
  color: var(--ink);
  transform: translateY(-1px);
}
.primary-button:focus-visible,
.quiet-button:focus-visible {
  outline: 3px solid rgba(180, 119, 63, 0.28);
  outline-offset: 2px;
}
.project-home {
  width: min(1180px, 100%);
  min-height: calc(100vh - 58px);
  margin: 0 auto;
  padding: clamp(30px, 5vw, 58px) 40px 64px;
  display: grid;
  align-content: start;
  gap: 26px;
}
.project-home-hero {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  padding-bottom: 24px;
  border-bottom: 1px solid var(--line);
}
.project-home-heading h1 {
  margin: 9px 0 8px;
  color: var(--ink);
  font: 500 clamp(34px, 5vw, 52px) / 1 var(--font-display);
  letter-spacing: -0.025em;
}
.project-home-heading p {
  max-width: 620px;
  margin: 0;
  color: var(--ink-soft);
  font-size: 14px;
  line-height: 1.55;
}
.project-home-actions,
.project-home-actions :global(.primary-button),
.project-home-actions :global(.quiet-button) {
  display: flex;
  align-items: center;
  gap: 7px;
}
.project-home-actions {
  flex: 0 0 auto;
}
.project-home-stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  overflow: hidden;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: var(--shadow-sm);
}
.project-home-stats > div {
  min-width: 0;
  display: grid;
  gap: 4px;
  padding: 16px 18px;
}
.project-home-stats > div + div {
  border-left: 1px solid var(--line);
}
.project-home-stats strong {
  color: var(--accent-dark);
  font-size: 22px;
}
.project-home-stats .attention strong {
  color: var(--accent);
}
.project-home-stats span {
  color: var(--ink-soft);
  font-size: 11px;
}
.project-home-section {
  display: grid;
  gap: 13px;
}
.project-home-section-heading {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 18px;
}
.project-home-section-heading h2 {
  margin: 6px 0 0;
  color: var(--ink);
  font: 500 23px/1.15 var(--font-display);
}
.project-home-section-heading small {
  color: var(--ink-faint);
  font-size: 10px;
}
.project-home-workspaces,
.project-home-recents {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}
.project-home-workspace,
.project-home-recent {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--line);
  border-radius: 11px;
  background: var(--surface);
  color: var(--ink);
  text-align: left;
  cursor: pointer;
  box-shadow: var(--shadow-sm);
  transition:
    border-color 0.16s ease,
    background 0.16s ease,
    transform 0.16s ease;
}
.project-home-workspace:hover,
.project-home-recent:hover {
  border-color: var(--theme-warning-border, #d8c3a5);
  background: var(--surface-muted);
  transform: translateY(-1px);
}
.project-home-workspace-icon {
  flex: 0 0 38px;
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: var(--accent-dark);
  color: var(--on-accent);
}
.project-home-workspace-copy,
.project-home-recent > span:nth-child(2) {
  min-width: 0;
  flex: 1;
}
.project-home-workspace strong,
.project-home-workspace small,
.project-home-recent strong,
.project-home-recent small {
  display: block;
}
.project-home-workspace strong,
.project-home-recent strong {
  overflow: hidden;
  font-size: 13px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.project-home-workspace small,
.project-home-recent small {
  margin-top: 4px;
  color: var(--ink-soft);
  font-size: 10px;
  line-height: 1.4;
}
.project-home-workspace-count {
  flex: 0 0 auto;
  min-width: 27px;
  padding: 4px 7px;
  border-radius: 999px;
  background: var(--surface-muted);
  color: var(--ink-soft);
  font-size: 10px;
  font-weight: 800;
  text-align: center;
}
:global(.project-home-card-icon) {
  flex: 0 0 auto;
  color: var(--ink-faint);
}
.project-home-empty {
  padding: 22px;
  border: 1px dashed var(--line-strong);
  border-radius: 11px;
  background: var(--surface-muted);
}
.project-home-empty strong {
  color: var(--ink);
  font-size: 13px;
}
.project-home-empty p {
  margin: 6px 0 13px;
  color: var(--ink-soft);
  font-size: 11px;
  line-height: 1.5;
}

@media (max-width: 760px) {
  .project-home {
    padding: 28px 17px 40px;
  }
  .project-home-hero,
  .project-home-section-heading {
    align-items: flex-start;
    flex-direction: column;
  }
  .project-home-actions {
    flex-wrap: wrap;
  }
  .project-home-stats,
  .project-home-workspaces,
  .project-home-recents {
    grid-template-columns: 1fr;
  }
  .project-home-stats > div + div {
    border-top: 1px solid var(--line);
    border-left: 0;
  }
}
</style>
