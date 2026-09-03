// AI provider resolution and credentials.
use super::*;

pub(super) struct LocalEndpoint {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) base_path: String,
}

pub(super) fn parse_loopback_endpoint(endpoint: &str) -> Result<LocalEndpoint, String> {
    let raw = endpoint
        .trim()
        .strip_prefix("http://")
        .ok_or_else(|| "Local providers require a loopback HTTP endpoint".to_string())?;
    let authority = raw.split('/').next().unwrap_or_default();
    let base_path = raw
        .strip_prefix(authority)
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_string();
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let end = rest
            .find(']')
            .ok_or_else(|| "Local AI endpoint has an invalid IPv6 host".to_string())?;
        let host = &rest[..end];
        let port = rest[end + 1..].strip_prefix(':').unwrap_or("1234");
        (host, port)
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        if port.chars().all(|character| character.is_ascii_digit()) {
            (host, port)
        } else {
            (authority, "1234")
        }
    } else {
        (authority, "1234")
    };
    let ip_is_local = host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback());
    if host != "localhost" && host != "localhost.localdomain" && !ip_is_local {
        return Err("Local providers require a loopback endpoint".to_string());
    }
    let port = port
        .parse::<u16>()
        .map_err(|_| "Local AI endpoint has an invalid port".to_string())?;
    Ok(LocalEndpoint {
        host: host.to_string(),
        port,
        base_path,
    })
}

pub(super) fn endpoint_is_remote(endpoint: &str) -> Result<bool, String> {
    if parse_loopback_endpoint(endpoint).is_ok() {
        Ok(false)
    } else {
        validate_remote_endpoint(endpoint).map(|_| true)
    }
}

/// Validate a remote origin before any credential-bearing request is created.
/// Redirects are disabled on the client as well, so the approved HTTPS origin
/// remains the origin that receives the request.
pub fn validate_remote_endpoint(endpoint: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(endpoint.trim())
        .map_err(|_| "Remote AI endpoint is not a valid URL".to_string())?;
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err("Remote AI endpoints must use HTTPS without embedded credentials".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("Remote AI endpoints cannot contain a query or fragment".into());
    }
    let host = url
        .host_str()
        .ok_or_else(|| "Remote AI endpoint has no host".to_string())?;
    let host_for_ip = host.trim_matches(['[', ']']);
    if host.eq_ignore_ascii_case("localhost")
        || host.ends_with(".localhost")
        || host.eq_ignore_ascii_case("localhost.localdomain")
    {
        return Err("Remote AI endpoints cannot target localhost".into());
    }
    if host_for_ip
        .parse::<IpAddr>()
        .is_ok_and(is_private_or_local_ip)
    {
        return Err("Remote AI endpoints cannot target private or local addresses".into());
    }
    Ok(url)
}

pub(super) fn is_private_or_local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_multicast()
                || octets[0] == 100 && (64..=127).contains(&octets[1])
                || octets[0] == 192 && octets[1] == 0 && octets[2] == 0
                || octets[0] == 198 && (18..=19).contains(&octets[1])
                || octets[0] == 203 && octets[1] == 0 && octets[2] == 113
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return is_private_or_local_ip(IpAddr::V4(mapped));
            }
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (ip.segments()[0] & 0xfe00) == 0xfc00
                || (ip.segments()[0] & 0xffc0) == 0xfe80
                || (ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8)
        }
    }
}

pub(super) fn resolve_remote_destination(
    url: &reqwest::Url,
) -> Result<(String, SocketAddr), AiError> {
    let host = url
        .host_str()
        .ok_or(AiError::InvalidProviderResponse)?
        .trim_matches(['[', ']'])
        .to_string();
    let port = url
        .port_or_known_default()
        .ok_or(AiError::InvalidProviderResponse)?;
    let mut addresses = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| AiError::ProviderUnavailable)?;
    let address = addresses
        .find(|address| !is_private_or_local_ip(address.ip()))
        .ok_or(AiError::RemoteContextDenied)?;
    Ok((host, address))
}

