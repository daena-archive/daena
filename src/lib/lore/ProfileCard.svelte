<script lang="ts">
import { evaluateProfile, formatProfileValue, profileCardShows, type ProfileDocument } from "./profile.ts";
import { profilePresetLabel } from "./profilePresets.ts";

let { profile }: { profile: ProfileDocument } = $props();

const evaluated = $derived(evaluateProfile(profile));
const rows = $derived(
  profile.components
    .filter((component) => profileCardShows(component, evaluated.get(component.id) ?? null))
    .map((component) => ({
      id: component.id,
      name: component.name,
      value: formatProfileValue(component, evaluated.get(component.id) ?? null),
    })),
);
const origin = $derived(
  profile.presetOrigin && profile.presetOrigin !== "custom" ? profilePresetLabel(profile.presetOrigin) : "",
);
</script>

<section class="profile-card" aria-label="Profile">
  <h3>Profile</h3>
  {#if origin}<p class="preset">{origin}</p>{/if}
  {#if rows.length === 0}
    <p class="card-empty">No profile values yet.</p>
  {:else}
    <dl>
      {#each rows as row (row.id)}
        <div>
          <dt>{row.name}</dt>
          <dd>{row.value}</dd>
        </div>
      {/each}
    </dl>
  {/if}
</section>

<style>
.profile-card {
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}
.profile-card h3 {
  margin: 0 0 8px;
  color: var(--ink-soft);
  font-size: 11px;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}
.preset {
  margin: -4px 0 8px;
  color: var(--ink-soft);
  font-size: 11px;
}
.profile-card dl {
  display: grid;
  gap: 6px;
  margin: 0;
}
.profile-card dl div {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
  align-items: baseline;
}
.profile-card dt {
  color: var(--ink-soft);
  font-size: 11px;
}
.profile-card dd {
  margin: 0;
  font-size: 12px;
  text-align: right;
}
</style>
