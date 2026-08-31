import { invoke } from "@tauri-apps/api/core";

export const DOWNLOAD_PAGE = "https://github.com/daena-archive/daena/releases";

export type AppUpdateCheck = {
  current: string;
  latest: string;
  newer: boolean;
  htmlUrl: string;
};

export async function openDownloadPage(): Promise<void> {
  await invoke("open_external_url", { url: DOWNLOAD_PAGE });
}

export async function checkAppUpdate(): Promise<AppUpdateCheck> {
  return invoke<AppUpdateCheck>("app_check_update");
}
