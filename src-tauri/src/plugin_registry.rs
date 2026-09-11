use std::io::Read;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use daena_plugin_host::{
    advertised_digest_matches, review_package, verify_archive_bytes, ArchiveLimits,
    DiscoveryCatalog, TrustSnapshot, VerificationPolicy, CATALOG_FILE, MAX_CATALOG_BYTES,
    MAX_TRUST_SNAPSHOT_BYTES, TRUST_SNAPSHOT_FILE,
};
use reqwest::header::LOCATION;
use reqwest::Url;
use tauri::Manager;

pub const DEFAULT_TRUST_URL: &str = "https://daena-archive.github.io/plugin-registry/trust.json";
pub const DEFAULT_CATALOG_URL: &str =
    "https://daena-archive.github.io/plugin-registry/catalog.json";
const MAX_FETCH_REDIRECTS: usize = 5;

fn plugins_root(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("plugins"))
}

pub(super) fn validate_registry_https_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value.trim()).map_err(|_| "registry URL is not valid".to_string())?;
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err("registry URLs must use HTTPS without embedded credentials".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("registry URLs cannot contain a query or fragment".into());
    }
    reject_local_host(&url)?;
    if !is_allowed_registry_host(url.host_str().unwrap_or_default()) {
        return Err("registry URLs must use GitHub Pages or github.com".into());
    }
    Ok(url)
}

pub(super) fn validate_artifact_url(value: &str) -> Result<Url, String> {
    let url =
        Url::parse(value.trim()).map_err(|_| "plugin artifact URL is not valid".to_string())?;
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err("plugin artifact URLs must use HTTPS without embedded credentials".into());
    }
    if url.fragment().is_some() {
        return Err("plugin artifact URLs cannot contain a fragment".into());
    }
    reject_local_host(&url)?;
    if !is_allowed_artifact_host(url.host_str().unwrap_or_default()) {
        return Err("plugin artifact URLs must use GitHub".into());
    }
    let path = url.path();
    if !path.ends_with(".daenaplugin") {
        return Err("plugin artifact must use the .daenaplugin extension".into());
    }
    Ok(url)
}

fn validate_artifact_redirect_url(url: &Url) -> Result<(), String> {
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err("plugin artifact URLs must use HTTPS without embedded credentials".into());
    }
    if url.fragment().is_some() {
        return Err("plugin artifact URLs cannot contain a fragment".into());
    }
    reject_local_host(url)?;
    if !is_allowed_artifact_host(url.host_str().unwrap_or_default()) {
        return Err("plugin artifact URLs must use GitHub".into());
    }
    Ok(())
}

fn normalized_host(host: &str) -> String {
    host.trim_end_matches('.').to_ascii_lowercase()
}

fn is_allowed_registry_host(host: &str) -> bool {
    let host = normalized_host(host);
    host == "github.com"
        || host == "www.github.com"
        || host == "github.io"
        || host.ends_with(".github.io")
}

fn is_allowed_artifact_host(host: &str) -> bool {
    let host = normalized_host(host);
    is_allowed_registry_host(&host)
        || host == "githubusercontent.com"
        || host.ends_with(".githubusercontent.com")
}

fn reject_local_host(url: &Url) -> Result<(), String> {
    let host = url
        .host_str()
        .ok_or_else(|| "registry URL has no host".to_string())?;
    let host_for_ip = host.trim_matches(['[', ']']);
    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.eq_ignore_ascii_case("localhost.localdomain")
    {
        return Err("registry URLs cannot target localhost".into());
    }
    if host_for_ip
        .parse::<IpAddr>()
        .is_ok_and(is_private_or_local_registry_ip)
    {
        return Err("registry URLs cannot target private or local addresses".into());
    }
    Ok(())
}

