<script lang="ts">
  import type { Snippet } from "svelte";

  type SettingsSection = "general" | "plugins" | "git";
  type RecentProject = { name: string; root: string };

  let {
    section = $bindable("general" as SettingsSection),
    recentProjects,
    projectOpen,
    onRemoveRecent,
    onClose,
    plugins,
    git,
  }: {
    section?: SettingsSection;
    recentProjects: RecentProject[];
    projectOpen: boolean;
    onRemoveRecent: (root: string) => void;
    onClose: () => void;
    plugins: Snippet;
    git: Snippet;
  } = $props();
</script>

<section class="settings-view" aria-label="Settings">
  <header class="settings-header">
    <div>
      <span class="panel-kicker">APPLICATION</span>
      <h1>Settings</h1>
      <p>App preferences and the plugins that power this project.</p>
    </div>
    <button type="button" class="quiet-button" onclick={onClose}>Back</button>
  </header>

  <div class="settings-layout">
    <nav class="settings-nav" aria-label="Settings sections">
      <button
        type="button"
        class:active={section === "general"}
        class="settings-nav-button"
        onclick={() => (section = "general")}
      >General</button>
      <button
        type="button"
        class:active={section === "plugins"}
        class="settings-nav-button"
        onclick={() => (section = "plugins")}
      >Plugins</button>
      <button
        type="button"
        class:active={section === "git"}
        class="settings-nav-button"
        onclick={() => (section = "git")}
      >Git</button>
    </nav>

    <div class="settings-panel">
      {#if section === "general"}
        <div class="settings-section-heading">
          <strong>General</strong>
          <p>Recent projects are stored in your application profile.</p>
        </div>
        {#if recentProjects.length === 0}
          <p class="settings-empty">No recent projects yet. Open a project to begin.</p>
        {:else}
          <ul class="settings-recent-list">
            {#each recentProjects as recent}
              <li>
                <div>
                  <strong>{recent.name}</strong>
                  <small>{recent.root}</small>
                </div>
                <button
                  type="button"
                  class="quiet-button"
                  onclick={() => onRemoveRecent(recent.root)}
                >Remove</button>
              </li>
            {/each}
          </ul>
        {/if}
      {:else if section === "plugins"}
        {#if !projectOpen}
          <div class="settings-section-heading">
            <strong>Plugins</strong>
            <p>Open a project to install, enable, and review plugin capabilities.</p>
          </div>
          <p class="settings-empty">No project is open.</p>
        {:else}
          {@render plugins()}
        {/if}
      {:else}
        {@render git()}
      {/if}
    </div>
  </div>
</section>

<style>
  .settings-view {
    display: flex;
    flex-direction: column;
    min-height: calc(100vh - 58px);
    padding: 28px 32px 40px;
    background: var(--canvas);
  }
  .settings-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 28px;
  }
  .settings-header h1 {
    margin: 6px 0 8px;
    font: 500 34px var(--font-display);
    color: var(--ink);
  }
  .settings-header p {
    margin: 0;
    max-width: 520px;
    color: var(--ink-soft);
    font-size: 13px;
    line-height: 1.55;
  }
  .settings-layout {
    display: grid;
    grid-template-columns: 180px minmax(0, 1fr);
    gap: 22px;
    align-items: start;
  }
  .settings-nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .settings-nav-button {
    width: 100%;
    padding: 10px 12px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--ink-soft);
    text-align: left;
    cursor: pointer;
    font-size: 13px;
  }
  .settings-nav-button.active,
  .settings-nav-button:hover {
    background: #efe6d6;
    color: var(--ink);
  }
  .settings-panel {
    min-width: 0;
    padding: 22px 24px;
    border: 1px solid var(--line);
    border-radius: 14px;
    background: var(--surface);
    box-shadow: var(--shadow-sm, 0 1px 2px rgb(40 40 20 / 4%));
  }
  .settings-section-heading {
    margin-bottom: 18px;
  }
  .settings-section-heading strong {
    display: block;
    font-size: 16px;
  }
  .settings-section-heading p {
    margin: 6px 0 0;
    color: var(--ink-soft);
    font-size: 12px;
    line-height: 1.5;
  }
  .settings-empty {
    margin: 0;
    color: var(--ink-soft);
    font-size: 13px;
  }
  .settings-recent-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .settings-recent-list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 0;
    border-bottom: 1px solid var(--line);
  }
  .settings-recent-list li:last-child {
    border-bottom: 0;
  }
  .settings-recent-list strong,
  .settings-recent-list small {
    display: block;
  }
  .settings-recent-list small {
    margin-top: 3px;
    color: var(--ink-soft);
    font-size: 11px;
  }
  @media (max-width: 760px) {
    .settings-view { padding: 18px 16px 28px; }
    .settings-layout { grid-template-columns: 1fr; }
    .settings-nav { flex-direction: row; flex-wrap: wrap; }
    .settings-nav-button { width: auto; }
  }
</style>
