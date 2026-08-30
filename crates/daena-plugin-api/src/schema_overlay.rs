//! Project-owned module schema overlays (host-side customization of package defaults).

#[cfg(test)]
use crate::TimelineFieldLayer;
use crate::{
    validate_entity_type_color, validate_icon_ref, EntityTemplate, EntityTypeColor,
    EntityTypeDefinition, FieldDefinition, IconRef, MetadataFieldDefinition, PluginManifest,
    TimelineFieldContribution, TimelineFieldRole,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SCHEMA_OVERLAY_CAPABILITY: &str = "schema.overlay";
pub const SCHEMA_OVERLAY_VERSION: u32 = 1;

const ALLOWED_FIELD_TYPES: &[&str] = &[
    "text",
    "number",
    "boolean",
    "date",
    "enum",
    "oneof",
    "relationship",
];

const ALLOWED_METADATA_FIELD_TYPES: &[&str] =
    &["text", "number", "boolean", "date", "enum", "oneof"];

/// Validate the relationship-only metadata declaration attached to a field.
pub fn validate_metadata_fields(
    field_type: &str,
    field_key: &str,
    metadata_fields: Option<&[MetadataFieldDefinition]>,
) -> Result<(), String> {
    let Some(metadata_fields) = metadata_fields else {
        return Ok(());
    };
    if field_type != "relationship" {
        return Err(format!(
            "non-relationship field {field_key} cannot declare metadataFields"
        ));
    }
    let mut keys = BTreeSet::new();
    for field in metadata_fields {
        if field.key.trim().is_empty() || field.label.trim().is_empty() {
            return Err(format!(
                "relationship metadata fields require key and label: {field_key}"
            ));
        }
        if !ALLOWED_METADATA_FIELD_TYPES.contains(&field.field_type.as_str()) {
            return Err(format!(
                "unsupported relationship metadata field type for {}: {}",
                field.key, field.field_type
            ));
        }
        if !keys.insert(&field.key) {
            return Err(format!(
                "duplicate relationship metadata field key: {}",
                field.key
            ));
        }
        if field.field_type == "enum" && field.options.as_ref().is_none_or(Vec::is_empty) {
            return Err(format!(
                "relationship metadata enum field requires non-empty options: {}",
                field.key
            ));
        }
        if field.field_type == "oneof" {
            let one_of = field.one_of.as_ref().ok_or_else(|| {
                format!(
                    "relationship metadata oneof field {} must declare oneOf",
                    field.key
                )
            })?;
            if one_of.is_empty() {
                return Err(format!(
                    "relationship metadata oneof field {} must have at least one variant",
                    field.key
                ));
            }
            for variant in one_of {
                if variant.field_type == "relationship" || variant.field_type == "oneof" {
                    return Err(format!(
                        "oneof variant for field {} cannot be relationship or oneof",
                        field.key
                    ));
                }
                if (variant.field_type == "enum" || variant.field_type == "oneof")
                    && variant.options.is_none()
                {
                    return Err(format!(
                        "relationship metadata oneof variant for field {} with type {} must declare options",
                        field.key, variant.field_type
                    ));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FieldTimelineOverride {
    pub field_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeline: Option<TimelineFieldContribution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ModuleSchemaOverlay {
    #[serde(default = "default_overlay_version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_entity_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disabled_templates: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_entity_types: Vec<EntityTypeDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_fields: Vec<FieldDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_templates: Vec<EntityTemplate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_scope_overrides: Vec<FieldScopeOverride>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub template_overrides: Vec<TemplateOverride>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_metadata_overrides: Vec<FieldMetadataOverride>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entity_type_appearance_overrides: Vec<EntityTypeAppearanceOverride>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_timeline_overrides: Vec<FieldTimelineOverride>,
}

/// Project-specific metadata extension for a packaged relationship field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FieldMetadataOverride {
    pub field_key: String,
    #[serde(rename = "metadataFields")]
    pub metadata_fields: Vec<MetadataFieldDefinition>,
}

/// Project-specific applicability for a packaged field. Package field metadata
/// remains immutable; the overlay only decides which entity types use it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct FieldScopeOverride {
    pub field_key: String,
    pub entity_types: Vec<String>,
}

/// Project-specific field selection for a packaged template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TemplateOverride {
    pub template_id: String,
    pub fields: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_fields: Option<Vec<String>>,
}

/// Project-specific icon and color overrides for a packaged entity type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct EntityTypeAppearanceOverride {
    pub entity_type_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_color: Option<EntityTypeColor>,
}

/// Package builtins + current overlay for the schema settings editor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleSchemaEditorState {
    pub id: String,
    pub name: String,
    pub schemas: Vec<crate::SchemaContribution>,
    pub templates: Vec<EntityTemplate>,
    pub overlay: ModuleSchemaOverlay,
}

fn default_overlay_version() -> u32 {
    SCHEMA_OVERLAY_VERSION
}

impl Default for ModuleSchemaOverlay {
    fn default() -> Self {
        Self {
            version: SCHEMA_OVERLAY_VERSION,
            disabled_entity_types: Vec::new(),
            disabled_fields: Vec::new(),
            disabled_templates: Vec::new(),
            custom_entity_types: Vec::new(),
            custom_fields: Vec::new(),
            custom_templates: Vec::new(),
            field_scope_overrides: Vec::new(),
            template_overrides: Vec::new(),
            field_metadata_overrides: Vec::new(),
            entity_type_appearance_overrides: Vec::new(),
            field_timeline_overrides: Vec::new(),
        }
    }
}

impl ModuleSchemaOverlay {
    pub fn is_empty(&self) -> bool {
        self.disabled_entity_types.is_empty()
            && self.disabled_fields.is_empty()
            && self.disabled_templates.is_empty()
            && self.custom_entity_types.is_empty()
            && self.custom_fields.is_empty()
            && self.custom_templates.is_empty()
            && self.field_scope_overrides.is_empty()
            && self.template_overrides.is_empty()
            && self.field_metadata_overrides.is_empty()
            && self.entity_type_appearance_overrides.is_empty()
            && self.field_timeline_overrides.is_empty()
    }
}

/// A packaged plugin may store a project-owned schema overlay when it declares
/// `schema.overlay` and contributes at least one entity type.
pub fn supports_schema_overlay(package: &PluginManifest) -> bool {
    package
        .capabilities
        .iter()
        .any(|capability| capability == SCHEMA_OVERLAY_CAPABILITY)
        && package
            .schemas
            .iter()
            .any(|schema| !schema.entity_types.is_empty())
}

fn primary_schema_namespace(package: &PluginManifest) -> Result<&str, String> {
    if let Some(namespace) = package.namespaces.first() {
        if package
            .schemas
            .iter()
            .any(|schema| schema.namespace == *namespace)
        {
            return Ok(namespace.as_str());
        }
    }
    package
        .schemas
        .first()
        .map(|schema| schema.namespace.as_str())
        .ok_or_else(|| format!("packaged schema is missing for {}", package.id))
}

fn is_field_key(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z')) && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_entity_type_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-' | '.' | ':'))
}

