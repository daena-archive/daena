//! Read-only schema overlay impact preview models and pure diff helpers.
//!
//! Live entity/field counts are filled by the trusted core; this module only
//! classifies overlay deltas and structures validation/impact results.

use crate::schema_overlay::{qualify_module_overlay, validate_module_overlay, ModuleSchemaOverlay};
use crate::PluginManifest;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Author-facing change classification for a candidate overlay.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaOverlayChangeKind {
    /// Only additions or purely additive appearance/template edits.
    Additive,
    /// Disables or hides packaged items without removing live custom types that still have entities.
    HidingOnly,
    /// Removes or disables types/fields that may require entity or field disposition.
    RequiresReassignment,
}

/// Stable issue targeting a schema item (or the overlay as a whole).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOverlayItemIssue {
    /// `type` | `field` | `template` | `overlay`
    pub kind: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    pub message: String,
}

/// Live-data impact for an affected entity type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOverlayTypeImpact {
    pub entity_type: String,
    /// `disabled` | `removed` | `added` | `appearance`
    pub change: String,
    pub entity_count: u64,
}

/// Live-data impact for an affected field key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOverlayFieldImpact {
    pub field_key: String,
    /// `disabled` | `removed` | `added` | `scope-changed` | `metadata-changed` | `timeline-changed`
    pub change: String,
    pub value_count: u64,
}

/// Structured, non-mutating preview of applying a candidate overlay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOverlayPreviewResult {
    pub ok: bool,
    pub change_kind: SchemaOverlayChangeKind,
    /// True when Save must show an impact review (live rows are affected).
    pub requires_acknowledgement: bool,
    pub errors: Vec<SchemaOverlayItemIssue>,
    pub warnings: Vec<SchemaOverlayItemIssue>,
    pub affected_types: Vec<SchemaOverlayTypeImpact>,
    pub affected_fields: Vec<SchemaOverlayFieldImpact>,
    pub affected_templates: Vec<String>,
    pub relationship_metadata_keys: Vec<String>,
    pub compatibility_notes: Vec<String>,
    /// Custom types removed while live entities still use them.
    pub unresolved_type_removals: Vec<String>,
}

/// Pure overlay delta before live counts are attached.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaOverlayDiff {
    pub disabled_types: Vec<String>,
    pub removed_custom_types: Vec<String>,
    pub added_custom_types: Vec<String>,
    pub appearance_types: Vec<String>,
    pub disabled_fields: Vec<String>,
    pub removed_custom_fields: Vec<String>,
    pub added_custom_fields: Vec<String>,
    pub scope_changed_fields: Vec<String>,
    pub metadata_changed_fields: Vec<String>,
    pub timeline_changed_fields: Vec<String>,
    pub disabled_templates: Vec<String>,
    pub removed_custom_templates: Vec<String>,
    pub added_custom_templates: Vec<String>,
    pub template_overrides_changed: Vec<String>,
}

impl SchemaOverlayDiff {
    pub fn is_empty(&self) -> bool {
        self.disabled_types.is_empty()
            && self.removed_custom_types.is_empty()
            && self.added_custom_types.is_empty()
            && self.appearance_types.is_empty()
            && self.disabled_fields.is_empty()
            && self.removed_custom_fields.is_empty()
            && self.added_custom_fields.is_empty()
            && self.scope_changed_fields.is_empty()
            && self.metadata_changed_fields.is_empty()
            && self.timeline_changed_fields.is_empty()
            && self.disabled_templates.is_empty()
            && self.removed_custom_templates.is_empty()
            && self.added_custom_templates.is_empty()
            && self.template_overrides_changed.is_empty()
    }

    pub fn change_kind(&self) -> SchemaOverlayChangeKind {
        if !self.removed_custom_types.is_empty()
            || !self.removed_custom_fields.is_empty()
            || !self.disabled_types.is_empty()
            || !self.disabled_fields.is_empty()
            || !self.scope_changed_fields.is_empty()
        {
            return SchemaOverlayChangeKind::RequiresReassignment;
        }
        if !self.disabled_templates.is_empty()
            || !self.removed_custom_templates.is_empty()
            || !self.metadata_changed_fields.is_empty()
            || !self.timeline_changed_fields.is_empty()
        {
            return SchemaOverlayChangeKind::HidingOnly;
        }
        SchemaOverlayChangeKind::Additive
    }