fn is_private_or_local_registry_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_multicast()
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_private_or_local_registry_ip(IpAddr::V4(mapped));
            }
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn fetch_https(
    url: &Url,
    max_bytes: u64,
    validate_redirect: impl Fn(&Url) -> Result<(), String>,
) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "could not download registry resource".to_string())?;
    let mut current = url.clone();
    for _ in 0..=MAX_FETCH_REDIRECTS {
        let response = client
            .get(current.clone())
            .header("User-Agent", "Daena-Archive")
            .header("Accept", "application/json, application/octet-stream")
            .send()
            .map_err(|_| "could not download registry resource".to_string())?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| "could not download registry resource".to_string())?;
            current = current
                .join(location)
                .map_err(|_| "could not download registry resource".to_string())?;
            validate_redirect(&current)?;
            continue;
        }
        if !response.status().is_success() {
            return Err("could not download registry resource".into());
        }
        let mut bytes = Vec::new();
        response
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| "could not download registry resource".to_string())?;
        if bytes.len() as u64 > max_bytes {
            return Err("registry resource exceeds size limit".into());
        }
        return Ok(bytes);
    }
    Err("could not download registry resource".into())
}

fn refresh_registry(install_root: &Path) -> Result<serde_json::Value, String> {
    let trust_url = validate_registry_https_url(DEFAULT_TRUST_URL)?;
    let catalog_url = validate_registry_https_url(DEFAULT_CATALOG_URL)?;
    let trust_bytes = fetch_https(&trust_url, MAX_TRUST_SNAPSHOT_BYTES, |redirect| {
        validate_registry_https_url(redirect.as_str()).map(|_| ())
    })?;
    let snapshot = TrustSnapshot::from_bytes(&trust_bytes).map_err(|error| error.to_string())?;
    snapshot
        .store(install_root.join(TRUST_SNAPSHOT_FILE))
        .map_err(|error| error.to_string())?;
    match fetch_https(&catalog_url, MAX_CATALOG_BYTES, |redirect| {
        validate_registry_https_url(redirect.as_str()).map(|_| ())
    }) {
        Ok(catalog_bytes) => {
            let catalog =
                DiscoveryCatalog::from_bytes(&catalog_bytes).map_err(|error| error.to_string())?;
            catalog
                .store(install_root.join(CATALOG_FILE))
                .map_err(|error| error.to_string())?;
            serde_json::to_value(serde_json::json!({
                "ok": true,
                "catalog": catalog,
                "trustLoaded": true,
                "error": null,
            }))
            .map_err(|error| error.to_string())
        }
        Err(error) => {
            let catalog = DiscoveryCatalog::load_from_install_root(install_root)
                .map_err(|load_error| load_error.to_string())?;
            serde_json::to_value(serde_json::json!({
                "ok": true,
                "catalog": catalog,
                "trustLoaded": true,
                "error": error,
            }))
            .map_err(|error| error.to_string())
        }
    }
}

