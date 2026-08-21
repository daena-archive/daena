use crate::CoreError;
use pulldown_cmark::{Event, Options, Parser, Tag};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub const STAGED_IMPORT_SCHEMA_VERSION: u32 = 1;
pub const IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION: u32 = 1;
pub const VALIDATED_IMPORT_PLAN_SCHEMA_VERSION: u32 = 1;
pub const GENERIC_DOCUMENT_IMPORTER_ID: &str = "daena.generic-documents";
pub const GENERIC_DOCUMENT_IMPORTER_VERSION: &str = "1";
pub const EXTERNAL_IMPORT_ANALYSIS_CANCELLED: &str = "external import analysis cancelled";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportSourceKind {
    File,
    Folder,
    Archive,
    Vault,
    WikiDump,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImporterIdentity {
    pub id: String,
    pub version: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportSource {
    pub id: String,
    pub kind: ImportSourceKind,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportDiagnosticSeverity {
    Fatal,
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportDiagnostic {
    pub severity: ImportDiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagedDocument {
    pub format: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagedLinkKind {
    Internal,
    External,
    Embed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StagedLinkResolution {
    Unresolved,
    Resolved,
    Ambiguous,
    Missing,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StagedLink {
    pub kind: StagedLinkKind,
    pub target: String,
    #[serde(default)]
    pub label: Option<String>,
    pub resolution: StagedLinkResolution,
    #[serde(default)]
    pub resolved_object_id: Option<String>,
    #[serde(default)]
    pub candidate_object_ids: Vec<String>,
    #[serde(default)]
    pub raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MappingHintKind {
    EntityType,
    Field,
    Relationship,
    Hierarchy,
    AssetRelationship,
    SourceCategory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagedMappingHint {
    pub kind: MappingHintKind,
    #[serde(default)]
    pub source_key: Option<String>,
    pub suggested_value: serde_json::Value,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagedObject {
    pub id: String,
    pub source_id: String,
    pub source_kind: String,
    pub source_path: String,
    pub content_hash: String,
    pub title: String,
    #[serde(default)]
    pub body: Option<StagedDocument>,
    #[serde(default)]
    pub parent_source_path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub raw_source_data: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub links: Vec<StagedLink>,
    #[serde(default)]
    pub mapping_hints: Vec<StagedMappingHint>,
    #[serde(default)]
    pub diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagedAsset {
    pub id: String,
    pub source_path: String,
    pub filename: String,
    pub size: u64,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub owner_object_id: Option<String>,
    #[serde(default)]
    pub relationship: Option<String>,
    #[serde(default)]
    pub raw_metadata: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnsupportedSourceData {
    pub source_path: String,
    pub source_kind: String,
    pub reason: String,
    #[serde(default)]
    pub raw_metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportAnalysisSummary {
    pub document_count: usize,
    pub candidate_entity_count: usize,
    pub folder_count: usize,
    pub asset_count: usize,
    pub link_count: usize,
    pub unresolved_link_count: usize,
    pub unsupported_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub total_source_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportAnalysisProgress {
    pub processed_entries: usize,
    pub staged_object_count: usize,
    pub unsupported_count: usize,
    pub source_bytes: u64,
    #[serde(default)]
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagedImport {
    pub schema_version: u32,
    pub importer: ImporterIdentity,
    pub source: ImportSource,
    #[serde(default)]
    pub objects: Vec<StagedObject>,
    #[serde(default)]
    pub assets: Vec<StagedAsset>,
    #[serde(default)]
    pub unsupported: Vec<UnsupportedSourceData>,
    #[serde(default)]
    pub diagnostics: Vec<ImportDiagnostic>,
    pub summary: ImportAnalysisSummary,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportMappingDecision {
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub field_mappings: BTreeMap<String, String>,
    #[serde(default)]
    pub relationship_mappings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportMappingOverrides {
    #[serde(default)]
    pub global: ImportMappingDecision,
    #[serde(default)]
    pub folders: BTreeMap<String, ImportMappingDecision>,
    #[serde(default)]
    pub items: BTreeMap<String, ImportMappingDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidateMapping {
    pub entity_type: Option<String>,
    pub field_mappings: BTreeMap<String, String>,
    pub relationship_mappings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidateIssue {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub object_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidateObject {
    pub staged_object_id: String,
    pub source_id: String,
    pub source_path: String,
    pub title: String,
    pub decision: String,
    pub mapping: ImportCandidateMapping,
    pub issues: Vec<ImportCandidateIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidatePlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub session_id: String,
    pub importer: ImporterIdentity,
    pub source: ImportSource,
    pub captured_content_generation: i64,
    pub current_content_generation: i64,
    pub manifest_fingerprint: String,
    pub objects: Vec<ImportCandidateObject>,
    pub unsupported_count: usize,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub issues: Vec<ImportCandidateIssue>,
    pub unresolved_decision_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportCandidatePlanBuild {
    pub session_id: String,
    pub importer: ImporterIdentity,
    pub source: ImportSource,
    pub captured_content_generation: i64,
    pub current_content_generation: i64,
    pub manifest_fingerprint: String,
    pub objects: Vec<StagedObject>,
    pub unsupported_count: usize,
    pub diagnostics: Vec<ImportDiagnostic>,
}

pub fn build_import_candidate_plan(
    input: ImportCandidatePlanBuild,
    overrides: &ImportMappingOverrides,
) -> Result<ImportCandidatePlan, CoreError> {
    let ImportCandidatePlanBuild {
        session_id,
        importer,
        source,
        captured_content_generation,
        current_content_generation,
        manifest_fingerprint,
        objects,
        unsupported_count,
        diagnostics,
    } = input;
    if session_id.trim().is_empty() || manifest_fingerprint.trim().is_empty() {
        return Err(CoreError::Validation(
            "candidate plan requires a session and manifest fingerprint".into(),
        ));
    }
    let mut seen_ids = BTreeSet::new();
    let mut candidate_objects = Vec::with_capacity(objects.len());
    let mut unresolved_decision_count = 0;
    for object in &objects {
        if !seen_ids.insert(&object.id) {
            return Err(CoreError::Validation(format!(
                "duplicate staged object id in candidate plan: {}",
                object.id
            )));
        }
        validate_source_path(&object.source_path)?;
        let mapping = resolve_import_mapping(object, overrides);
        let mut issues = Vec::new();
        if mapping.entity_type.is_none() {
            issues.push(ImportCandidateIssue {
                code: "entity_type_required".into(),
                message: "Choose an enabled entity type for this item.".into(),
                source_path: Some(object.source_path.clone()),
                object_id: Some(object.id.clone()),
            });
            unresolved_decision_count += 1;
        }
        candidate_objects.push(ImportCandidateObject {
            staged_object_id: object.id.clone(),
            source_id: object.source_id.clone(),
            source_path: object.source_path.clone(),
            title: object.title.clone(),
            decision: "create".into(),
            mapping,
            issues,
        });
    }
    let mut issues = Vec::new();
    if captured_content_generation != current_content_generation {
        issues.push(ImportCandidateIssue {
            code: "project_generation_changed".into(),
            message: "The project changed after analysis; analyze the source again before commit."
                .into(),
            source_path: None,
            object_id: None,
        });
    }
    let mut plan = ImportCandidatePlan {
        schema_version: IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION,
        plan_id: String::new(),
        session_id,
        importer,
        source,
        captured_content_generation,
        current_content_generation,
        manifest_fingerprint,
        objects: candidate_objects,
        unsupported_count,
        diagnostics,
        issues,
        unresolved_decision_count,
    };
    let bytes = serde_json::to_vec(&plan).map_err(|error| {
        CoreError::Validation(format!("candidate plan serialization failed: {error}"))
    })?;
    plan.plan_id = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(plan)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ImportObjectDecision {
    Create,
    Skip,
    MapToExisting {
        entity_id: String,
        expected_revision: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportFieldTarget {
    pub namespace: String,
    pub key: String,
    #[serde(default)]
    pub entity_types: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportMappingCatalog {
    pub fingerprint: String,
    pub entity_types: BTreeSet<String>,
    pub fields: BTreeMap<String, ImportFieldTarget>,
    pub relationship_types: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportExistingTarget {
    pub entity_id: String,
    pub entity_type: Option<String>,
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportValidationIssue {
    pub severity: ImportValidationSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub object_id: Option<String>,
    #[serde(default)]
    pub existing_entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedImportField {
    pub namespace: String,
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedImportObject {
    pub staged_object_id: String,
    pub source_id: String,
    pub source_path: String,
    pub content_hash: String,
    pub title: String,
    pub entity_type: Option<String>,
    pub document: Option<StagedDocument>,
    pub fields: Vec<ValidatedImportField>,
    pub decision: ImportObjectDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedImportAsset {
    pub staged_asset_id: String,
    pub owner_staged_object_id: String,
    pub source_path: String,
    pub filename: String,
    pub content_hash: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedImportPlan {
    pub schema_version: u32,
    pub plan_id: String,
    pub candidate_plan_id: String,
    pub session_id: String,
    pub importer: ImporterIdentity,
    pub source: ImportSource,
    pub content_generation: i64,
    pub manifest_fingerprint: String,
    pub objects: Vec<ValidatedImportObject>,
    #[serde(default)]
    pub assets: Vec<ValidatedImportAsset>,
    pub warnings: Vec<ImportValidationIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedObjectReport {
    pub staged_object_id: String,
    pub source_path: String,
    pub entity_id: String,
    pub entity_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAssetReport {
    pub staged_asset_id: String,
    pub source_path: String,
    pub asset_id: String,
    pub entity_id: String,
    pub filename: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportCommitReport {
    pub request_id: String,
    pub plan_id: String,
    pub importer: ImporterIdentity,
    pub source: ImportSource,
    pub created: Vec<ImportedObjectReport>,
    pub mapped: Vec<ImportedObjectReport>,
    #[serde(default)]
    pub assets: Vec<ImportedAssetReport>,
    pub skipped_source_paths: Vec<String>,
    pub warnings: Vec<ImportValidationIssue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportValidationBuild {
    pub candidate: ImportCandidatePlan,
    pub staged_objects: Vec<StagedObject>,
    pub staged_assets: Vec<StagedAsset>,
    pub catalog: ImportMappingCatalog,
    pub decisions: BTreeMap<String, ImportObjectDecision>,
    pub existing_targets: BTreeMap<String, ImportExistingTarget>,
    pub duplicate_targets: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportValidationOutcome {
    pub plan: Option<ValidatedImportPlan>,
    pub issues: Vec<ImportValidationIssue>,
}

pub fn validate_import_candidate_plan(
    input: ImportValidationBuild,
) -> Result<ImportValidationOutcome, CoreError> {
    let ImportValidationBuild {
        candidate,
        staged_objects,
        staged_assets,
        catalog,
        decisions,
        existing_targets,
        duplicate_targets,
    } = input;
    let staged = staged_objects
        .into_iter()
        .map(|object| (object.id.clone(), object))
        .collect::<BTreeMap<_, _>>();
    let mut issues = Vec::new();
    if candidate.captured_content_generation != candidate.current_content_generation {
        issues.push(validation_issue(
            ImportValidationSeverity::Error,
            "project_generation_changed",
            "The project changed after analysis. Analyze the source again.",
            None,
            None,
            None,
        ));
    }
    if candidate.manifest_fingerprint != catalog.fingerprint {
        issues.push(validation_issue(
            ImportValidationSeverity::Error,
            "manifest_changed",
            "Enabled schema contributions changed. Review the mappings again.",
            None,
            None,
            None,
        ));
    }
    for diagnostic in &candidate.diagnostics {
        let severity = match diagnostic.severity {
            ImportDiagnosticSeverity::Warning => ImportValidationSeverity::Warning,
            ImportDiagnosticSeverity::Fatal | ImportDiagnosticSeverity::Error => {
                ImportValidationSeverity::Error
            }
        };
        issues.push(validation_issue(
            severity,
            &diagnostic.code,
            &diagnostic.message,
            diagnostic.source_path.clone(),
            diagnostic.object_id.clone(),
            None,
        ));
    }
    if candidate.unsupported_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "unsupported_source_data",
            &format!(
                "{} unsupported source item(s) will not be imported.",
                candidate.unsupported_count
            ),
            None,
            None,
            None,
        ));
    }
    let candidate_ids = candidate
        .objects
        .iter()
        .map(|object| object.staged_object_id.as_str())
        .collect::<BTreeSet<_>>();
    for decision_id in decisions.keys() {
        if !candidate_ids.contains(decision_id.as_str()) {
            return Err(CoreError::Validation(format!(
                "decision references unknown staged object: {decision_id}"
            )));
        }
    }
    let mut validated = Vec::with_capacity(candidate.objects.len());
    for candidate_object in &candidate.objects {
        let Some(object) = staged.get(&candidate_object.staged_object_id) else {
            return Err(CoreError::Validation(format!(
                "candidate object is missing from staged data: {}",
                candidate_object.staged_object_id
            )));
        };
        let decision = decisions
            .get(&object.id)
            .cloned()
            .unwrap_or(ImportObjectDecision::Create);
        let mut entity_type = None;
        let mut fields = Vec::new();
        match &decision {
            ImportObjectDecision::Skip => {}
            ImportObjectDecision::MapToExisting {
                entity_id,
                expected_revision,
            } => match existing_targets.get(entity_id) {
                Some(target) if target.revision == *expected_revision => {
                    entity_type = target.entity_type.clone();
                }
                Some(_) => issues.push(validation_issue(
                    ImportValidationSeverity::Error,
                    "existing_target_revision_changed",
                    "The selected existing entity changed. Select it again.",
                    Some(object.source_path.clone()),
                    Some(object.id.clone()),
                    Some(entity_id.clone()),
                )),
                None => issues.push(validation_issue(
                    ImportValidationSeverity::Error,
                    "existing_target_missing",
                    "The selected existing entity no longer exists.",
                    Some(object.source_path.clone()),
                    Some(object.id.clone()),
                    Some(entity_id.clone()),
                )),
            },
            ImportObjectDecision::Create => {
                if !decisions.contains_key(&object.id) {
                    if let Some(existing) = duplicate_targets
                        .get(&object.id)
                        .and_then(|targets| targets.first())
                    {
                        issues.push(validation_issue(
                            ImportValidationSeverity::Error,
                            "duplicate_source_identity",
                            "This source was imported before. Choose create, skip, or map to existing explicitly.",
                            Some(object.source_path.clone()),
                            Some(object.id.clone()),
                            Some(existing.clone()),
                        ));
                    }
                }
                let selected_type = candidate_object
                    .mapping
                    .entity_type
                    .as_deref()
                    .filter(|value| catalog.entity_types.contains(*value));
                match selected_type {
                    Some(selected) => entity_type = Some(selected.into()),
                    None => issues.push(validation_issue(
                        ImportValidationSeverity::Error,
                        "entity_type_unavailable",
                        "Choose an entity type contributed by an enabled plugin.",
                        Some(object.source_path.clone()),
                        Some(object.id.clone()),
                        None,
                    )),
                }
                for (source_key, target_id) in &candidate_object.mapping.field_mappings {
                    let Some(value) = object.fields.get(source_key) else {
                        issues.push(validation_issue(
                            ImportValidationSeverity::Error,
                            "source_field_missing",
                            &format!("The source field {source_key} is not present."),
                            Some(object.source_path.clone()),
                            Some(object.id.clone()),
                            None,
                        ));
                        continue;
                    };
                    let Some(target) = catalog.fields.get(target_id) else {
                        issues.push(validation_issue(
                            ImportValidationSeverity::Error,
                            "target_field_unavailable",
                            &format!("The mapped field {target_id} is not available."),
                            Some(object.source_path.clone()),
                            Some(object.id.clone()),
                            None,
                        ));
                        continue;
                    };
                    if let Some(selected) = entity_type.as_deref() {
                        if !target.entity_types.is_empty()
                            && !target.entity_types.contains(selected)
                        {
                            issues.push(validation_issue(
                                ImportValidationSeverity::Error,
                                "target_field_scope_mismatch",
                                &format!("The field {target_id} does not apply to {selected}."),
                                Some(object.source_path.clone()),
                                Some(object.id.clone()),
                                None,
                            ));
                            continue;
                        }
                    }
                    fields.push(ValidatedImportField {
                        namespace: target.namespace.clone(),
                        key: target.key.clone(),
                        value: value.clone(),
                    });
                }
                for source_key in object.fields.keys() {
                    if !candidate_object
                        .mapping
                        .field_mappings
                        .contains_key(source_key)
                    {
                        issues.push(validation_issue(
                            ImportValidationSeverity::Warning,
                            "unmapped_source_field",
                            &format!("The source field {source_key} will not be imported."),
                            Some(object.source_path.clone()),
                            Some(object.id.clone()),
                            None,
                        ));
                    }
                }
                if !candidate_object.mapping.relationship_mappings.is_empty() {
                    issues.push(validation_issue(
                        ImportValidationSeverity::Error,
                        "relationship_mapping_not_supported",
                        "Relationship commit is not enabled in this import iteration.",
                        Some(object.source_path.clone()),
                        Some(object.id.clone()),
                        None,
                    ));
                } else if !object.links.is_empty() {
                    issues.push(validation_issue(
                        ImportValidationSeverity::Warning,
                        "source_links_preserved",
                        "Markdown links remain in the imported document; entity relationships are not created automatically.",
                        Some(object.source_path.clone()),
                        Some(object.id.clone()),
                        None,
                    ));
                }
            }
        }
        validated.push(ValidatedImportObject {
            staged_object_id: object.id.clone(),
            source_id: object.source_id.clone(),
            source_path: object.source_path.clone(),
            content_hash: object.content_hash.clone(),
            title: object.title.clone(),
            entity_type,
            document: object.body.clone(),
            fields,
            decision,
        });
    }
    let has_errors = issues
        .iter()
        .any(|issue| issue.severity == ImportValidationSeverity::Error);
    if has_errors {
        return Ok(ImportValidationOutcome { plan: None, issues });
    }
    let decisions_by_object = validated
        .iter()
        .map(|object| (object.staged_object_id.as_str(), &object.decision))
        .collect::<BTreeMap<_, _>>();
    let mut validated_assets = Vec::new();
    for asset in staged_assets {
        let Some(owner_id) = asset.owner_object_id.as_deref() else {
            issues.push(validation_issue(
                ImportValidationSeverity::Warning,
                "unreferenced_asset_skipped",
                "This unreferenced asset will not be imported.",
                Some(asset.source_path),
                None,
                None,
            ));
            continue;
        };
        if matches!(
            decisions_by_object.get(owner_id),
            Some(ImportObjectDecision::Skip)
        ) {
            continue;
        }
        if !decisions_by_object.contains_key(owner_id) {
            issues.push(validation_issue(
                ImportValidationSeverity::Error,
                "asset_owner_unavailable",
                "The entity selected for this asset is not available.",
                Some(asset.source_path),
                Some(owner_id.into()),
                None,
            ));
            continue;
        }
        let Some(content_hash) = asset.content_hash else {
            issues.push(validation_issue(
                ImportValidationSeverity::Error,
                "asset_hash_missing",
                "The asset did not produce a content hash during analysis.",
                Some(asset.source_path),
                Some(owner_id.into()),
                None,
            ));
            continue;
        };
        let Some(mime_type) = asset.mime_type else {
            issues.push(validation_issue(
                ImportValidationSeverity::Error,
                "asset_mime_type_missing",
                "The asset did not produce a media type during analysis.",
                Some(asset.source_path),
                Some(owner_id.into()),
                None,
            ));
            continue;
        };
        validated_assets.push(ValidatedImportAsset {
            staged_asset_id: asset.id,
            owner_staged_object_id: owner_id.into(),
            source_path: asset.source_path,
            filename: asset.filename,
            content_hash,
            size: asset.size,
            mime_type,
        });
    }
    let has_errors = issues
        .iter()
        .any(|issue| issue.severity == ImportValidationSeverity::Error);
    if has_errors {
        return Ok(ImportValidationOutcome { plan: None, issues });
    }
    let warnings = issues.clone();
    let mut plan = ValidatedImportPlan {
        schema_version: VALIDATED_IMPORT_PLAN_SCHEMA_VERSION,
        plan_id: String::new(),
        candidate_plan_id: candidate.plan_id,
        session_id: candidate.session_id,
        importer: candidate.importer,
        source: candidate.source,
        content_generation: candidate.current_content_generation,
        manifest_fingerprint: catalog.fingerprint,
        objects: validated,
        assets: validated_assets,
        warnings,
    };
    let bytes =
        serde_json::to_vec(&plan).map_err(|error| CoreError::Serialization(error.to_string()))?;
    plan.plan_id = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(ImportValidationOutcome {
        plan: Some(plan),
        issues,
    })
}

fn validation_issue(
    severity: ImportValidationSeverity,
    code: &str,
    message: &str,
    source_path: Option<String>,
    object_id: Option<String>,
    existing_entity_id: Option<String>,
) -> ImportValidationIssue {
    ImportValidationIssue {
        severity,
        code: code.into(),
        message: message.into(),
        source_path,
        object_id,
        existing_entity_id,
    }
}

fn resolve_import_mapping(
    object: &StagedObject,
    overrides: &ImportMappingOverrides,
) -> ImportCandidateMapping {
    let mut resolved = ImportCandidateMapping {
        entity_type: None,
        field_mappings: BTreeMap::new(),
        relationship_mappings: BTreeMap::new(),
    };
    apply_mapping_decision(&mut resolved, &overrides.global);
    let segments = object.source_path.split('/').collect::<Vec<_>>();
    for end in 1..segments.len() {
        let folder = segments[..end].join("/");
        if let Some(decision) = overrides.folders.get(&folder) {
            apply_mapping_decision(&mut resolved, decision);
        }
    }
    if let Some(decision) = overrides.items.get(&object.id) {
        apply_mapping_decision(&mut resolved, decision);
    }
    resolved
}

fn apply_mapping_decision(target: &mut ImportCandidateMapping, decision: &ImportMappingDecision) {
    if let Some(entity_type) = decision
        .entity_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        target.entity_type = Some(entity_type.into());
    }
    for (source, destination) in &decision.field_mappings {
        if !source.trim().is_empty() && !destination.trim().is_empty() {
            target
                .field_mappings
                .insert(source.clone(), destination.clone());
        }
    }
    for (source, destination) in &decision.relationship_mappings {
        if !source.trim().is_empty() && !destination.trim().is_empty() {
            target
                .relationship_mappings
                .insert(source.clone(), destination.clone());
        }
    }
}

impl StagedImport {
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != STAGED_IMPORT_SCHEMA_VERSION {
            return Err(CoreError::Validation(format!(
                "unsupported staged import schema version: {}",
                self.schema_version
            )));
        }
        if self.importer.id.trim().is_empty() || self.importer.version.trim().is_empty() {
            return Err(CoreError::Validation(
                "staged import importer id and version are required".into(),
            ));
        }
        if self.importer.name.trim().is_empty()
            || self.source.id.trim().is_empty()
            || self.source.display_name.trim().is_empty()
        {
            return Err(CoreError::Validation(
                "staged import importer name, source id, and source display name are required"
                    .into(),
            ));
        }

        let mut object_ids = BTreeSet::new();
        let mut source_ids = BTreeSet::new();
        for object in &self.objects {
            if object.id.trim().is_empty() || !object_ids.insert(object.id.as_str()) {
                return Err(CoreError::Validation(
                    "staged import object ids must be non-empty and unique".into(),
                ));
            }
            if object.source_id.trim().is_empty() || !source_ids.insert(object.source_id.as_str()) {
                return Err(CoreError::Validation(
                    "staged import source ids must be non-empty and unique".into(),
                ));
            }
            if object.title.trim().is_empty() {
                return Err(CoreError::Validation(
                    "staged import object title cannot be empty".into(),
                ));
            }
            if object.source_kind.trim().is_empty() || object.content_hash.trim().is_empty() {
                return Err(CoreError::Validation(
                    "staged import objects require a source kind and content hash".into(),
                ));
            }
            validate_source_path(&object.source_path)?;
            if let Some(parent) = &object.parent_source_path {
                validate_source_path(parent)?;
            }
            if let Some(body) = &object.body {
                if body.format.trim().is_empty() {
                    return Err(CoreError::Validation(
                        "staged import document format cannot be empty".into(),
                    ));
                }
            }
            validate_non_empty_unique_values("tag", &object.tags)?;
            validate_non_empty_unique_values("alias", &object.aliases)?;
            if object.fields.keys().any(|key| key.trim().is_empty())
                || object.metadata.keys().any(|key| key.trim().is_empty())
                || object
                    .raw_source_data
                    .keys()
                    .any(|key| key.trim().is_empty())
            {
                return Err(CoreError::Validation(
                    "staged import field and metadata keys cannot be empty".into(),
                ));
            }
            for hint in &object.mapping_hints {
                if hint
                    .source_key
                    .as_deref()
                    .is_some_and(|source_key| source_key.trim().is_empty())
                {
                    return Err(CoreError::Validation(
                        "staged import mapping hint source key cannot be empty".into(),
                    ));
                }
                if hint.confidence.is_some_and(|confidence| {
                    !confidence.is_finite() || !(0.0..=1.0).contains(&confidence)
                }) {
                    return Err(CoreError::Validation(
                        "staged import mapping hint confidence must be between zero and one".into(),
                    ));
                }
            }
            validate_diagnostics(&object.diagnostics)?;
        }

        for object in &self.objects {
            for link in &object.links {
                if link.target.trim().is_empty() {
                    return Err(CoreError::Validation(
                        "staged import link target cannot be empty".into(),
                    ));
                }
                match link.resolution {
                    StagedLinkResolution::Resolved => {
                        let target = link.resolved_object_id.as_deref().ok_or_else(|| {
                            CoreError::Validation(
                                "resolved staged import links require an object id".into(),
                            )
                        })?;
                        if !object_ids.contains(target) {
                            return Err(CoreError::Validation(
                                "resolved staged import link references an unknown object".into(),
                            ));
                        }
                    }
                    StagedLinkResolution::Ambiguous => {
                        let unique_candidates = link
                            .candidate_object_ids
                            .iter()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>();
                        if unique_candidates.len() < 2
                            || unique_candidates.len() != link.candidate_object_ids.len()
                            || unique_candidates
                                .iter()
                                .any(|candidate| !object_ids.contains(candidate))
                        {
                            return Err(CoreError::Validation(
                                "ambiguous staged import links require at least two unique known candidates"
                                    .into(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut asset_ids = BTreeSet::new();
        for asset in &self.assets {
            if asset.id.trim().is_empty() || !asset_ids.insert(asset.id.as_str()) {
                return Err(CoreError::Validation(
                    "staged import asset ids must be non-empty and unique".into(),
                ));
            }
            validate_source_path(&asset.source_path)?;
            validate_portable_basename(&asset.filename)?;
            if asset
                .owner_object_id
                .as_deref()
                .is_some_and(|owner| !object_ids.contains(owner))
            {
                return Err(CoreError::Validation(
                    "staged import asset references an unknown owner object".into(),
                ));
            }
            validate_diagnostics(&asset.diagnostics)?;
        }
        for unsupported in &self.unsupported {
            validate_source_path(&unsupported.source_path)?;
            if unsupported.source_kind.trim().is_empty() || unsupported.reason.trim().is_empty() {
                return Err(CoreError::Validation(
                    "unsupported staged data requires a source kind and reason".into(),
                ));
            }
        }
        validate_diagnostics(&self.diagnostics)?;
        for diagnostic in self
            .diagnostics
            .iter()
            .chain(self.objects.iter().flat_map(|object| &object.diagnostics))
            .chain(self.assets.iter().flat_map(|asset| &asset.diagnostics))
        {
            if diagnostic
                .object_id
                .as_deref()
                .is_some_and(|object_id| !object_ids.contains(object_id))
            {
                return Err(CoreError::Validation(
                    "staged import diagnostic references an unknown object".into(),
                ));
            }
        }
        Ok(())
    }

    fn refresh_summary(&mut self, folder_count: usize, total_source_bytes: u64) {
        let object_diagnostics = self.objects.iter().flat_map(|object| &object.diagnostics);
        let asset_diagnostics = self.assets.iter().flat_map(|asset| &asset.diagnostics);
        let diagnostics = self
            .diagnostics
            .iter()
            .chain(object_diagnostics)
            .chain(asset_diagnostics)
            .collect::<Vec<_>>();
        self.summary = ImportAnalysisSummary {
            document_count: self
                .objects
                .iter()
                .filter(|object| object.body.is_some())
                .count(),
            candidate_entity_count: self.objects.len(),
            folder_count,
            asset_count: self.assets.len(),
            link_count: self.objects.iter().map(|object| object.links.len()).sum(),
            unresolved_link_count: self
                .objects
                .iter()
                .flat_map(|object| &object.links)
                .filter(|link| {
                    matches!(
                        link.resolution,
                        StagedLinkResolution::Unresolved
                            | StagedLinkResolution::Ambiguous
                            | StagedLinkResolution::Missing
                    )
                })
                .count(),
            unsupported_count: self.unsupported.len(),
            warning_count: diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == ImportDiagnosticSeverity::Warning)
                .count(),
            error_count: diagnostics
                .iter()
                .filter(|diagnostic| {
                    matches!(
                        diagnostic.severity,
                        ImportDiagnosticSeverity::Fatal | ImportDiagnosticSeverity::Error
                    )
                })
                .count(),
            total_source_bytes,
        };
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenericDocumentImportLimits {
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    pub max_files: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_depth: usize,
    pub max_diagnostics: usize,
}

impl Default for GenericDocumentImportLimits {
    fn default() -> Self {
        Self {
            max_entries: default_max_entries(),
            max_files: 10_000,
            max_file_bytes: 16 * 1024 * 1024,
            max_total_bytes: 512 * 1024 * 1024,
            max_depth: 64,
            max_diagnostics: 10_000,
        }
    }
}

fn default_max_entries() -> usize {
    20_000
}

pub fn analyze_generic_documents(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
) -> Result<StagedImport, CoreError> {
    analyze_generic_documents_with_progress(source, limits, |_| Ok(()))
}

pub fn analyze_generic_documents_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    mut progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    validate_limits(&limits)?;
    let source = source.as_ref();
    let metadata = fs::symlink_metadata(source).map_err(|source| CoreError::Io {
        operation: "read import source metadata",
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CoreError::Validation(
            "import source root cannot be a symbolic link".into(),
        ));
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(CoreError::Validation(
            "import source must be a regular file or directory".into(),
        ));
    }

    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Selected source")
        .to_owned();
    let canonical_source = fs::canonicalize(source).map_err(|source| CoreError::Io {
        operation: "resolve import source path",
        source,
    })?;
    let source_id = hex_digest(canonical_source.to_string_lossy().as_bytes());
    let source_kind = if metadata.is_dir() {
        ImportSourceKind::Folder
    } else {
        ImportSourceKind::File
    };
    let mut analyzer = GenericDocumentAnalyzer {
        limits,
        import: StagedImport {
            schema_version: STAGED_IMPORT_SCHEMA_VERSION,
            importer: ImporterIdentity {
                id: GENERIC_DOCUMENT_IMPORTER_ID.into(),
                version: GENERIC_DOCUMENT_IMPORTER_VERSION.into(),
                name: "Generic documents".into(),
            },
            source: ImportSource {
                id: source_id,
                kind: source_kind,
                display_name: source_name.clone(),
            },
            objects: Vec::new(),
            assets: Vec::new(),
            unsupported: Vec::new(),
            diagnostics: Vec::new(),
            summary: ImportAnalysisSummary::default(),
        },
        discovered_entries: usize::from(metadata.is_file()),
        discovered_files: 0,
        processed_entries: 0,
        total_source_bytes: 0,
        folders: BTreeSet::new(),
        progress: &mut progress,
    };

    analyzer.report_progress(None)?;

    if metadata.is_dir() {
        analyzer.analyze_directory(source, &[], 0)?;
    } else {
        analyzer.analyze_file(source, &source_name, &metadata)?;
        analyzer.finish_entry(Some(source_name))?;
    }
    analyzer.resolve_markdown_references()?;
    analyzer
        .import
        .objects
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .assets
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .unsupported
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .refresh_summary(analyzer.folders.len(), analyzer.total_source_bytes);
    analyzer.import.validate()?;
    Ok(analyzer.import)
}

struct GenericDocumentAnalyzer<'a> {
    limits: GenericDocumentImportLimits,
    import: StagedImport,
    discovered_entries: usize,
    discovered_files: usize,
    processed_entries: usize,
    total_source_bytes: u64,
    folders: BTreeSet<String>,
    progress: &'a mut dyn FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
}

impl GenericDocumentAnalyzer<'_> {
    fn analyze_directory(
        &mut self,
        directory: &Path,
        relative_parts: &[String],
        depth: usize,
    ) -> Result<(), CoreError> {
        if depth > self.limits.max_depth {
            return Err(CoreError::Validation(format!(
                "import source exceeds the maximum folder depth of {}",
                self.limits.max_depth
            )));
        }
        let entries = fs::read_dir(directory).map_err(|source| CoreError::Io {
            operation: "read import source directory",
            source,
        })?;
        let mut named_entries = Vec::new();
        for entry in entries {
            self.report_progress(None)?;
            let entry = entry.map_err(|source| CoreError::Io {
                operation: "read import source directory entry",
                source,
            })?;
            self.discovered_entries = self.discovered_entries.saturating_add(1);
            if self.discovered_entries > self.limits.max_entries {
                return Err(CoreError::Validation(format!(
                    "import source exceeds the maximum entry count of {}",
                    self.limits.max_entries
                )));
            }
            let name = match entry.file_name().into_string() {
                Ok(name) if !name.is_empty() => name,
                _ => {
                    self.record_unsupported(
                        non_utf8_entry_label(relative_parts),
                        "filesystem_entry",
                        "entry name is not valid UTF-8",
                    )?;
                    continue;
                }
            };
            named_entries.push((name, entry.path()));
        }
        named_entries.sort_by(|left, right| left.0.cmp(&right.0));

        for (name, path) in named_entries {
            let mut child_parts = relative_parts.to_vec();
            child_parts.push(name);
            let relative_path = child_parts.join("/");
            self.report_progress(Some(relative_path.clone()))?;
            let metadata = fs::symlink_metadata(&path).map_err(|source| CoreError::Io {
                operation: "read import source entry metadata",
                source,
            })?;
            if metadata.file_type().is_symlink() {
                self.record_unsupported(
                    relative_path.clone(),
                    "symlink",
                    "symbolic links are not followed during import analysis",
                )?;
            } else if metadata.is_dir() {
                self.folders.insert(relative_path.clone());
                self.analyze_directory(&path, &child_parts, depth + 1)?;
            } else if metadata.is_file() {
                self.analyze_file(&path, &relative_path, &metadata)?;
            } else {
                self.record_unsupported(
                    relative_path.clone(),
                    "filesystem_entry",
                    "entry is not a regular file or directory",
                )?;
            }
            self.finish_entry(Some(relative_path))?;
        }
        Ok(())
    }

    fn analyze_file(
        &mut self,
        path: &Path,
        source_path: &str,
        metadata: &fs::Metadata,
    ) -> Result<(), CoreError> {
        self.discovered_files = self.discovered_files.saturating_add(1);
        if self.discovered_files > self.limits.max_files {
            return Err(CoreError::Validation(format!(
                "import source exceeds the maximum file count of {}",
                self.limits.max_files
            )));
        }
        let size = metadata.len();
        if size > self.limits.max_file_bytes {
            return Err(CoreError::Validation(format!(
                "import file '{source_path}' exceeds the maximum size of {} bytes",
                self.limits.max_file_bytes
            )));
        }
        let next_total = self
            .total_source_bytes
            .checked_add(size)
            .ok_or_else(|| CoreError::Validation("import source byte count overflowed".into()))?;
        if next_total > self.limits.max_total_bytes {
            return Err(CoreError::Validation(format!(
                "import source exceeds the maximum total size of {} bytes",
                self.limits.max_total_bytes
            )));
        }
        if let Some(mime_type) = asset_mime_type(source_path) {
            let bytes = fs::read(path).map_err(|source| CoreError::Io {
                operation: "read import asset",
                source,
            })?;
            if bytes.len() as u64 != size {
                return Err(CoreError::Conflict(format!(
                    "import asset '{source_path}' changed during analysis"
                )));
            }
            self.total_source_bytes = next_total;
            if !asset_signature_matches(mime_type, &bytes) {
                self.record_unsupported(
                    source_path.to_owned(),
                    "asset",
                    "asset bytes do not match the supported file signature",
                )?;
                self.record_diagnostic(ImportDiagnostic {
                    severity: ImportDiagnosticSeverity::Error,
                    code: "invalid_asset_content".into(),
                    message: "asset bytes do not match the supported file signature".into(),
                    source_path: Some(source_path.to_owned()),
                    object_id: None,
                })?;
                return Ok(());
            }
            let filename = source_path
                .rsplit('/')
                .next()
                .unwrap_or(source_path)
                .to_owned();
            let source_id = hex_digest(
                format!(
                    "{}\0{}\0asset\0{}",
                    GENERIC_DOCUMENT_IMPORTER_ID, self.import.source.id, source_path
                )
                .as_bytes(),
            );
            self.import.assets.push(StagedAsset {
                id: source_id,
                source_path: source_path.to_owned(),
                filename,
                size,
                mime_type: Some(mime_type.into()),
                content_hash: Some(format!("sha256:{}", hex_digest(&bytes))),
                owner_object_id: None,
                relationship: Some("attachment".into()),
                raw_metadata: BTreeMap::new(),
                diagnostics: Vec::new(),
            });
            return Ok(());
        }
        let format = match document_format(source_path) {
            Some(format) => format,
            None => {
                return self.record_unsupported(
                    source_path.to_owned(),
                    "file",
                    "file type is not supported by the generic document importer",
                );
            }
        };
        let bytes = fs::read(path).map_err(|source| CoreError::Io {
            operation: "read import source file",
            source,
        })?;
        if bytes.len() as u64 != size {
            return Err(CoreError::Conflict(format!(
                "import file '{source_path}' changed during analysis"
            )));
        }
        self.total_source_bytes = next_total;
        let body = if let Ok(body) = String::from_utf8(bytes) {
            body
        } else {
            self.record_unsupported(
                source_path.to_owned(),
                "document",
                "document content is not valid UTF-8",
            )?;
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Error,
                code: "invalid_utf8".into(),
                message: "document content is not valid UTF-8".into(),
                source_path: Some(source_path.to_owned()),
                object_id: None,
            })?;
            return Ok(());
        };
        let content_hash = hex_digest(body.as_bytes());
        let source_id = hex_digest(
            format!(
                "{}\0{}\0{}",
                GENERIC_DOCUMENT_IMPORTER_ID, self.import.source.id, source_path
            )
            .as_bytes(),
        );
        let title = document_title(source_path);
        let parent_source_path = source_path
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_owned());
        let (frontmatter, fields, raw_source_data) = if format == "markdown" {
            markdown_frontmatter(&body)
                .map(|frontmatter| {
                    (
                        Some(frontmatter.to_owned()),
                        BTreeMap::from([(
                            "frontmatter".into(),
                            serde_json::Value::String(frontmatter.to_owned()),
                        )]),
                        BTreeMap::from([(
                            "frontmatter".into(),
                            serde_json::Value::String(frontmatter.to_owned()),
                        )]),
                    )
                })
                .unwrap_or_default()
        } else {
            Default::default()
        };
        let links = if format == "markdown" {
            discover_markdown_links(&body)
        } else {
            Vec::new()
        };
        let mut metadata = BTreeMap::new();
        if frontmatter.is_some() {
            metadata.insert(
                "frontmatter_format".into(),
                serde_json::Value::String("yaml".into()),
            );
        }
        self.import.objects.push(StagedObject {
            id: source_id.clone(),
            source_id,
            source_kind: format.to_owned(),
            source_path: source_path.to_owned(),
            content_hash,
            title,
            body: Some(StagedDocument {
                format: format.to_owned(),
                body,
            }),
            parent_source_path,
            tags: Vec::new(),
            aliases: Vec::new(),
            fields,
            metadata,
            raw_source_data,
            links,
            mapping_hints: Vec::new(),
            diagnostics: Vec::new(),
        });
        Ok(())
    }

    fn resolve_markdown_references(&mut self) -> Result<(), CoreError> {
        let objects_by_path = self
            .import
            .objects
            .iter()
            .map(|object| (object.source_path.clone(), object.id.clone()))
            .collect::<BTreeMap<_, _>>();
        let assets_by_path = self
            .import
            .assets
            .iter()
            .enumerate()
            .map(|(index, asset)| (asset.source_path.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut missing = Vec::new();
        for object in &mut self.import.objects {
            for link in &mut object.links {
                if is_external_markdown_target(&link.target) {
                    link.resolution = StagedLinkResolution::NotApplicable;
                    continue;
                }
                let Some(target_path) =
                    resolve_relative_source_path(&object.source_path, &link.target)
                else {
                    link.resolution = StagedLinkResolution::Missing;
                    missing.push((
                        object.id.clone(),
                        object.source_path.clone(),
                        link.target.clone(),
                    ));
                    continue;
                };
                if let Some(target_id) = objects_by_path.get(&target_path) {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(target_id.clone());
                } else if let Some(asset_index) = assets_by_path.get(&target_path) {
                    link.resolution = StagedLinkResolution::NotApplicable;
                    let asset = &mut self.import.assets[*asset_index];
                    if asset.owner_object_id.is_none() {
                        asset.owner_object_id = Some(object.id.clone());
                    }
                    asset.raw_metadata.insert(
                        "resolved_from".into(),
                        serde_json::Value::String(link.target.clone()),
                    );
                    object.mapping_hints.push(StagedMappingHint {
                        kind: MappingHintKind::AssetRelationship,
                        source_key: Some(target_path),
                        suggested_value: serde_json::Value::String("attachment".into()),
                        confidence: Some(1.0),
                        reason: Some("standard Markdown file reference".into()),
                    });
                } else {
                    link.resolution = StagedLinkResolution::Missing;
                    missing.push((
                        object.id.clone(),
                        object.source_path.clone(),
                        link.target.clone(),
                    ));
                }
            }
        }
        for (object_id, source_path, target) in missing {
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: "markdown_target_missing".into(),
                message: format!(
                    "Markdown target '{target}' was not found in the selected source."
                ),
                source_path: Some(source_path),
                object_id: Some(object_id),
            })?;
        }
        Ok(())
    }

    fn record_unsupported(
        &mut self,
        source_path: String,
        source_kind: &str,
        reason: &str,
    ) -> Result<(), CoreError> {
        self.import.unsupported.push(UnsupportedSourceData {
            source_path: source_path.clone(),
            source_kind: source_kind.into(),
            reason: reason.into(),
            raw_metadata: BTreeMap::new(),
        });
        self.record_diagnostic(ImportDiagnostic {
            severity: ImportDiagnosticSeverity::Warning,
            code: "unsupported_source_entry".into(),
            message: reason.into(),
            source_path: Some(source_path),
            object_id: None,
        })
    }

    fn record_diagnostic(&mut self, diagnostic: ImportDiagnostic) -> Result<(), CoreError> {
        if self.import.diagnostics.len() >= self.limits.max_diagnostics {
            return Err(CoreError::Validation(format!(
                "import analysis exceeds the maximum diagnostic count of {}",
                self.limits.max_diagnostics
            )));
        }
        self.import.diagnostics.push(diagnostic);
        Ok(())
    }

    fn finish_entry(&mut self, source_path: Option<String>) -> Result<(), CoreError> {
        self.processed_entries = self.processed_entries.saturating_add(1);
        self.report_progress(source_path)
    }

    fn report_progress(&mut self, source_path: Option<String>) -> Result<(), CoreError> {
        (self.progress)(ImportAnalysisProgress {
            processed_entries: self.processed_entries,
            staged_object_count: self.import.objects.len(),
            unsupported_count: self.import.unsupported.len(),
            source_bytes: self.total_source_bytes,
            source_path,
        })
    }
}

fn discover_markdown_links(body: &str) -> Vec<StagedLink> {
    Parser::new_ext(body, Options::all())
        .filter_map(|event| match event {
            Event::Start(Tag::Link { dest_url, .. }) => Some(StagedLink {
                kind: if is_external_markdown_target(&dest_url) {
                    StagedLinkKind::External
                } else {
                    StagedLinkKind::Internal
                },
                target: dest_url.to_string(),
                label: None,
                resolution: StagedLinkResolution::Unresolved,
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: None,
            }),
            Event::Start(Tag::Image { dest_url, .. }) => Some(StagedLink {
                kind: StagedLinkKind::Embed,
                target: dest_url.to_string(),
                label: None,
                resolution: StagedLinkResolution::Unresolved,
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: None,
            }),
            _ => None,
        })
        .collect()
}

fn markdown_frontmatter(body: &str) -> Option<&str> {
    let remainder = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))?;
    let mut offset = 0;
    for line in remainder.split_inclusive('\n') {
        let value = line.trim_end_matches(['\r', '\n']);
        if value == "---" || value == "..." {
            return Some(&remainder[..offset]);
        }
        offset += line.len();
    }
    None
}

fn asset_mime_type(source_path: &str) -> Option<&'static str> {
    let extension = Path::new(source_path)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

fn asset_signature_matches(mime_type: &str, bytes: &[u8]) -> bool {
    match mime_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        _ => false,
    }
}

fn is_external_markdown_target(target: &str) -> bool {
    let target = target.trim();
    if target.starts_with("//") {
        return true;
    }
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|value| value.is_ascii_alphabetic())
        && characters.all(|value| value.is_ascii_alphanumeric() || matches!(value, '+' | '-' | '.'))
}

fn resolve_relative_source_path(source_path: &str, target: &str) -> Option<String> {
    let path = target
        .split(['?', '#'])
        .next()
        .map(str::trim)
        .unwrap_or_default();
    if path.is_empty() {
        return Some(source_path.to_owned());
    }
    let decoded = percent_decode_utf8(path)?;
    if decoded.starts_with('/') || decoded.contains('\\') || decoded.chars().any(char::is_control) {
        return None;
    }
    let mut components = source_path
        .rsplit_once('/')
        .map(|(parent, _)| parent.split('/').map(str::to_owned).collect::<Vec<_>>())
        .unwrap_or_default();
    for component in decoded.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            value => components.push(value.to_owned()),
        }
    }
    (!components.is_empty()).then(|| components.join("/"))
}

fn percent_decode_utf8(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push(hex_nibble(high)? << 4 | hex_nibble(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn validate_limits(limits: &GenericDocumentImportLimits) -> Result<(), CoreError> {
    if limits.max_entries == 0
        || limits.max_files == 0
        || limits.max_file_bytes == 0
        || limits.max_total_bytes == 0
        || limits.max_diagnostics == 0
    {
        return Err(CoreError::Validation(
            "import analysis limits must be greater than zero".into(),
        ));
    }
    Ok(())
}

fn validate_diagnostics(diagnostics: &[ImportDiagnostic]) -> Result<(), CoreError> {
    for diagnostic in diagnostics {
        if diagnostic.code.trim().is_empty() || diagnostic.message.trim().is_empty() {
            return Err(CoreError::Validation(
                "staged import diagnostics require a code and message".into(),
            ));
        }
        if let Some(source_path) = &diagnostic.source_path {
            validate_source_path(source_path)?;
        }
    }
    Ok(())
}

fn validate_non_empty_unique_values(label: &str, values: &[String]) -> Result<(), CoreError> {
    let mut unique = BTreeSet::new();
    if values
        .iter()
        .any(|value| value.trim().is_empty() || !unique.insert(value.as_str()))
    {
        return Err(CoreError::Validation(format!(
            "staged import {label} values must be non-empty and unique"
        )));
    }
    Ok(())
}

fn validate_source_path(path: &str) -> Result<(), CoreError> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return Err(CoreError::Validation(
            "staged import source paths must be non-empty portable relative paths".into(),
        ));
    }
    let components = path.split('/').collect::<Vec<_>>();
    let windows_prefix = components.first().is_some_and(|component| {
        let bytes = component.as_bytes();
        bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    });
    if windows_prefix
        || components
            .iter()
            .any(|component| component.is_empty() || *component == "." || *component == "..")
    {
        return Err(CoreError::Validation(
            "staged import source paths must be normalized relative paths".into(),
        ));
    }
    Ok(())
}

fn validate_portable_basename(name: &str) -> Result<(), CoreError> {
    let trimmed = name.trim();
    let bytes = trimmed.as_bytes();
    let windows_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || windows_prefix
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_control)
    {
        return Err(CoreError::Validation(
            "staged import asset filename must be a portable basename".into(),
        ));
    }
    Ok(())
}

fn document_format(source_path: &str) -> Option<&'static str> {
    let extension = Path::new(source_path)
        .extension()
        .and_then(|extension| extension.to_str())?;
    if extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown") {
        Some("markdown")
    } else if extension.eq_ignore_ascii_case("txt") {
        Some("plain_text")
    } else {
        None
    }
}

fn document_title(source_path: &str) -> String {
    Path::new(source_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Untitled document")
        .to_owned()
}

fn non_utf8_entry_label(relative_parts: &[String]) -> String {
    if relative_parts.is_empty() {
        "[non-utf8 entry]".into()
    } else {
        format!("{}/[non-utf8 entry]", relative_parts.join("/"))
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectStore;
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("daena-external-import-test-{}", Uuid::new_v4()));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn folder_analysis_is_deterministic_and_does_not_mutate_a_project() {
        let source = TestDirectory::new();
        fs::create_dir(source.path().join("Characters")).unwrap();
        fs::write(
            source.path().join("Characters/Alice.md"),
            "# Alice\n\nA cartographer.",
        )
        .unwrap();
        fs::write(source.path().join("Notes.txt"), "Remember the north road.").unwrap();
        fs::write(source.path().join("portrait.png"), [0_u8, 1, 2]).unwrap();
        let project = ProjectStore::in_memory().unwrap();

        let first =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let second =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();

        assert_eq!(first, second);
        assert!(project.list_entities().unwrap().is_empty());
        assert_eq!(first.summary.document_count, 2);
        assert_eq!(first.summary.candidate_entity_count, 2);
        assert_eq!(first.summary.folder_count, 1);
        assert_eq!(first.summary.unsupported_count, 1);
        assert_eq!(first.summary.warning_count, 1);
        assert_eq!(
            first
                .objects
                .iter()
                .map(|object| object.source_path.as_str())
                .collect::<Vec<_>>(),
            vec!["Characters/Alice.md", "Notes.txt"]
        );
        assert_eq!(
            first.objects[0].parent_source_path.as_deref(),
            Some("Characters")
        );
        assert_eq!(first.objects[0].title, "Alice");
        assert_eq!(
            first.objects[0].body.as_ref().unwrap().body,
            "# Alice\n\nA cartographer."
        );
    }

    #[test]
    fn invalid_utf8_is_preserved_as_an_explicit_unsupported_result() {
        let source = TestDirectory::new();
        fs::write(source.path().join("broken.md"), [0xff, 0xfe]).unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();

        assert!(staged.objects.is_empty());
        assert_eq!(staged.summary.unsupported_count, 1);
        assert_eq!(staged.summary.error_count, 1);
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "invalid_utf8"));
    }

    #[test]
    fn source_identity_is_stable_when_document_content_changes() {
        let source = TestDirectory::new();
        let path = source.path().join("Changing.md");
        fs::write(&path, "first version").unwrap();
        let first = analyze_generic_documents(&path, GenericDocumentImportLimits::default())
            .unwrap()
            .objects
            .remove(0);

        fs::write(&path, "second version").unwrap();
        let second = analyze_generic_documents(&path, GenericDocumentImportLimits::default())
            .unwrap()
            .objects
            .remove(0);

        assert_eq!(first.source_id, second.source_id);
        assert_eq!(first.id, second.id);
        assert_ne!(first.content_hash, second.content_hash);
    }

    #[test]
    fn progress_is_incremental_and_can_cancel_analysis() {
        let source = TestDirectory::new();
        fs::write(source.path().join("one.md"), "one").unwrap();
        fs::write(source.path().join("two.md"), "two").unwrap();
        let mut updates = Vec::new();

        let error = analyze_generic_documents_with_progress(
            source.path(),
            GenericDocumentImportLimits::default(),
            |progress| {
                updates.push(progress.clone());
                if progress.processed_entries >= 1 {
                    Err(CoreError::Conflict(
                        EXTERNAL_IMPORT_ANALYSIS_CANCELLED.into(),
                    ))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();

        assert_eq!(error.to_string(), EXTERNAL_IMPORT_ANALYSIS_CANCELLED);
        assert_eq!(updates[0].processed_entries, 0);
        assert_eq!(updates.last().unwrap().processed_entries, 1);
        assert!(updates.last().unwrap().staged_object_count <= 1);
    }

    #[test]
    fn analysis_enforces_file_and_total_byte_limits() {
        let source = TestDirectory::new();
        fs::write(source.path().join("one.md"), "1234").unwrap();
        fs::write(source.path().join("two.txt"), "5678").unwrap();
        let limits = GenericDocumentImportLimits {
            max_total_bytes: 7,
            ..Default::default()
        };

        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum total size of 7 bytes"));

        let limits = GenericDocumentImportLimits {
            max_files: 1,
            ..Default::default()
        };
        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum file count of 1"));

        let limits = GenericDocumentImportLimits {
            max_file_bytes: 3,
            ..Default::default()
        };
        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum size of 3 bytes"));

        fs::create_dir(source.path().join("nested")).unwrap();
        fs::write(source.path().join("nested/deep.md"), "deep").unwrap();
        let limits = GenericDocumentImportLimits {
            max_depth: 0,
            ..Default::default()
        };
        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum folder depth of 0"));

        let limits = GenericDocumentImportLimits {
            max_entries: 1,
            ..Default::default()
        };
        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum entry count of 1"));
    }

    #[test]
    fn staged_validation_rejects_duplicate_ids_and_traversal_paths() {
        let source = TestDirectory::new();
        fs::write(source.path().join("one.md"), "one").unwrap();
        fs::write(source.path().join("two.md"), "two").unwrap();
        let mut staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();

        staged.objects[1].id = staged.objects[0].id.clone();
        assert!(staged.validate().is_err());
        staged.objects[1].id = "unique".into();
        staged.objects[1].source_path = "../outside.md".into();
        assert!(staged.validate().is_err());
        staged.objects[1].source_path = "C:/outside.md".into();
        assert!(staged.validate().is_err());
        staged.objects[1].source_path = "nested//outside.md".into();
        assert!(staged.validate().is_err());

        staged.objects[1].source_path = "two.md".into();
        let repeated_candidate = staged.objects[0].id.clone();
        staged.objects[1].links.push(StagedLink {
            kind: StagedLinkKind::Internal,
            target: "ambiguous".into(),
            label: None,
            resolution: StagedLinkResolution::Ambiguous,
            resolved_object_id: None,
            candidate_object_ids: vec![repeated_candidate; 2],
            raw: None,
        });
        assert!(staged.validate().is_err());
    }

    #[test]
    fn candidate_plan_resolves_global_folder_and_item_overrides_deterministically() {
        let source = TestDirectory::new();
        fs::create_dir_all(source.path().join("People/Heroes")).unwrap();
        fs::write(source.path().join("People/Heroes/Alice.md"), "Alice").unwrap();
        fs::write(source.path().join("People/Bob.md"), "Bob").unwrap();
        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let alice = staged
            .objects
            .iter()
            .find(|object| object.title == "Alice")
            .unwrap();
        let mut overrides = ImportMappingOverrides::default();
        overrides.global.entity_type = Some("note".into());
        overrides
            .global
            .field_mappings
            .insert("tag".into(), "core:tag".into());
        overrides.folders.insert(
            "People".into(),
            ImportMappingDecision {
                entity_type: Some("person".into()),
                field_mappings: BTreeMap::from([("tag".into(), "lore:tag".into())]),
                relationship_mappings: BTreeMap::new(),
            },
        );
        overrides.folders.insert(
            "People/Heroes".into(),
            ImportMappingDecision {
                entity_type: Some("hero".into()),
                ..ImportMappingDecision::default()
            },
        );
        overrides.items.insert(
            alice.id.clone(),
            ImportMappingDecision {
                entity_type: Some("protagonist".into()),
                ..ImportMappingDecision::default()
            },
        );

        let first = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "session".into(),
                importer: staged.importer.clone(),
                source: staged.source.clone(),
                captured_content_generation: 7,
                current_content_generation: 7,
                manifest_fingerprint: "manifest-v1".into(),
                objects: staged.objects.clone(),
                unsupported_count: staged.unsupported.len(),
                diagnostics: staged.diagnostics.clone(),
            },
            &overrides,
        )
        .unwrap();
        let second = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "session".into(),
                importer: staged.importer,
                source: staged.source,
                captured_content_generation: 7,
                current_content_generation: 7,
                manifest_fingerprint: "manifest-v1".into(),
                objects: staged.objects.clone(),
                unsupported_count: staged.unsupported.len(),
                diagnostics: staged.diagnostics,
            },
            &overrides,
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.unresolved_decision_count, 0);
        assert_eq!(
            first
                .objects
                .iter()
                .find(|object| object.title == "Alice")
                .unwrap()
                .mapping
                .entity_type
                .as_deref(),
            Some("protagonist")
        );
        let bob = first
            .objects
            .iter()
            .find(|object| object.title == "Bob")
            .unwrap();
        assert_eq!(bob.mapping.entity_type.as_deref(), Some("person"));
        assert_eq!(bob.mapping.field_mappings["tag"], "lore:tag");
    }

    #[test]
    fn candidate_plan_surfaces_unresolved_types_and_stale_generation() {
        let source = TestDirectory::new();
        fs::write(source.path().join("note.md"), "note").unwrap();
        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();

        let plan = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "session".into(),
                importer: staged.importer,
                source: staged.source,
                captured_content_generation: 1,
                current_content_generation: 2,
                manifest_fingerprint: "manifest-v1".into(),
                objects: staged.objects,
                unsupported_count: 0,
                diagnostics: Vec::new(),
            },
            &ImportMappingOverrides::default(),
        )
        .unwrap();

        assert_eq!(plan.unresolved_decision_count, 1);
        assert_eq!(plan.objects[0].issues[0].code, "entity_type_required");
        assert_eq!(plan.issues[0].code, "project_generation_changed");
    }

    #[test]
    fn markdown_analysis_preserves_frontmatter_and_resolves_safe_links_and_assets() {
        let source = TestDirectory::new();
        fs::create_dir_all(source.path().join("Notes")).unwrap();
        fs::create_dir_all(source.path().join("assets")).unwrap();
        fs::write(
            source.path().join("Notes/Note.md"),
            "---\ncategory: place\n---\n# Note\n\n[Other][other]\n![Map](../assets/map.png)\n![Missing](../../outside.png)\n[Web](https://example.com)\n\n[other]: Other%20Note.md\n",
        )
        .unwrap();
        fs::write(source.path().join("Notes/Other Note.md"), "# Other").unwrap();
        fs::write(
            source.path().join("assets/map.png"),
            b"\x89PNG\r\n\x1a\nfixture",
        )
        .unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let note = staged
            .objects
            .iter()
            .find(|object| object.source_path == "Notes/Note.md")
            .unwrap();
        assert_eq!(note.fields["frontmatter"], "category: place\n");
        assert_eq!(note.raw_source_data["frontmatter"], "category: place\n");
        assert!(note
            .body
            .as_ref()
            .unwrap()
            .body
            .starts_with("---\ncategory: place\n---\n"));
        assert_eq!(note.links.len(), 4);
        assert!(note.links.iter().any(|link| {
            link.target == "Other%20Note.md" && link.resolution == StagedLinkResolution::Resolved
        }));
        assert!(note.links.iter().any(|link| {
            link.target == "../assets/map.png"
                && link.resolution == StagedLinkResolution::NotApplicable
        }));
        assert!(note.links.iter().any(|link| {
            link.target == "../../outside.png" && link.resolution == StagedLinkResolution::Missing
        }));
        assert_eq!(staged.assets.len(), 1);
        assert_eq!(
            staged.assets[0].owner_object_id.as_deref(),
            Some(note.id.as_str())
        );
        assert_eq!(staged.summary.asset_count, 1);
        assert_eq!(staged.summary.unresolved_link_count, 1);
    }

    #[test]
    fn markdown_analysis_rejects_malformed_asset_signatures() {
        let source = TestDirectory::new();
        fs::write(source.path().join("fake.png"), b"not a png").unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();

        assert!(staged.assets.is_empty());
        assert_eq!(staged.unsupported.len(), 1);
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "invalid_asset_content"));
    }

    #[test]
    fn validation_requires_explicit_duplicate_decision_and_uses_enabled_catalog() {
        let source = TestDirectory::new();
        fs::write(source.path().join("note.md"), "# Note").unwrap();
        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let object = staged.objects[0].clone();
        let mut overrides = ImportMappingOverrides::default();
        overrides.global.entity_type = Some("note".into());
        let candidate = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "session".into(),
                importer: staged.importer,
                source: staged.source,
                captured_content_generation: 4,
                current_content_generation: 4,
                manifest_fingerprint: "manifest-v1".into(),
                objects: vec![object.clone()],
                unsupported_count: 0,
                diagnostics: Vec::new(),
            },
            &overrides,
        )
        .unwrap();
        let build = ImportValidationBuild {
            candidate,
            staged_objects: vec![object.clone()],
            staged_assets: vec![StagedAsset {
                id: "asset".into(),
                source_path: "map.png".into(),
                filename: "map.png".into(),
                size: 8,
                mime_type: Some("image/png".into()),
                content_hash: Some(format!("sha256:{}", "0".repeat(64))),
                owner_object_id: Some(object.id.clone()),
                relationship: Some("attachment".into()),
                raw_metadata: BTreeMap::new(),
                diagnostics: Vec::new(),
            }],
            catalog: ImportMappingCatalog {
                fingerprint: "manifest-v1".into(),
                entity_types: BTreeSet::from(["note".into()]),
                fields: BTreeMap::new(),
                relationship_types: BTreeSet::new(),
            },
            decisions: BTreeMap::new(),
            existing_targets: BTreeMap::new(),
            duplicate_targets: BTreeMap::from([(object.id.clone(), vec!["existing".into()])]),
        };

        let unresolved = validate_import_candidate_plan(build.clone()).unwrap();
        assert!(unresolved.plan.is_none());
        assert!(unresolved
            .issues
            .iter()
            .any(|issue| issue.code == "duplicate_source_identity"));

        let accepted = validate_import_candidate_plan(ImportValidationBuild {
            decisions: BTreeMap::from([(object.id, ImportObjectDecision::Create)]),
            ..build
        })
        .unwrap();
        let plan = accepted.plan.unwrap();
        assert_eq!(plan.content_generation, 4);
        assert_eq!(plan.objects.len(), 1);
        assert_eq!(plan.assets.len(), 1);
        assert_eq!(plan.objects[0].entity_type.as_deref(), Some("note"));
    }

    #[cfg(unix)]
    #[test]
    fn folder_analysis_reports_symlinks_without_following_them() {
        use std::os::unix::fs::symlink;

        let source = TestDirectory::new();
        let outside = TestDirectory::new();
        fs::write(outside.path().join("secret.md"), "not imported").unwrap();
        symlink(outside.path(), source.path().join("linked")).unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();

        assert!(staged.objects.is_empty());
        assert_eq!(staged.unsupported.len(), 1);
        assert_eq!(staged.unsupported[0].source_path, "linked");
        assert_eq!(staged.unsupported[0].source_kind, "symlink");

        let error = analyze_generic_documents(
            source.path().join("linked"),
            GenericDocumentImportLimits::default(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("import source root cannot be a symbolic link"));
    }
}
