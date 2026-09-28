<script lang="ts">
import { ChevronDown } from "@lucide/svelte";
import { profileCardGroups, profileCardRows, profileCardSummary, type ProfileDocument } from "./profile.ts";
import { profilePresetLabel, type LoadedProfilePreset } from "./profilePresets.ts";
import { isCustomPresetOrigin } from "./profilePresetSeed.ts";

let {
  profile,
  asOfLabel = "",
  presets = [],
}: {
  profile: ProfileDocument;
  asOfLabel?: string;
  presets?: LoadedProfilePreset[];
} = $props();

const rows = $derived(profileCardRows(profile));
const summary = $derived(profileCardSummary(rows));
const groups = $derived(profileCardGroups(rows));
const origin = $derived(
  profile.presetOrigin && !isCustomPresetOrigin(profile.presetOrigin)
    ? profilePresetLabel(profile.presetOrigin, presets)
    : "",
);
const canCollapse = $derived(rows.length > summary.length);
</script>

{#snippet valueList(items: typeof rows)}
  <dl>
    {#each items as row (row.id)}
      <div>
        <dt>{row.name}</dt>
        {#if row.value}<dd>{row.value}</dd>{/if}
      </div>
    {/each}
  </dl>
{/snippet}

<section class="profile-card" aria-label="Profile">
  <header>
    <h3>Profile</h3>
    {#if asOfLabel}<p>{asOfLabel}</p>{/if}
    {#if origin}<p>{origin}</p>{/if}
  </header>
  {#if rows.length === 0}
    <p class="empty">No profile values yet.</p>
  {:else if canCollapse}
    <dl class="summary">
      {#each summary as row (row.id)}
        <div>
          <dt>{row.name}</dt>
          <dd>{row.value || "—"}</dd>
        </div>
      {/each}
    </dl>
    <div class="groups">
      {#each groups as group (group.kind)}
        <details>
          <summary>
            <ChevronDown size={12} strokeWidth={1.8} aria-hidden="true" />
            <span>{group.label}</span>
            <small>{group.rows.length}</small>
          </summary>
          {@render valueList(group.rows)}
        </details>
      {/each}
    </div>
  {:else}
    {#each groups as group (group.kind)}
      <h4>{group.label}</h4>
      {@render valueList(group.rows)}
    {/each}
  {/if}
</section>

<style>
.profile-card {
  min-width: 0;
  margin-top: 12px;
  padding: 12px 16px 8px;
  border-top: 1px solid var(--theme-neutral-border, var(--line));
}
.profile-card header h3,
.profile-card h4 {
  margin: 0;
  color: var(--theme-neutral-text-muted, var(--ink-soft));
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.profile-card header h3 {
  font-size: 8px;
}
.profile-card header p,
.empty {
  margin: 4px 0 0;
  color: var(--theme-neutral-text, var(--ink));
  font-size: 12px;
}
.empty {
  color: var(--theme-neutral-text-muted, var(--ink-soft));
  font-size: 11px;
}
.profile-card h4 {
  margin: 12px 0 6px;
  font-size: 8px;
}
.summary {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px 12px;
  margin: 12px 0 0;
}
.summary div {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.summary dt {
  color: var(--theme-neutral-text-muted, var(--ink-soft));
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.summary dd {
  margin: 0;
  overflow-wrap: anywhere;
  color: var(--theme-neutral-text, var(--ink));
  font-size: 12px;
}
.groups {
  display: grid;
  min-width: 0;
  margin-top: 12px;
  border-top: 1px solid var(--theme-neutral-border, var(--line));
}
.groups details + details {
  border-top: 1px solid var(--theme-neutral-border, var(--line));
}
.groups summary {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 0;
  color: var(--theme-neutral-text-soft, var(--ink-soft));
  font-size: 8px;
  font-weight: 800;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  cursor: pointer;
}
.groups summary::-webkit-details-marker {
  display: none;
}
.groups summary small {
  min-width: 16px;
  margin-left: auto;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--theme-muted-bg, var(--surface-muted));
  color: var(--theme-neutral-text-muted, var(--ink-soft));
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0;
  text-align: center;
}
.groups summary :global(svg) {
  flex: none;
  color: var(--theme-neutral-text-muted, var(--ink-soft));
}
.groups details[open] summary :global(svg) {
  transform: rotate(180deg);
}
.profile-card dl:not(.summary) {
  display: grid;
  min-width: 0;
  gap: 7px;
  margin: 0;
}
.groups dl {
  padding: 0 0 10px 18px;
}
.profile-card dl:not(.summary) div {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 8px;
  align-items: start;
}
.profile-card dl:not(.summary) dt,
.profile-card dl:not(.summary) dd {
  min-width: 0;
  overflow-wrap: anywhere;
}
.profile-card dl:not(.summary) dt {
  color: var(--theme-neutral-text-soft, var(--ink-soft));
  font-size: 11px;
}
.profile-card dl:not(.summary) dd {
  margin: 0;
  color: var(--theme-neutral-text, var(--ink));
  font-size: 12px;
  text-align: right;
}
</style>