fn qualify_type_id(plugin_id: &str, id: &str) -> Result<String, String> {
    crate::normalize_local_id(plugin_id, id).map_err(|error| error.0)
}

fn qualify_type_ids(plugin_id: &str, ids: &mut [String]) -> Result<(), String> {
    for id in ids {
        *id = qualify_type_id(plugin_id, id)?;
    }
    Ok(())
}

fn qualify_field_type_id(plugin_id: &str, id: &str) -> Result<String, String> {
    if let Some((prefix, _)) = id.split_once(':') {
        if prefix != plugin_id {
            if !is_entity_type_id(id) {
                return Err(format!("invalid entity type id: {id}"));
            }
            return Ok(id.to_string());
        }
    }
    qualify_type_id(plugin_id, id)
}

fn qualify_field_type_ids(plugin_id: &str, ids: &mut [String]) -> Result<(), String> {
    for id in ids {
        *id = qualify_field_type_id(plugin_id, id)?;
    }
    Ok(())
}

fn overlay_entity_type_allowed(
    package: &PluginManifest,
    effective_types: &BTreeSet<&str>,
    id: &str,
) -> bool {
    if effective_types.contains(id) {
        return true;
    }
    let Some((prefix, _)) = id.split_once(':') else {
        return false;
    };
    prefix != package.id
        && crate::is_identifier(prefix)
        && package
            .dependencies
            .get(prefix)
            .is_some_and(|dependency| dependency.required)
}

fn qualify_overlay_entity_types(
    plugin_id: &str,
    overlay: &mut ModuleSchemaOverlay,
) -> Result<(), String> {
    qualify_type_ids(plugin_id, &mut overlay.disabled_entity_types)?;
    for entity_type in &mut overlay.custom_entity_types {
        entity_type.id = qualify_type_id(plugin_id, &entity_type.id)?;
    }
    for field in &mut overlay.custom_fields {
        if let Some(entity_types) = &mut field.entity_types {
            qualify_field_type_ids(plugin_id, entity_types)?;
        }
        if let Some(targets) = &mut field.target_entity_types {
            qualify_field_type_ids(plugin_id, targets)?;
        }
    }
    for template in &mut overlay.custom_templates {
        template.entity_type = qualify_type_id(plugin_id, &template.entity_type)?;
    }
    for scope in &mut overlay.field_scope_overrides {
        qualify_field_type_ids(plugin_id, &mut scope.entity_types)?;
    }
    for appearance in &mut overlay.entity_type_appearance_overrides {
        appearance.entity_type_id = qualify_type_id(plugin_id, &appearance.entity_type_id)?;
    }
    Ok(())
}

fn overlay_with_qualified_types(
    package: &PluginManifest,
    overlay: &ModuleSchemaOverlay,
) -> Result<ModuleSchemaOverlay, String> {
    let mut overlay = overlay.clone();
    qualify_overlay_entity_types(&package.id, &mut overlay)?;
    Ok(overlay)
}

/// Qualify overlay entity-type IDs to `pluginId:local` in place.
pub fn qualify_module_overlay(
    package: &PluginManifest,
    overlay: &mut ModuleSchemaOverlay,
) -> Result<(), String> {
    qualify_overlay_entity_types(&package.id, overlay)
}

fn is_relationship_type(value: &str) -> bool {
    is_entity_type_id(value)
}

