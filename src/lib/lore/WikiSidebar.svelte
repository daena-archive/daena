<script lang="ts">
import { BookOpen, Clock3, Home, Search } from "@lucide/svelte";

export interface WikiSidebarItem {
  id: string;
  name: string;
  typeLabel: string;
}

export interface WikiSidebarGroup {
  type: string;
  label: string;
  count: number;
  list: WikiSidebarItem[];
}

let {
  query = $bindable(""),
  groups,
  recent,
  currentId,
  searching,
  total,
  offset,
  pageSize,
  hasMore,
  onPrevious,
  onNext,
  onHome,
  onOpen,
}: {
  query?: string;
  groups: WikiSidebarGroup[];
  recent: WikiSidebarItem[];
  currentId: string | null;
  searching: boolean;
  total: number;
  offset: number;
  pageSize: number;
  hasMore: boolean;
  onPrevious: () => void;
  onNext: () => void;
  onHome: () => void;
  onOpen: (id: string) => void;
} = $props();
</script>

<aside class:has-query={query.trim()} class="kb-sidebar" aria-label="Wiki navigation">
  <div class="sidebar-search">
    <Search size={14} strokeWidth={1.8} aria-hidden="true" />
    <input bind:value={query} placeholder="Search the knowledge base" aria-label="Search the knowledge base" />
    {#if searching}<span class="search-dot" aria-label="Searching"></span>{/if}
  </div>

  <nav class="sidebar-scroll">
    <button class:active={currentId === null} class="home-link" type="button" onclick={onHome}>
      <Home size={15} strokeWidth={1.8} /> <span>Wiki home</span>
    </button>

    {#if !query.trim() && recent.length > 0}
      <section class="sidebar-section">
        <h2><Clock3 size={12} strokeWidth={1.8} /> Recently updated</h2>
        <ul>
          {#each recent as item}
            <li>
              <button class:active={item.id === currentId} type="button" onclick={() => onOpen(item.id)}>
                <span>{item.name}</span><small>{item.typeLabel}</small>
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <section class="sidebar-section categories">
      <h2><BookOpen size={12} strokeWidth={1.8} /> {query.trim() ? "Search results" : "Browse by category"}</h2>
      {#if groups.length === 0}
        <p class="sidebar-empty">
          {searching ? "Searching…" : query.trim() ? "No matching pages." : "No wiki pages yet."}
        </p>
      {:else}
        {#each groups as group}
          <details open>
            <summary><span>{group.label}</span><small>{group.count}</small></summary>
            <ul>
              {#each group.list as item}
                <li>
                  <button class:active={item.id === currentId} type="button" onclick={() => onOpen(item.id)}>
                    <span>{item.name}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </details>
        {/each}
      {/if}
    </section>
    {#if total > pageSize}
      <nav class="sidebar-pagination" aria-label="Wiki pages">
        <button type="button" disabled={offset === 0 || searching} onclick={onPrevious}>Previous</button>
        <span>{offset + 1}–{Math.min(offset + pageSize, total)} of {total}</span>
        <button type="button" disabled={!hasMore || searching} onclick={onNext}>Next</button>
      </nav>
    {/if}
  </nav>
</aside>

<style>
.kb-sidebar {
  display: flex;
  min-width: 0;
  min-height: 0;
  flex-direction: column;
  border-right: 1px solid var(--theme-neutral-border, #e1e4de);
  background: var(--theme-success-bg, #f7f8f5);
}
.sidebar-search {
  position: relative;
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 16px 13px 10px;
  padding: 9px 10px;
  border: 1px solid var(--theme-neutral-border, #dce0da);
  border-radius: 9px;
  background: var(--theme-surface-bg, #fff);
  color: var(--theme-neutral-text-muted, #899087);
  box-shadow: 0 1px 2px rgba(34, 40, 34, 0.03);
}
.sidebar-search:focus-within {
  border-color: var(--theme-neutral-border-strong, #9cad9e);
  box-shadow: 0 0 0 3px rgba(74, 103, 78, 0.08);
}
.sidebar-search input {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--theme-neutral-text, #29302a);
  font: 11px var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.search-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #6c8a71;
  animation: pulse 0.8s ease-in-out infinite alternate;
}
.sidebar-scroll {
  min-height: 0;
  overflow: auto;
  padding: 0 9px 24px;
}
.home-link,
.sidebar-section button {
  width: 100%;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--theme-neutral-text-soft, #525a53);
  text-align: left;
  cursor: pointer;
}
.home-link {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 9px;
  font: 650 11px var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.home-link:hover,
.sidebar-section button:hover {
  background: var(--theme-muted-bg, #ecefea);
  color: var(--theme-neutral-text, #263229);
}
.home-link.active,
.sidebar-section button.active {
  background: var(--theme-success-bg, #e3ebe3);
  color: var(--theme-success-text, #2f5136);
}
.sidebar-section {
  margin-top: 21px;
}
.sidebar-section h2 {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 8px 7px;
  color: var(--theme-neutral-text-muted, #8a918a);
  font: 750 9px/1 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.sidebar-section ul {
  display: grid;
  gap: 1px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.sidebar-section button {
  display: grid;
  gap: 2px;
  padding: 7px 9px;
  font: 500 11px/1.2 var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
}
.sidebar-section button span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sidebar-section button small {
  color: var(--theme-neutral-text-muted, #929891);
  font-size: 9px;
}
.categories details + details {
  margin-top: 4px;
}
.categories summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 7px 8px;
  color: var(--theme-neutral-text-soft, #4d564f);
  font: 650 10px var(--font-body, Inter, ui-sans-serif, system-ui, sans-serif);
  cursor: pointer;
}
.categories summary small {
  min-width: 22px;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--theme-muted-bg, #e8ebe6);
  color: var(--theme-neutral-text-soft, #7b827b);
  font-size: 9px;
  text-align: center;
}
.categories details ul {
  padding-left: 7px;
}
.sidebar-empty {
  margin: 9px;
  color: var(--theme-neutral-text-muted, #8a918a);
  font-size: 10px;
  line-height: 1.45;
}
.sidebar-pagination {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 6px;
  margin: 16px 5px 0;
  padding-top: 11px;
  border-top: 1px solid var(--theme-neutral-border, #e1e4de);
  color: var(--theme-neutral-text-muted, #8a918a);
  font-size: 9px;
  text-align: center;
}
.sidebar-pagination button {
  width: auto;
  padding: 5px 7px;
  border: 1px solid var(--theme-neutral-border, #dce0da);
  background: var(--theme-surface-bg, #fff);
  font-size: 9px;
  font-weight: 650;
}
.sidebar-pagination button:disabled {
  opacity: 0.45;
  cursor: default;
}
@keyframes pulse {
  to {
    opacity: 0.35;
  }
}
@media (max-width: 900px) {
  .kb-sidebar {
    border-right: 0;
    border-bottom: 1px solid var(--theme-neutral-border, #e1e4de);
  }
  .sidebar-scroll {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding-bottom: 10px;
  }
  .home-link {
    width: auto;
    flex: 0 0 auto;
  }
  .sidebar-section {
    display: none;
  }
  .kb-sidebar.has-query .categories {
    display: block;
    min-width: 240px;
    margin-top: 0;
  }
  .kb-sidebar.has-query .categories h2 {
    margin-top: 9px;
  }
}
@media print {
  .kb-sidebar {
    display: none !important;
  }
}
</style>
