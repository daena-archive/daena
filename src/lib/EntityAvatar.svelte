<script lang="ts">
import EntityGlyph from "$lib/entity-colors/EntityGlyph.svelte";
import { project } from "$lib/project/client";

const PROFILE_TYPES = new Set(["image/png", "image/jpeg", "image/gif", "image/webp"]);

let {
  entityId,
  name,
  size = 28,
}: {
  entityId: string;
  name: string;
  size?: number;
} = $props();

let url = $state("");

$effect(() => {
  const id = entityId;
  let disposed = false;
  let objectUrl = "";
  url = "";
  void project
    .listAssets(id)
    .then((assets) => {
      const asset = assets.find((candidate) => candidate.role === "profile" && PROFILE_TYPES.has(candidate.mime_type));
      if (!asset || disposed) return;
      return project.readAssetBytes(asset.id).then((bytes) => {
        if (disposed) return;
        objectUrl = URL.createObjectURL(new Blob([Uint8Array.from(bytes)], { type: asset.mime_type }));
        url = objectUrl;
      });
    })
    .catch(() => {
      if (!disposed) url = "";
    });
  return () => {
    disposed = true;
    if (objectUrl) URL.revokeObjectURL(objectUrl);
  };
});
</script>

{#if url}
  <img class="avatar" src={url} alt="" width={size} height={size} />
{:else}
  <EntityGlyph
    icon={{ kind: "catalog", id: "person" }}
    iconColor={{ kind: "preset", id: "rose" }}
    pluginId="daena.lore"
    size={Math.max(12, Math.round(size * 0.55))}
    box={size} />
{/if}

<style>
.avatar {
  display: block;
  border-radius: 50%;
  object-fit: cover;
  background: var(--surface-warm, var(--surface));
}
</style>