    pub fn type_ids_needing_counts(&self) -> Vec<String> {
        let mut ids = BTreeSet::new();
        ids.extend(self.disabled_types.iter().cloned());
        ids.extend(self.removed_custom_types.iter().cloned());
        ids.into_iter().collect()
    }

    pub fn field_keys_needing_counts(&self) -> Vec<String> {
        let mut keys = BTreeSet::new();
        keys.extend(self.disabled_fields.iter().cloned());
        keys.extend(self.removed_custom_fields.iter().cloned());
        keys.extend(self.scope_changed_fields.iter().cloned());
        keys.extend(self.metadata_changed_fields.iter().cloned());
        keys.extend(self.timeline_changed_fields.iter().cloned());
        keys.into_iter().collect()
    }
}

fn set_of(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn sorted_added(current: &BTreeSet<&str>, next: &BTreeSet<&str>) -> Vec<String> {
    next.difference(current)
        .map(|value| (*value).to_string())
        .collect()
}

fn sorted_removed(current: &BTreeSet<&str>, next: &BTreeSet<&str>) -> Vec<String> {
    current
        .difference(next)
        .map(|value| (*value).to_string())
        .collect()
}

/// Diff `current` → `candidate` overlay (both should already be qualified).
pub fn diff_module_schema_overlays(
    current: &ModuleSchemaOverlay,
    candidate: &ModuleSchemaOverlay,
) -> SchemaOverlayDiff {
    let current_disabled_types = set_of(&current.disabled_entity_types);
    let next_disabled_types = set_of(&candidate.disabled_entity_types);
    let current_custom_types: BTreeSet<&str> = current
        .custom_entity_types
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let next_custom_types: BTreeSet<&str> = candidate
        .custom_entity_types
        .iter()
        .map(|item| item.id.as_str())
        .collect();

    let current_disabled_fields = set_of(&current.disabled_fields);
    let next_disabled_fields = set_of(&candidate.disabled_fields);
    let current_custom_fields: BTreeMap<&str, &crate::FieldDefinition> = current
        .custom_fields
        .iter()
        .map(|field| (field.key.as_str(), field))
        .collect();
    let next_custom_fields: BTreeMap<&str, &crate::FieldDefinition> = candidate
        .custom_fields
        .iter()
        .map(|field| (field.key.as_str(), field))
        .collect();

    let current_disabled_templates = set_of(&current.disabled_templates);
    let next_disabled_templates = set_of(&candidate.disabled_templates);
    let current_custom_templates: BTreeSet<&str> = current
        .custom_templates
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    let next_custom_templates: BTreeSet<&str> = candidate
        .custom_templates
        .iter()
        .map(|item| item.id.as_str())
        .collect();

    let mut appearance_types = BTreeSet::new();
    let current_appearance: BTreeMap<&str, _> = current
        .entity_type_appearance_overrides
        .iter()
        .map(|item| (item.entity_type_id.as_str(), item))
        .collect();
    let next_appearance: BTreeMap<&str, _> = candidate
        .entity_type_appearance_overrides
        .iter()
        .map(|item| (item.entity_type_id.as_str(), item))
        .collect();
    for (id, next) in &next_appearance {
        if current_appearance.get(id).map(|value| *value) != Some(next) {
            appearance_types.insert((*id).to_string());
        }
    }
    for id in current_appearance.keys() {
        if !next_appearance.contains_key(id) {
            appearance_types.insert((*id).to_string());
        }
    }

    let mut scope_changed = BTreeSet::new();
    let current_scopes: BTreeMap<&str, _> = current
        .field_scope_overrides
        .iter()
        .map(|item| (item.field_key.as_str(), item))
        .collect();
    let next_scopes: BTreeMap<&str, _> = candidate
        .field_scope_overrides
        .iter()
        .map(|item| (item.field_key.as_str(), item))
        .collect();
    for (key, next) in &next_scopes {
        if current_scopes.get(key).map(|value| *value) != Some(next) {
            scope_changed.insert((*key).to_string());
        }
    }
    for key in current_scopes.keys() {
        if !next_scopes.contains_key(key) {
            scope_changed.insert((*key).to_string());
        }
    }

    let mut metadata_changed = BTreeSet::new();
    let current_metadata: BTreeMap<&str, _> = current
        .field_metadata_overrides
        .iter()
        .map(|item| (item.field_key.as_str(), item))
        .collect();
    let next_metadata: BTreeMap<&str, _> = candidate
        .field_metadata_overrides
        .iter()
        .map(|item| (item.field_key.as_str(), item))
        .collect();
    for (key, next) in &next_metadata {
        if current_metadata.get(key).map(|value| *value) != Some(next) {
            metadata_changed.insert((*key).to_string());
        }
    }
    for key in current_metadata.keys() {
        if !next_metadata.contains_key(key) {
            metadata_changed.insert((*key).to_string());
        }
    }

    let mut timeline_changed = BTreeSet::new();
    let current_timeline: BTreeMap<&str, _> = current
        .field_timeline_overrides
        .iter()
        .map(|item| (item.field_key.as_str(), item))
        .collect();
    let next_timeline: BTreeMap<&str, _> = candidate
        .field_timeline_overrides
        .iter()
        .map(|item| (item.field_key.as_str(), item))
        .collect();
    for (key, next) in &next_timeline {
        if current_timeline.get(key).map(|value| *value) != Some(next) {
            timeline_changed.insert((*key).to_string());
        }
    }
    for key in current_timeline.keys() {
        if !next_timeline.contains_key(key) {
            timeline_changed.insert((*key).to_string());
        }
    }

    let mut template_overrides_changed = BTreeSet::new();
    let current_template_overrides: BTreeMap<&str, _> = current
        .template_overrides
        .iter()
        .map(|item| (item.template_id.as_str(), item))
        .collect();
    let next_template_overrides: BTreeMap<&str, _> = candidate
        .template_overrides
        .iter()
        .map(|item| (item.template_id.as_str(), item))
        .collect();
    for (id, next) in &next_template_overrides {
        if current_template_overrides.get(id).map(|value| *value) != Some(next) {
            template_overrides_changed.insert((*id).to_string());
        }
    }
    for id in current_template_overrides.keys() {
        if !next_template_overrides.contains_key(id) {
            template_overrides_changed.insert((*id).to_string());
        }
    }

    let mut removed_custom_fields = Vec::new();
    let mut added_custom_fields = Vec::new();
    for key in current_custom_fields.keys() {
        if !next_custom_fields.contains_key(key) {
            removed_custom_fields.push((*key).to_string());
        }
    }
    for key in next_custom_fields.keys() {
        if !current_custom_fields.contains_key(key) {
            added_custom_fields.push((*key).to_string());
        } else if current_custom_fields.get(key) != next_custom_fields.get(key) {
            // Treat in-place custom field edits as scope-sensitive when entityTypes change.
            let before = current_custom_fields[key];
            let after = next_custom_fields[key];
            if before.entity_types != after.entity_types {
                scope_changed.insert((*key).to_string());
            }
        }
    }
    removed_custom_fields.sort();
    added_custom_fields.sort();

    SchemaOverlayDiff {
        disabled_types: sorted_added(&current_disabled_types, &next_disabled_types),
        removed_custom_types: sorted_removed(&current_custom_types, &next_custom_types),
        added_custom_types: sorted_added(&current_custom_types, &next_custom_types),
        appearance_types: appearance_types.into_iter().collect(),
        disabled_fields: sorted_added(&current_disabled_fields, &next_disabled_fields),
        removed_custom_fields,
        added_custom_fields,
        scope_changed_fields: scope_changed.into_iter().collect(),
        metadata_changed_fields: metadata_changed.into_iter().collect(),
        timeline_changed_fields: timeline_changed.into_iter().collect(),
        disabled_templates: sorted_added(&current_disabled_templates, &next_disabled_templates),
        removed_custom_templates: sorted_removed(&current_custom_templates, &next_custom_templates),
        added_custom_templates: sorted_added(&current_custom_templates, &next_custom_templates),
        template_overrides_changed: template_overrides_changed.into_iter().collect(),
    }
}

/// Qualify + validate a candidate overlay and return structured issues on failure.
pub fn normalize_candidate_overlay(
    package: &PluginManifest,
    overlay: &ModuleSchemaOverlay,
) -> Result<ModuleSchemaOverlay, Vec<SchemaOverlayItemIssue>> {
    let mut normalized = overlay.clone();
    if normalized.version == 0 {
        normalized.version = crate::schema_overlay::SCHEMA_OVERLAY_VERSION;
    }
    if let Err(message) = qualify_module_overlay(package, &mut normalized) {
        return Err(vec![SchemaOverlayItemIssue {
            kind: "overlay".into(),
            id: package.id.clone(),
            property: None,
            message,
        }]);
    }
    if let Err(message) = validate_module_overlay(package, &normalized) {
        return Err(classify_validation_message(&message));
    }
    Ok(normalized)
}

fn classify_validation_message(message: &str) -> Vec<SchemaOverlayItemIssue> {
    let lower = message.to_ascii_lowercase();
    let property = infer_validation_property(&lower);
    let (kind, id) = if lower.contains("entity type")
        || lower.contains("custom entity")
        || lower.contains("disabledentitytypes")
        || lower.contains("appearance")
    {
        ("type", extract_trailing_token(message).unwrap_or("overlay"))
    } else if lower.contains("template") {
        (
            "template",
            extract_trailing_token(message).unwrap_or("overlay"),
        )
    } else if lower.contains("field") {
        (
            "field",
            extract_trailing_token(message).unwrap_or("overlay"),
        )
    } else {
        ("overlay", "overlay")
    };
    vec![SchemaOverlayItemIssue {
        kind: kind.into(),
        id: id.into(),
        property,
        message: message.to_string(),
    }]
}

fn infer_validation_property(lower: &str) -> Option<String> {
    const RULES: &[(&str, &str)] = &[
        ("disabledentitytypes", "disabledEntityTypes"),
        (
            "cannot disable unknown builtin entity type",
            "disabledEntityTypes",
        ),
        ("disabledfields", "disabledFields"),
        ("disabledtemplates", "disabledTemplates"),
        ("customentitytypes", "customEntityTypes"),
        ("customfields", "customFields"),
        ("customtemplates", "customTemplates"),
        ("fieldscopeoverrides", "fieldScopeOverrides"),
        ("templateoverrides", "templateOverrides"),
        ("fieldmetadataoverrides", "fieldMetadataOverrides"),
        (
            "entitytypeappearanceoverrides",
            "entityTypeAppearanceOverrides",
        ),
        ("fieldtimelineoverrides", "fieldTimelineOverrides"),
        ("relationshiptype", "relationshipType"),
        ("relationship type", "relationshipType"),
        ("targetentitytypes", "targetEntityTypes"),
        ("entitytypes", "entityTypes"),
        ("entity types", "entityTypes"),
        ("metadatafields", "metadataFields"),
        ("cardinality", "cardinality"),
        ("oneof", "oneOf"),
        ("one of", "oneOf"),
        ("options", "options"),
        ("iconcolor", "iconColor"),
        ("icon color", "iconColor"),
        ("icon", "icon"),
        ("label", "label"),
        ("duplicate", "key"),
        (" key", "key"),
    ];
    for (needle, property) in RULES {
        if lower.contains(needle) {
            return Some((*property).into());
        }
    }
    None
}

fn extract_trailing_token(message: &str) -> Option<&str> {
    message
        .rsplit([' ', ':', ','])
        .map(str::trim)
        .find(|token| {
            !token.is_empty()
                && *token != "type"
                && *token != "field"
                && *token != "template"
                && !token.chars().all(|c| c.is_ascii_digit())
        })
}

/// Assemble a preview result from a validated diff plus live counts.
pub fn assemble_schema_overlay_preview(
    diff: &SchemaOverlayDiff,
    type_counts: &BTreeMap<String, u64>,
    field_counts: &BTreeMap<String, u64>,
    errors: Vec<SchemaOverlayItemIssue>,
) -> SchemaOverlayPreviewResult {
    assemble_schema_overlay_preview_with_bounds(
        diff,
        type_counts,
        field_counts,
        errors,
        false,
        false,
    )
}

/// Assemble a preview result, marking incomplete counts when the SQL IN-list was capped.
pub fn assemble_schema_overlay_preview_with_bounds(
    diff: &SchemaOverlayDiff,
    type_counts: &BTreeMap<String, u64>,
    field_counts: &BTreeMap<String, u64>,
    mut errors: Vec<SchemaOverlayItemIssue>,
    types_truncated: bool,
    fields_truncated: bool,
) -> SchemaOverlayPreviewResult {
    let mut affected_types = Vec::new();
    for id in &diff.disabled_types {
        affected_types.push(SchemaOverlayTypeImpact {
            entity_type: id.clone(),
            change: "disabled".into(),
            entity_count: *type_counts.get(id).unwrap_or(&0),
        });
    }
    for id in &diff.removed_custom_types {
        affected_types.push(SchemaOverlayTypeImpact {
            entity_type: id.clone(),
            change: "removed".into(),
            entity_count: *type_counts.get(id).unwrap_or(&0),
        });
    }
    for id in &diff.added_custom_types {
        affected_types.push(SchemaOverlayTypeImpact {
            entity_type: id.clone(),
            change: "added".into(),
            entity_count: 0,
        });
    }
    for id in &diff.appearance_types {
        if affected_types.iter().any(|item| item.entity_type == *id) {
            continue;
        }
        affected_types.push(SchemaOverlayTypeImpact {
            entity_type: id.clone(),
            change: "appearance".into(),
            entity_count: *type_counts.get(id).unwrap_or(&0),
        });
    }

    let mut affected_fields = Vec::new();
    for key in &diff.disabled_fields {
        affected_fields.push(SchemaOverlayFieldImpact {
            field_key: key.clone(),
            change: "disabled".into(),
            value_count: *field_counts.get(key).unwrap_or(&0),
        });
    }
    for key in &diff.removed_custom_fields {
        affected_fields.push(SchemaOverlayFieldImpact {
            field_key: key.clone(),
            change: "removed".into(),
            value_count: *field_counts.get(key).unwrap_or(&0),
        });
    }
    for key in &diff.added_custom_fields {
        affected_fields.push(SchemaOverlayFieldImpact {
            field_key: key.clone(),
            change: "added".into(),
            value_count: 0,
        });
    }
    for key in &diff.scope_changed_fields {
        if affected_fields.iter().any(|item| item.field_key == *key) {
            continue;
        }
        affected_fields.push(SchemaOverlayFieldImpact {
            field_key: key.clone(),
            change: "scope-changed".into(),
            value_count: *field_counts.get(key).unwrap_or(&0),
        });
    }
    for key in &diff.metadata_changed_fields {
        if affected_fields.iter().any(|item| item.field_key == *key) {
            continue;
        }
        affected_fields.push(SchemaOverlayFieldImpact {
            field_key: key.clone(),
            change: "metadata-changed".into(),
            value_count: *field_counts.get(key).unwrap_or(&0),
        });
    }
    for key in &diff.timeline_changed_fields {
        if affected_fields.iter().any(|item| item.field_key == *key) {
            continue;
        }
        affected_fields.push(SchemaOverlayFieldImpact {
            field_key: key.clone(),
            change: "timeline-changed".into(),
            value_count: *field_counts.get(key).unwrap_or(&0),
        });
    }

    let mut affected_templates = BTreeSet::new();
    affected_templates.extend(diff.disabled_templates.iter().cloned());
    affected_templates.extend(diff.removed_custom_templates.iter().cloned());
    affected_templates.extend(diff.added_custom_templates.iter().cloned());
    affected_templates.extend(diff.template_overrides_changed.iter().cloned());

    let relationship_metadata_keys = diff.metadata_changed_fields.clone();

    let mut compatibility_notes = Vec::new();
    if !diff.timeline_changed_fields.is_empty() {
        compatibility_notes.push(format!(
            "Timeline contributions change for fields: {}",
            diff.timeline_changed_fields.join(", ")
        ));
    }
    if !diff.disabled_types.is_empty() || !diff.removed_custom_types.is_empty() {
        compatibility_notes.push(
            "Projections (Tree, Timeline, Wiki, Graph) that filter by Type may hide or regroup entities after this save; reopen those surfaces to confirm labels."
                .into(),
        );
    }
    if !diff.disabled_fields.is_empty() {
        compatibility_notes
            .push("Disabled fields stay stored on entities; values are hidden, not purged.".into());
    }
    if !diff.removed_custom_fields.is_empty() {
        compatibility_notes.push(
            "Removed custom fields leave existing stored values until cleaned up separately."
                .into(),
        );
    }
    if !diff.metadata_changed_fields.is_empty() {
        compatibility_notes.push(format!(
            "Relationship metadata schemas change for fields: {}",
            diff.metadata_changed_fields.join(", ")
        ));
    }

    if types_truncated {
        errors.push(SchemaOverlayItemIssue {
            kind: "overlay".into(),
            id: "overlay".into(),
            property: Some("affectedTypes".into()),
            message: "Preview type counts were truncated at 256 entries; narrow the change set and preview again before saving.".into(),
        });
    }
    if fields_truncated {
        errors.push(SchemaOverlayItemIssue {
            kind: "overlay".into(),
            id: "overlay".into(),
            property: Some("affectedFields".into()),
            message: "Preview field-value counts were truncated at 256 entries; narrow the change set and preview again before saving.".into(),
        });
    }

    let unresolved_type_removals = affected_types
        .iter()
        .filter(|item| {
            item.change == "removed"
                && (item.entity_count > 0
                    || (types_truncated && !type_counts.contains_key(&item.entity_type)))
        })
        .map(|item| item.entity_type.clone())
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    for item in &affected_types {
        if item.entity_count > 0 && (item.change == "disabled" || item.change == "removed") {
            warnings.push(SchemaOverlayItemIssue {
                kind: "type".into(),
                id: item.entity_type.clone(),
                property: Some("entityCount".into()),
                message: format!(
                    "{} {} use this type",
                    item.entity_count,
                    if item.entity_count == 1 {
                        "entity"
                    } else {
                        "entities"
                    }
                ),
            });
        }
    }
    for item in &affected_fields {
        if item.value_count > 0 && (item.change == "disabled" || item.change == "removed") {
            warnings.push(SchemaOverlayItemIssue {
                kind: "field".into(),
                id: item.field_key.clone(),
                property: Some("valueCount".into()),
                message: format!(
                    "{} stored {} for this field",
                    item.value_count,
                    if item.value_count == 1 {
                        "value"
                    } else {
                        "values"
                    }
                ),
            });
        }
    }
    for type_id in &unresolved_type_removals {
        warnings.push(SchemaOverlayItemIssue {
            kind: "type".into(),
            id: type_id.clone(),
            property: Some("reassignment".into()),
            message: "Reassign existing entities before removing this type.".into(),
        });
    }

    let mut errors = errors;
    for type_id in &unresolved_type_removals {
        errors.push(SchemaOverlayItemIssue {
            kind: "type".into(),
            id: type_id.clone(),
            property: Some("reassignment".into()),
            message: format!(
                "Cannot remove type {type_id} while entities still use it; reassign them first."
            ),
        });
    }

    let requires_acknowledgement = affected_types
        .iter()
        .any(|item| item.entity_count > 0 && item.change != "added" && item.change != "appearance")
        || affected_fields.iter().any(|item| {
            item.value_count > 0
                && matches!(
                    item.change.as_str(),
                    "disabled" | "removed" | "scope-changed"
                )
        });

    let change_kind = if !unresolved_type_removals.is_empty() {
        SchemaOverlayChangeKind::RequiresReassignment
    } else {
        diff.change_kind()
    };

    SchemaOverlayPreviewResult {
        ok: errors.is_empty(),
        change_kind,
        requires_acknowledgement,
        errors,
        warnings,
        affected_types,
        affected_fields,
        affected_templates: affected_templates.into_iter().collect(),
        relationship_metadata_keys,
        compatibility_notes,
        unresolved_type_removals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_overlay::SCHEMA_OVERLAY_VERSION;
    use crate::{EntityTypeDefinition, IconRef};

    fn overlay_with_type(id: &str) -> ModuleSchemaOverlay {
        ModuleSchemaOverlay {
            version: SCHEMA_OVERLAY_VERSION,
            custom_entity_types: vec![EntityTypeDefinition {
                id: id.into(),
                name: "Custom".into(),
                icon: IconRef::Catalog {
                    id: "unknown".into(),
                },
                icon_color: crate::EntityTypeColor::Preset { id: "brass".into() },
            }],
            ..ModuleSchemaOverlay::default()
        }
    }

    #[test]
    fn diff_detects_removed_custom_type() {
        let current = overlay_with_type("daena.lore:order");
        let candidate = ModuleSchemaOverlay::default();
        let diff = diff_module_schema_overlays(&current, &candidate);
        assert_eq!(
            diff.removed_custom_types,
            vec!["daena.lore:order".to_string()]
        );
        assert_eq!(
            diff.change_kind(),
            SchemaOverlayChangeKind::RequiresReassignment
        );
    }

    #[test]
    fn assemble_blocks_unresolved_type_removal() {
        let diff = SchemaOverlayDiff {
            removed_custom_types: vec!["daena.lore:order".into()],
            ..SchemaOverlayDiff::default()
        };
        let mut counts = BTreeMap::new();
        counts.insert("daena.lore:order".into(), 2);
        let preview = assemble_schema_overlay_preview(&diff, &counts, &BTreeMap::new(), Vec::new());
        assert!(!preview.ok);
        assert_eq!(
            preview.unresolved_type_removals,
            vec!["daena.lore:order".to_string()]
        );
        assert!(preview.requires_acknowledgement);
    }

    #[test]
    fn assemble_additive_field_add_is_ok() {
        let diff = SchemaOverlayDiff {
            added_custom_fields: vec!["motto".into()],
            ..SchemaOverlayDiff::default()
        };
        let preview =
            assemble_schema_overlay_preview(&diff, &BTreeMap::new(), &BTreeMap::new(), Vec::new());
        assert!(preview.ok);
        assert!(!preview.requires_acknowledgement);
        assert_eq!(preview.change_kind, SchemaOverlayChangeKind::Additive);
    }

    #[test]
    fn truncated_counts_mark_preview_not_ok() {
        let diff = SchemaOverlayDiff {
            removed_custom_types: vec!["daena.lore:order".into()],
            ..SchemaOverlayDiff::default()
        };
        let preview = assemble_schema_overlay_preview_with_bounds(
            &diff,
            &BTreeMap::new(),
            &BTreeMap::new(),
            Vec::new(),
            true,
            false,
        );
        assert!(!preview.ok);
        assert!(preview
            .errors
            .iter()
            .any(|issue| issue.property.as_deref() == Some("affectedTypes")));
    }

    #[test]
    fn field_keys_needing_counts_include_metadata_and_timeline() {
        let diff = SchemaOverlayDiff {
            metadata_changed_fields: vec!["marriage".into()],
            timeline_changed_fields: vec!["born".into()],
            ..SchemaOverlayDiff::default()
        };
        assert_eq!(
            diff.field_keys_needing_counts(),
            vec!["born".to_string(), "marriage".to_string()]
        );
    }

    #[test]
    fn validation_errors_include_property() {
        let issues = classify_validation_message(
            "cannot disable unknown builtin entity type: daena.lore:person",
        );
        assert_eq!(issues[0].kind, "type");
        assert_eq!(issues[0].property.as_deref(), Some("disabledEntityTypes"));
    }
}
