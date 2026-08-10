//! Project-owned module schema overlays (host-side customization of package defaults).

use crate::{EntityTemplate, FieldDefinition, PluginManifest};
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
    "entity-ref",
    "relationship",
];

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
    pub custom_entity_types: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_fields: Vec<FieldDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_templates: Vec<EntityTemplate>,
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
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_entity_type_id(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
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

    let namespace = primary_schema_namespace(package)?;
    let package_schema = package
        .schemas
        .iter()
        .find(|schema| schema.namespace == namespace)
        .ok_or_else(|| format!("packaged schema is missing for {}", package.id))?;
    let package_types: BTreeSet<&str> = package_schema
        .entity_types
        .iter()
        .map(String::as_str)
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
            return Err(format!("cannot disable unknown builtin entity type: {name}"));
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

    for name in &overlay.custom_entity_types {
        if !is_entity_type_id(name) {
            return Err(format!("invalid custom entity type: {name}"));
        }
        if package_types.contains(name.as_str()) {
            return Err(format!(
                "custom entity type collides with builtin type: {name}"
            ));
        }
    }
    if unique_len(&overlay.custom_entity_types) != overlay.custom_entity_types.len() {
        return Err("customEntityTypes must be unique".into());
    }

    let effective_types: BTreeSet<&str> = package_types
        .iter()
        .copied()
        .filter(|name| !overlay.disabled_entity_types.iter().any(|disabled| disabled == name))
        .chain(overlay.custom_entity_types.iter().map(String::as_str))
        .collect();

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
        let entity_types = field.entity_types.as_ref().ok_or_else(|| {
            format!(
                "custom field requires at least one entity type: {}",
                field.key
            )
        })?;
        if entity_types.is_empty() {
            return Err(format!(
                "custom field requires at least one entity type: {}",
                field.key
            ));
        }
        for entity_type in entity_types {
            if !effective_types.contains(entity_type.as_str()) {
                return Err(format!(
                    "custom field {} references unknown entity type: {entity_type}",
                    field.key
                ));
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
                if !effective_types.contains(target.as_str()) {
                    return Err(format!(
                        "custom field {} references unknown target type: {target}",
                        field.key
                    ));
                }
            }
        }
        if field.field_type == "enum" {
            let options = field.options.as_ref().ok_or_else(|| {
                format!("custom enum field requires options: {}", field.key)
            })?;
            if options.is_empty() {
                return Err(format!(
                    "custom enum field options must be non-empty: {}",
                    field.key
                ));
            }
        }
    }

    let effective_fields: BTreeSet<&str> = package_fields
        .iter()
        .copied()
        .filter(|key| !overlay.disabled_fields.iter().any(|disabled| disabled == key))
        .chain(custom_field_keys.iter().copied())
        .collect();

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

    Ok(())
}

/// Merge packaged schemas/templates with a project overlay.
pub fn merge_module_manifest(
    package: &PluginManifest,
    overlay: &ModuleSchemaOverlay,
) -> Result<PluginManifest, String> {
    validate_module_overlay(package, overlay)?;
    let namespace = primary_schema_namespace(package)?.to_string();
    let mut merged = package.clone();
    let Some(schema) = merged
        .schemas
        .iter_mut()
        .find(|schema| schema.namespace == namespace)
    else {
        return Err(format!("packaged schema is missing for {}", package.id));
    };

    schema.entity_types.retain(|name| {
        !overlay
            .disabled_entity_types
            .iter()
            .any(|disabled| disabled == name)
    });
    for name in &overlay.custom_entity_types {
        if !schema.entity_types.iter().any(|existing| existing == name) {
            schema.entity_types.push(name.clone());
        }
    }
    schema.entity_types.sort();
    schema.entity_types.dedup();

    schema.fields.retain(|field| {
        !overlay
            .disabled_fields
            .iter()
            .any(|disabled| disabled == &field.key)
    });
    schema.fields.extend(overlay.custom_fields.clone());

    merged.templates.retain(|template| {
        !overlay
            .disabled_templates
            .iter()
            .any(|disabled| disabled == &template.id)
            && schema
                .entity_types
                .iter()
                .any(|entity_type| entity_type == &template.entity_type)
    });
    merged
        .templates
        .extend(overlay.custom_templates.clone());

    Ok(merged)
}

fn unique_len(values: &[String]) -> usize {
    values.iter().collect::<BTreeSet<_>>().len()
}

/// Parse overlay JSON; empty object becomes default empty overlay.
pub fn parse_module_overlay(value: &serde_json::Value) -> Result<ModuleSchemaOverlay, String> {
    if value.is_null() {
        return Ok(ModuleSchemaOverlay::default());
    }
    if value.as_object().is_some_and(|object| object.is_empty()) {
        return Ok(ModuleSchemaOverlay::default());
    }
    let mut overlay: ModuleSchemaOverlay =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
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
        parse_manifest(include_str!(
            "../../../packages/modules/lore/manifest.json"
        ))
        .expect("lore manifest")
    }

    fn timeline_manifest() -> PluginManifest {
        parse_manifest(include_str!(
            "../../../packages/modules/timeline/manifest.json"
        ))
        .expect("timeline manifest")
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
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("collides"));
    }

    #[test]
    fn rejects_custom_field_without_entity_types() {
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
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("at least one entity type"));
    }

    #[test]
    fn rejects_custom_field_with_empty_entity_types() {
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
            }],
            ..ModuleSchemaOverlay::default()
        };
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("at least one entity type"));
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
        let package = parse_manifest(include_str!(
            "../../../packages/modules/maps/manifest.json"
        ))
        .expect("maps manifest");
        assert!(!supports_schema_overlay(&package));
        let overlay = ModuleSchemaOverlay::default();
        assert!(validate_module_overlay(&package, &overlay)
            .unwrap_err()
            .contains("not supported"));
    }

    #[test]
    fn merges_custom_type_and_disables_builtin_template() {
        let package = lore_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            disabled_templates: vec!["concept".into()],
            custom_entity_types: vec!["species".into()],
            custom_fields: vec![FieldDefinition {
                key: "lifespan".into(),
                label: "Lifespan".into(),
                field_type: "text".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["species".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
            }],
            custom_templates: vec![EntityTemplate {
                id: "species".into(),
                name: "Species".into(),
                entity_type: "species".into(),
                description: Some("A kind of being.".into()),
                icon: Some("S".into()),
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
        assert!(schema.entity_types.iter().any(|name| name == "species"));
        assert!(schema.fields.iter().any(|field| field.key == "lifespan"));
        assert!(!merged.templates.iter().any(|template| template.id == "concept"));
        assert!(merged.templates.iter().any(|template| template.id == "species"));
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
                entity_types: Some(vec!["event".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
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
    fn merges_writing_custom_template() {
        let package = writing_manifest();
        let overlay = ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_entity_types: vec!["chapter".into()],
            custom_fields: vec![FieldDefinition {
                key: "wordCount".into(),
                label: "Word count".into(),
                field_type: "number".into(),
                required: None,
                options: None,
                entity_types: Some(vec!["chapter".into()]),
                relationship_type: None,
                target_entity_types: None,
                shared: false,
                multiple: false,
            }],
            custom_templates: vec![EntityTemplate {
                id: "chapter".into(),
                name: "Chapter".into(),
                entity_type: "chapter".into(),
                description: Some("A chapter draft.".into()),
                icon: Some("C".into()),
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
        assert!(schema.entity_types.iter().any(|name| name == "chapter"));
        assert!(merged.templates.iter().any(|template| template.id == "chapter"));
    }
}
