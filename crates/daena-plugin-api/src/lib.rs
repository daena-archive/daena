//! Versioned, runtime-independent contracts shared by the plugin host and SDK.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub mod catalog;
pub mod rpc;
pub mod schema_overlay;
pub use catalog::*;
pub use rpc::*;
pub use schema_overlay::*;

pub const MANIFEST_VERSION: u32 = 1;
pub const RPC_VERSION: u32 = 1;

pub const KNOWN_CAPABILITIES: &[&str] = &[
    "ai.text.generate",
    "ai.text.generate-structured",
    "entity.read",
    "entity.write",
    "entity.delete",
    "document.read",
    "document.write",
    "field.read:self",
    "field.read:shared",
    "field.write:self",
    "record.read:self",
    "record.write:self",
    "relationship.read",
    "relationship.write",
    "asset.read:self",
    "asset.read:shared",
    "asset.write:self",
    "asset.register",
    "search.query",
    "schema.overlay",
    "event.publish:<type>",
    "event.subscribe:<type>",
    "host.surface:<name>@<major>",
    "service.provide:<name>",
    "service.call:<name>",
];

/// The canonical capability registry: every capability id plus the resource it
/// targets and the operations it grants. `confirmation` is set for destructive
/// capabilities that require interactive confirmation.
pub struct CapabilityEntry {
    pub id: &'static str,
    pub resource: &'static str,
    pub operations: &'static [&'static str],
    pub confirmation: Option<&'static str>,
}