fn is_template_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Validate an overlay against a packaged module manifest (builtins).
pub fn validate_module_overlay(
    package: &PluginManifest,
    overlay: &ModuleSchemaOverlay,
) -> Result<(), String> {
    if !supports_schema_overlay(package) {
        return Err(format!(
            "schema overlays are not supported for {}",
            package.id
        ));
    }
    if overlay.version != SCHEMA_OVERLAY_VERSION && overlay.version != 0 {
        return Err(format!(
            "unsupported schema overlay version: {}",
            overlay.version
        ));
    }

    let overlay = overlay_with_qualified_types(package, overlay)?;

    let namespace = primary_schema_namespace(package)?;
    let package_schema = package
        .schemas
        .iter()
        .find(|schema| schema.namespace == namespace)
        .ok_or_else(|| format!("packaged schema is missing for {}", package.id))?;
    let package_types: BTreeSet<&str> = package_schema
        .entity_types
        .iter()
        .map(|entity_type| entity_type.id.as_str())
        .collect();
    let package_fields: BTreeSet<&str> = package_schema
        .fields
        .iter()
        .map(|field| field.key.as_str())
        .collect();
    let package_templates: BTreeSet<&str> = package
        .templates
        .iter()
        .map(|template| template.id.as_str())
        .collect();

    for name in &overlay.disabled_entity_types {
        if !package_types.contains(name.as_str()) {
            return Err(format!(
                "cannot disable unknown builtin entity type: {name}"
            ));
        }
    }
    if unique_len(&overlay.disabled_entity_types) != overlay.disabled_entity_types.len() {
        return Err("disabledEntityTypes must be unique".into());
    }

    for key in &overlay.disabled_fields {
        if !package_fields.contains(key.as_str()) {
            return Err(format!("cannot disable unknown builtin field: {key}"));
        }
    }
    if unique_len(&overlay.disabled_fields) != overlay.disabled_fields.len() {
        return Err("disabledFields must be unique".into());
    }

    for id in &overlay.disabled_templates {
        if !package_templates.contains(id.as_str()) {
            return Err(format!("cannot disable unknown builtin template: {id}"));
        }
    }
    if unique_len(&overlay.disabled_templates) != overlay.disabled_templates.len() {
        return Err("disabledTemplates must be unique".into());
    }

    for entity_type in &overlay.custom_entity_types {
        if !is_entity_type_id(&entity_type.id) || entity_type.name.trim().is_empty() {
            return Err(format!("invalid custom entity type: {}", entity_type.id));
        }
        validate_icon_ref(&entity_type.icon).map_err(|error| error.0)?;
        validate_entity_type_color(&entity_type.icon_color).map_err(|error| error.0)?;
        if package_types.contains(entity_type.id.as_str()) {
            return Err(format!(
                "custom entity type collides with builtin type: {}",
                entity_type.id
            ));
        }
    }
    if overlay
        .custom_entity_types
        .iter()
        .map(|entity_type| entity_type.id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
        != overlay.custom_entity_types.len()
    {
        return Err("customEntityTypes must be unique".into());
    }

    let effective_types: BTreeSet<&str> = package_types
        .iter()
        .copied()
        .filter(|name| {
            !overlay
                .disabled_entity_types
                .iter()
                .any(|disabled| disabled == name)
        })
        .chain(
            overlay
                .custom_entity_types
                .iter()
                .map(|entity_type| entity_type.id.as_str()),
        )
        .collect();

    let mut field_scope_keys = BTreeSet::new();
    for scope in &overlay.field_scope_overrides {
        if !package_fields.contains(scope.field_key.as_str()) {
            return Err(format!(
                "field scope override references unknown builtin field: {}",
                scope.field_key
            ));
        }
        if overlay
            .disabled_fields
            .iter()
            .any(|disabled| disabled == &scope.field_key)
        {
            return Err(format!(
                "field scope override references disabled field: {}",
                scope.field_key
            ));
        }
        if !field_scope_keys.insert(scope.field_key.as_str()) {
            return Err(format!(
                "duplicate field scope override: {}",
                scope.field_key
            ));
        }
        if scope.entity_types.is_empty() {
            return Err(format!(
                "field scope override requires at least one entity type: {}",
                scope.field_key
            ));
        }
        if unique_len(&scope.entity_types) != scope.entity_types.len() {
            return Err(format!(
                "field scope override entityTypes must be unique: {}",
                scope.field_key
            ));
        }
        for entity_type in &scope.entity_types {
            if !overlay_entity_type_allowed(package, &effective_types, entity_type) {
                return Err(format!(
                    "field scope override {} references unknown entity type: {entity_type}",
                    scope.field_key
                ));
            }
        }
    }

    let mut custom_field_keys = BTreeSet::new();
    for field in &overlay.custom_fields {
        if !is_field_key(&field.key) {
            return Err(format!("invalid custom field key: {}", field.key));
        }
        if field.label.trim().is_empty() {
            return Err(format!("custom field label is required: {}", field.key));
        }
        if !ALLOWED_FIELD_TYPES.contains(&field.field_type.as_str()) {
            return Err(format!(
                "unsupported custom field type for {}: {}",
                field.key, field.field_type
            ));
        }
        if package_fields.contains(field.key.as_str()) {
            return Err(format!(
                "custom field collides with builtin field: {}",
                field.key
            ));
        }
        if !custom_field_keys.insert(field.key.as_str()) {
            return Err(format!("duplicate custom field key: {}", field.key));
        }
        if let Some(entity_types) = &field.entity_types {
            for entity_type in entity_types {
                if !overlay_entity_type_allowed(package, &effective_types, entity_type) {
                    return Err(format!(
                        "custom field {} references unknown entity type: {entity_type}",
                        field.key
                    ));
                }
            }
        }
        if field.field_type == "relationship" {
            let relationship_type = field.relationship_type.as_ref().ok_or_else(|| {
                format!(
                    "custom relationship field requires relationshipType: {}",
                    field.key
                )
            })?;
            if !is_relationship_type(relationship_type) {
                return Err(format!(
                    "invalid relationshipType on {}: {relationship_type}",
                    field.key
                ));
            }
            let targets = field.target_entity_types.as_ref().ok_or_else(|| {
                format!(
                    "custom relationship field requires targetEntityTypes: {}",
                    field.key
                )
            })?;
            if targets.is_empty() {
                return Err(format!(
                    "custom relationship field targetEntityTypes must be non-empty: {}",
                    field.key
                ));
            }
            for target in targets {
                if !overlay_entity_type_allowed(package, &effective_types, target) {
                    return Err(format!(
                        "custom field {} references unknown target type: {target}",
                        field.key
                    ));
                }
            }
        }
        if field.field_type == "enum" {
            let options = field
                .options
                .as_ref()
                .ok_or_else(|| format!("custom enum field requires options: {}", field.key))?;
            if options.is_empty() {
                return Err(format!(
                    "custom enum field options must be non-empty: {}",
                    field.key
                ));
            }
        }
        if field.field_type == "oneof" {
            let one_of = field
                .one_of
                .as_ref()
                .ok_or_else(|| format!("custom oneof field requires oneOf: {}", field.key))?;
            if one_of.is_empty() {
                return Err(format!(
                    "custom oneof field oneOf must be non-empty: {}",
                    field.key
                ));
            }
            for variant in one_of {
                if variant.field_type == "relationship" || variant.field_type == "oneof" {
                    return Err(format!(
                        "oneof variant for field {} cannot be relationship or oneof",
                        field.key
                    ));
                }
                if (variant.field_type == "enum" || variant.field_type == "oneof")
                    && variant.options.is_none()
                {
                    return Err(format!(
                        "oneof variant for field {} with type {} must declare options",
                        field.key, variant.field_type
                    ));
                }
            }
        }
        if let Some(card) = &field.cardinality {
            if field.field_type != "relationship" {
                return Err(format!(
                    "field {}: cardinality is only allowed for relationship fields",
                    field.key
                ));
            }
            if card != "one" && card != "many" {
                return Err(format!(
                    "field {}: cardinality must be 'one' or 'many'",
                    field.key
                ));
            }
        }
        if let Some(direction) = &field.relationship_direction {
            if field.field_type != "relationship" {
                return Err(format!(
                    "field {}: relationshipDirection is only allowed for relationship fields",
                    field.key
                ));
            }
            if direction != "outgoing" && direction != "incoming" && direction != "undirected" {
                return Err(format!(
                    "field {}: relationshipDirection must be 'outgoing', 'incoming', or 'undirected'",
                    field.key
                ));
            }
        }
        if field.field_type == "relationship" && field.one_of.is_some() {
            return Err(format!(
                "field {}: oneOf is only allowed for oneof fields",
                field.key
            ));
        }
        validate_metadata_fields(
            &field.field_type,
            &field.key,
            field.metadata_fields.as_deref(),
        )?;
        if let Some(timeline) = &field.timeline {
            if field.field_type != "date" {
                return Err(format!(
                    "field {}: timeline contribution is only allowed for date fields",
                    field.key
                ));
            }
            if !field.shared {
                return Err(format!(
                    "field {}: timeline contribution must be shared",
                    field.key
                ));
            }
            if matches!(
                timeline.role,
                TimelineFieldRole::Start | TimelineFieldRole::End
            ) && timeline
                .group
                .as_deref()
                .is_none_or(|group| group.trim().is_empty())
            {
                return Err(format!(
                    "field {}: timeline start/end contribution must declare a group",
                    field.key
                ));
            }
            if timeline
                .label
                .as_deref()
                .is_some_and(|label| label.trim().is_empty())
            {
                return Err(format!(
                    "field {}: timeline contribution label cannot be empty",
                    field.key
                ));
            }
        }
    }

    // Validate fieldTimelineOverrides (builtin + custom date field timeline overrides)
    let mut timeline_override_keys = BTreeSet::new();
    for ov in &overlay.field_timeline_overrides {
        if ov.field_key.trim().is_empty() {
            return Err("field timeline override requires non-empty fieldKey".into());
        }
        if !package_fields.contains(ov.field_key.as_str())
            && !custom_field_keys.contains(ov.field_key.as_str())
        {
            return Err(format!(
                "field timeline override references unknown field: {}",
                ov.field_key
            ));
        }
        if overlay
            .disabled_fields
            .iter()
            .any(|disabled| disabled == &ov.field_key)
        {
            return Err(format!(
                "field timeline override references disabled field: {}",
                ov.field_key
            ));
        }
        if !timeline_override_keys.insert(ov.field_key.as_str()) {
            return Err(format!(
                "duplicate field timeline override: {}",
                ov.field_key
            ));
        }
        if let Some(timeline) = &ov.timeline {
            let field = package_schema
                .fields
                .iter()
                .find(|f| f.key == ov.field_key)
                .or_else(|| overlay.custom_fields.iter().find(|f| f.key == ov.field_key))
                .expect("field was checked above");
            if field.field_type != "date" {
                return Err(format!(
                    "field {}: timeline contribution is only allowed for date fields",
                    ov.field_key
                ));
            }
            if !field.shared {
                return Err(format!(
                    "field {}: timeline contribution must be shared",
                    ov.field_key
                ));
            }
            if matches!(
                timeline.role,
                TimelineFieldRole::Start | TimelineFieldRole::End
            ) && timeline
                .group
                .as_deref()
                .is_none_or(|group| group.trim().is_empty())
            {
                return Err(format!(
                    "field {}: timeline start/end contribution must declare a group",
                    ov.field_key
                ));
            }
            if timeline
                .label
                .as_deref()
                .is_some_and(|label| label.trim().is_empty())
            {
                return Err(format!(
                    "field {}: timeline contribution label cannot be empty",
                    ov.field_key
                ));
            }
        }
    }

    // Validate fieldMetadataOverrides (builtin relationship metadata extensions)
    let mut metadata_override_keys = BTreeSet::new();
    for ov in &overlay.field_metadata_overrides {
        if !package_fields.contains(ov.field_key.as_str()) {
            return Err(format!(
                "field metadata override references unknown builtin field: {}",
                ov.field_key
            ));
        }
        if overlay
            .disabled_fields
            .iter()
            .any(|disabled| disabled == &ov.field_key)
        {
            return Err(format!(
                "field metadata override references disabled field: {}",
                ov.field_key
            ));
        }
        if !metadata_override_keys.insert(ov.field_key.as_str()) {
            return Err(format!(
                "duplicate field metadata override: {}",
                ov.field_key
            ));
        }
        let builtin_field = package_schema
            .fields
            .iter()
            .find(|f| f.key == ov.field_key)
            .expect("package field was checked above");
        if builtin_field.field_type != "relationship" {
            return Err(format!(
                "field metadata override is only allowed for relationship fields: {}",
                ov.field_key
            ));
        }
        if ov.metadata_fields.is_empty() {
            return Err(format!(
                "field metadata override requires at least one metadata field: {}",
                ov.field_key
            ));
        }
        validate_metadata_fields("relationship", &ov.field_key, Some(&ov.metadata_fields))?;
        // Check for conflicting type with builtin metadata for same key (additive merge)
        if let Some(existing) = builtin_field.metadata_fields.as_deref() {
            for new_field in &ov.metadata_fields {
                if let Some(prev) = existing.iter().find(|f| f.key == new_field.key) {
                    if prev.field_type != new_field.field_type {
                        return Err(format!(
                            "conflicting relationship metadata field type for {} in override of {}: expected {}, got {}",
                            new_field.key, ov.field_key, prev.field_type, new_field.field_type
                        ));
                    }
                }
            }
        }
    }

    let effective_fields: BTreeSet<&str> = package_fields
        .iter()
        .copied()
        .filter(|key| {
            !overlay
                .disabled_fields
                .iter()
                .any(|disabled| disabled == key)
        })
        .chain(custom_field_keys.iter().copied())
        .collect();

    let field_applies_to = |key: &str, entity_type: &str| {
        if let Some(scope) = overlay
            .field_scope_overrides
            .iter()
            .find(|scope| scope.field_key == key)
        {
            return scope
                .entity_types
                .iter()
                .any(|candidate| candidate == entity_type);
        }
        package_schema
            .fields
            .iter()
            .chain(overlay.custom_fields.iter())
            .find(|field| field.key == key)
            .is_some_and(|field| {
                field
                    .entity_types
                    .as_ref()
                    .is_none_or(|types| types.iter().any(|candidate| candidate == entity_type))
            })
    };

    let mut custom_template_ids = BTreeSet::new();
    for template in &overlay.custom_templates {
        if template.id.trim().is_empty() || template.name.trim().is_empty() {
            return Err("custom templates require id and name".into());
        }
        if !is_template_id(&template.id) {
            return Err(format!("invalid custom template id: {}", template.id));
        }
        if package_templates.contains(template.id.as_str()) {
            return Err(format!(
                "custom template collides with builtin template: {}",
                template.id
            ));
        }
        if !custom_template_ids.insert(template.id.as_str()) {
            return Err(format!("duplicate custom template id: {}", template.id));
        }
        if let Some(icon) = &template.icon {
            validate_icon_ref(icon).map_err(|error| error.0)?;
        }
        if !effective_types.contains(template.entity_type.as_str()) {
            return Err(format!(
                "custom template {} references unknown entity type: {}",
                template.id, template.entity_type
            ));
        }
        let fields = template
            .fields
            .as_object()
            .ok_or_else(|| format!("custom template {} fields must be an object", template.id))?;
        for key in fields.keys() {
            if !effective_fields.contains(key.as_str()) {
                return Err(format!(
                    "custom template {} references unknown field: {key}",
                    template.id
                ));
            }
            if !field_applies_to(key, &template.entity_type) {
                return Err(format!(
                    "custom template {} references field outside its entity type: {key}",
                    template.id
                ));
            }
        }
        if let Some(required) = &template.required_fields {
            for key in required {
                if !fields.contains_key(key) {
                    return Err(format!(
                        "custom template {} requiredFields entry missing from fields: {key}",
                        template.id
                    ));
                }
            }
        }
    }

    let mut template_override_ids = BTreeSet::new();
    for template in &overlay.template_overrides {
        if !package_templates.contains(template.template_id.as_str()) {
            return Err(format!(
                "template override references unknown builtin template: {}",
                template.template_id
            ));
        }
        if overlay
            .disabled_templates
            .iter()
            .any(|disabled| disabled == &template.template_id)
        {
            return Err(format!(
                "template override references disabled template: {}",
                template.template_id
            ));
        }
        if !template_override_ids.insert(template.template_id.as_str()) {
            return Err(format!(
                "duplicate template override: {}",
                template.template_id
            ));
        }
        let package_template = package
            .templates
            .iter()
            .find(|candidate| candidate.id == template.template_id)
            .expect("package template was checked above");
        if !effective_types.contains(package_template.entity_type.as_str()) {
            return Err(format!(
                "template override references disabled entity type: {}",
                template.template_id
            ));
        }
        let fields = template.fields.as_object().ok_or_else(|| {
            format!(
                "template override {} fields must be an object",
                template.template_id
            )
        })?;
        for key in fields.keys() {
            if !effective_fields.contains(key.as_str()) {
                return Err(format!(
                    "template override {} references unknown field: {key}",
                    template.template_id
                ));
            }
            if !field_applies_to(key, &package_template.entity_type) {
                return Err(format!(
                    "template override {} references field outside its entity type: {key}",
                    template.template_id
                ));
            }
        }
        if let Some(required) = &template.required_fields {
            if unique_len(required) != required.len() {
                return Err(format!(
                    "template override {} requiredFields must be unique",
                    template.template_id
                ));
            }
            for key in required {
                if !fields.contains_key(key) {
                    return Err(format!(
                        "template override {} requiredFields entry missing from fields: {key}",
                        template.template_id
                    ));
                }
            }
        }
    }

    let mut appearance_override_ids = BTreeSet::new();
    for appearance in &overlay.entity_type_appearance_overrides {
        if !is_entity_type_id(&appearance.entity_type_id) {
            return Err(format!(
                "invalid entity type appearance override id: {}",
                appearance.entity_type_id
            ));
        }
        if appearance.icon.is_none() && appearance.icon_color.is_none() {
            return Err(format!(
                "entity type appearance override {} must set icon or iconColor",
                appearance.entity_type_id
            ));
        }
        if !appearance_override_ids.insert(appearance.entity_type_id.as_str()) {
            return Err(format!(
                "duplicate entity type appearance override: {}",
                appearance.entity_type_id
            ));
        }
        if !effective_types.contains(appearance.entity_type_id.as_str()) {
            return Err(format!(
                "entity type appearance override references unknown type: {}",
                appearance.entity_type_id
            ));
        }
        if let Some(icon) = &appearance.icon {
            validate_icon_ref(icon).map_err(|error| error.0)?;
        }
        if let Some(color) = &appearance.icon_color {
            validate_entity_type_color(color).map_err(|error| error.0)?;
        }
    }

    Ok(())
}

/// Merge packaged schemas/templates with a project overlay.
pub fn merge_module_manifest(
    package: &PluginManifest,
    overlay: &ModuleSchemaOverlay,
) -> Result<PluginManifest, String> {
    let overlay = overlay_with_qualified_types(package, overlay)?;
    validate_module_overlay(package, &overlay)?;
    let namespace = primary_schema_namespace(package)?.to_string();
    let mut merged = package.clone();
    let Some(schema) = merged
        .schemas
        .iter_mut()
        .find(|schema| schema.namespace == namespace)
    else {
        return Err(format!("packaged schema is missing for {}", package.id));
    };

    schema.entity_types.retain(|entity_type| {
        !overlay
            .disabled_entity_types
            .iter()
            .any(|disabled| disabled == &entity_type.id)
    });
    for entity_type in &overlay.custom_entity_types {
        if !schema
            .entity_types
            .iter()
            .any(|existing| existing.id == entity_type.id)
        {
            schema.entity_types.push(entity_type.clone());
        }
    }
    schema
        .entity_types
        .sort_by(|left, right| left.id.cmp(&right.id));
    schema
        .entity_types
        .dedup_by(|left, right| left.id == right.id);

    for appearance in &overlay.entity_type_appearance_overrides {
        if let Some(entity_type) = schema
            .entity_types
            .iter_mut()
            .find(|entity_type| entity_type.id == appearance.entity_type_id)
        {
            if let Some(icon) = &appearance.icon {
                entity_type.icon = icon.clone();
            }
            if let Some(color) = &appearance.icon_color {
                entity_type.icon_color = color.clone();
            }
        }
    }

    schema.fields.retain(|field| {
        !overlay
            .disabled_fields
            .iter()
            .any(|disabled| disabled == &field.key)
    });
    schema.fields.extend(overlay.custom_fields.clone());
    for field in &mut schema.fields {
        if let Some(ov) = overlay
            .field_timeline_overrides
            .iter()
            .find(|ov| ov.field_key == field.key)
        {
            field.timeline = ov.timeline.clone();
        }
        if let Some(scope) = overlay
            .field_scope_overrides
            .iter()
            .find(|scope| scope.field_key == field.key)
        {
            field.entity_types = Some(scope.entity_types.clone());
        }
        if let Some(ov) = overlay
            .field_metadata_overrides
            .iter()
            .find(|ov| ov.field_key == field.key)
        {
            // Additive merge: builtin + override (override wins for duplicate key)
            let mut merged: std::collections::BTreeMap<String, MetadataFieldDefinition> =
                std::collections::BTreeMap::new();
            if let Some(existing) = field.metadata_fields.clone() {
                for mf in existing {
                    merged.insert(mf.key.clone(), mf);
                }
            }
            for mf in &ov.metadata_fields {
                merged.insert(mf.key.clone(), mf.clone());
            }
            let mut vals: Vec<MetadataFieldDefinition> = merged.into_values().collect();
            vals.sort_by(|a, b| a.key.cmp(&b.key));
            field.metadata_fields = Some(vals);
        }
    }

    merged.templates.retain(|template| {
        !overlay
            .disabled_templates
            .iter()
            .any(|disabled| disabled == &template.id)
            && schema
                .entity_types
                .iter()
                .any(|entity_type| entity_type.id == template.entity_type)
    });
    for template in &mut merged.templates {
        if let Some(fields) = template.fields.as_object_mut() {
            fields.retain(|key, _| {
                schema
                    .fields
                    .iter()
                    .find(|field| field.key == *key)
                    .is_some_and(|field| {
                        field.entity_types.as_ref().is_none_or(|types| {
                            types.iter().any(|type_id| type_id == &template.entity_type)
                        })
                    })
            });
            if let Some(required) = &mut template.required_fields {
                required.retain(|key| fields.contains_key(key));
            }
        }
        if let Some(override_template) = overlay
            .template_overrides
            .iter()
            .find(|override_template| override_template.template_id == template.id)
        {
            template.fields = override_template.fields.clone();
            template.required_fields = override_template.required_fields.clone();
        }
    }
    merged.templates.extend(overlay.custom_templates.clone());

    Ok(merged)
}

fn unique_len(values: &[String]) -> usize {
    values.iter().collect::<BTreeSet<_>>().len()
}

fn default_entity_type_color() -> EntityTypeColor {
    EntityTypeColor::Preset { id: "brass".into() }
}

fn migrate_overlay_custom_entity_type_colors(value: &mut serde_json::Value) {
    let Some(custom_types) = value
        .as_object_mut()
        .and_then(|object| object.get_mut("customEntityTypes"))
        .and_then(|entry| entry.as_array_mut())
    else {
        return;
    };
    let default_color = serde_json::to_value(default_entity_type_color())
        .expect("default entity type color serializes");
    for entry in custom_types {
        if let Some(object) = entry.as_object_mut() {
            object
                .entry("iconColor".to_string())
                .or_insert(default_color.clone());
        }
    }
}

/// Parse overlay JSON; empty object becomes default empty overlay.
pub fn parse_module_overlay(value: &serde_json::Value) -> Result<ModuleSchemaOverlay, String> {
    if value.is_null() {
        return Ok(ModuleSchemaOverlay::default());
    }
    if value.as_object().is_some_and(|object| object.is_empty()) {
        return Ok(ModuleSchemaOverlay::default());
    }
    let mut migrated = value.clone();
    migrate_overlay_custom_entity_type_colors(&mut migrated);
    let mut overlay: ModuleSchemaOverlay =
        serde_json::from_value(migrated).map_err(|error| error.to_string())?;
    if overlay.version == 0 {
        overlay.version = SCHEMA_OVERLAY_VERSION;
    }
    Ok(overlay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_manifest;

    fn lore_manifest() -> PluginManifest {
        parse_manifest(include_str!("../../../packages/modules/lore/manifest.json"))
            .expect("lore manifest")
    }

    fn timeline_manifest() -> PluginManifest {
        parse_manifest(include_str!(
            "../../../packages/modules/timeline/manifest.json"
        ))
        .expect("timeline manifest")
    }

    fn family_tree_manifest() -> PluginManifest {
        parse_manifest(include_str!(
            "../../../packages/modules/family-tree/manifest.json"
        ))
        .expect("family tree manifest")
    }

    fn writing_manifest() -> PluginManifest {
        parse_manifest(include_str!(
            "../../../packages/modules/writing/manifest.json"
        ))
        .expect("writing manifest")
    }

    #[test]
    fn rejects_custom_field_collision() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_fields: vec![FieldDefinition {
                key: "summary".into(),
                label: "Summary".into(),
                field_type: "text".into(),
                required: None,
                options: None,
                entity_types: None,
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("collides"));
    }

    #[test]
    fn allows_custom_field_without_entity_types() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_fields: vec![FieldDefinition {
                key: "lifespan".into(),
                label: "Lifespan".into(),
                field_type: "text".into(),
                required: None,
                options: None,
                entity_types: None,
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay).is_ok());
    }

    #[test]
    fn allows_custom_field_with_empty_entity_types() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_fields: vec![FieldDefinition {
                key: "lifespan".into(),
                label: "Lifespan".into(),
                field_type: "text".into(),
                required: None,
                options: None,
                entity_types: Some(vec![]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay).is_ok());
    }

    #[test]
    fn rejects_plugin_without_schema_overlay_capability() {
        let mut package = lore_manifest();
        package
            .capabilities
            .retain(|capability| capability != SCHEMA_OVERLAY_CAPABILITY);
        let overlay = ModuleSchemaOverlay::default();
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("not supported"));
    }

    #[test]
    fn rejects_maps_without_schema_overlay_capability() {
        let package = parse_manifest(include_str!("../../../packages/modules/maps/manifest.json"))
            .expect("maps manifest");
        assert!(!supports_schema_overlay(&package));
    }

    #[test]
    fn family_tree_supports_schema_overlay() {
        assert!(supports_schema_overlay(&family_tree_manifest()));
    }

    #[test]
    fn allows_family_tree_custom_kinship_on_lore_person() {
        let package = family_tree_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_fields: vec![FieldDefinition {
                key: "godparents".into(),
                label: "Godparents".into(),
                field_type: "relationship".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["daena.lore:person".into()]),
                relationship_type: Some("family_godparent_of".into()),
                target_entity_types: Some(vec!["daena.lore:person".into()]),
                shared: false,
                multiple: false,
                cardinality: Some("many".into()),
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay).is_ok());
    }

    #[test]
    fn rejects_family_tree_custom_field_on_undeclared_dependency() {
        let package = family_tree_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_fields: vec![FieldDefinition {
                key: "ghosts".into(),
                label: "Ghosts".into(),
                field_type: "text".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["daena.ghost:ghost".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("unknown entity type"));
    }

    #[test]
    fn merges_custom_type_and_disables_builtin_template() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            disabled_templates: vec!["concept".into()],
            custom_entity_types: vec![EntityTypeDefinition {
                id: "daena.lore:species".into(),
                name: "Species".into(),
                icon: crate::IconRef::UserSvg {
                    svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 4h16v16H4z"/></svg>"#.into(),
                },
                icon_color: EntityTypeColor::Preset {
                    id: "moss".into(),
                },
            }],
            custom_fields: vec![FieldDefinition {
                key: "lifespan".into(),
                label: "Lifespan".into(),
                field_type: "text".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["daena.lore:species".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            custom_templates: vec![EntityTemplate {
                id: "species".into(),
                name: "Species".into(),
                entity_type: "daena.lore:species".into(),
                description: Some("A kind of being.".into()),
                icon: None,
                fields: serde_json::json!({ "summary": "", "lifespan": "" }),
                required_fields: None,
                document: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        let merged = merge_module_manifest(&package, &overlay).expect("merge");
        let schema = merged
            .schemas
            .iter()
            .find(|schema| schema.namespace == "lore")
            .unwrap();
        assert!(schema
            .entity_types
            .iter()
            .any(|kind| kind.id == "daena.lore:species"));
        assert!(schema.fields.iter().any(|field| field.key == "lifespan"));
        assert!(!merged
            .templates
            .iter()
            .any(|template| template.id == "concept"));
        assert!(merged
            .templates
            .iter()
            .any(|template| template.id == "species"));
        assert_eq!(
            merged
                .templates
                .iter()
                .find(|template| template.id == "species")
                .unwrap()
                .entity_type,
            "daena.lore:species"
        );
    }

    #[test]
    fn rejects_custom_type_that_collides_with_qualified_builtin() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_entity_types: vec![EntityTypeDefinition {
                id: "daena.lore:person".into(),
                name: "Person clone".into(),
                icon: crate::IconRef::Catalog {
                    id: "person".into(),
                },
                icon_color: EntityTypeColor::Preset { id: "rose".into() },
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("collides with builtin type"));
    }

    #[test]
    fn rejects_custom_type_with_foreign_plugin_prefix() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_entity_types: vec![EntityTypeDefinition {
                id: "daena.timeline:event".into(),
                name: "Event".into(),
                icon: crate::IconRef::Catalog { id: "event".into() },
                icon_color: EntityTypeColor::Preset { id: "amber".into() },
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("must be prefixed with plugin id 'daena.lore'"));
    }

    #[test]
    fn merges_timeline_custom_field() {
        let package = timeline_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            disabled_fields: vec!["endsAt".into()],
            custom_fields: vec![FieldDefinition {
                key: "importance".into(),
                label: "Importance".into(),
                field_type: "number".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["daena.timeline:event".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        let merged = merge_module_manifest(&package, &overlay).expect("merge");
        let schema = merged
            .schemas
            .iter()
            .find(|schema| schema.namespace == "timeline")
            .unwrap();
        assert!(!schema.fields.iter().any(|field| field.key == "endsAt"));
        assert!(schema.fields.iter().any(|field| field.key == "importance"));
    }

    #[test]
    fn merges_builtin_field_scope_and_template_selection() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            field_scope_overrides: vec![FieldScopeOverride {
                field_key: "aliases".into(),
                entity_types: vec!["daena.lore:person".into(), "daena.lore:faction".into()],
            }],
            template_overrides: vec![TemplateOverride {
                template_id: "person".into(),
                fields: serde_json::json!({ "summary": "", "aliases": "", "occupation": "" }),
                required_fields: Some(vec!["occupation".into()]),
            }],
            ..ModuleSchemaOverlay::default()
        };

        let merged = merge_module_manifest(&package, &overlay).expect("merge");
        let schema = merged
            .schemas
            .iter()
            .find(|schema| schema.namespace == "lore")
            .unwrap();
        assert_eq!(
            schema
                .fields
                .iter()
                .find(|field| field.key == "aliases")
                .and_then(|field| field.entity_types.clone()),
            Some(vec![
                "daena.lore:person".into(),
                "daena.lore:faction".into()
            ])
        );
        let person = merged
            .templates
            .iter()
            .find(|template| template.id == "person")
            .unwrap();
        assert_eq!(
            person.fields,
            serde_json::json!({ "summary": "", "aliases": "", "occupation": "" })
        );
        assert_eq!(person.required_fields, Some(vec!["occupation".into()]));
    }

    #[test]
    fn rejects_template_override_field_outside_its_scope() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            field_scope_overrides: vec![FieldScopeOverride {
                field_key: "aliases".into(),
                entity_types: vec!["daena.lore:faction".into()],
            }],
            template_overrides: vec![TemplateOverride {
                template_id: "person".into(),
                fields: serde_json::json!({ "aliases": "" }),
                required_fields: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("outside its entity type"));
    }

    #[test]
    fn merges_writing_custom_template() {
        let package = writing_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_entity_types: vec![EntityTypeDefinition {
                id: "daena.writing:chapter".into(),
                name: "Chapter".into(),
                icon: crate::IconRef::Catalog {
                    id: "manuscript".into(),
                },
                icon_color: EntityTypeColor::Preset { id: "ink".into() },
            }],
            custom_fields: vec![FieldDefinition {
                key: "wordCount".into(),
                label: "Word count".into(),
                field_type: "number".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["daena.writing:chapter".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            custom_templates: vec![EntityTemplate {
                id: "chapter".into(),
                name: "Chapter".into(),
                entity_type: "daena.writing:chapter".into(),
                description: Some("A chapter draft.".into()),
                icon: None,
                fields: serde_json::json!({ "wordCount": 0 }),
                required_fields: None,
                document: Some("".into()),
            }],
            ..ModuleSchemaOverlay::default()
        };
        let merged = merge_module_manifest(&package, &overlay).expect("merge");
        let schema = merged
            .schemas
            .iter()
            .find(|schema| schema.namespace == "writing")
            .unwrap();
        assert!(schema
            .entity_types
            .iter()
            .any(|kind| kind.id == "daena.writing:chapter"));
        assert!(merged
            .templates
            .iter()
            .any(|template| template.id == "chapter"));
    }

    #[test]
    fn merges_builtin_relationship_metadata_override() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            field_metadata_overrides: vec![FieldMetadataOverride {
                field_key: "affiliation".into(),
                metadata_fields: vec![MetadataFieldDefinition {
                    key: "role".into(),
                    label: "Role".into(),
                    field_type: "text".into(),
                    required: None,
                    options: None,
                    one_of: None,
                }],
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay).is_ok());
        let merged = merge_module_manifest(&package, &overlay).expect("merge");
        let schema = merged
            .schemas
            .iter()
            .find(|s| s.namespace == "lore")
            .unwrap();
        let affiliation = schema
            .fields
            .iter()
            .find(|f| f.key == "affiliation")
            .unwrap();
        let meta = affiliation.metadata_fields.as_ref().unwrap();
        assert!(meta.iter().any(|m| m.key == "role"));
        assert!(meta.iter().any(|m| m.key == "start"));
        assert!(meta.iter().any(|m| m.key == "end"));
    }

    #[test]
    fn rejects_metadata_override_on_non_relationship() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            field_metadata_overrides: vec![FieldMetadataOverride {
                field_key: "summary".into(),
                metadata_fields: vec![MetadataFieldDefinition {
                    key: "note".into(),
                    label: "Note".into(),
                    field_type: "text".into(),
                    required: None,
                    options: None,
                    one_of: None,
                }],
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("relationship"));
    }

    #[test]
    fn rejects_duplicate_metadata_override() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            field_metadata_overrides: vec![
                FieldMetadataOverride {
                    field_key: "affiliation".into(),
                    metadata_fields: vec![MetadataFieldDefinition {
                        key: "role".into(),
                        label: "Role".into(),
                        field_type: "text".into(),
                        required: None,
                        options: None,
                        one_of: None,
                    }],
                },
                FieldMetadataOverride {
                    field_key: "affiliation".into(),
                    metadata_fields: vec![MetadataFieldDefinition {
                        key: "other".into(),
                        label: "Other".into(),
                        field_type: "text".into(),
                        required: None,
                        options: None,
                        one_of: None,
                    }],
                },
            ],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("duplicate field metadata override"));
    }

    #[test]
    fn parse_module_overlay_migrates_missing_icon_color_on_custom_types() {
        let value = serde_json::json!({
            "version": 1,
            "customEntityTypes": [{
                "id": "chapter",
                "name": "Chapter",
                "icon": { "kind": "catalog", "id": "reference" }
            }]
        });
        let overlay = parse_module_overlay(&value).expect("parse legacy overlay");
        assert_eq!(overlay.custom_entity_types.len(), 1);
        assert_eq!(
            overlay.custom_entity_types[0].icon_color,
            EntityTypeColor::Preset { id: "brass".into() }
        );
    }

    #[test]
    fn rejects_appearance_override_for_disabled_entity_type() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            disabled_entity_types: vec!["daena.lore:person".into()],
            entity_type_appearance_overrides: vec![EntityTypeAppearanceOverride {
                entity_type_id: "daena.lore:person".into(),
                icon: Some(crate::IconRef::Catalog {
                    id: "person".into(),
                }),
                icon_color: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("entity type appearance override references unknown type"));
    }

    #[test]
    fn custom_date_field_timeline_requires_shared_and_group() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_fields: vec![FieldDefinition {
                key: "myDate".into(),
                label: "My Date".into(),
                field_type: "date".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["daena.lore:person".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: Some(TimelineFieldContribution {
                    role: TimelineFieldRole::Point,
                    group: None,
                    label: None,
                    layer: Some(TimelineFieldLayer::Dates),
                }),
                relationship_constraints: None,
                relationship_direction: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("must be shared"));
        let mut overlay = overlay;
        overlay.custom_fields[0].shared = true;
        overlay.custom_fields[0].field_type = "text".into();
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("only allowed for date"));
        overlay.custom_fields[0].field_type = "date".into();
        overlay.custom_fields[0].timeline = Some(TimelineFieldContribution {
            role: TimelineFieldRole::Start,
            group: None,
            label: None,
            layer: None,
        });
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("must declare a group"));
        overlay.custom_fields[0].timeline = Some(TimelineFieldContribution {
            role: TimelineFieldRole::Start,
            group: Some("life".into()),
            label: Some("".into()),
            layer: None,
        });
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("label cannot be empty"));
    }

    #[test]
    fn timeline_override_enables_and_disables_builtin() {
        let package = lore_manifest();
        // disable builtin birth timeline
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            field_timeline_overrides: vec![FieldTimelineOverride {
                field_key: "birth".into(),
                timeline: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay).is_ok());
        let merged = merge_module_manifest(&package, &overlay).expect("merge");
        let birth = merged
            .schemas
            .iter()
            .find(|s| s.namespace == "lore")
            .unwrap()
            .fields
            .iter()
            .find(|f| f.key == "birth")
            .unwrap();
        assert!(birth.timeline.is_none());

        // enable timeline for custom date field via override (alternative to direct timeline)
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_fields: vec![FieldDefinition {
                key: "founded".into(),
                label: "Founded".into(),
                field_type: "date".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["daena.lore:faction".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: true,
                multiple: false,
                cardinality: None,
                one_of: None,
                metadata_fields: None,
                timeline: None,
                relationship_constraints: None,
                relationship_direction: None,
            }],
            field_timeline_overrides: vec![FieldTimelineOverride {
                field_key: "founded".into(),
                timeline: Some(TimelineFieldContribution {
                    role: TimelineFieldRole::Point,
                    group: None,
                    label: Some("Founded".into()),
                    layer: Some(TimelineFieldLayer::Dates),
                }),
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay).is_ok());
        let merged = merge_module_manifest(&package, &overlay).expect("merge");
        let founded = merged
            .schemas
            .iter()
            .find(|s| s.namespace == "lore")
            .unwrap()
            .fields
            .iter()
            .find(|f| f.key == "founded")
            .unwrap();
        assert!(founded.timeline.is_some());
        assert_eq!(
            founded.timeline.as_ref().unwrap().role,
            TimelineFieldRole::Point
        );
    }

    #[test]
    fn rejects_timeline_override_for_non_date_or_disabled() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            field_timeline_overrides: vec![FieldTimelineOverride {
                field_key: "summary".into(),
                timeline: Some(TimelineFieldContribution {
                    role: TimelineFieldRole::Point,
                    group: None,
                    label: None,
                    layer: None,
                }),
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("only allowed for date"));
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            disabled_fields: vec!["birth".into()],
            field_timeline_overrides: vec![FieldTimelineOverride {
                field_key: "birth".into(),
                timeline: None,
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("references disabled field"));
    }
}
