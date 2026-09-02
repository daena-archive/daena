use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

pub const DOWNLOAD_PAGE: &str = "https://github.com/daena-archive/daena/releases";
const RELEASES_URL: &str = "https://api.github.com/repos/daena-archive/daena/releases?per_page=100";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Alpha,
}

impl ReleaseChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Alpha => "alpha",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateCheck {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub html_url: String,
    pub release_channel: String,
    pub latest_prerelease: bool,
    pub update_channel_preference: String,
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

pub fn parse_version(raw: &str) -> Option<Version> {
    let trimmed = crate::version::normalize_override(raw)?;
    Version::parse(trimmed).ok()
}

pub fn release_channel(version: &Version) -> ReleaseChannel {
    if version.pre.is_empty() {
        return ReleaseChannel::Stable;
    }
    let pre = version.pre.as_str();
    if pre.starts_with("alpha") {
        ReleaseChannel::Alpha
    } else {
        ReleaseChannel::Beta
    }
}

pub fn eligible_for_channel(version: &Version, channel: ReleaseChannel) -> bool {
    if version.pre.is_empty() {
        return true;
    }
    let pre = version.pre.as_str();
    match channel {
        ReleaseChannel::Stable => false,
        ReleaseChannel::Beta => pre.starts_with("beta"),
        ReleaseChannel::Alpha => true,
    }
}

pub fn effective_channel(current: &Version, releases: &[GithubRelease]) -> ReleaseChannel {
    let channel = release_channel(current);
    if pick_best_release(releases, channel).is_some() {
        return channel;
    }
    if channel == ReleaseChannel::Stable {
        if pick_best_release(releases, ReleaseChannel::Beta).is_some() {
            return ReleaseChannel::Beta;
        }
        if pick_best_release(releases, ReleaseChannel::Alpha).is_some() {
            return ReleaseChannel::Alpha;
        }
    }
    if channel == ReleaseChannel::Beta
        && pick_best_release(releases, ReleaseChannel::Alpha).is_some()
    {
        return ReleaseChannel::Alpha;
    }
    channel
}

pub fn pick_best_release(
    releases: &[GithubRelease],
    channel: ReleaseChannel,
) -> Option<(&GithubRelease, Version)> {
    releases
        .iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = parse_version(&release.tag_name)?;
            if !eligible_for_channel(&version, channel) {
                return None;
            }
            Some((release, version))
        })
        .max_by(|(_, left), (_, right)| left.cmp(right))
}

pub fn compare_update(current: &Version, latest: &Version) -> bool {
    latest > current
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannelPreference {
    Auto,
    Stable,
    Beta,
    Alpha,
}

impl UpdateChannelPreference {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "stable" => Self::Stable,
            "beta" => Self::Beta,
            "alpha" => Self::Alpha,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Alpha => "alpha",
        }
    }

    pub fn to_release_channel(self) -> ReleaseChannel {
        match self {
            Self::Auto => ReleaseChannel::Stable,
            Self::Stable => ReleaseChannel::Stable,
            Self::Beta => ReleaseChannel::Beta,
            Self::Alpha => ReleaseChannel::Alpha,
        }
    }
}

pub fn resolve_channel(
    current: &Version,
    releases: &[GithubRelease],
    preference: UpdateChannelPreference,
) -> ReleaseChannel {
    match preference {
        UpdateChannelPreference::Auto => effective_channel(current, releases),
        other => other.to_release_channel(),
    }
}

pub fn compare_current_for_update(current: &Version, channel: ReleaseChannel) -> Version {
    if current.pre.is_empty() && channel != ReleaseChannel::Stable {
        Version::parse(&format!("{}-alpha.0", current)).unwrap_or_else(|_| current.clone())
    } else {
        current.clone()
    }
}

pub fn channel_empty_message(channel: ReleaseChannel) -> &'static str {
    match channel {
        ReleaseChannel::Stable => {
            "No stable releases published yet. Try Beta or Alpha in Settings → About."
        }
        ReleaseChannel::Beta => "No beta releases published yet. Try Alpha in Settings → About.",
        ReleaseChannel::Alpha => "Could not read the latest version.",
    }
}

pub fn check_update(current: &str, channel_preference: &str) -> Result<AppUpdateCheck, String> {
    let current_version =
        parse_version(current).ok_or_else(|| "Current version is invalid.".to_string())?;
    let preference = UpdateChannelPreference::parse(channel_preference);
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
    if releases.iter().all(|release| release.draft) {
        return Err("No releases published yet.".into());
    }
    let channel = resolve_channel(&current_version, &releases, preference);
    let current_for_compare = compare_current_for_update(&current_version, channel);
    let (release, latest_version) = pick_best_release(&releases, channel)
        .ok_or_else(|| channel_empty_message(channel).to_string())?;
    let newer = compare_update(&current_for_compare, &latest_version);
    Ok(AppUpdateCheck {
        current: current.to_string(),
        latest: release.tag_name.clone(),
        newer,
        html_url: release.html_url.clone(),
        release_channel: channel.as_str().to_string(),
        latest_prerelease: !latest_version.pre.is_empty(),
        update_channel_preference: preference.as_str().to_string(),
    })
}

