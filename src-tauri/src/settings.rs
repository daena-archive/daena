//! Application-profile settings (`{app_data}/settings.json`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const SETTINGS_FORMAT_VERSION: u32 = 1;
const MAX_RECENT_PROJECTS: usize = 6;
const DEFAULT_IMAGE_ENDPOINT: &str = "http://127.0.0.1:8188";
const DEFAULT_PROVIDER_ADAPTER: &str = "openai-compatible";

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
    #[serde(default)]
    pub appearance: AppearanceSettings,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannelPreference {
    #[default]
    Auto,
    Stable,
    Beta,
    Alpha,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppearanceSettings {
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub update_channel: UpdateChannelPreference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiSettings {
    #[serde(default)]
    pub project_bindings: BTreeMap<String, AiProviderSettings>,
    #[serde(default)]
    pub image_provider: ImageProviderSettings,
    #[serde(default)]
    pub consents: Vec<RemoteConsent>,
}

impl AiSettings {
    pub fn binding(&self, project_id: &str) -> AiProviderSettings {
        self.project_bindings
            .get(project_id)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageProviderSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_image_provider_id")]
    pub id: String,
    #[serde(default = "default_image_provider_name")]
    pub name: String,
    #[serde(default = "default_image_provider_adapter")]
    pub adapter: String,
    #[serde(default = "default_image_endpoint")]
    pub endpoint: String,
    #[serde(default)]
    pub model: String,
}

impl Default for ImageProviderSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            id: default_image_provider_id(),
            name: default_image_provider_name(),
            adapter: default_image_provider_adapter(),
            endpoint: default_image_endpoint(),
            model: String::new(),
        }
    }
}

fn default_image_provider_id() -> String {
    "comfyui-local".into()
}

fn default_image_provider_name() -> String {
    "ComfyUI".into()
}

fn default_image_provider_adapter() -> String {
    "comfyui".into()
}

