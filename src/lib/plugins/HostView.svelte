<script lang="ts">
  import { project, type Entity, type PluginAdminEntry, type HostViewData } from "$lib/project/client";

  type View = PluginAdminEntry["views"][number];
  type ViewComponent = NonNullable<View["components"]>[number];
  type FieldDefinition = PluginAdminEntry["schemas"][number]["fields"][number];

  let { plugin, view }: { plugin: PluginAdminEntry; view: View } = $props();
  let lists = $state<Record<string, Entity[]>>({});
  let selected = $state<Entity | null>(null);
  let selectedEntityId = $state<string | null>(null);
  let fields = $state<Record<string, unknown>>({});
  let loading = $state(true);
  let saving = $state("");
  let error = $state("");

  function fieldDefinition(key: string): FieldDefinition | undefined {
    for (const schema of plugin.schemas) {
      const field = schema.fields.find((candidate) => candidate.key === key);
      if (field) return field;
    }
    return undefined;
  }

  async function refresh(entityId = selectedEntityId) {
    loading = true;
    error = "";
    try {
      const result: HostViewData = await project.hostViewData(plugin.id, view.id, entityId ?? undefined);
      lists = result.lists;
      selected = result.selected;
      selectedEntityId = result.selected?.id ?? null;
      fields = result.fields;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void refresh(null);
  });

  function selectEntity(entity: Entity) {
    selectedEntityId = entity.id;
    void refresh(entity.id);
  }

  function commandAction(commandId: string): string | undefined {
    return plugin.commands.find((command) => command.id === commandId)?.action?.type;
  }

  async function invokeCommand(commandId: string) {
    error = "";
    try {
      const action = await project.hostViewInvokeCommand(plugin.id, view.id, commandId);
      if (action === "refresh-view") await refresh();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function displayValue(value: unknown): string {
    if (value === null || value === undefined || value === "") return "Not set";
    if (typeof value === "boolean") return value ? "Yes" : "No";
    return String(value);
  }

  async function saveField(component: Extract<ViewComponent, { type: "field-form" }>, field: FieldDefinition, raw: unknown) {
    if (!selected || !component.editable) return;
    let value = raw;
    if (field.type === "number") value = raw === "" ? null : Number(raw);
    if (field.type === "boolean") value = Boolean(raw);
    saving = `${component.id}:${field.key}`;
    error = "";
    try {
      await project.hostViewSetField(plugin.id, view.id, component.id, selected.id, field.key, value);
      await refresh(selected.id);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      saving = "";
    }
  }

  function inputValue(key: string): string | number | boolean {
    const value = fields[key];
    if (typeof value === "number" || typeof value === "boolean") return value;
    return value == null ? "" : String(value);
  }
</script>

<section class="host-view" aria-label={`${plugin.name} · ${view.title}`}>
  <header class="host-view-heading">
    <div>
      <span class="overline">{plugin.name}</span>
      <h1>{view.title}</h1>
    </div>
    <span class="host-view-badge">Host-rendered</span>
  </header>

  {#if loading}
    <p class="host-view-state">Loading…</p>
  {:else if error}
    <p class="host-view-state error">{error}</p>
  {:else}
    <div class="host-view-components">
      {#each view.components ?? [] as component (component.id)}
        {#if component.type === "heading"}
          <h2>{component.text}</h2>
        {:else if component.type === "text"}
          <p class="host-view-copy">{component.text}</p>
        {:else if component.type === "entity-list"}
          <article class="host-view-card">
            <div class="host-view-card-heading">
              <h2>{component.title}</h2>
              <span>{lists[component.id]?.length ?? 0}</span>
            </div>
            {#if (lists[component.id]?.length ?? 0) === 0}
              <p class="host-view-state">No {component.entityType} entries yet.</p>
            {:else}
              <ul class="host-view-list">
                {#each lists[component.id] ?? [] as entity (entity.id)}
                  <li>
                    <button type="button" class:selected={selectedEntityId === entity.id} onclick={() => selectEntity(entity)}>
                      <strong>{entity.name}</strong><span>{entity.entity_type ?? component.entityType}</span>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </article>
        {:else if component.type === "entity-detail"}
          <article class="host-view-card">
            <div class="host-view-card-heading"><h2>{component.title}</h2></div>
            {#if selected}
              <dl class="host-view-detail">
                <div><dt>Name</dt><dd>{selected.name}</dd></div>
                <div><dt>Type</dt><dd>{selected.entity_type ?? "Unspecified"}</dd></div>
                <div><dt>Updated</dt><dd>{selected.updated_at}</dd></div>
              </dl>
            {:else}
              <p class="host-view-state">Select an entry to inspect it.</p>
            {/if}
          </article>
        {:else if component.type === "field-form"}
          <article class="host-view-card">
            <div class="host-view-card-heading"><h2>{component.title}</h2></div>
            {#if selected}
              <div class="host-view-form">
                {#each component.fields as key (key)}
                  {@const field = fieldDefinition(key)}
                  {#if field}
                    <label>
                      <span>{field.label}</span>
                      {#if field.type === "enum"}
                        <select disabled={!component.editable || saving === `${component.id}:${field.key}`} value={String(inputValue(field.key))} onchange={(event) => void saveField(component, field, (event.currentTarget as HTMLSelectElement).value)}>
                          <option value="">Not set</option>
                          {#each field.options ?? [] as option}<option value={option}>{option}</option>{/each}
                        </select>
                      {:else if field.type === "boolean"}
                        <input type="checkbox" disabled={!component.editable || saving === `${component.id}:${field.key}`} checked={inputValue(field.key) === true} onchange={(event) => void saveField(component, field, (event.currentTarget as HTMLInputElement).checked)} />
                      {:else}
                        <input type={field.type === "number" ? "number" : field.type === "date" ? "date" : "text"} disabled={!component.editable || saving === `${component.id}:${field.key}`} value={String(inputValue(field.key))} onchange={(event) => void saveField(component, field, (event.currentTarget as HTMLInputElement).value)} />
                      {/if}
                    </label>
                  {/if}
                {/each}
              </div>
            {:else}
              <p class="host-view-state">Select an entry to edit its fields.</p>
            {/if}
          </article>
        {:else if component.type === "button"}
          <div class="host-view-action">
            <button type="button" class="host-view-button" disabled={commandAction(component.command) !== "refresh-view"} onclick={() => void invokeCommand(component.command)}>{component.label}</button>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</section>

<style>
  .host-view { padding: 28px 40px 40px; }
  .host-view-heading { display: flex; align-items: start; justify-content: space-between; gap: 20px; margin-bottom: 24px; }
  .host-view-heading h1 { margin: 7px 0 0; font: 500 34px/1.05 var(--font-display); }
  .host-view-badge { padding: 6px 9px; border: 1px solid #d8c3a5; border-radius: 999px; color: var(--accent-dark); font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: .08em; }
  .host-view-components { display: grid; gap: 16px; max-width: 860px; }
  .host-view-components > h2 { margin: 0; font: 500 25px/1.1 var(--font-display); }
  .host-view-copy { max-width: 60ch; margin: 0; color: var(--ink-soft); font-size: 13px; line-height: 1.6; }
  .host-view-card { padding: 18px 20px; border: 1px solid var(--line); border-radius: 14px; background: var(--paper); box-shadow: 0 8px 24px rgba(62, 42, 25, .05); }
  .host-view-card-heading { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .host-view-card-heading h2 { margin: 0; font: 500 21px var(--font-display); }
  .host-view-card-heading > span { color: var(--ink-faint); font-size: 12px; }
  .host-view-list { display: grid; gap: 8px; margin: 16px 0 0; padding: 0; list-style: none; }
  .host-view-list li { border-bottom: 1px solid var(--line); }
  .host-view-list button { display: flex; width: 100%; justify-content: space-between; gap: 18px; padding: 10px 0; border: 0; background: transparent; color: inherit; text-align: left; cursor: pointer; font-size: 12px; }
  .host-view-list button.selected { color: var(--accent-dark); }
  .host-view-list button span { color: var(--ink-faint); }
  .host-view-detail { display: grid; gap: 10px; margin: 16px 0 0; }
  .host-view-detail div { display: flex; justify-content: space-between; gap: 18px; font-size: 12px; }
  .host-view-detail dt { color: var(--ink-faint); }
  .host-view-detail dd { margin: 0; text-align: right; }
  .host-view-form { display: grid; gap: 12px; margin-top: 16px; }
  .host-view-form label { display: grid; gap: 6px; color: var(--ink-soft); font-size: 11px; }
  .host-view-form input, .host-view-form select { min-height: 34px; padding: 7px 9px; border: 1px solid var(--line); border-radius: 7px; background: var(--paper); color: var(--ink); font: inherit; }
  .host-view-form input[type="checkbox"] { width: 16px; min-height: 16px; }
  .host-view-action { display: flex; justify-content: flex-start; }
  .host-view-button { padding: 9px 14px; border: 1px solid var(--accent-dark); border-radius: 8px; background: var(--accent-dark); color: white; cursor: pointer; font-size: 12px; }
  .host-view-button:disabled { opacity: .55; cursor: default; }
  .host-view-state { margin: 0; color: var(--ink-soft); font-size: 12px; }
  .host-view-state.error { color: #a14f42; }
  @media (max-width: 760px) { .host-view { padding: 24px 17px 30px; } .host-view-heading { display: block; } .host-view-badge { display: inline-block; margin-top: 16px; } }
</style>
