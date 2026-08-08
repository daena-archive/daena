import { invoke } from "@tauri-apps/api/core";
import type { ModuleManifest } from "../../../packages/module-api/src/index";

export interface Entity {
  id: string;
  name: string;
  entity_type: string | null;
  deleted: boolean;
  created_at: string;
  updated_at: string;
  revision: string;
}
export interface Document { id: string; entity_id: string; format: string; body: string; updated_at: string; revision: string; }
export interface FieldValue { entity_id: string; namespace: string; key: string; value: unknown; revision: string; }
export interface Relationship { id: string; source_id: string; target_id: string; relationship_type: string; metadata: string; revision: string; }
export interface MapLocation { id: string; mapEntityId: string; role: string; label: string; anchor: unknown; validity: { from: unknown | null; to: unknown | null }; anchorKind?: string; resolution?: string; }
export interface MapRecoveryCopy { fileName: string; path: string; createdAt: string; }
export interface Asset { id: string; entity_id: string; namespace: string; filename: string; content_hash: string; size: number; mime_type: string; path: string; created_at: string; revision: string; }
export interface ProjectInfo { name: string; root: string; index_status: string; assets: string; }
export interface ExternalChangeReport { changed: boolean; paths: string[]; diagnostics: string[]; }
export interface GitStatus {
  repository: boolean;
  branch: string | null;
  changes: string[];
  canonical_changes: string[];
  staged_canonical_changes: string[];
}
export interface GitPreflight {
  ready: boolean;
  diagnostics: string[];
  canonical_paths: string[];
  asset_paths: string[];
  staging_paths: string[];
  staged_paths: string[];
  unmerged_paths: string[];
}
export interface GitLogEntry { hash: string; date: string; subject: string; }
export interface GitToolInfo {
  available: boolean;
  version: string | null;
  error: string | null;
}
export interface GitRemote {
  name: string;
  fetchUrl: string;
  pushUrl: string;
}
export interface GitUpstream {
  remote: string;
  branch: string;
  remoteHash: string | null;
}
export interface GitResetResult {
  status: GitStatus;
  previousHead: string | null;
  currentHead: string | null;
  upstream: GitUpstream | null;
  divergedFromUpstream: boolean;
  rebuild: ExternalChangeReport;
}
export type { ModuleManifest };
export type ProjectModuleManifest = ModuleManifest & { enabled: boolean };
export interface InstalledPluginVersion { id: string; version: string; publisher: string; digest: string; signed: boolean; }
export interface LifecycleInfo { state: string; failures: number; lastError: string | null; }
export interface InstalledVersionInfo {
  version: string;
  publisher: string;
  digest: string;
  signed: boolean;
  unsignedConsent: boolean;
  installedAt: number;
  isSelected: boolean;
  isActiveCandidate: boolean;
  bundled: boolean;
  rollbackAvailable: boolean;
}
export interface DependencyState { resolved: boolean; order: string[]; error: string | null; }
export interface PluginAdminEntry extends ProjectModuleManifest {
  enabled: boolean;
  selectedVersion: string | null;
  dataVersion: number;
  lifecycle: LifecycleInfo;
  runtimeRunning: boolean;
  grantedCapabilities: string[];
  installedVersions: InstalledVersionInfo[];
  dependencyState: DependencyState;
}
export interface PluginAdminView { plugins: PluginAdminEntry[]; }
export interface RecentProjectSetting { name: string; root: string; }
export interface GeneralSettings {
  recentProjects: RecentProjectSetting[];
}
export interface AppSettings {
  formatVersion: number;
  general: GeneralSettings;
}
export interface AppSettingsUpdate {
  general?: { recentProjects?: RecentProjectSetting[] };
}
export interface HostViewData {
  lists: Record<string, Entity[]>;
  selected: Entity | null;
  fields: Record<string, unknown>;
}
export interface PluginUpgradePlan {
  pluginId: string;
  fromVersion: string | null;
  toVersion: string;
  consent: { added: string[]; removed: string[]; requiresRenewal: boolean };
  migrations: { from: number; to: number; migrationIds: string[]; requiresBackup: boolean };
  target: { signed: boolean; publisher: string };
}
type DialogSelection = string | string[] | null;
export interface MutationOptions { expectedRevision?: string; requestId?: string; }

