//! Versioned, runtime-independent contracts shared by the plugin host and SDK.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MANIFEST_VERSION: u32 = 1;
pub const RPC_VERSION: u32 = 1;

pub const KNOWN_CAPABILITIES: &[&str] = &[
    "entity.read",
    "entity.write",
    "entity.delete",
    "document.read",
    "document.write",
    "field.read:self",
    "field.read:shared",
    "field.write:self",
    "relationship.read",
    "relationship.write",
    "asset.read:self",
    "asset.import",
    "search.query",
    "event.publish:<type>",
    "event.subscribe:<type>",
    "service.provide:<name>",
    "service.call:<name>",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractError(pub String);

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ContractError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Declarative,
    Sandboxed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entrypoints {
    pub ui: Option<String>,
    pub wasm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub version: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FieldDefinition {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: Option<bool>,
    pub options: Option<Vec<String>>,
    #[serde(rename = "entityTypes")]
    pub entity_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SchemaContribution {
    pub namespace: String,
    #[serde(rename = "entityTypes")]
    pub entity_types: Vec<String>,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EntityTemplate {
    pub id: String,
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub fields: serde_json::Value,
    #[serde(rename = "requiredFields")]
    pub required_fields: Option<Vec<String>>,
    pub document: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MigrationOperation {
    CreateNamespace {
        namespace: String,
    },
    AddField {
        namespace: String,
        field: FieldDefinition,
    },
    RenameField {
        namespace: String,
        from: String,
        to: String,
    },
    DropField {
        namespace: String,
        key: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Migration {
    pub id: String,
    pub from: u32,
    pub to: u32,
    pub recovery: String,
    pub operations: Vec<MigrationOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub name: String,
    pub major: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub name: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Services {
    pub provides: Vec<Service>,
    pub consumes: Vec<Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Events {
    pub publishes: Vec<Event>,
    pub subscribes: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    #[serde(rename = "manifestVersion")]
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    #[serde(rename = "hostApi")]
    pub host_api: String,
    pub kind: PluginKind,
    pub entrypoints: Entrypoints,
    pub capabilities: Vec<String>,
    pub dependencies: BTreeMap<String, Dependency>,
    pub namespaces: Vec<String>,
    pub schemas: Vec<SchemaContribution>,
    pub templates: Vec<EntityTemplate>,
    pub views: Vec<View>,
    pub commands: Vec<Command>,
    pub services: Services,
    pub events: Events,
    pub migrations: Vec<Migration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RpcRequest {
    #[serde(rename = "rpcVersion")]
    pub rpc_version: u32,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub method: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RpcError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RpcResponse {
    #[serde(rename = "rpcVersion")]
    pub rpc_version: u32,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub ok: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcError>,
}

pub fn parse_manifest(json: &str) -> Result<PluginManifest, ContractError> {
    let manifest: PluginManifest =
        serde_json::from_str(json).map_err(|e| ContractError(format!("invalid manifest: {e}")))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_manifest(manifest: &PluginManifest) -> Result<(), ContractError> {
    if manifest.manifest_version != MANIFEST_VERSION {
        return Err(ContractError("unsupported manifest version".into()));
    }
    for (label, value) in [("id", &manifest.id), ("publisher", &manifest.publisher)] {
        if !is_identifier(value) {
            return Err(ContractError(format!("invalid {label}: {value}")));
        }
    }
    if manifest.name.trim().is_empty() || !is_semver(&manifest.version) {
        return Err(ContractError("invalid name or semantic version".into()));
    }
    if manifest.host_api.trim().is_empty() {
        return Err(ContractError("hostApi must not be empty".into()));
    }
    if manifest.entrypoints.ui.is_none() && manifest.entrypoints.wasm.is_none() {
        return Err(ContractError("at least one entrypoint is required".into()));
    }
    for path in [
        manifest.entrypoints.ui.as_ref(),
        manifest.entrypoints.wasm.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if !is_package_path(path) {
            return Err(ContractError(format!("invalid package path: {path}")));
        }
    }
    for capability in &manifest.capabilities {
        if !KNOWN_CAPABILITIES.contains(&capability.as_str())
            && !capability.starts_with("event.publish:")
            && !capability.starts_with("event.subscribe:")
            && !capability.starts_with("service.provide:")
            && !capability.starts_with("service.call:")
        {
            return Err(ContractError(format!("unknown capability: {capability}")));
        }
    }
    let namespaces: BTreeSet<_> = manifest.namespaces.iter().collect();
    if namespaces.len() != manifest.namespaces.len() {
        return Err(ContractError("duplicate namespace".into()));
    }
    let schema_namespaces: BTreeSet<_> = manifest
        .schemas
        .iter()
        .map(|schema| &schema.namespace)
        .collect();
    if !schema_namespaces.is_subset(&namespaces) {
        return Err(ContractError(
            "schema namespace is not owned by plugin".into(),
        ));
    }
    let entity_types = manifest
        .schemas
        .iter()
        .flat_map(|schema| schema.entity_types.iter())
        .collect::<BTreeSet<_>>();
    let mut fields = BTreeMap::new();
    for schema in &manifest.schemas {
        for field in &schema.fields {
            if let Some(field_entity_types) = &field.entity_types {
                let declared_field_entity_types = field_entity_types.iter().collect::<BTreeSet<_>>();
                if field_entity_types.is_empty()
                    || declared_field_entity_types.len() != field_entity_types.len()
                    || !declared_field_entity_types.is_subset(&entity_types)
                {
                    return Err(ContractError(format!(
                        "field {} declares unknown or duplicate entity types",
                        field.key
                    )));
                }
            }
            if fields
                .insert(&field.key, (schema, field))
                .is_some()
            {
                return Err(ContractError(format!(
                    "duplicate field key across schemas: {}",
                    field.key
                )));
            }
        }
    }
    let mut template_ids = BTreeSet::new();
    for template in &manifest.templates {
        if !template_ids.insert(&template.id) {
            return Err(ContractError(format!(
                "duplicate template id: {}",
                template.id
            )));
        }
        if !entity_types.contains(&template.entity_type) {
            return Err(ContractError(format!(
                "template uses undeclared entity type: {}",
                template.entity_type
            )));
        }
        if let Some(required_fields) = &template.required_fields {
            let mut required_field_ids = BTreeSet::new();
            for key in required_fields {
                if !required_field_ids.insert(key) {
                    return Err(ContractError(format!(
                        "template {} has duplicate required field: {key}",
                        template.id
                    )));
                }
                let (_, field) = fields.get(key).ok_or_else(|| {
                    ContractError(format!(
                        "template {} requires undeclared field: {key}",
                        template.id
                    ))
                })?;
                if let Some(field_entity_types) = &field.entity_types {
                    if !field_entity_types.contains(&template.entity_type) {
                        return Err(ContractError(format!(
                            "template {} requires field not applicable to entity type: {key}",
                            template.id
                        )));
                    }
                }
            }
        }
        let values = template
            .fields
            .as_object()
            .ok_or_else(|| ContractError(format!("template fields must be an object: {}", template.id)))?;
        for (key, value) in values {
            let (_, field) = fields.get(key).ok_or_else(|| {
                ContractError(format!(
                    "template {} uses undeclared field: {key}",
                    template.id
                ))
            })?;
            if let Some(field_entity_types) = &field.entity_types {
                if !field_entity_types.contains(&template.entity_type) {
                    return Err(ContractError(format!(
                        "template {} uses field not applicable to entity type: {key}",
                        template.id
                    )));
                }
            }
            if value.is_null() || value == "" {
                continue;
            }
            let valid = match field.field_type.as_str() {
                "text" | "entity-ref" => value.is_string(),
                "number" => value.as_f64().is_some(),
                "boolean" => value.is_boolean(),
                "date" => value.is_string() || value.is_object(),
                "enum" => value
                    .as_str()
                    .is_some_and(|candidate| field.options.as_ref().is_some_and(|options| options.contains(&candidate.to_owned()))),
                _ => false,
            };
            if !valid {
                return Err(ContractError(format!(
                    "template {} has invalid preset for field: {key}",
                    template.id
                )));
            }
        }
    }
    let mut current = 0;
    let mut migrations = manifest.migrations.iter().collect::<Vec<_>>();
    migrations.sort_by_key(|migration| migration.from);
    let mut migration_ids = BTreeSet::new();
    for migration in migrations {
        if migration.from != current
            || migration.to <= migration.from
            || !migration_ids.insert(&migration.id)
        {
            return Err(ContractError(
                "migration chain is not contiguous or contains duplicates".into(),
            ));
        }
        current = migration.to;
        for operation in &migration.operations {
            let namespace = match operation {
                MigrationOperation::CreateNamespace { namespace }
                | MigrationOperation::AddField { namespace, .. }
                | MigrationOperation::RenameField { namespace, .. }
                | MigrationOperation::DropField { namespace, .. } => namespace,
            };
            if !namespaces.contains(namespace) {
                return Err(ContractError(format!(
                    "migration uses unowned namespace: {namespace}"
                )));
            }
        }
    }
    Ok(())
}

pub fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
                && part
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

pub fn is_semver(value: &str) -> bool {
    let core = value.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<_> = core.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && (part == &"0" || !part.starts_with('0'))
                && part.chars().all(|c| c.is_ascii_digit())
        })
}

pub fn is_package_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.split('/').any(|part| part == ".." || part.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Discovered,
    Validated,
    Installed,
    Resolved,
    Activating,
    Active,
    Deactivating,
    Failed,
    Quarantined,
    Incompatible,
    Uninstalling,
    Removed,
}

pub fn lifecycle_transition(from: LifecycleState, to: LifecycleState) -> bool {
    matches!(
        (from, to),
        (LifecycleState::Discovered, LifecycleState::Validated)
            | (LifecycleState::Validated, LifecycleState::Installed)
            | (
                LifecycleState::Installed,
                LifecycleState::Resolved | LifecycleState::Incompatible
            )
            | (
                LifecycleState::Resolved,
                LifecycleState::Activating | LifecycleState::Uninstalling
            )
            | (
                LifecycleState::Activating,
                LifecycleState::Active | LifecycleState::Failed
            )
            | (
                LifecycleState::Active,
                LifecycleState::Deactivating | LifecycleState::Failed
            )
            | (
                LifecycleState::Deactivating,
                LifecycleState::Resolved | LifecycleState::Failed
            )
            | (
                LifecycleState::Failed,
                LifecycleState::Resolved | LifecycleState::Quarantined
            )
            | (LifecycleState::Uninstalling, LifecycleState::Removed)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_bundled_manifests_validate() {
        let lore = include_str!("../../../packages/modules/lore/manifest.json");
        let timeline = include_str!("../../../packages/modules/timeline/manifest.json");
        assert_eq!(parse_manifest(lore).unwrap().id, "worldbuilder.lore");
        assert_eq!(
            parse_manifest(timeline).unwrap().id,
            "worldbuilder.timeline"
        );
    }

    #[test]
    fn unknown_manifest_fields_are_rejected() {
        let json = include_str!("../../../packages/modules/lore/manifest.json").replace(
            "\"name\": \"Lore\"",
            "\"name\": \"Lore\", \"unexpected\": true",
        );
        assert!(parse_manifest(&json).is_err());
    }

    #[test]
    fn template_references_must_match_declared_schema() {
        let json = include_str!("../../../packages/modules/lore/manifest.json");
        let mut manifest = parse_manifest(json).unwrap();
        manifest.templates[0].entity_type = "unknown".into();
        assert!(validate_manifest(&manifest).is_err());

        let mut manifest = parse_manifest(json).unwrap();
        manifest.templates[0].fields = serde_json::json!({"unknown": "value"});
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn field_entity_types_must_match_declared_types() {
        let json = include_str!("../../../packages/modules/lore/manifest.json");
        let mut manifest = parse_manifest(json).unwrap();
        assert!(validate_manifest(&manifest).is_ok());

        manifest.schemas[0].fields[2].entity_types = Some(vec!["unknown".into()]);
        assert!(validate_manifest(&manifest).is_err());

        let mut manifest = parse_manifest(json).unwrap();
        manifest.templates[0].fields = serde_json::json!({"occupation": "Archivist"});
        assert!(validate_manifest(&manifest).is_ok());
        manifest.templates[1].fields = serde_json::json!({"occupation": "Archivist"});
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn template_required_fields_must_match_template_schema() {
        let json = include_str!("../../../packages/modules/lore/manifest.json");
        let mut manifest = parse_manifest(json).unwrap();
        manifest.templates[0].required_fields = Some(vec!["occupation".into()]);
        assert!(validate_manifest(&manifest).is_ok());

        manifest.templates[0].required_fields = Some(vec!["region".into()]);
        assert!(validate_manifest(&manifest).is_err());

        manifest.templates[0].required_fields = Some(vec!["unknown".into()]);
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn lifecycle_is_fail_closed() {
        assert!(lifecycle_transition(
            LifecycleState::Resolved,
            LifecycleState::Activating
        ));
        assert!(!lifecycle_transition(
            LifecycleState::Active,
            LifecycleState::Installed
        ));
    }
}
