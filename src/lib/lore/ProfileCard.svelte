<script lang="ts">
import { formatProfileValue, type ProfileDocument } from "./profile";

let { profile }: { profile: ProfileDocument } = $props();

const rows = $derived(
  profile.components
    .map((component) => ({
      id: component.id,
      name: component.name,
      value: formatProfileValue(component),
    }))
    .filter((row) => row.name.trim()),
);
</script>

<section class="profile-card" aria-label="Profile">
  <h3>Profile</h3>
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
