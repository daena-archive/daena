import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// @ts-ignore process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

/** Vite only treats `.json` as JSON. A `.geojson` file is otherwise parsed as JS, where `{ "type":` is a syntax error. */
/** @returns {import("vite").Plugin} */
function geojson() {
  return {
    name: "geojson",
    enforce: "pre",
    /**
     * @param {string} code
     * @param {string} id
     */
    transform(code, id) {
      const path = String(id).split("?", 1)[0];
      if (!path.endsWith(".geojson")) return null;
      return { code: `export default ${code}`, map: null };
    },
  };
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [geojson(), sveltekit()],
  build: {
    chunkSizeWarningLimit: 4000,
  },

  // Keep Deno's node_modules layout from stale uuid paths during dep prebundling.
  optimizeDeps: {
    include: ["vis-timeline/standalone", "uuid", "maplibre-gl", "terra-draw", "terra-draw-maplibre-gl-adapter", "d3-contour"],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