const requestId = (options?: MutationOptions) => options?.requestId ?? crypto.randomUUID();

export const project = {
  openMemory: () => invoke<void>("project_open_memory"),
  openDefault: () => invoke<void>("project_open_default"),
  open: (path: string) => invoke<void>("project_open", { path }),
  openDirectory: (path: string) => invoke<ProjectInfo>("project_open_directory", { path }),
  pickDirectory: () => invoke<DialogSelection>("plugin:dialog|open", { options: { directory: true, multiple: false } }),
  pickFile: () => invoke<DialogSelection>("plugin:dialog|open", { options: { directory: false, multiple: false } }),
  create: (path: string) => invoke<ProjectInfo>("project_new", { path }),
  close: () => invoke<void>("project_close"),
  info: () => invoke<ProjectInfo | null>("project_info"),
  reconcileExternalChanges: () => invoke<ExternalChangeReport>("project_reconcile_external_changes"),
  saveRecoveryCopy: (entityId: string, body: string) => invoke<string>("project_save_recovery_copy", { entityId, body }),
  gitStatus: () => invoke<GitStatus>("project_git_status"),
  gitPreflight: () => invoke<GitPreflight>("project_git_preflight"),
  gitStagingPreview: () => invoke<GitPreflight>("project_git_staging_preview"),
  gitInit: () => invoke<GitStatus>("project_git_init"),
  gitLog: () => invoke<GitLogEntry[]>("project_git_log"),
  gitCommit: (message: string, paths?: string[]) =>
    invoke<GitStatus>("project_git_commit", { message, paths: paths ?? null }),
  gitToolInfo: () => invoke<GitToolInfo>("git_tool_info"),
  gitShowTree: (hash: string) => invoke<string[]>("project_git_show_tree", { hash }),
  gitShowFile: (hash: string, path: string) => invoke<string>("project_git_show_file", { hash, path }),
  gitResetHard: (hash: string) => invoke<GitResetResult>("project_git_reset_hard", { hash }),
  gitRemoteList: () => invoke<GitRemote[]>("project_git_remote_list"),
  gitRemoteAdd: (name: string, url: string) => invoke<GitRemote[]>("project_git_remote_add", { name, url }),
  gitRemoteSetUrl: (name: string, url: string) => invoke<GitRemote[]>("project_git_remote_set_url", { name, url }),
  gitRemoteRemove: (name: string) => invoke<GitRemote[]>("project_git_remote_remove", { name }),
  gitPush: (remote: string, branch?: string | null, forceWithLease = false) =>
    invoke<GitStatus>("project_git_push", { remote, branch: branch ?? null, forceWithLease }),
  gitRestoreFromUpstream: () => invoke<GitResetResult>("project_git_restore_from_upstream"),
  openExternalUrl: (url: string) => invoke<void>("open_external_url", { url }),
  listEntities: () => invoke<Entity[]>("project_list_entities"),
  search: (query: string) => invoke<Entity[]>("project_search", { query }),
  deleteEntity: (id: string, options?: MutationOptions) => invoke<void>("project_delete_entity", { id, expected_revision: options?.expectedRevision ?? null, request_id: requestId(options) }),
  createEntity: (name: string, entityType?: string, options?: MutationOptions) =>
    invoke<Entity>("project_create_entity", {
      input: { name, entity_type: entityType || null },
      request_id: requestId(options),
    }),
  createMap: (name = "Untitled map") => invoke<Entity>("project_create_map", { name }),
  updateEntity: (id: string, name?: string | null, entityType?: string | null, options?: MutationOptions) =>
    invoke<Entity>("project_update_entity", { id, name: name ?? null, entity_type: entityType ?? null, expected_revision: options?.expectedRevision ?? null, request_id: requestId(options) }),
  listDocuments: (entityId: string) => invoke<Document[]>("project_list_documents", { entityId }),
  saveDocument: (entityId: string, body: string, format: "markdown" | "plain-text" | "rich-text" = "markdown", options?: MutationOptions) => invoke<void>("project_save_document", { input: { entity_id: entityId, body, format }, expected_revision: options?.expectedRevision ?? null, request_id: requestId(options) }),
  saveEntry: (input: { document: { entity_id: string; body: string; format?: "markdown" | "plain-text" | "rich-text" }; fields: FieldValue[] }, options?: MutationOptions) => invoke<void>("project_save_entry", { input, expected_revision: options?.expectedRevision ?? null, request_id: requestId(options) }),
  listFields: (entityId: string) => invoke<FieldValue[]>("project_list_fields", { entityId }),
  setField: (field: FieldValue, options?: MutationOptions) => invoke<void>("project_set_field", { field, request_id: requestId(options) }),
  createRelationship: (sourceId: string, targetId: string, type: string, metadata?: Record<string, unknown>, options?: MutationOptions) =>
    invoke<Relationship>("project_create_relationship", { input: { source_id: sourceId, target_id: targetId, relationship_type: type, metadata: metadata ? JSON.stringify(metadata) : null }, expected_revision: options?.expectedRevision ?? null, request_id: requestId(options) }),
  listRelationships: (entityId: string) => invoke<Relationship[]>("project_list_relationships", { entityId }),
  listMapLocations: (entityId: string) => invoke<MapLocation[]>("project_list_map_locations", { entityId }),
  upsertMapLocation: (entityId: string, location: MapLocation, options?: MutationOptions) => invoke<void>("project_upsert_map_location", { entityId, location, request_id: requestId(options) }),
  unlinkMapLocation: (entityId: string, locationId: string, options?: MutationOptions) => invoke<void>("project_unlink_map_location", { entityId, locationId, request_id: requestId(options) }),
  mapsNavigation: (operation: string, input: { mapEntityId?: string; entityId?: string; linkId?: string; date?: unknown; entityIds?: string[] } = {}) => invoke<unknown>("maps_navigation", { operation, map_entity_id: input.mapEntityId ?? null, entity_id: input.entityId ?? null, link_id: input.linkId ?? null, date: input.date ?? null, entity_ids: input.entityIds ?? null }),
  mapsEditorSave: () => invoke<void>("maps_editor_save"),
  mapsEditorCaptureAnchor: () => invoke<void>("maps_editor_capture_anchor"),
  mapsEditorSetOverlay: (frame: unknown) => invoke<void>("maps_editor_set_overlay", { frame }),
  mapsEditorSetDate: (date: unknown) => invoke<void>("maps_editor_set_date", { date }),
  mapsEditorFocusLink: (linkId: string) => invoke<void>("maps_editor_focus_link", { linkId }),
  mapsRecoveryList: (entityId: string) => invoke<MapRecoveryCopy[]>("maps_recovery_list", { entityId }),
  mapsRecoveryRestore: (entityId: string, fileName: string) => invoke<Asset>("maps_recovery_restore", { entityId, fileName }),
  registerAsset: (input: { entity_id: string; namespace: string; filename: string; content_hash: string; size: number; mime_type: string; path: string }, options?: MutationOptions) =>
    invoke<Asset>("project_register_asset", { input, expected_revision: options?.expectedRevision ?? null, request_id: requestId(options) }),
  registerAssetFile: (input: { entity_id: string; namespace: string; source_path: string; filename: string; mime_type: string }, options?: MutationOptions) =>
    invoke<Asset>("project_register_asset_file", { input, expected_revision: options?.expectedRevision ?? null, request_id: requestId(options) }),
  listAssets: (entityId: string) => invoke<Asset[]>("project_list_assets", { entityId }),
  backup: () => invoke<string>("project_backup"),
  restore: (path: string) => invoke<void>("project_restore", { path }),
  restorePayload: (payload: string, options?: MutationOptions) => invoke<void>("project_restore_payload", { payload, request_id: requestId(options) }),
  rebuildSearch: () => invoke<void>("project_rebuild_search"),
  seedExample: () => invoke<number>("project_seed_example"),
  listModuleManifests: () => invoke<ProjectModuleManifest[]>("module_list_manifests"),
  enableModule: (id: string, grantedCapabilities?: string[]) => invoke<void>("module_enable", { id, grantedCapabilities }),
  disableModule: (id: string) => invoke<void>("module_disable", { id }),
  adminView: () => invoke<PluginAdminView>("plugin_admin_view"),
  openPluginWebview: (pluginId: string, viewId?: string) =>
    invoke<void>("plugin_open_webview", { pluginId, viewId }),
  mountPluginWebview: (pluginId: string, viewId: string | undefined, bounds: { x: number; y: number; width: number; height: number; viewportWidth: number; viewportHeight: number }, mapEntityId?: string, linkId?: string) =>
    invoke<void>("plugin_mount_webview", { pluginId, viewId, mapEntityId, linkId, bounds }),
  resizePluginWebview: (pluginId: string, bounds: { x: number; y: number; width: number; height: number; viewportWidth: number; viewportHeight: number }) =>
    invoke<void>("plugin_resize_webview", { pluginId, bounds }),
  unmountPluginWebview: (pluginId: string) => invoke<void>("plugin_unmount_webview", { pluginId }),
  closeAllPluginWebviews: () => invoke<void>("plugin_close_all_webviews"),
  hostViewData: (pluginId: string, viewId: string, selectedEntityId?: string) =>
    invoke<HostViewData>("plugin_host_view_data", { pluginId, viewId, selectedEntityId: selectedEntityId ?? null }),
  hostViewSetField: (pluginId: string, viewId: string, componentId: string, entityId: string, key: string, value: unknown) =>
    invoke<void>("plugin_host_view_set_field", { pluginId, viewId, componentId, entityId, key, value }),
  hostViewInvokeCommand: (pluginId: string, viewId: string, commandId: string, payload?: Record<string, unknown>) =>
    invoke<string>("plugin_host_invoke_command", { pluginId, viewId, commandId, payload: payload ?? {} }),
  closePluginWebview: (pluginId: string) => invoke<void>("plugin_close_webview", { pluginId }),
  installPlugin: (archive: string, allowUnsigned = false) =>
    invoke<InstalledPluginVersion>("plugin_install_package", { archive, allowUnsigned }),
  upgradePlugin: (pluginId: string, version: string, consent: boolean) =>
    invoke<void>("plugin_upgrade", { pluginId, version, consent }),
  pluginUpgradePlan: (pluginId: string, version: string) =>
    invoke<PluginUpgradePlan>("plugin_upgrade_plan", { pluginId, version }),
  rollbackPlugin: (pluginId: string, version: string) =>
    invoke<void>("plugin_rollback", { pluginId, version }),
  uninstallPluginCode: (pluginId: string, version: string) =>
    invoke<void>("plugin_uninstall_code", { pluginId, version }),
  deletePluginData: (pluginId: string, confirmation: string) =>
    invoke<string>("plugin_delete_data", { pluginId, confirmation }),
  retryPlugin: (pluginId: string) => invoke<void>("plugin_retry", { pluginId }),
  pickPluginPackage: () =>
    invoke<DialogSelection>("plugin:dialog|open", {
      options: {
        directory: false,
        multiple: false,
      filters: [{ name: "Daena Archive plugin", extensions: ["wbplugin"] }],
      },
    }),
  settingsGet: () => invoke<AppSettings>("settings_get"),
  settingsUpdate: (update: AppSettingsUpdate) => invoke<AppSettings>("settings_update", { update }),
};
