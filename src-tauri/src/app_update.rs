use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const DOWNLOAD_PAGE: &str = "https://github.com/daena-archive/daena/releases";
const RELEASES_URL: &str = "https://api.github.com/repos/daena-archive/daena/releases?per_page=5";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionCore {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateCheck {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub draft: bool,
    pub html_url: String,
}

pub fn allowed_external_url(url: &str) -> bool {
    url == "https://git-scm.com/downloads"
        || url.starts_with("https://git-scm.com/")
        || url == DOWNLOAD_PAGE
        || url.starts_with(&format!("{DOWNLOAD_PAGE}/"))
}

pub fn version_core(raw: &str) -> Option<VersionCore> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    let numeric = trimmed.split(['-', '+']).next()?.trim();
    if numeric.is_empty() {
        return None;
    }
    let mut parts = numeric.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some(VersionCore {
        major,
        minor,
        patch,
    })
}

pub fn pick_latest_release(releases: &[GithubRelease]) -> Option<&GithubRelease> {
    releases.iter().find(|release| !release.draft)
}

pub fn compare_update(current: &str, latest_tag: &str) -> Option<bool> {
    Some(version_core(latest_tag)? > version_core(current)?)
}

pub fn check_update(current: &str) -> Result<AppUpdateCheck, String> {
    if version_core(current).is_none() {
        return Err("Current version is invalid.".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "Could not check for updates.".to_string())?;
    let response = client
        .get(RELEASES_URL)
        .header("User-Agent", "Daena-Archive")
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|_| "Could not check for updates.".to_string())?;
    if !response.status().is_success() {
        return Err("Could not check for updates.".into());
    }
    let releases: Vec<GithubRelease> = response
        .json()
        .map_err(|_| "Could not check for updates.".to_string())?;
    let release =
        pick_latest_release(&releases).ok_or_else(|| "No releases published yet.".to_string())?;
    let newer = compare_update(current, &release.tag_name)
        .ok_or_else(|| "Could not read the latest version.".to_string())?;
    Ok(AppUpdateCheck {
        current: current.to_string(),
        latest: release.tag_name.clone(),
        newer,
        html_url: release.html_url.clone(),
    })
}

#[tauri::command]
pub async fn app_check_update() -> Result<AppUpdateCheck, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    tauri::async_runtime::spawn_blocking(move || check_update(&current))
        .await
        .map_err(|_| "Could not check for updates.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_core_strips_prefix_and_prerelease() {
        assert_eq!(
            version_core("v0.1.0-alpha"),
            Some(VersionCore {
                major: 0,
                minor: 1,
                patch: 0
            })
        );
        assert_eq!(
            version_core("0.1.0"),
            Some(VersionCore {
                major: 0,
                minor: 1,
                patch: 0
            })
        );
        assert_eq!(
            version_core("1.2"),
            Some(VersionCore {
                major: 1,
                minor: 2,
                patch: 0
            })
        );
    }

    #[test]
    fn equal_cores_are_not_newer_when_app_omits_suffix() {
        assert_eq!(compare_update("0.1.0", "v0.1.0-alpha"), Some(false));
        assert_eq!(compare_update("0.1.0", "v0.1.0"), Some(false));
    }

    #[test]
    fn later_patch_is_newer() {
        assert_eq!(compare_update("0.1.0", "v0.1.1"), Some(true));
        assert_eq!(compare_update("0.1.0", "v0.2.0-beta"), Some(true));
        assert_eq!(compare_update("0.2.0", "v0.1.9"), Some(false));
    }

    #[test]
    fn allowlist_includes_releases_and_git() {
        assert!(allowed_external_url("https://git-scm.com/downloads"));
        assert!(allowed_external_url(DOWNLOAD_PAGE));
        assert!(allowed_external_url(&format!("{DOWNLOAD_PAGE}/tag/v0.1.0")));
        assert!(!allowed_external_url(
            "https://github.com/daena-archive/daena"
        ));
        assert!(!allowed_external_url("https://example.com"));
    }

    #[test]
    fn pick_latest_skips_drafts() {
        let releases = [
            GithubRelease {
                tag_name: "v0.2.0".into(),
                draft: true,
                html_url: "https://example.com/2".into(),
            },
            GithubRelease {
                tag_name: "v0.1.1".into(),
                draft: false,
                html_url: "https://example.com/1".into(),
            },
        ];
        assert_eq!(pick_latest_release(&releases).unwrap().tag_name, "v0.1.1");
    }
}
