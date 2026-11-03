import { invoke } from "@tauri-apps/api/core";
import {
  createBrokerModuleContext,
  type BrokerModuleContextOptions,
  type ModuleContext,
  type ModuleManifest,
  type MutationOptions,
} from "../../../packages/module-api/src/index";
import type { PluginRpcTransport } from "../../../packages/plugin-sdk/src/index";
import type { MapLocationReference } from "../../../packages/plugin-sdk/src/maps";

export function buildModuleContext(
  manifest: ModuleManifest,
  projectId: string,
  options?: Omit<BrokerModuleContextOptions, "modules" | "maps">,
): ModuleContext {
  void projectId;
  const transport: PluginRpcTransport = {
    call: (method, payload, requestId) =>
      invoke("trusted_module_rpc", {
        method,
        payload,
        requestId: requestId ?? crypto.randomUUID(),
        pluginId: manifest.id,
      }),
  };
  const modules: ModuleContext["modules"] = {
    list: async () => {
      const manifests = await invoke<Array<ModuleManifest & { enabled?: boolean }>>("module_list_manifests");
      return manifests.filter((candidate) => candidate.enabled !== false);
    },
  };
  const maps: ModuleContext["maps"] = {
    openMap: async (input) =>
      await invoke("maps_navigation", {
        operation: "openMap",
        mapEntityId: input.mapEntityId,
        linkId: input.linkId ?? null,
        entityId: null,
      }),
    focusEntity: async (input) =>
      await invoke("maps_navigation", {
        operation: "focusEntity",
        mapEntityId: input.mapEntityId ?? null,
        entityId: input.entityId,
        linkId: null,
      }),
    setDate: async (input) =>
      await invoke("maps_navigation", {
        operation: "setDate",
        mapEntityId: null,
        entityId: null,
        linkId: null,
        date: input.date,
      }),
    showResults: async (input) =>
      await invoke("maps_navigation", {
        operation: "showResults",
        mapEntityId: input.mapEntityId ?? null,
        entityId: null,
        linkId: null,
        entityIds: input.entityIds,
      }),
    listLocations: async (input) =>
      await invoke<readonly MapLocationReference[]>("project_list_map_locations", { entityId: input.entityId }),
    upsertLocation: async (input, options?: MutationOptions) => {
      await invoke<void>("project_upsert_map_location", {
        entityId: input.entityId,
        location: input.location,
        request_id: options?.requestId ?? null,
      });
    },
    unlinkLocation: async (input, options?: MutationOptions) => {
      await invoke<void>("project_unlink_map_location", {
        entityId: input.entityId,
        locationId: input.locationId,
        request_id: options?.requestId ?? null,
      });
    },
  };
  return createBrokerModuleContext(manifest, transport, { ...options, modules, maps });
}