pub(super) fn remote_secret_service(provider: &str) -> String {
    format!("com.daena.ai.remote.{}", provider.trim())
}

pub(super) fn read_remote_api_key(provider: &str) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(&remote_secret_service(provider), "daena")
        .map_err(|error| format!("OS secret storage unavailable: {error}"))?;
    match entry.get_password() {
        Ok(value) if !value.trim().is_empty() => Ok(Some(value)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("OS secret storage unavailable: {error}")),
    }
}

pub(super) fn import_remote_api_key(provider: &str, api_key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(&remote_secret_service(provider), "daena")
        .map_err(|error| format!("OS secret storage unavailable: {error}"))?;
    entry
        .set_password(api_key)
        .map_err(|error| format!("OS secret storage rejected the credential: {error}"))
}

/// Returns true when an entry was removed, false when none existed.
pub(super) fn delete_remote_api_key(provider: &str) -> Result<bool, String> {
    let entry = keyring::Entry::new(&remote_secret_service(provider), "daena")
        .map_err(|error| format!("OS secret storage unavailable: {error}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(error) => Err(format!("OS secret storage unavailable: {error}")),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCredentialStatus {
    pub provider: String,
    pub configured: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedAiProvider {
    pub provider_id: String,
    pub endpoint: String,
    pub model: String,
    pub embedding_model: String,
    pub remote: bool,
    pub api_key: Option<String>,
    pub embedding_available: bool,
    pub capability_identity: String,
}

impl ResolvedAiProvider {
    pub(super) fn embedding_model_or_model(&self) -> String {
        if self.embedding_model.is_empty() {
            self.model.clone()
        } else {
            self.embedding_model.clone()
        }
    }
}

pub(super) fn capability_identity(capabilities: &[String]) -> String {
    let mut capabilities = capabilities
        .iter()
        .map(|capability| capability.trim())
        .filter(|capability| !capability.is_empty())
        .collect::<Vec<_>>();
    capabilities.sort_unstable();
    capabilities.dedup();
    capabilities.join(",")
}

pub fn resolve_ai_provider(
    settings: &AppSettings,
    project_id: Option<&str>,
    include_project_context: bool,
) -> Result<ResolvedAiProvider, String> {
    resolve_ai_provider_with_credential(settings, project_id, include_project_context, true)
}

pub(super) fn resolve_ai_provider_with_credential(
    settings: &AppSettings,
    project_id: Option<&str>,
    include_project_context: bool,
    require_credential: bool,
) -> Result<ResolvedAiProvider, String> {
    let provider = &settings.ai.provider;
    let endpoint = provider.endpoint.trim().to_string();
    let model = provider.model.trim().to_string();
    if endpoint.is_empty() {
        return Err("Configure an AI provider endpoint first".into());
    }
    let remote = endpoint_is_remote(&endpoint)?;
    if remote {
        if model.is_empty() {
            return Err("Configure an AI provider model first".into());
        }
        validate_remote_endpoint(&endpoint)?;
        if include_project_context {
            let project_id = project_id.ok_or_else(|| AiError::RemoteContextDenied.to_string())?;
            if !remote_consent_matches(settings, project_id, &provider.id, &endpoint) {
                return Err(AiError::RemoteContextDenied.to_string());
            }
        }
        let api_key = read_remote_api_key(&provider.id)?;
        if require_credential && api_key.is_none() {
            return Err(AiError::AuthenticationFailed.to_string());
        }
        Ok(ResolvedAiProvider {
            provider_id: provider.id.clone(),
            endpoint,
            model,
            embedding_model: provider.embedding_model.trim().to_string(),
            remote,
            api_key,
            embedding_available: provider
                .capabilities
                .iter()
                .any(|capability| capability == "text.embed"),
            capability_identity: capability_identity(&provider.capabilities),
        })
    } else {
        parse_loopback_endpoint(&endpoint)?;
        Ok(ResolvedAiProvider {
            provider_id: provider.id.clone(),
            endpoint,
            model,
            embedding_model: provider.embedding_model.trim().to_string(),
            remote,
            api_key: None,
            embedding_available: provider
                .capabilities
                .iter()
                .any(|capability| capability == "text.embed"),
            capability_identity: capability_identity(&provider.capabilities),
        })
    }
}

#[tauri::command]
pub fn ai_provider_credential_status(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
) -> Result<RemoteCredentialStatus, String> {
    let provider = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?
        .ai
        .provider;
    Ok(RemoteCredentialStatus {
        configured: endpoint_is_remote(&provider.endpoint)?
            && read_remote_api_key(&provider.id)?.is_some(),
        provider: provider.id,
    })
}

/// Imports a key from the process environment into OS-backed storage. The key
/// is intentionally not a command argument, so it never crosses the frontend
/// or plugin bridge. Launch the app with `DAENA_REMOTE_API_KEY` set once, then
/// remove it from the environment.
#[tauri::command]
pub fn ai_provider_import_credential(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
) -> Result<RemoteCredentialStatus, String> {
    let provider = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?
        .ai
        .provider;
    if !endpoint_is_remote(&provider.endpoint)? {
        return Err("The active provider does not require a remote credential".into());
    }
    if read_remote_api_key(&provider.id)?.is_some() {
        return Ok(RemoteCredentialStatus {
            provider: provider.id,
            configured: true,
        });
    }
    let key = std::env::var("DAENA_REMOTE_API_KEY")
        .map_err(|_| "DAENA_REMOTE_API_KEY is not set for this import".to_string())?;
    if key.trim().is_empty() {
        return Err("DAENA_REMOTE_API_KEY is empty".into());
    }
    import_remote_api_key(&provider.id, key.trim())?;
    Ok(RemoteCredentialStatus {
        provider: provider.id,
        configured: true,
    })
}

/// Stores a user-provided credential in OS-backed storage. The value crosses the
/// IPC bridge once on its way in and is never returned to the frontend; only the
/// boolean status is readable afterwards.
#[tauri::command]
pub fn ai_provider_set_credential(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    api_key: String,
) -> Result<RemoteCredentialStatus, String> {
    let provider = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?
        .ai
        .provider;
    if !endpoint_is_remote(&provider.endpoint)? {
        return Err("The active provider does not require a remote credential".into());
    }
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Provide an API key before saving".into());
    }
    if key.len() > 4096 {
        return Err("The API key is too long".into());
    }
    import_remote_api_key(&provider.id, key)?;
    Ok(RemoteCredentialStatus {
        provider: provider.id,
        configured: true,
    })
}

#[tauri::command]
pub fn ai_provider_clear_credential(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
) -> Result<RemoteCredentialStatus, String> {
    let provider = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?
        .load()?
        .ai
        .provider;
    // Clearing is intentionally allowed even when the active endpoint is local,
    // so a stale key for this provider can always be removed.
    delete_remote_api_key(&provider.id)?;
    Ok(RemoteCredentialStatus {
        provider: provider.id,
        configured: false,
    })
}

#[tauri::command]
pub fn ai_remote_set_consent(
    settings: State<'_, Arc<Mutex<SettingsStore>>>,
    project_id: String,
    allowed: bool,
) -> Result<(), String> {
    let store = settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?;
    let active_provider = store.load()?.ai.provider;
    if !endpoint_is_remote(&active_provider.endpoint)? {
        return Err("The active provider is local; remote consent is not applicable".into());
    }
    validate_remote_endpoint(&active_provider.endpoint)?;
    store
        .set_remote_consent(
            &project_id,
            &active_provider.id,
            &active_provider.endpoint,
            allowed,
        )
        .map(|_| ())
}

pub(super) fn remote_consent_matches(
    settings: &crate::settings::AppSettings,
    project_id: &str,
    provider: &str,
    endpoint: &str,
) -> bool {
    settings.ai.consents.iter().any(|consent| {
        consent.project_id == project_id
            && consent.provider == provider
            && consent.endpoint == endpoint
    })
}
