import { invoke } from "@tauri-apps/api/core";
import { en } from "./i18n/en.ts";

export const DOWNLOAD_PAGE = "https://github.com/daena-archive/daena/releases";
export const UPDATE_CHANNEL_STORAGE_KEY = "daena-update-channel";

export type UpdateChannelPreference = "auto" | "stable" | "beta" | "alpha";

export type AppUpdateCheck = {
  current: string;
  latest: string;
  newer: boolean;
  htmlUrl: string;
  releaseChannel: "stable" | "beta" | "alpha";
  latestPrerelease: boolean;
  updateChannelPreference: UpdateChannelPreference;
};

export function normalizeUpdateChannelPreference(value: unknown): UpdateChannelPreference {
  return value === "stable" || value === "beta" || value === "alpha" || value === "auto" ? value : "auto";
}

export function readUpdateChannelPreference(storage: Pick<Storage, "getItem"> = localStorage): UpdateChannelPreference {
  try {
    return normalizeUpdateChannelPreference(storage.getItem(UPDATE_CHANNEL_STORAGE_KEY));
  } catch {
    return "auto";
  }
}

export function cacheUpdateChannelPreference(
  preference: UpdateChannelPreference,
  storage: Pick<Storage, "setItem"> = localStorage,
): void {
  try {
    storage.setItem(UPDATE_CHANNEL_STORAGE_KEY, preference);
  } catch {
    // Preference still applies for this session when storage is unavailable.
  }
}

export function formatUpdateMessage(
  result: AppUpdateCheck,
  copy: Pick<typeof en.settings, "updateAvailable" | "upToDate" | "beta" | "alpha" | "stable"> = en.settings,
): string {
  const preferenceLabel =
    result.updateChannelPreference === "auto" ? result.releaseChannel : result.updateChannelPreference;
  if (result.newer) {
    return copy.updateAvailable(result.latest, result.latestPrerelease ? copy[result.releaseChannel] : null);
  }
  return copy.upToDate(result.current, preferenceLabel === "stable" ? null : copy[preferenceLabel]);
}

export async function openDownloadPage(): Promise<void> {
  await invoke("open_external_url", { url: DOWNLOAD_PAGE });
}

export async function checkAppUpdate(
  channelPreference: UpdateChannelPreference = readUpdateChannelPreference(),
): Promise<AppUpdateCheck> {
  return invoke<AppUpdateCheck>("app_check_update", { channelPreference });
}
