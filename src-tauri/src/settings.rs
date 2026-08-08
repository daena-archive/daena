//! Application-profile settings (`{app_data}/settings.json`).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const SETTINGS_FORMAT_VERSION: u32 = 1;
const MAX_RECENT_PROJECTS: usize = 6;
const DEFAULT_AI_ENDPOINT: &str = "http://127.0.0.1:1234/v1";
const DEFAULT_AI_MODEL: &str = "";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecentProject {
    pub name: String,
    pub root: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSettings {
    #[serde(default)]
    pub recent_projects: Vec<RecentProject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiSettings {
    #[serde(default = "default_ai_endpoint")]
    pub local_endpoint: String,
    #[serde(default = "default_ai_model")]
    pub local_model: String,
    #[serde(default)]
    pub remote_policy: AiRemotePolicy,
    #[serde(default)]
    pub remote: RemoteAiSettings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AiRemotePolicy {
    Disabled,
    #[default]
    LocalOnly,
    Ask,
    ApprovedPairs,
    RemoteAllowed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteAiSettings {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub consents: Vec<RemoteConsent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteConsent {
    pub project_id: String,
    pub provider: String,
    pub endpoint: String,
}

fn default_ai_endpoint() -> String {
    DEFAULT_AI_ENDPOINT.to_string()
}
fn default_ai_model() -> String {
    DEFAULT_AI_MODEL.to_string()
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            local_endpoint: default_ai_endpoint(),
            local_model: default_ai_model(),
            remote_policy: AiRemotePolicy::default(),
            remote: RemoteAiSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub format_version: u32,
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub ai: AiSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            format_version: SETTINGS_FORMAT_VERSION,
            general: GeneralSettings::default(),
            ai: AiSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneralSettingsUpdate {
    pub recent_projects: Option<Vec<RecentProject>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiSettingsUpdate {
    pub local_endpoint: Option<String>,
    pub local_model: Option<String>,
    pub remote_policy: Option<AiRemotePolicy>,
    pub remote: Option<RemoteAiSettingsUpdate>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteAiSettingsUpdate {
    pub provider: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettingsUpdate {
    pub general: Option<GeneralSettingsUpdate>,
    pub ai: Option<AiSettingsUpdate>,
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(app_data_dir: impl AsRef<Path>) -> Self {
        Self {
            path: app_data_dir.as_ref().join("settings.json"),
        }
    }

    pub fn load(&self) -> Result<AppSettings, String> {
        if !self.path.is_file() {
            return Ok(AppSettings::default());
        }
        let bytes = fs::read(&self.path).map_err(|error| error.to_string())?;
        let settings: AppSettings = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid settings.json: {error}"))?;
        if settings.format_version != SETTINGS_FORMAT_VERSION {
            return Err(format!(
                "unsupported settings format version {}",
                settings.format_version
            ));
        }
        Ok(normalize(settings))
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        let normalized = normalize(settings.clone());
        if normalized.format_version != SETTINGS_FORMAT_VERSION {
            return Err(format!(
                "unsupported settings format version {}",
                normalized.format_version
            ));
        }
        let mut bytes =
            serde_json::to_vec_pretty(&normalized).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, &bytes).map_err(|error| error.to_string())?;
        fs::rename(&temporary, &self.path).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn update(&self, update: AppSettingsUpdate) -> Result<AppSettings, String> {
        let mut settings = self.load()?;
        if let Some(general) = update.general {
            if let Some(recent_projects) = general.recent_projects {
                settings.general.recent_projects = recent_projects;
            }
        }
        if let Some(ai) = update.ai {
            if let Some(endpoint) = ai.local_endpoint {
                settings.ai.local_endpoint = endpoint;
            }
            if let Some(model) = ai.local_model {
                settings.ai.local_model = model;
            }
            if let Some(remote_policy) = ai.remote_policy {
                settings.ai.remote_policy = remote_policy;
            }
            if let Some(remote) = ai.remote {
                if let Some(provider) = remote.provider {
                    settings.ai.remote.provider = provider;
                }
                if let Some(endpoint) = remote.endpoint {
                    settings.ai.remote.endpoint = endpoint;
                }
                if let Some(model) = remote.model {
                    settings.ai.remote.model = model;
                }
            }
        }
        settings = normalize(settings);
        self.save(&settings)?;
        Ok(settings)
    }

    pub fn set_remote_consent(
        &self,
        project_id: &str,
        provider: &str,
        endpoint: &str,
        allowed: bool,
    ) -> Result<AppSettings, String> {
        let mut settings = self.load()?;
        settings
            .ai
            .remote
            .consents
            .retain(|consent| !(consent.project_id == project_id && consent.provider == provider));
        if allowed {
            settings.ai.remote.consents.push(RemoteConsent {
                project_id: project_id.to_string(),
                provider: provider.to_string(),
                endpoint: endpoint.to_string(),
            });
        }
        self.save(&settings)?;
        Ok(settings)
    }
}

fn normalize(mut settings: AppSettings) -> AppSettings {
    settings.format_version = SETTINGS_FORMAT_VERSION;
    settings.general.recent_projects = settings
        .general
        .recent_projects
        .into_iter()
        .filter(|project| !project.name.trim().is_empty() && !project.root.trim().is_empty())
        .take(MAX_RECENT_PROJECTS)
        .collect();
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_loads_defaults() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        assert_eq!(store.load().unwrap(), AppSettings::default());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn round_trip_preserves_recent_projects() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        let settings = AppSettings {
            format_version: SETTINGS_FORMAT_VERSION,
            general: GeneralSettings {
                recent_projects: vec![RecentProject {
                    name: "Atlas".into(),
                    root: "/tmp/atlas".into(),
                }],
            },
            ai: AiSettings::default(),
        };
        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap(), settings);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn update_merges_recent_projects() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        store
            .update(AppSettingsUpdate {
                general: Some(GeneralSettingsUpdate {
                    recent_projects: Some(vec![RecentProject {
                        name: "One".into(),
                        root: "/one".into(),
                    }]),
                }),
                ai: None,
            })
            .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.general.recent_projects.len(), 1);
        assert_eq!(loaded.general.recent_projects[0].name, "One");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(
            &path,
            b"{\"formatVersion\":1,\"general\":{},\"extra\":true}\n",
        )
        .unwrap();
        let store = SettingsStore::new(&directory);
        assert!(store.load().unwrap_err().contains("invalid settings.json"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn remote_consent_replaces_endpoint_and_revocation_fails_closed() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        store
            .set_remote_consent("project", "provider", "https://one.example/v1", true)
            .unwrap();
        store
            .set_remote_consent("project", "provider", "https://two.example/v1", true)
            .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.ai.remote.consents.len(), 1);
        assert_eq!(
            loaded.ai.remote.consents[0].endpoint,
            "https://two.example/v1"
        );
        store
            .set_remote_consent("project", "provider", "https://two.example/v1", false)
            .unwrap();
        assert!(store.load().unwrap().ai.remote.consents.is_empty());
        let _ = fs::remove_dir_all(directory);
    }
}