pub const CAPABILITY_REGISTRY: &[CapabilityEntry] = &[
    CapabilityEntry {
        id: "ai.text.generate",
        resource: "ai.inference",
        operations: &["text"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "ai.text.generate-structured",
        resource: "ai.inference",
        operations: &["structured-text"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "entity.read",
        resource: "project.entities",
        operations: &["read"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "entity.write",
        resource: "project.entities",
        operations: &["create", "update"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "entity.delete",
        resource: "project.entities",
        operations: &["delete"],
        confirmation: Some("interactive"),
    },
    CapabilityEntry {
        id: "document.read",
        resource: "project.documents",
        operations: &["read"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "document.write",
        resource: "project.documents",
        operations: &["create", "update"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "field.read:self",
        resource: "plugin.namespace",
        operations: &["read"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "field.read:shared",
        resource: "shared.field",
        operations: &["read"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "field.write:self",
        resource: "plugin.namespace",
        operations: &["create", "update"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "record.read:self",
        resource: "plugin.records",
        operations: &["read"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "record.write:self",
        resource: "plugin.records",
        operations: &["create", "update", "delete"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "relationship.read",
        resource: "project.relationships",
        operations: &["read"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "relationship.write",
        resource: "project.relationships",
        operations: &["create"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "asset.read:self",
        resource: "plugin.assets",
        operations: &["read-metadata"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "asset.read:shared",
        resource: "plugin.assets",
        operations: &["read-shared"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "asset.write:self",
        resource: "plugin.assets",
        operations: &["replace"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "asset.register",
        resource: "plugin.assets",
        operations: &["register"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "search.query",
        resource: "project.search",
        operations: &["query"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "schema.overlay",
        resource: "project.schema",
        operations: &["overlay"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "event.publish:<type>",
        resource: "declared.event",
        operations: &["publish"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "event.subscribe:<type>",
        resource: "declared.event",
        operations: &["subscribe"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "host.surface:<name>@<major>",
        resource: "host.surface",
        operations: &["use"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "service.provide:<name>",
        resource: "declared.service",
        operations: &["provide"],
        confirmation: None,
    },
    CapabilityEntry {
        id: "service.call:<name>",
        resource: "declared.service",
        operations: &["call"],
        confirmation: None,
    },
];

/// Host capabilities that are never granted unless a plugin explicitly opts in
/// through a review flow; the SDK refuses to mint them from a manifest.
pub const DENIED_BY_DEFAULT_CAPABILITIES: &[&str] = &[
    "filesystem",
    "shell",
    "process",
    "dialog",
    "tauri",
    "network",
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
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Declarative,
    Sandboxed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PluginStability {
    Stable,
    Beta,
    Experimental,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Entrypoints {
    pub ui: Option<String>,
    pub wasm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub version: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct MetadataFieldDefinition {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: Option<bool>,
    pub options: Option<Vec<String>>,
    #[serde(rename = "oneOf", default)]
    pub one_of: Option<Vec<OneOfVariant>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OneOfVariant {
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TimelineFieldRole {
    Point,
    Start,
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum TimelineFieldLayer {
    Dates,
    Lifelines,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TimelineFieldContribution {
    pub role: TimelineFieldRole,
    pub group: Option<String>,
    pub label: Option<String>,
    pub layer: Option<TimelineFieldLayer>,
}

/// Stable, renderer-neutral icon references used by entity types and templates.
/// Catalog IDs are owned by Daena; plugin SVG paths are resolved relative to the
/// verified package that declares them; user SVGs are stored in project overlays.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum IconRef {
    Catalog { id: String },
    PluginSvg { path: String },
    UserSvg { svg: String },
}

impl IconRef {
    pub fn plugin_svg_path(&self) -> Option<&str> {
        match self {
            Self::PluginSvg { path } => Some(path),
            Self::Catalog { .. } | Self::UserSvg { .. } => None,
        }
    }
}

/// Curated entity-type colors. Preset IDs are owned by Daena; custom colors
/// require explicit light and dark foreground hex values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum EntityTypeColor {
    Preset { id: String },
    Custom { light: String, dark: String },
}

pub const TYPE_COLOR_PRESET_IDS: &[&str] = &[
    "brass", "copper", "ember", "moss", "pine", "ocean", "sky", "frost", "amber", "gold", "sand",
    "rose", "plum", "violet", "slate", "ink",
];

fn normalize_hex_color(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.len() != 7 || !trimmed.starts_with('#') {
        return None;
    }
    let hex = &trimmed[1..];
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex.to_ascii_lowercase()))
}

pub fn validate_entity_type_color(color: &EntityTypeColor) -> Result<(), ContractError> {
    match color {
        EntityTypeColor::Preset { id } => {
            if !TYPE_COLOR_PRESET_IDS.contains(&id.as_str()) {
                return Err(ContractError(format!("unknown type color preset: {id}")));
            }
        }
        EntityTypeColor::Custom { light, dark } => {
            normalize_hex_color(light)
                .ok_or_else(|| ContractError(format!("invalid light type color: {light}")))?;
            normalize_hex_color(dark)
                .ok_or_else(|| ContractError(format!("invalid dark type color: {dark}")))?;
        }
    }
    Ok(())
}

pub const MAX_ICON_SVG_BYTES: usize = 32 * 1024;

pub const CATALOG_ICON_IDS: &[&str] = &[
    "agriculture",
    "anchor",
    "animal",
    "art",
    "artifact",
    "bird",
    "calendar",
    "camp",
    "castle",
    "collection",
    "compass",
    "concept",
    "craft",
    "crown",
    "culture",
    "danger",
    "encounter",
    "era",
    "event",
    "faction",
    "fire",
    "fish",
    "flower",
    "forest",
    "group",
    "heart",
    "home",
    "ice",
    "insect",
    "key",
    "language",
    "library",
    "lock",
    "magic",
    "manuscript",
    "map",
    "mine",
    "moon",
    "mountain",
    "music",
    "object",
    "person",
    "place",
    "plant",
    "reference",
    "science",
    "scroll",
    "settlement",
    "ship",
    "spirit",
    "star",
    "storm",
    "sun",
    "theatre",
    "unknown",
    "wand",
    "wealth",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityTypeDefinition {
    pub id: String,
    pub name: String,
    pub icon: IconRef,
    #[serde(rename = "iconColor")]
    pub icon_color: EntityTypeColor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
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
    #[serde(rename = "relationshipType")]
    pub relationship_type: Option<String>,
    #[serde(rename = "targetEntityTypes")]
    pub target_entity_types: Option<Vec<String>>,
    /// A shared field can be read by other plugins with `field.read:shared`,
    /// but remains writable only by the namespace owner.
    #[serde(default)]
    pub shared: bool,
    /// An enum field with multiple enabled values is stored as a string array.
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub cardinality: Option<String>,
    #[serde(rename = "oneOf", default)]
    pub one_of: Option<Vec<OneOfVariant>>,
    #[serde(rename = "metadataFields", default)]
    pub metadata_fields: Option<Vec<MetadataFieldDefinition>>,
    /// Optional renderer-neutral chronology semantics for shared date fields.
    #[serde(default)]
    pub timeline: Option<TimelineFieldContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SchemaContribution {
    pub namespace: String,
    #[serde(rename = "entityTypes")]
    pub entity_types: Vec<EntityTypeDefinition>,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EntityTemplate {
    pub id: String,
    pub name: String,
    #[serde(rename = "entityType")]
    pub entity_type: String,
    pub description: Option<String>,
    pub icon: Option<IconRef>,
    pub fields: serde_json::Value,
    #[serde(rename = "requiredFields")]
    pub required_fields: Option<Vec<String>>,
    pub document: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RecordCollection {
    pub id: String,
    #[serde(rename = "ownerEntityTypes")]
    pub owner_entity_types: Vec<String>,
    pub schema: CommandSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Migration {
    pub id: String,
    pub from: u32,
    pub to: u32,
    pub recovery: String,
    pub operations: Vec<MigrationOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ViewRenderer {
    #[default]
    Declarative,
    Sandboxed,
    HostSurface {
        id: String,
        major: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub components: Vec<ViewComponent>,
    #[serde(default)]
    pub renderer: ViewRenderer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ViewComponent {
    Heading {
        id: String,
        text: String,
    },
    Text {
        id: String,
        text: String,
    },
    EntityList {
        id: String,
        title: String,
        #[serde(rename = "entityType")]
        entity_type: String,
        limit: u32,
    },
    EntityDetail {
        id: String,
        title: String,
        source: String,
    },
    FieldForm {
        id: String,
        title: String,
        source: String,
        namespace: String,
        fields: Vec<String>,
        editable: bool,
    },
    Button {
        id: String,
        label: String,
        command: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CommandAction {
    RefreshView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CommandExposure {
    View,
    Broker,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CommandValueType {
    Object,
    String,
    Number,
    Boolean,
    Array,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CommandProperty {
    #[serde(rename = "type")]
    pub value_type: CommandValueType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct CommandSchema {
    #[serde(rename = "type")]
    pub schema_type: CommandValueType,
    #[serde(default)]
    pub properties: BTreeMap<String, CommandProperty>,
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(
        rename = "additionalProperties",
        default = "default_additional_properties"
    )]
    pub additional_properties: bool,
}

fn default_additional_properties() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub action: Option<CommandAction>,
    #[serde(default)]
    pub input: Option<CommandSchema>,
    #[serde(default)]
    pub output: Option<CommandSchema>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub exposure: Vec<CommandExposure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub name: String,
    pub major: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub name: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Services {
    pub provides: Vec<Service>,
    pub consumes: Vec<Service>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Events {
    pub publishes: Vec<Event>,
    pub subscribes: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    #[serde(rename = "manifestVersion")]
    pub manifest_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    #[serde(rename = "enabledByDefault")]
    pub enabled_by_default: Option<bool>,
    pub stability: Option<PluginStability>,
    #[serde(rename = "hostApi")]
    pub host_api: String,
    pub kind: PluginKind,
    pub entrypoints: Entrypoints,
    pub capabilities: Vec<String>,
    pub dependencies: BTreeMap<String, Dependency>,
    pub namespaces: Vec<String>,
    pub schemas: Vec<SchemaContribution>,
    pub templates: Vec<EntityTemplate>,
    #[serde(default)]
    pub records: Vec<RecordCollection>,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationOptions {
    pub expected_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "gen", derive(schemars::JsonSchema))]
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

pub fn command_exposes(command: &Command, exposure: CommandExposure) -> bool {
    command.exposure.is_empty() && matches!(exposure, CommandExposure::View)
        || command.exposure.contains(&exposure)
}

pub fn validate_command_value(
    schema: &CommandSchema,
    value: &serde_json::Value,
) -> Result<(), ContractError> {
    if !matches!(schema.schema_type, CommandValueType::Object) {
        return Err(ContractError(
            "command schemas must have an object root".into(),
        ));
    }
    let object = value
        .as_object()
        .ok_or_else(|| ContractError("command payload must be an object".into()))?;
    for required in &schema.required {
        if !schema.properties.contains_key(required) {
            return Err(ContractError(format!(
                "command schema requires an undeclared property: {required}"
            )));
        }
        if !object.contains_key(required) {
            return Err(ContractError(format!(
                "command payload is missing required property: {required}"
            )));
        }
    }
    if !schema.additional_properties
        && object
            .keys()
            .any(|key| !schema.properties.contains_key(key))
    {
        return Err(ContractError(
            "command payload contains an undeclared property".into(),
        ));
    }
    for (key, property) in &schema.properties {
        let Some(value) = object.get(key) else {
            continue;
        };
        let valid = match property.value_type {
            CommandValueType::Object => value.is_object(),
            CommandValueType::String => value.is_string(),
            CommandValueType::Number => value.is_number(),
            CommandValueType::Boolean => value.is_boolean(),
            CommandValueType::Array => value.is_array(),
            CommandValueType::Null => value.is_null(),
        };
        if !valid {
            return Err(ContractError(format!(
                "command property has the wrong type: {key}"
            )));
        }
    }
    Ok(())
}

fn validate_entity_type_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|character| matches!(character, 'a'..='z' | '0'..='9' | '_' | '-' | '.' | ':'))
}

pub(crate) fn validate_icon_ref(icon: &IconRef) -> Result<(), ContractError> {
    match icon {
        IconRef::Catalog { id } => {
            if !CATALOG_ICON_IDS.contains(&id.as_str()) {
                return Err(ContractError(format!("unknown catalog icon: {id}")));
            }
        }
        IconRef::PluginSvg { path } => {
            if !is_package_path(path) || !path.to_ascii_lowercase().ends_with(".svg") {
                return Err(ContractError(format!(
                    "plugin SVG icon must be a package-relative .svg path: {path}"
                )));
            }
        }
        IconRef::UserSvg { svg } => validate_passive_svg(svg.as_bytes())?,
    }
    Ok(())
}

fn validate_manifest_icon_ref(icon: &IconRef) -> Result<(), ContractError> {
    if matches!(icon, IconRef::UserSvg { .. }) {
        return Err(ContractError(
            "user SVG icons are project-owned and cannot appear in plugin manifests".into(),
        ));
    }
    validate_icon_ref(icon)
}

pub fn validate_passive_svg(bytes: &[u8]) -> Result<(), ContractError> {
    if bytes.is_empty() || bytes.len() > MAX_ICON_SVG_BYTES {
        return Err(ContractError(format!(
            "SVG icon must be between 1 and {MAX_ICON_SVG_BYTES} bytes"
        )));
    }
    let source =
        std::str::from_utf8(bytes).map_err(|_| ContractError("SVG icon must be UTF-8".into()))?;
    let lower = source.to_ascii_lowercase();
    if ["<!doctype", "<!entity", "<?xml-stylesheet"]
        .iter()
        .any(|value| lower.contains(value))
    {
        return Err(ContractError("SVG icon contains forbidden markup".into()));
    }
    let document = roxmltree::Document::parse(source)
        .map_err(|error| ContractError(format!("invalid SVG icon: {error}")))?;
    let root = document.root_element();
    if root.tag_name().name() != "svg"
        || root.tag_name().namespace() != Some("http://www.w3.org/2000/svg")
    {
        return Err(ContractError(
            "SVG icon requires an SVG namespace root".into(),
        ));
    }
    let values = root
        .attribute("viewBox")
        .ok_or_else(|| ContractError("SVG icon requires viewBox".into()))?
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ContractError("SVG icon has an invalid viewBox".into()))?;
    if values.len() != 4
        || values.iter().any(|value| !value.is_finite())
        || values[2] <= 0.0
        || values[3] <= 0.0
        || values[2] > 4096.0
        || values[3] > 4096.0
    {
        return Err(ContractError("SVG icon has an unsafe viewBox".into()));
    }
    const ELEMENTS: &[&str] = &[
        "svg", "g", "path", "circle", "ellipse", "line", "polyline", "polygon", "rect",
    ];
    const ATTRIBUTES: &[&str] = &[
        "viewBox",
        "width",
        "height",
        "preserveAspectRatio",
        "fill",
        "fill-rule",
        "clip-rule",
        "stroke",
        "stroke-width",
        "stroke-linecap",
        "stroke-linejoin",
        "stroke-miterlimit",
        "stroke-dasharray",
        "stroke-dashoffset",
        "vector-effect",
        "opacity",
        "fill-opacity",
        "stroke-opacity",
        "transform",
        "d",
        "x",
        "y",
        "x1",
        "y1",
        "x2",
        "y2",
        "cx",
        "cy",
        "r",
        "rx",
        "ry",
        "points",
        "role",
        "aria-hidden",
        "focusable",
    ];
    for node in document.descendants() {
        if node.is_text() && node.text().is_some_and(|text| !text.trim().is_empty()) {
            return Err(ContractError("SVG icons cannot contain text".into()));
        }
        if !node.is_element() {
            continue;
        }
        if node.tag_name().namespace() != Some("http://www.w3.org/2000/svg")
            || !ELEMENTS.contains(&node.tag_name().name())
        {
            return Err(ContractError(
                "SVG icon contains a forbidden element".into(),
            ));
        }
        for attribute in node.attributes() {
            if attribute.namespace().is_some() || !ATTRIBUTES.contains(&attribute.name()) {
                return Err(ContractError(
                    "SVG icon contains a forbidden attribute".into(),
                ));
            }
            let value = attribute.value().to_ascii_lowercase();
            if ["url(", "javascript:", "data:", "http:", "https:"]
                .iter()
                .any(|forbidden| value.contains(forbidden))
            {
                return Err(ContractError("SVG icon contains an active value".into()));
            }
        }
    }
    Ok(())
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
    if !is_host_api_range(&manifest.host_api) {
        return Err(ContractError("hostApi must be a valid semver range".into()));
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
            && !capability.starts_with("host.surface:")
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
    let mut entity_types = BTreeSet::new();
    for schema in &manifest.schemas {
        for entity_type in &schema.entity_types {
            if !validate_entity_type_id(&entity_type.id)
                || entity_type.name.trim().is_empty()
                || !entity_types.insert(&entity_type.id)
            {
                return Err(ContractError(format!(
                    "invalid or duplicate entity type: {}",
                    entity_type.id
                )));
            }
            validate_manifest_icon_ref(&entity_type.icon)?;
            validate_entity_type_color(&entity_type.icon_color)?;
        }
    }
    let mut fields = BTreeMap::new();
    for schema in &manifest.schemas {
        for field in &schema.fields {
            if let Some(field_entity_types) = &field.entity_types {
                let declared_field_entity_types =
                    field_entity_types.iter().collect::<BTreeSet<_>>();
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
            if field.field_type == "relationship" {
                if field.relationship_type.as_deref().is_none_or(str::is_empty) {
                    return Err(ContractError(format!(
                        "relationship field {} must declare relationshipType",
                        field.key
                    )));
                }
                let target_entity_types = field.target_entity_types.as_ref().ok_or_else(|| {
                    ContractError(format!(
                        "relationship field {} must declare targetEntityTypes",
                        field.key
                    ))
                })?;
                let declared_target_types = target_entity_types.iter().collect::<BTreeSet<_>>();
                if target_entity_types.is_empty()
                    || declared_target_types.len() != target_entity_types.len()
                {
                    return Err(ContractError(format!(
                        "relationship field {} declares duplicate target entity types",
                        field.key
                    )));
                }
            } else if field.relationship_type.is_some() || field.target_entity_types.is_some() {
                return Err(ContractError(format!(
                    "non-relationship field {} cannot declare relationship metadata",
                    field.key
                )));
            }
            if let Some(card) = &field.cardinality {
                if field.field_type != "relationship" {
                    return Err(ContractError(format!(
                        "field {}: cardinality is only allowed for relationship fields",
                        field.key
                    )));
                }
                if card != "one" && card != "many" {
                    return Err(ContractError(format!(
                        "field {}: cardinality must be 'one' or 'many'",
                        field.key
                    )));
                }
            }
            if field.field_type == "oneof" {
                let one_of = field.one_of.as_ref().ok_or_else(|| {
                    ContractError(format!("oneof field {} must declare oneOf", field.key))
                })?;
                if one_of.is_empty() {
                    return Err(ContractError(format!(
                        "oneof field {} must have at least one variant",
                        field.key
                    )));
                }
                for variant in one_of {
                    if variant.field_type == "relationship" || variant.field_type == "oneof" {
                        return Err(ContractError(format!(
                            "oneof variant for field {} cannot be relationship or oneof",
                            field.key
                        )));
                    }
                    if (variant.field_type == "enum" || variant.field_type == "oneof")
                        && variant.options.is_none()
                    {
                        return Err(ContractError(format!(
                            "oneof variant for field {} with type {} must declare options",
                            field.key, variant.field_type
                        )));
                    }
                }
            } else if field.one_of.is_some() {
                return Err(ContractError(format!(
                    "field {}: oneOf is only allowed for oneof fields",
                    field.key
                )));
            }
            if let Some(timeline) = &field.timeline {
                if field.field_type != "date" {
                    return Err(ContractError(format!(
                        "field {}: timeline contribution is only allowed for date fields",
                        field.key
                    )));
                }
                if !field.shared {
                    return Err(ContractError(format!(
                        "field {}: timeline contribution must be shared",
                        field.key
                    )));
                }
                if matches!(
                    timeline.role,
                    TimelineFieldRole::Start | TimelineFieldRole::End
                ) && timeline
                    .group
                    .as_deref()
                    .is_none_or(|group| group.trim().is_empty())
                {
                    return Err(ContractError(format!(
                        "field {}: timeline start/end contribution must declare a group",
                        field.key
                    )));
                }
                if timeline
                    .label
                    .as_deref()
                    .is_some_and(|label| label.trim().is_empty())
                {
                    return Err(ContractError(format!(
                        "field {}: timeline contribution label cannot be empty",
                        field.key
                    )));
                }
            }
            validate_metadata_fields(
                &field.field_type,
                &field.key,
                field.metadata_fields.as_deref(),
            )
            .map_err(ContractError)?;
            if fields.insert(&field.key, (schema, field)).is_some() {
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
        if let Some(icon) = &template.icon {
            validate_manifest_icon_ref(icon)?;
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
        let values = template.fields.as_object().ok_or_else(|| {
            ContractError(format!(
                "template fields must be an object: {}",
                template.id
            ))
        })?;
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
                "text" => value.is_string(),
                "relationship" => {
                    if field.cardinality.as_deref() == Some("one") {
                        if value.is_string() {
                            true
                        } else if let Some(arr) = value.as_array() {
                            arr.len() <= 1 && arr.iter().all(|v| v.is_string())
                        } else {
                            false
                        }
                    } else {
                        value
                            .as_array()
                            .is_some_and(|targets| targets.iter().all(serde_json::Value::is_string))
                    }
                }
                "number" => value.as_f64().is_some(),
                "boolean" => value.is_boolean(),
                "date" => value.is_string() || value.is_object(),
                "enum" => value.as_str().is_some_and(|candidate| {
                    field
                        .options
                        .as_ref()
                        .is_some_and(|options| options.contains(&candidate.to_owned()))
                }),
                "oneof" => {
                    if let Some(one_of) = &field.one_of {
                        let mut matches = 0;
                        for variant in one_of {
                            let variant_valid = match variant.field_type.as_str() {
                                "text" => value.is_string(),
                                "number" => value.as_f64().is_some(),
                                "boolean" => value.is_boolean(),
                                "date" => value.is_string() || value.is_object(),
                                "enum" | "oneof" => value.as_str().is_some_and(|c| {
                                    variant
                                        .options
                                        .as_ref()
                                        .is_some_and(|o| o.contains(&c.to_owned()))
                                }),
                                _ => false,
                            };
                            if variant_valid {
                                matches += 1;
                            }
                        }
                        matches == 1
                    } else {
                        false
                    }
                }
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
    let mut record_ids = BTreeSet::new();
    for collection in &manifest.records {
        if !is_identifier(&collection.id) || !record_ids.insert(&collection.id) {
            return Err(ContractError(format!(
                "invalid or duplicate record collection: {}",
                collection.id
            )));
        }
        if collection.owner_entity_types.is_empty()
            || collection
                .owner_entity_types
                .iter()
                .any(|entity_type| !entity_types.contains(entity_type))
        {
            return Err(ContractError(format!(
                "record collection {} declares unknown owner entity types",
                collection.id
            )));
        }
        validate_command_value(
            &collection.schema,
            &serde_json::Value::Object(
                collection
                    .schema
                    .properties
                    .iter()
                    .map(|(key, property)| {
                        let value = match property.value_type {
                            CommandValueType::Object => serde_json::json!({}),
                            CommandValueType::String => serde_json::json!("value"),
                            CommandValueType::Number => serde_json::json!(1),
                            CommandValueType::Boolean => serde_json::json!(true),
                            CommandValueType::Array => serde_json::json!([]),
                            CommandValueType::Null => serde_json::Value::Null,
                        };
                        (key.clone(), value)
                    })
                    .collect(),
            ),
        )?;
    }
    let mut command_ids = BTreeSet::new();
    for command in &manifest.commands {
        if command.id.trim().is_empty() || command.title.trim().is_empty() {
            return Err(ContractError(
                "command id and title must not be empty".into(),
            ));
        }
        if !command_ids.insert(&command.id) {
            return Err(ContractError(format!(
                "duplicate command id: {}",
                command.id
            )));
        }
        let mut exposures = BTreeSet::new();
        for exposure in &command.exposure {
            if !exposures.insert(exposure) {
                return Err(ContractError(format!(
                    "command {} has duplicate exposure",
                    command.id
                )));
            }
        }
        for capability in &command.capabilities {
            if !manifest.capabilities.contains(capability) {
                return Err(ContractError(format!(
                    "command {} requires an undeclared capability: {}",
                    command.id, capability
                )));
            }
        }
        for schema in [command.input.as_ref(), command.output.as_ref()]
            .into_iter()
            .flatten()
        {
            if !matches!(schema.schema_type, CommandValueType::Object) {
                return Err(ContractError(format!(
                    "command {} schema root must be object",
                    command.id
                )));
            }
            let required = schema.required.iter().collect::<BTreeSet<_>>();
            if required.len() != schema.required.len()
                || schema
                    .required
                    .iter()
                    .any(|key| !schema.properties.contains_key(key))
            {
                return Err(ContractError(format!(
                    "command {} schema has invalid required properties",
                    command.id
                )));
            }
        }
    }
    let mut view_ids = BTreeSet::new();
    for view in &manifest.views {
        if view.id.trim().is_empty() || view.title.trim().is_empty() {
            return Err(ContractError("view id and title must not be empty".into()));
        }
        if !view_ids.insert(&view.id) {
            return Err(ContractError(format!("duplicate view id: {}", view.id)));
        }
        if let ViewRenderer::HostSurface { id, major } = &view.renderer {
            if !is_host_surface_id(id) || *major == 0 {
                return Err(ContractError(format!(
                    "view {} declares an invalid host surface",
                    view.id
                )));
            }
            let capability = format!("host.surface:{id}@{major}");
            if !manifest
                .capabilities
                .iter()
                .any(|candidate| candidate == &capability)
            {
                return Err(ContractError(format!(
                    "view {} requires undeclared capability: {capability}",
                    view.id
                )));
            }
        }
        let mut component_ids = BTreeSet::new();
        let mut list_entity_types = BTreeMap::new();
        for component in &view.components {
            let (id, requires_entity_read, requires_field_read, requires_field_write) =
                match component {
                    ViewComponent::Heading { id, text } | ViewComponent::Text { id, text } => {
                        if text.trim().is_empty() {
                            return Err(ContractError(format!(
                                "view {} contains an empty text component",
                                view.id
                            )));
                        }
                        (id, false, false, false)
                    }
                    ViewComponent::EntityList {
                        id,
                        title,
                        entity_type,
                        limit,
                    } => {
                        if title.trim().is_empty() {
                            return Err(ContractError(format!(
                                "view {} contains an entity list without a title",
                                view.id
                            )));
                        }
                        if *limit == 0 || *limit > 100 {
                            return Err(ContractError(format!(
                                "view {} entity list limit must be between 1 and 100",
                                view.id
                            )));
                        }
                        if !entity_types.contains(entity_type) {
                            return Err(ContractError(format!(
                                "view {} lists undeclared entity type: {}",
                                view.id, entity_type
                            )));
                        }
                        list_entity_types.insert(id.clone(), entity_type.clone());
                        (id, true, false, false)
                    }
                    ViewComponent::EntityDetail { id, title, source } => {
                        if title.trim().is_empty() || source.trim().is_empty() {
                            return Err(ContractError(format!(
                                "view {} entity detail requires title and source",
                                view.id
                            )));
                        }
                        (id, true, false, false)
                    }
                    ViewComponent::FieldForm {
                        id,
                        title,
                        source,
                        namespace,
                        fields: form_fields,
                        editable,
                    } => {
                        if title.trim().is_empty()
                            || source.trim().is_empty()
                            || namespace.trim().is_empty()
                            || form_fields.is_empty()
                        {
                            return Err(ContractError(format!(
                                "view {} field form is incomplete",
                                view.id
                            )));
                        }
                        if form_fields.iter().any(|field| field.trim().is_empty()) {
                            return Err(ContractError(format!(
                                "view {} field form contains an empty field",
                                view.id
                            )));
                        }
                        (id, false, true, *editable)
                    }
                    ViewComponent::Button { id, label, command } => {
                        if label.trim().is_empty() || command.trim().is_empty() {
                            return Err(ContractError(format!(
                                "view {} button requires label and command",
                                view.id
                            )));
                        }
                        let declared = manifest
                            .commands
                            .iter()
                            .find(|candidate| candidate.id == *command)
                            .is_some_and(|candidate| {
                                candidate.action.is_some()
                                    && command_exposes(candidate, CommandExposure::View)
                            });
                        if !declared {
                            return Err(ContractError(format!(
                                "view {} button references a command without a host action: {}",
                                view.id, command
                            )));
                        }
                        (id, false, false, false)
                    }
                };
            if id.trim().is_empty() || !component_ids.insert(id) {
                return Err(ContractError(format!(
                    "view {} contains a duplicate or empty component id",
                    view.id
                )));
            }
            if requires_entity_read
                && !manifest
                    .capabilities
                    .iter()
                    .any(|capability| capability == "entity.read")
            {
                return Err(ContractError(format!(
                    "view {} entity list requires entity.read",
                    view.id
                )));
            }
            if requires_field_read
                && !manifest
                    .capabilities
                    .iter()
                    .any(|capability| capability == "field.read:self")
            {
                return Err(ContractError(format!(
                    "view {} field form requires field.read:self",
                    view.id
                )));
            }
            if requires_field_write
                && !manifest
                    .capabilities
                    .iter()
                    .any(|capability| capability == "field.write:self")
            {
                return Err(ContractError(format!(
                    "view {} editable field form requires field.write:self",
                    view.id
                )));
            }
        }
        for component in &view.components {
            match component {
                ViewComponent::EntityDetail { source, .. } => {
                    if !list_entity_types.contains_key(source) {
                        return Err(ContractError(format!(
                            "view {} references an unknown entity list: {}",
                            view.id, source
                        )));
                    }
                }
                ViewComponent::FieldForm {
                    source,
                    namespace,
                    fields: form_fields,
                    ..
                } => {
                    if !list_entity_types.contains_key(source) {
                        return Err(ContractError(format!(
                            "view {} references an unknown entity list: {}",
                            view.id, source
                        )));
                    }
                    if !namespaces.contains(namespace) {
                        return Err(ContractError(format!(
                            "view {} field form uses an unowned namespace: {}",
                            view.id, namespace
                        )));
                    }
                    let entity_type = list_entity_types.get(source).expect("source validated");
                    for field_key in form_fields {
                        let (schema, field) = fields.get(field_key).ok_or_else(|| {
                            ContractError(format!(
                                "view {} field form uses an undeclared field: {}",
                                view.id, field_key
                            ))
                        })?;
                        if schema.namespace != *namespace
                            || field
                                .entity_types
                                .as_ref()
                                .is_some_and(|types| !types.contains(entity_type))
                        {
                            return Err(ContractError(format!(
                                "view {} field form field is outside its source schema: {}",
                                view.id, field_key
                            )));
                        }
                    }
                }
                _ => {}
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

pub fn is_host_surface_id(value: &str) -> bool {
    let Some((namespace, surface)) = value.split_once('/') else {
        return false;
    };
    is_identifier(namespace)
        && !surface.is_empty()
        && surface.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.'
        })
        && surface
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

pub fn is_semver(value: &str) -> bool {
    fn numeric_part(part: &str) -> bool {
        !part.is_empty()
            && (part == "0" || !part.starts_with('0'))
            && part.chars().all(|c| c.is_ascii_digit())
    }
    fn pre_build_part(part: &str) -> bool {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    }
    let (core, pre, build) = match value.split_once('+') {
        Some((before, build)) => match before.split_once('-') {
            Some((core, pre)) => (core, Some(pre), Some(build)),
            None => (before, None, Some(build)),
        },
        None => match value.split_once('-') {
            Some((core, pre)) => (core, Some(pre), None),
            None => (value, None, None),
        },
    };
    let core_parts: Vec<_> = core.split('.').collect();
    core_parts.len() == 3
        && core_parts.iter().all(|part| numeric_part(part))
        && pre.is_none_or(pre_build_part)
        && build.is_none_or(pre_build_part)
}

/// A hostApi range is a space-separated list of semver constraints, each
/// optionally prefixed with `^`, `~`, `>=`, `<=`, `>`, `<`, or `=`.
pub fn is_host_api_range(value: &str) -> bool {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }
    parts.iter().all(|raw| {
        let part = raw.trim();
        let rest = if part.starts_with(">=") || part.starts_with("<=") {
            &part[2..]
        } else if part.starts_with('^')
            || part.starts_with('~')
            || part.starts_with('>')
            || part.starts_with('<')
            || part.starts_with('=')
        {
            &part[1..]
        } else {
            part
        };
        is_semver(rest)
    })
}

pub fn is_package_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
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
mod tests;