#[tauri::command]
pub(super) async fn plugin_refresh_registry(
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    let install_root = plugins_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || refresh_registry(&install_root))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(super) fn plugin_registry_catalog(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let install_root = plugins_root(&app)?;
    let catalog = DiscoveryCatalog::load_from_install_root(&install_root)
        .map_err(|error| error.to_string())?;
    let trust_loaded = install_root.join(TRUST_SNAPSHOT_FILE).is_file();
    serde_json::to_value(serde_json::json!({
        "catalog": catalog,
        "trustLoaded": trust_loaded,
        "error": null,
    }))
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub(super) fn plugin_review_package(
    app: tauri::AppHandle,
    archive: String,
) -> Result<serde_json::Value, String> {
    let install_root = plugins_root(&app)?;
    let archive_path = std::path::Path::new(&archive);
    if archive_path.extension().and_then(|value| value.to_str()) != Some("daenaplugin") {
        return Err(
            "package must use the .daenaplugin extension; .wbplugin is not accepted".into(),
        );
    }
    let policy = VerificationPolicy::from_install_root(&install_root, true)
        .map_err(|error| error.to_string())?;
    let limits = ArchiveLimits::default();
    let compressed = std::fs::metadata(archive_path)
        .map_err(|error| error.to_string())?
        .len();
    if compressed > limits.max_compressed_bytes {
        return Err("compressed package exceeds size limit".into());
    }
    let bytes = std::fs::read(archive_path).map_err(|error| error.to_string())?;
    let verified =
        verify_archive_bytes(&bytes, limits, &policy).map_err(|error| error.to_string())?;
    let review = review_package(&verified, &policy.trust);
    serde_json::to_value(review).map_err(|error| error.to_string())
}

#[tauri::command]
pub(super) async fn plugin_prepare_catalog_package(
    app: tauri::AppHandle,
    artifact_url: String,
) -> Result<serde_json::Value, String> {
    let install_root = plugins_root(&app)?;
    let catalog = DiscoveryCatalog::load_from_install_root(&install_root)
        .map_err(|error| error.to_string())?;
    let listed = catalog
        .plugins
        .iter()
        .find(|plugin| plugin.artifact_url == artifact_url)
        .cloned()
        .ok_or_else(|| "plugin artifact is not listed in the local catalog".to_string())?;
    let url = validate_artifact_url(&listed.artifact_url)?;
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = fetch_https(
            &url,
            ArchiveLimits::default().max_compressed_bytes,
            validate_artifact_redirect_url,
        )?;
        let policy = VerificationPolicy::from_install_root(&install_root, true)
            .map_err(|error| error.to_string())?;
        let verified = verify_archive_bytes(&bytes, ArchiveLimits::default(), &policy)
            .map_err(|error| error.to_string())?;
        advertised_digest_matches(listed.digest.as_deref(), &verified.digest)
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&install_root).map_err(|error| error.to_string())?;
        let archive = install_root.join(format!(".download-{}.daenaplugin", uuid::Uuid::new_v4()));
        std::fs::write(&archive, &bytes).map_err(|error| error.to_string())?;
        let review = review_package(&verified, &policy.trust);
        serde_json::to_value(serde_json::json!({
            "archive": archive,
            "review": review,
        }))
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(super) fn plugin_discard_prepared_package(
    app: tauri::AppHandle,
    archive: String,
) -> Result<(), String> {
    let install_root = plugins_root(&app)?;
    cleanup_registry_download(&install_root, Path::new(&archive));
    Ok(())
}

pub(super) fn cleanup_registry_download(install_root: &Path, archive: &Path) {
    let Some(name) = archive.file_name().and_then(|value| value.to_str()) else {
        return;
    };
    if name.starts_with(".download-") && archive.starts_with(install_root) {
        let _ = std::fs::remove_file(archive);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_urls_must_be_https_public_hosts() {
        validate_registry_https_url(DEFAULT_TRUST_URL).unwrap();
        validate_artifact_url(
            "https://github.com/example/daena-plugins/releases/download/v1/app.daenaplugin",
        )
        .unwrap();
        assert!(validate_registry_https_url("http://example.test/trust.json").is_err());
        assert!(validate_registry_https_url("https://127.0.0.1/trust.json").is_err());
        assert!(validate_artifact_url("https://example.test/app.daenaplugin").is_err());
        assert!(validate_artifact_url(
            "https://github.com/example/daena-plugins/releases/download/v1/app.zip"
        )
        .is_err());
        assert!(validate_artifact_url(
            "https://user:pass@github.com/example/daena-plugins/releases/download/v1/app.daenaplugin"
        )
        .is_err());
        assert!(validate_registry_https_url("https://[::1]/trust.json").is_err());
        assert!(validate_registry_https_url("https://[fc00::1]/trust.json").is_err());
        validate_artifact_redirect_url(
            &Url::parse(
                "https://release-assets.githubusercontent.com/github-production-release-asset/app",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(validate_artifact_redirect_url(
            &Url::parse("https://example.test/app.daenaplugin").unwrap()
        )
        .is_err());
    }

    #[test]
    fn cleanup_only_removes_host_download_files() {
        let root =
            std::env::temp_dir().join(format!("daena-registry-cleanup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let download = root.join(".download-abc.daenaplugin");
        let keep = root.join("keep.daenaplugin");
        std::fs::write(&download, b"x").unwrap();
        std::fs::write(&keep, b"y").unwrap();
        cleanup_registry_download(&root, &download);
        cleanup_registry_download(&root, &keep);
        assert!(!download.exists());
        assert!(keep.exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