fn default_image_endpoint() -> String {
    DEFAULT_IMAGE_ENDPOINT.into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiProviderSettings {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_provider_adapter")]
    pub adapter: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub embedding_model: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_provider_adapter() -> String {
    DEFAULT_PROVIDER_ADAPTER.to_string()
}

impl Default for AiProviderSettings {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            adapter: default_provider_adapter(),
            endpoint: String::new(),
            model: String::new(),
            embedding_model: String::new(),
            capabilities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoteConsent {
    pub project_id: String,
    pub provider: String,
    pub endpoint: String,
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
    pub appearance: Option<AppearanceSettingsUpdate>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppearanceSettingsUpdate {
    pub theme: Option<ThemePreference>,
    pub update_channel: Option<UpdateChannelPreference>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiSettingsUpdate {
    pub project_id: Option<String>,
    pub provider: Option<AiProviderSettingsUpdate>,
    pub image_provider: Option<ImageProviderSettingsUpdate>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageProviderSettingsUpdate {
    pub enabled: Option<bool>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub adapter: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiProviderSettingsUpdate {
    pub id: Option<String>,
    pub name: Option<String>,
    pub adapter: Option<String>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
    pub embedding_model: Option<String>,
    pub capabilities: Option<Vec<String>>,
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
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid settings.json: {error}"))?;
        let version = value
            .get("formatVersion")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        if version != u64::from(SETTINGS_FORMAT_VERSION) {
            return Err(format!("unsupported settings format version {version}"));
        }
        let settings: AppSettings = serde_json::from_value(value)
            .map_err(|error| format!("invalid settings.json: {error}"))?;
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
            if let Some(appearance) = general.appearance {
                if let Some(theme) = appearance.theme {
                    settings.general.appearance.theme = theme;
                }
                if let Some(update_channel) = appearance.update_channel {
                    settings.general.appearance.update_channel = update_channel;
                }
            }
        }
        if let Some(ai) = update.ai {
            if let Some(provider) = ai.provider {
                let project_id = ai
                    .project_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| "projectId is required to update an AI provider".to_string())?
                    .to_string();
                let entry = settings.ai.project_bindings.entry(project_id).or_default();
                if let Some(id) = provider.id {
                    entry.id = id;
                }
                if let Some(name) = provider.name {
                    entry.name = name;
                }
                if let Some(adapter) = provider.adapter {
                    entry.adapter = adapter;
                }
                if let Some(endpoint) = provider.endpoint {
                    entry.endpoint = endpoint;
                }
                if let Some(model) = provider.model {
                    entry.model = model;
                }
                if let Some(model) = provider.embedding_model {
                    entry.embedding_model = model;
                }
                if let Some(capabilities) = provider.capabilities {
                    entry.capabilities = capabilities;
                }
            }
            if let Some(provider) = ai.image_provider {
                if let Some(enabled) = provider.enabled {
                    settings.ai.image_provider.enabled = enabled;
                }
                if let Some(id) = provider.id {
                    settings.ai.image_provider.id = id;
                }
                if let Some(name) = provider.name {
                    settings.ai.image_provider.name = name;
                }
                if let Some(adapter) = provider.adapter {
                    settings.ai.image_provider.adapter = adapter;
                }
                if let Some(endpoint) = provider.endpoint {
                    settings.ai.image_provider.endpoint = endpoint;
                }
                if let Some(model) = provider.model {
                    settings.ai.image_provider.model = model;
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
            .consents
            .retain(|consent| !(consent.project_id == project_id && consent.provider == provider));
        if allowed {
            settings.ai.consents.push(RemoteConsent {
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
    settings.ai.image_provider.id = settings.ai.image_provider.id.trim().to_string();
    settings.ai.image_provider.name = settings.ai.image_provider.name.trim().to_string();
    settings.ai.image_provider.adapter = settings.ai.image_provider.adapter.trim().to_string();
    settings.ai.image_provider.endpoint = settings
        .ai
        .image_provider
        .endpoint
        .trim()
        .trim_end_matches('/')
        .to_string();
    settings.ai.image_provider.model = settings.ai.image_provider.model.trim().to_string();
    settings.ai.project_bindings = settings
        .ai
        .project_bindings
        .into_iter()
        .filter(|(project_id, _)| !project_id.trim().is_empty())
        .map(|(project_id, provider)| (project_id, normalize_provider(provider)))
        .collect();
    settings
}

fn normalize_provider(mut provider: AiProviderSettings) -> AiProviderSettings {
    provider.id = provider.id.trim().to_string();
    provider.name = provider.name.trim().to_string();
    provider.adapter = if provider.adapter.trim().is_empty() {
        default_provider_adapter()
    } else {
        provider.adapter.trim().to_string()
    };
    provider.endpoint = provider.endpoint.trim().trim_end_matches('/').to_string();
    provider.model = provider.model.trim().to_string();
    provider.embedding_model = provider.embedding_model.trim().to_string();
    provider.capabilities = provider
        .capabilities
        .into_iter()
        .map(|capability| capability.trim().to_string())
        .filter(|capability| !capability.is_empty())
        .collect();
    provider
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
                appearance: AppearanceSettings {
                    theme: ThemePreference::Dark,
                    ..Default::default()
                },
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
                    appearance: None,
                }),
                ai: None,
            })
            .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.general.recent_projects.len(), 1);
        assert_eq!(loaded.general.recent_projects[0].name, "One");
        assert_eq!(loaded.general.appearance.theme, ThemePreference::System);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn update_merges_theme_without_replacing_recent_projects() {
        let directory =
            std::env::temp_dir().join(format!("daena-theme-settings-{}", uuid::Uuid::new_v4()));
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
                    appearance: None,
                }),
                ai: None,
            })
            .unwrap();
        store
            .update(AppSettingsUpdate {
                general: Some(GeneralSettingsUpdate {
                    recent_projects: None,
                    appearance: Some(AppearanceSettingsUpdate {
                        theme: Some(ThemePreference::Dark),
                        ..Default::default()
                    }),
                }),
                ai: None,
            })
            .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(loaded.general.recent_projects.len(), 1);
        assert_eq!(loaded.general.appearance.theme, ThemePreference::Dark);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn update_persists_update_channel() {
        let directory =
            std::env::temp_dir().join(format!("daena-update-channel-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        store
            .update(AppSettingsUpdate {
                general: Some(GeneralSettingsUpdate {
                    recent_projects: None,
                    appearance: Some(AppearanceSettingsUpdate {
                        theme: None,
                        update_channel: Some(UpdateChannelPreference::Alpha),
                    }),
                }),
                ai: None,
            })
            .unwrap();
        let loaded = store.load().unwrap();
        assert_eq!(
            loaded.general.appearance.update_channel,
            UpdateChannelPreference::Alpha
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn update_merges_active_provider_fields() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        store
            .update(AppSettingsUpdate {
                general: None,
                ai: Some(AiSettingsUpdate {
                    project_id: Some("/project".into()),
                    provider: Some(AiProviderSettingsUpdate {
                        name: Some("Remote Writer".into()),
                        endpoint: Some("https://api.example.com/v1".into()),
                        capabilities: Some(vec!["text.generate".into(), "text.embed".into()]),
                        ..AiProviderSettingsUpdate::default()
                    }),
                    image_provider: None,
                }),
            })
            .unwrap();
        let loaded = store.load().unwrap();
        let provider = loaded.ai.binding("/project");
        assert_eq!(provider.name, "Remote Writer");
        assert_eq!(provider.endpoint, "https://api.example.com/v1");
        assert_eq!(provider.capabilities.len(), 2);
        assert!(provider.model.is_empty());
        assert!(loaded.ai.binding("/other").endpoint.is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn provider_update_requires_project_id() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        let error = store
            .update(AppSettingsUpdate {
                general: None,
                ai: Some(AiSettingsUpdate {
                    project_id: None,
                    provider: Some(AiProviderSettingsUpdate {
                        endpoint: Some("http://127.0.0.1:1234/v1".into()),
                        ..AiProviderSettingsUpdate::default()
                    }),
                    image_provider: None,
                }),
            })
            .unwrap_err();
        assert!(error.contains("projectId"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_file_defaults_have_no_provider() {
        let directory =
            std::env::temp_dir().join(format!("daena-empty-ai-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let loaded = SettingsStore::new(&directory).load().unwrap();
        assert!(loaded.ai.project_bindings.is_empty());
        assert!(loaded.ai.binding("/project").endpoint.is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn update_merges_local_image_provider_fields() {
        let directory =
            std::env::temp_dir().join(format!("daena-image-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        store
            .update(AppSettingsUpdate {
                general: None,
                ai: Some(AiSettingsUpdate {
                    project_id: None,
                    provider: None,
                    image_provider: Some(ImageProviderSettingsUpdate {
                        enabled: Some(true),
                        endpoint: Some("http://127.0.0.1:8188/".into()),
                        model: Some(" world.safetensors ".into()),
                        ..ImageProviderSettingsUpdate::default()
                    }),
                }),
            })
            .unwrap();
        let loaded = store.load().unwrap();
        assert!(loaded.ai.image_provider.enabled);
        assert_eq!(loaded.ai.image_provider.endpoint, "http://127.0.0.1:8188");
        assert_eq!(loaded.ai.image_provider.model, "world.safetensors");
        assert_eq!(loaded.ai.image_provider.adapter, "comfyui");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn non_current_settings_format_is_rejected() {
        let directory =
            std::env::temp_dir().join(format!("daena-v2-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("settings.json"),
            br#"{
  "formatVersion": 2,
  "ai": {
    "provider": {
      "id": "lm-studio",
      "name": "LM Studio",
      "adapter": "openai-compatible",
      "endpoint": "http://127.0.0.1:1234/v1",
      "model": "writer",
      "embeddingModel": "",
      "capabilities": []
    },
    "consents": []
  }
}"#,
        )
        .unwrap();
        let error = SettingsStore::new(&directory).load().unwrap_err();
        assert!(error.contains("unsupported settings format version 2"));
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
    fn legacy_local_remote_shape_is_not_migrated() {
        let directory =
            std::env::temp_dir().join(format!("daena-settings-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(
            &path,
            br#"{"formatVersion":1,"general":{},"ai":{"localEndpoint":"http://127.0.0.1:1234/v1","localModel":"model","remotePolicy":"ask","remote":{}}}"#,
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
        assert_eq!(loaded.ai.consents.len(), 1);
        assert_eq!(loaded.ai.consents[0].endpoint, "https://two.example/v1");
        store
            .set_remote_consent("project", "provider", "https://two.example/v1", false)
            .unwrap();
        assert!(store.load().unwrap().ai.consents.is_empty());
        let _ = fs::remove_dir_all(directory);
    }
}