#[tauri::command]
pub async fn app_check_update(
    channel_preference: Option<String>,
) -> Result<AppUpdateCheck, String> {
    let current = crate::version::current().to_string();
    let preference = channel_preference.unwrap_or_else(|| "auto".to_string());
    tauri::async_runtime::spawn_blocking(move || check_update(&current, &preference))
        .await
        .map_err(|_| "Could not check for updates.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, draft: bool) -> GithubRelease {
        GithubRelease {
            tag_name: tag.into(),
            draft,
            html_url: format!("https://example.com/{tag}"),
        }
    }

    #[test]
    fn parse_version_accepts_prefix_and_prerelease() {
        assert_eq!(
            parse_version("v0.1.0-alpha.1").map(|v| v.to_string()),
            Some("0.1.0-alpha.1".into())
        );
        assert_eq!(
            parse_version("0.2.0-beta.3").map(|v| v.to_string()),
            Some("0.2.0-beta.3".into())
        );
        assert!(parse_version("0.1").is_none());
    }

    #[test]
    fn release_channel_detects_prerelease_kind() {
        assert_eq!(
            release_channel(&parse_version("0.1.0").unwrap()),
            ReleaseChannel::Stable
        );
        assert_eq!(
            release_channel(&parse_version("0.1.0-beta.1").unwrap()),
            ReleaseChannel::Beta
        );
        assert_eq!(
            release_channel(&parse_version("0.1.0-alpha.2").unwrap()),
            ReleaseChannel::Alpha
        );
    }

    #[test]
    fn beta_prerelease_updates_within_same_core() {
        let current = parse_version("0.1.0-beta.1").unwrap();
        let latest = parse_version("0.1.0-beta.2").unwrap();
        assert!(compare_update(&current, &latest));
        assert!(!compare_update(&latest, &current));
    }

    #[test]
    fn stable_users_ignore_beta_tags_even_when_semver_is_higher() {
        let releases = [
            release("v0.3.0-beta.1", false),
            release("v0.2.1", false),
            release("v0.2.0", false),
        ];
        let picked = pick_best_release(&releases, ReleaseChannel::Stable).unwrap();
        assert_eq!(picked.1.to_string(), "0.2.1");
    }

    #[test]
    fn beta_users_get_beta_and_stable_but_not_alpha() {
        let releases = [
            release("v0.3.0-alpha.1", false),
            release("v0.2.0-beta.2", false),
            release("v0.2.0", false),
        ];
        let current = parse_version("0.2.0-beta.1").unwrap();
        let picked = pick_best_release(&releases, ReleaseChannel::Beta).unwrap();
        assert!(compare_update(&current, &picked.1));
        assert_eq!(picked.0.tag_name, "v0.2.0");

        let only_alpha = [release("v0.3.0-alpha.1", false)];
        assert!(pick_best_release(&only_alpha, ReleaseChannel::Beta).is_none());

        let beta_only = [
            release("v0.2.0-beta.1", false),
            release("v0.2.0-beta.2", false),
        ];
        let picked_beta = pick_best_release(&beta_only, ReleaseChannel::Beta).unwrap();
        assert_eq!(picked_beta.0.tag_name, "v0.2.0-beta.2");
    }

    #[test]
    fn alpha_users_consider_alpha_and_beta_prereleases() {
        let releases = [
            release("v0.2.0-beta.1", false),
            release("v0.2.0-alpha.3", false),
        ];
        let picked = pick_best_release(&releases, ReleaseChannel::Alpha).unwrap();
        assert_eq!(picked.0.tag_name, "v0.2.0-beta.1");
    }

    #[test]
    fn matching_prerelease_is_not_newer() {
        let current = parse_version("0.1.0-alpha.2").unwrap();
        let latest = parse_version("v0.1.0-alpha.2").unwrap();
        let current_for_compare = compare_current_for_update(&current, ReleaseChannel::Alpha);
        assert_eq!(current_for_compare, current);
        assert!(!compare_update(&current_for_compare, &latest));
    }

    #[test]
    fn equal_versions_are_not_newer() {
        let current = parse_version("0.1.0").unwrap();
        let latest = parse_version("v0.1.0").unwrap();
        assert!(!compare_update(&current, &latest));
    }

    #[test]
    fn later_patch_is_newer() {
        assert!(compare_update(
            &parse_version("0.1.0").unwrap(),
            &parse_version("v0.1.1").unwrap()
        ));
        assert!(!compare_update(
            &parse_version("0.2.0").unwrap(),
            &parse_version("v0.1.9").unwrap()
        ));
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
    fn pick_best_skips_drafts() {
        let releases = [release("v0.2.0", true), release("v0.1.1", false)];
        assert_eq!(
            pick_best_release(&releases, ReleaseChannel::Stable)
                .unwrap()
                .0
                .tag_name,
            "v0.1.1"
        );
    }

    #[test]
    fn dev_stable_version_uses_alpha_releases_when_no_stable_tags_exist() {
        let releases = [
            release("v0.1.0-alpha.2", false),
            release("v0.1.0-alpha.1", false),
        ];
        let current = parse_version("0.1.0").unwrap();
        let channel = effective_channel(&current, &releases);
        assert_eq!(channel, ReleaseChannel::Alpha);
        let picked = pick_best_release(&releases, channel).unwrap();
        assert_eq!(picked.0.tag_name, "v0.1.0-alpha.2");
        let current_for_compare = compare_current_for_update(&current, channel);
        assert!(compare_update(&current_for_compare, &picked.1));
    }

    #[test]
    fn explicit_stable_channel_does_not_fall_back_to_alpha() {
        let releases = [release("v0.1.0-alpha.2", false)];
        let current = parse_version("0.1.0").unwrap();
        let channel = resolve_channel(&current, &releases, UpdateChannelPreference::Stable);
        assert_eq!(channel, ReleaseChannel::Stable);
        assert!(pick_best_release(&releases, channel).is_none());
    }

    #[test]
    fn explicit_alpha_channel_finds_alpha_release() {
        let releases = [release("v0.1.0-alpha.2", false)];
        let current = parse_version("0.1.0").unwrap();
        let channel = resolve_channel(&current, &releases, UpdateChannelPreference::Alpha);
        let picked = pick_best_release(&releases, channel).unwrap();
        assert_eq!(picked.0.tag_name, "v0.1.0-alpha.2");
    }
}
