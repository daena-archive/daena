<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let { pluginId }: { pluginId: string } = $props();
  let opening = $state(false);
  let error = $state("");

  async function openPluginView() {
    opening = true;
    error = "";
    try {
      await invoke("plugin_open_webview", { pluginId });
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      opening = false;
    }
  }

  $effect(() => {
    void openPluginView();
  });
</script>

<div class="launcher">
  {#if opening}<small>Opening…</small>{/if}
  {#if error}<p class="error">{error}</p>{/if}
</div>

<style>
  .launcher { display:flex; align-items:center; justify-content:space-between; gap:16px; padding:14px 16px; border:1px solid rgba(229,214,195,.12); border-radius:12px; background:rgba(255,255,255,.025); }
  small { display:block; color:#9f9488; margin-top:4px; } .error { color:#e69b8b; flex-basis:100%; }
</style>
