use crate::CoreError;
use dom_query::{Document, NodeRef};
use pulldown_cmark::{Event, Options, Parser, Tag};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

pub const STAGED_IMPORT_SCHEMA_VERSION: u32 = 1;
pub const IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION: u32 = 1;
pub const VALIDATED_IMPORT_PLAN_SCHEMA_VERSION: u32 = 1;
pub const GENERIC_DOCUMENT_IMPORTER_ID: &str = "daena.generic-documents";
pub const GENERIC_DOCUMENT_IMPORTER_VERSION: &str = "1";
pub const EXTERNAL_IMPORT_ANALYSIS_CANCELLED: &str = "external import analysis cancelled";
const MAX_ARCHIVE_COMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ARCHIVE_COMPRESSION_RATIO: u64 = 200;
const MAX_ARCHIVE_PATH_BYTES: usize = 1_024;
const MAX_ARCHIVE_EXPANDED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 20_000;
const MAX_ARCHIVE_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_HTML_DOM_NODES: usize = 100_000;
const MAX_HTML_DOM_DEPTH: usize = 128;
const MAX_HTML_MARKDOWN_BYTES: usize = 32 * 1024 * 1024;
const MAX_DOCX_ENTRIES: usize = 4_096;
const MAX_DOCX_DEPTH: usize = 32;
const MAX_DOCX_XML_NODES: u32 = 200_000;
const MAX_DOCX_MARKDOWN_BYTES: usize = 32 * 1024 * 1024;

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
    let is_zip_archive = metadata.is_file() && is_zip_path(source);
    let source_kind = if metadata.is_dir() {
        ImportSourceKind::Folder
    } else if is_zip_archive {
        ImportSourceKind::Archive
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
        discovered_entries: usize::from(metadata.is_file() && !is_zip_archive),
        discovered_files: 0,
        processed_entries: 0,
        total_source_bytes: 0,
        folders: BTreeSet::new(),
        progress: &mut progress,
    };

    analyzer.report_progress(None)?;

    if metadata.is_dir() {
        analyzer.analyze_directory(source, &[], 0)?;
    } else if is_zip_archive {
        analyzer.analyze_archive(source, metadata.len())?;
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
    fn analyze_archive(&mut self, path: &Path, compressed_bytes: u64) -> Result<(), CoreError> {
        if compressed_bytes > MAX_ARCHIVE_COMPRESSED_BYTES {
            return Err(CoreError::Validation(format!(
                "ZIP archive exceeds the maximum compressed size of {MAX_ARCHIVE_COMPRESSED_BYTES} bytes"
            )));
        }
        let file = fs::File::open(path).map_err(|source| CoreError::Io {
            operation: "open import ZIP archive",
            source,
        })?;
        let mut archive = ZipArchive::new(file)
            .map_err(|error| CoreError::Validation(format!("invalid ZIP archive: {error}")))?;
        if archive.len() > self.limits.max_entries {
            return Err(CoreError::Validation(format!(
                "ZIP archive exceeds the maximum entry count of {}",
                self.limits.max_entries
            )));
        }

        struct ArchiveEntryPlan {
            index: usize,
            source_path: String,
            is_dir: bool,
            size: u64,
        }
        let mut entries = Vec::with_capacity(archive.len());
        let mut names = BTreeSet::new();
        let mut folded_names = BTreeSet::new();
        let mut expanded_bytes = 0_u64;
        for index in 0..archive.len() {
            let entry = archive.by_index(index).map_err(|error| {
                CoreError::Validation(format!("invalid ZIP central-directory entry: {error}"))
            })?;
            let is_dir = entry.is_dir();
            let source_path = validate_archive_source_path(entry.name_raw(), is_dir)?;
            self.report_progress(Some(source_path.clone()))?;
            if !names.insert(source_path.clone())
                || !folded_names.insert(source_path.to_lowercase())
            {
                return Err(CoreError::Validation(format!(
                    "ZIP archive contains duplicate or case-colliding path: {source_path}"
                )));
            }
            if !is_dir
                && entry
                    .unix_mode()
                    .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
            {
                return Err(CoreError::Validation(format!(
                    "ZIP links and special files are not allowed: {source_path}"
                )));
            }
            let depth = source_path.split('/').count().saturating_sub(1);
            if depth > self.limits.max_depth {
                return Err(CoreError::Validation(format!(
                    "ZIP entry exceeds the maximum folder depth of {}: {source_path}",
                    self.limits.max_depth
                )));
            }
            let size = entry.size();
            if !is_dir && size > self.limits.max_file_bytes {
                return Err(CoreError::Validation(format!(
                    "ZIP entry '{source_path}' exceeds the maximum file size of {} bytes",
                    self.limits.max_file_bytes
                )));
            }
            expanded_bytes = expanded_bytes
                .checked_add(size)
                .ok_or_else(|| CoreError::Validation("ZIP expanded size overflowed".into()))?;
            if expanded_bytes > self.limits.max_total_bytes {
                return Err(CoreError::Validation(format!(
                    "ZIP archive exceeds the maximum expanded size of {} bytes",
                    self.limits.max_total_bytes
                )));
            }
            let packed = entry.compressed_size();
            if size > 0
                && (packed == 0 || size > packed.saturating_mul(MAX_ARCHIVE_COMPRESSION_RATIO))
            {
                return Err(CoreError::Validation(format!(
                    "ZIP entry exceeds the maximum compression ratio of {MAX_ARCHIVE_COMPRESSION_RATIO}:1: {source_path}"
                )));
            }
            entries.push(ArchiveEntryPlan {
                index,
                source_path,
                is_dir,
                size,
            });
        }

        self.discovered_entries = entries.len();
        for planned in entries {
            record_parent_folders(&mut self.folders, &planned.source_path);
            if planned.is_dir {
                self.folders.insert(planned.source_path.clone());
                self.finish_entry(Some(planned.source_path))?;
                continue;
            }
            self.discovered_files = self.discovered_files.saturating_add(1);
            if self.discovered_files > self.limits.max_files {
                return Err(CoreError::Validation(format!(
                    "ZIP archive exceeds the maximum file count of {}",
                    self.limits.max_files
                )));
            }
            if asset_mime_type(&planned.source_path).is_none()
                && document_format(&planned.source_path).is_none()
            {
                self.record_unsupported(
                    planned.source_path.clone(),
                    "archive_entry",
                    "file type is not supported by the generic document importer",
                )?;
                self.finish_entry(Some(planned.source_path))?;
                continue;
            }
            let next_total = self
                .total_source_bytes
                .checked_add(planned.size)
                .ok_or_else(|| {
                    CoreError::Validation("import source byte count overflowed".into())
                })?;
            let mut entry = archive.by_index(planned.index).map_err(|error| {
                CoreError::Validation(format!("invalid ZIP entry data: {error}"))
            })?;
            let mut bytes = Vec::with_capacity(planned.size.min(1024 * 1024) as usize);
            entry
                .by_ref()
                .take(planned.size.saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|source| CoreError::Io {
                    operation: "read import ZIP entry",
                    source,
                })?;
            if bytes.len() as u64 != planned.size {
                return Err(CoreError::Validation(format!(
                    "ZIP entry size does not match its central-directory declaration: {}",
                    planned.source_path
                )));
            }
            drop(entry);
            self.analyze_loaded_file(&planned.source_path, planned.size, next_total, bytes)?;
            self.finish_entry(Some(planned.source_path))?;
        }
        Ok(())
    }

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
        if asset_mime_type(source_path).is_none() && document_format(source_path).is_none() {
            return self.record_unsupported(
                source_path.to_owned(),
                "file",
                "file type is not supported by the generic document importer",
            );
        }
        let bytes = fs::read(path).map_err(|source| CoreError::Io {
            operation: "read import source file",
            source,
        })?;
        if bytes.len() as u64 != size {
            return Err(CoreError::Conflict(format!(
                "import file '{source_path}' changed during analysis"
            )));
        }
        self.analyze_loaded_file(source_path, size, next_total, bytes)
    }

    fn analyze_loaded_file(
        &mut self,
        source_path: &str,
        size: u64,
        next_total: u64,
        bytes: Vec<u8>,
    ) -> Result<(), CoreError> {
        self.total_source_bytes = next_total;
        if let Some(mime_type) = asset_mime_type(source_path) {
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
        let source_format =
            document_format(source_path).expect("supported file format was checked");
        if source_format == "docx" {
            return self.analyze_docx_file(source_path, size, bytes);
        }
        let mut body = if let Ok(body) = String::from_utf8(bytes) {
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
        let mut title = document_title(source_path);
        let parent_source_path = source_path
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_owned());
        let mut body_format = source_format;
        let (frontmatter, fields, mut raw_source_data) = if source_format == "markdown" {
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
        let mut metadata = BTreeMap::new();
        if source_format == "html" {
            let conversion = convert_html_to_markdown(&body)?;
            if let Some(html_title) = conversion.title {
                title = html_title;
            }
            raw_source_data.insert("html".into(), serde_json::Value::String(body));
            body = conversion.markdown;
            body_format = "markdown";
            metadata.insert(
                "converted_from".into(),
                serde_json::Value::String("html".into()),
            );
            for warning in conversion.warnings {
                self.record_diagnostic(ImportDiagnostic {
                    severity: ImportDiagnosticSeverity::Warning,
                    code: warning.code.into(),
                    message: warning.message,
                    source_path: Some(source_path.to_owned()),
                    object_id: Some(source_id.clone()),
                })?;
            }
        }
        let links = if body_format == "markdown" {
            discover_markdown_links(&body)
        } else {
            Vec::new()
        };
        if frontmatter.is_some() {
            metadata.insert(
                "frontmatter_format".into(),
                serde_json::Value::String("yaml".into()),
            );
        }
        self.import.objects.push(StagedObject {
            id: source_id.clone(),
            source_id,
            source_kind: source_format.to_owned(),
            source_path: source_path.to_owned(),
            content_hash,
            title,
            body: Some(StagedDocument {
                format: body_format.to_owned(),
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

    fn analyze_docx_file(
        &mut self,
        source_path: &str,
        size: u64,
        bytes: Vec<u8>,
    ) -> Result<(), CoreError> {
        let content_hash = hex_digest(&bytes);
        let source_id = hex_digest(
            format!(
                "{}\0{}\0{}",
                GENERIC_DOCUMENT_IMPORTER_ID, self.import.source.id, source_path
            )
            .as_bytes(),
        );
        let conversion = convert_docx_to_markdown(&bytes, source_path)?;
        let title = conversion
            .title
            .unwrap_or_else(|| document_title(source_path));
        for warning in conversion.warnings {
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: warning.code.into(),
                message: warning.message,
                source_path: Some(source_path.to_owned()),
                object_id: Some(source_id.clone()),
            })?;
        }
        for asset in conversion.assets {
            let asset_source_path = format!("{source_path}!/{}", asset.entry_path);
            let asset_id = hex_digest(
                format!(
                    "{}\0{}\0asset\0{}",
                    GENERIC_DOCUMENT_IMPORTER_ID, self.import.source.id, asset_source_path
                )
                .as_bytes(),
            );
            self.import.assets.push(StagedAsset {
                id: asset_id,
                source_path: asset_source_path,
                filename: asset.filename,
                size: asset.bytes.len() as u64,
                mime_type: Some(asset.mime_type.into()),
                content_hash: Some(format!("sha256:{}", hex_digest(&asset.bytes))),
                owner_object_id: None,
                relationship: Some("attachment".into()),
                raw_metadata: BTreeMap::from([(
                    "docx_entry".into(),
                    serde_json::Value::String(asset.entry_path),
                )]),
                diagnostics: Vec::new(),
            });
        }
        let links = discover_markdown_links(&conversion.markdown);
        self.import.objects.push(StagedObject {
            id: source_id.clone(),
            source_id,
            source_kind: "docx".into(),
            source_path: source_path.to_owned(),
            content_hash,
            title,
            body: Some(StagedDocument {
                format: "markdown".into(),
                body: conversion.markdown,
            }),
            parent_source_path: source_path
                .rsplit_once('/')
                .map(|(parent, _)| parent.to_owned()),
            tags: Vec::new(),
            aliases: Vec::new(),
            fields: BTreeMap::new(),
            metadata: BTreeMap::from([
                (
                    "converted_from".into(),
                    serde_json::Value::String("docx".into()),
                ),
                (
                    "package_entry_count".into(),
                    serde_json::Value::from(conversion.package_entry_count),
                ),
                ("source_size".into(), serde_json::Value::from(size)),
            ]),
            raw_source_data: {
                let mut raw = BTreeMap::from([(
                    "package_entries".into(),
                    serde_json::Value::Array(
                        conversion
                            .package_entries
                            .into_iter()
                            .map(serde_json::Value::String)
                            .collect(),
                    ),
                )]);
                if let Some(value) = conversion.core_properties {
                    raw.insert(
                        "core_properties_xml".into(),
                        serde_json::Value::String(value),
                    );
                }
                raw
            },
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

fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn validate_archive_source_path(raw_name: &[u8], is_dir: bool) -> Result<String, CoreError> {
    let name = std::str::from_utf8(raw_name)
        .map_err(|_| CoreError::Validation("ZIP entry path is not valid UTF-8".into()))?;
    if name.is_empty()
        || name.len() > MAX_ARCHIVE_PATH_BYTES
        || name.starts_with('/')
        || name.contains('\\')
        || name.contains(':')
        || name.chars().any(char::is_control)
    {
        return Err(CoreError::Validation(format!(
            "ZIP entry path is not portable: {name}"
        )));
    }
    let normalized = if is_dir {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if normalized.is_empty()
        || normalized
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(CoreError::Validation(format!(
            "ZIP entry path escapes or is not normalized: {name}"
        )));
    }
    Ok(normalized.into())
}

pub(crate) fn read_archive_asset_bytes(
    archive_path: &Path,
    target_path: &str,
    expected_size: u64,
) -> Result<Vec<u8>, CoreError> {
    read_archive_entry_bytes(archive_path, target_path, Some(expected_size))
}

fn read_archive_entry_bytes(
    archive_path: &Path,
    target_path: &str,
    expected_size: Option<u64>,
) -> Result<Vec<u8>, CoreError> {
    let metadata = fs::symlink_metadata(archive_path).map_err(|source| CoreError::Io {
        operation: "read import ZIP metadata",
        source,
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_ARCHIVE_COMPRESSED_BYTES
    {
        return Err(CoreError::Validation(
            "import ZIP source is unavailable or exceeds its compressed-size limit".into(),
        ));
    }
    let file = fs::File::open(archive_path).map_err(|source| CoreError::Io {
        operation: "open import ZIP archive",
        source,
    })?;
    let mut archive = ZipArchive::new(file)
        .map_err(|error| CoreError::Validation(format!("invalid ZIP archive: {error}")))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(CoreError::Validation(
            "ZIP archive exceeds the entry limit during asset preflight".into(),
        ));
    }
    let mut names = BTreeSet::new();
    let mut folded_names = BTreeSet::new();
    let mut expanded_bytes = 0_u64;
    let mut target_index = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| {
            CoreError::Validation(format!("invalid ZIP central-directory entry: {error}"))
        })?;
        let is_dir = entry.is_dir();
        let source_path = validate_archive_source_path(entry.name_raw(), is_dir)?;
        if !names.insert(source_path.clone()) || !folded_names.insert(source_path.to_lowercase()) {
            return Err(CoreError::Validation(format!(
                "ZIP archive contains duplicate or case-colliding path: {source_path}"
            )));
        }
        if !is_dir
            && entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err(CoreError::Validation(format!(
                "ZIP links and special files are not allowed: {source_path}"
            )));
        }
        let size = entry.size();
        if !is_dir && size > MAX_ARCHIVE_FILE_BYTES {
            return Err(CoreError::Validation(format!(
                "ZIP entry exceeds the file-size limit: {source_path}"
            )));
        }
        expanded_bytes = expanded_bytes
            .checked_add(size)
            .ok_or_else(|| CoreError::Validation("ZIP expanded size overflowed".into()))?;
        if expanded_bytes > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err(CoreError::Validation(
                "ZIP archive exceeds the expanded-size limit".into(),
            ));
        }
        let packed = entry.compressed_size();
        if size > 0 && (packed == 0 || size > packed.saturating_mul(MAX_ARCHIVE_COMPRESSION_RATIO))
        {
            return Err(CoreError::Validation(format!(
                "ZIP entry exceeds the compression-ratio limit: {source_path}"
            )));
        }
        if !is_dir && source_path == target_path {
            target_index = Some(index);
        }
    }
    let target_index = target_index.ok_or_else(|| {
        CoreError::Conflict(format!(
            "import ZIP asset disappeared after analysis: {target_path}"
        ))
    })?;
    let mut entry = archive
        .by_index(target_index)
        .map_err(|error| CoreError::Validation(format!("invalid ZIP asset entry: {error}")))?;
    if expected_size.is_some_and(|expected_size| entry.size() != expected_size) {
        return Err(CoreError::Conflict(format!(
            "import ZIP asset changed size after analysis: {target_path}"
        )));
    }
    let actual_size = entry.size();
    let mut bytes = Vec::with_capacity(actual_size.min(1024 * 1024) as usize);
    entry
        .by_ref()
        .take(actual_size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CoreError::Io {
            operation: "read import ZIP asset",
            source,
        })?;
    if bytes.len() as u64 != actual_size {
        return Err(CoreError::Conflict(format!(
            "import ZIP asset data changed after analysis: {target_path}"
        )));
    }
    Ok(bytes)
}

pub(crate) fn read_docx_import_asset_bytes(
    source_root: &Path,
    source_kind: &ImportSourceKind,
    asset_source_path: &str,
    expected_size: u64,
) -> Result<Vec<u8>, CoreError> {
    let (container_path, entry_path) = asset_source_path.split_once("!/").ok_or_else(|| {
        CoreError::Validation("DOCX import asset path is missing its container boundary".into())
    })?;
    validate_source_path(container_path)?;
    if document_format(container_path) != Some("docx") {
        return Err(CoreError::Validation(
            "DOCX import asset container is not a DOCX source".into(),
        ));
    }
    let entry_path = validate_archive_source_path(entry_path.as_bytes(), false)?;
    let package_bytes = match source_kind {
        ImportSourceKind::File => {
            if source_root.file_name().and_then(|name| name.to_str()) != Some(container_path) {
                return Err(CoreError::Conflict(
                    "DOCX import source path changed after analysis".into(),
                ));
            }
            let metadata = fs::symlink_metadata(source_root).map_err(|source| CoreError::Io {
                operation: "read DOCX import source metadata",
                source,
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_ARCHIVE_COMPRESSED_BYTES
            {
                return Err(CoreError::Validation(
                    "DOCX import source must remain a bounded regular file".into(),
                ));
            }
            fs::read(source_root).map_err(|source| CoreError::Io {
                operation: "read DOCX import source",
                source,
            })?
        }
        ImportSourceKind::Folder => {
            let path = crate::storage::normalized_project_path(source_root, container_path)?;
            let metadata = fs::symlink_metadata(&path).map_err(|source| CoreError::Io {
                operation: "read DOCX import source metadata",
                source,
            })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.len() > MAX_ARCHIVE_COMPRESSED_BYTES
            {
                return Err(CoreError::Validation(
                    "DOCX import source must remain a bounded regular file".into(),
                ));
            }
            fs::read(path).map_err(|source| CoreError::Io {
                operation: "read DOCX import source",
                source,
            })?
        }
        ImportSourceKind::Archive => read_archive_entry_bytes(source_root, container_path, None)?,
        _ => {
            return Err(CoreError::Validation(
                "this import source kind cannot provide DOCX attachments".into(),
            ));
        }
    };
    let mut package = ZipArchive::new(Cursor::new(package_bytes.as_slice()))
        .map_err(|error| CoreError::Validation(format!("invalid DOCX package: {error}")))?;
    let entries = preflight_docx_package(&mut package)?;
    let content_types = read_docx_entry(&mut package, &entries, "[Content_Types].xml")?;
    let content_types = decode_docx_xml(&content_types, "[Content_Types].xml")?;
    validate_docx_content_types(&content_types)?;
    let bytes = read_docx_entry(&mut package, &entries, &entry_path)?;
    if bytes.len() as u64 != expected_size {
        return Err(CoreError::Conflict(format!(
            "DOCX import asset changed size after analysis: {asset_source_path}"
        )));
    }
    Ok(bytes)
}

pub(crate) fn is_docx_import_asset_source_path(source_path: &str) -> bool {
    source_path
        .split_once("!/")
        .and_then(|(container, entry)| (!entry.is_empty()).then_some(container))
        .is_some_and(|container| document_format(container) == Some("docx"))
}

fn record_parent_folders(folders: &mut BTreeSet<String>, source_path: &str) {
    let parts = source_path.split('/').collect::<Vec<_>>();
    for end in 1..parts.len() {
        folders.insert(parts[..end].join("/"));
    }
}

#[derive(Debug)]
struct DocxConversion {
    markdown: String,
    title: Option<String>,
    assets: Vec<DocxAsset>,
    warnings: Vec<DocxWarning>,
    core_properties: Option<String>,
    package_entry_count: usize,
    package_entries: Vec<String>,
}

#[derive(Debug)]
struct DocxAsset {
    entry_path: String,
    filename: String,
    mime_type: &'static str,
    bytes: Vec<u8>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DocxWarning {
    code: &'static str,
    message: String,
}

#[derive(Debug, Clone)]
struct DocxEntryPlan {
    index: usize,
    size: u64,
}

#[derive(Debug, Clone)]
struct DocxRelationship {
    target: String,
    external: bool,
    relationship_type: String,
}

fn preflight_docx_package(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<BTreeMap<String, DocxEntryPlan>, CoreError> {
    if archive.len() > MAX_DOCX_ENTRIES {
        return Err(CoreError::Validation(format!(
            "DOCX package exceeds the maximum entry count of {MAX_DOCX_ENTRIES}"
        )));
    }
    let mut entries = BTreeMap::new();
    let mut folded_names = BTreeSet::new();
    let mut expanded_bytes = 0_u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| {
            CoreError::Validation(format!("invalid DOCX central-directory entry: {error}"))
        })?;
        let is_dir = entry.is_dir();
        let source_path = validate_archive_source_path(entry.name_raw(), is_dir)?;
        if entries.contains_key(&source_path) || !folded_names.insert(source_path.to_lowercase()) {
            return Err(CoreError::Validation(format!(
                "DOCX package contains a duplicate or case-colliding path: {source_path}"
            )));
        }
        if !is_dir
            && entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 != 0 && mode & 0o170000 != 0o100000)
        {
            return Err(CoreError::Validation(format!(
                "DOCX links and special files are not allowed: {source_path}"
            )));
        }
        let depth = source_path.split('/').count().saturating_sub(1);
        if depth > MAX_DOCX_DEPTH {
            return Err(CoreError::Validation(format!(
                "DOCX entry exceeds the maximum package depth of {MAX_DOCX_DEPTH}: {source_path}"
            )));
        }
        let size = entry.size();
        if !is_dir && size > MAX_ARCHIVE_FILE_BYTES {
            return Err(CoreError::Validation(format!(
                "DOCX entry exceeds the file-size limit: {source_path}"
            )));
        }
        expanded_bytes = expanded_bytes
            .checked_add(size)
            .ok_or_else(|| CoreError::Validation("DOCX expanded size overflowed".into()))?;
        if expanded_bytes > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err(CoreError::Validation(
                "DOCX package exceeds the expanded-size limit".into(),
            ));
        }
        let packed = entry.compressed_size();
        if size > 0 && (packed == 0 || size > packed.saturating_mul(MAX_ARCHIVE_COMPRESSION_RATIO))
        {
            return Err(CoreError::Validation(format!(
                "DOCX entry exceeds the compression-ratio limit: {source_path}"
            )));
        }
        entries.insert(source_path, DocxEntryPlan { index, size });
    }
    Ok(entries)
}

fn read_docx_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    entries: &BTreeMap<String, DocxEntryPlan>,
    path: &str,
) -> Result<Vec<u8>, CoreError> {
    let plan = entries.get(path).ok_or_else(|| {
        CoreError::Validation(format!("DOCX package is missing required entry: {path}"))
    })?;
    let mut entry = archive
        .by_index(plan.index)
        .map_err(|error| CoreError::Validation(format!("invalid DOCX entry '{path}': {error}")))?;
    let mut bytes = Vec::with_capacity(plan.size.min(1024 * 1024) as usize);
    entry
        .by_ref()
        .take(plan.size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CoreError::Io {
            operation: "read DOCX package entry",
            source,
        })?;
    if bytes.len() as u64 != plan.size {
        return Err(CoreError::Validation(format!(
            "DOCX entry size does not match its declaration: {path}"
        )));
    }
    Ok(bytes)
}

fn decode_docx_xml(bytes: &[u8], label: &str) -> Result<String, CoreError> {
    if let Some(bytes) = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8(bytes.to_vec()).map_err(|_| {
            CoreError::Validation(format!("DOCX XML entry is not valid UTF-8: {label}"))
        });
    }
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little_endian = bytes.starts_with(&[0xff, 0xfe]);
        let payload = &bytes[2..];
        let (pairs, remainder) = payload.as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(CoreError::Validation(format!(
                "DOCX XML entry has malformed UTF-16 data: {label}"
            )));
        }
        let units = pairs
            .iter()
            .map(|pair| {
                if little_endian {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect::<Vec<_>>();
        return String::from_utf16(&units).map_err(|_| {
            CoreError::Validation(format!("DOCX XML entry is not valid UTF-16: {label}"))
        });
    }
    String::from_utf8(bytes.to_vec())
        .map_err(|_| CoreError::Validation(format!("DOCX XML entry is not valid UTF-8: {label}")))
}

fn parse_docx_xml<'a>(xml: &'a str, label: &str) -> Result<roxmltree::Document<'a>, CoreError> {
    roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: MAX_DOCX_XML_NODES,
        },
    )
    .map_err(|error| CoreError::Validation(format!("invalid DOCX XML in {label}: {error}")))
}

fn docx_attribute<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.name() == local_name)
        .map(|attribute| attribute.value())
}

fn docx_child<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    local_name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == local_name)
}

fn docx_relationships(xml: &str) -> Result<BTreeMap<String, DocxRelationship>, CoreError> {
    let document = parse_docx_xml(xml, "word/_rels/document.xml.rels")?;
    let mut relationships = BTreeMap::new();
    for node in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "Relationship")
    {
        let id = docx_attribute(node, "Id")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| CoreError::Validation("DOCX relationship is missing an ID".into()))?;
        let target = docx_attribute(node, "Target")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| CoreError::Validation("DOCX relationship is missing a target".into()))?;
        let relationship_type = docx_attribute(node, "Type").unwrap_or_default();
        let relationship = DocxRelationship {
            target: target.into(),
            external: docx_attribute(node, "TargetMode")
                .is_some_and(|value| value.eq_ignore_ascii_case("External")),
            relationship_type: relationship_type.into(),
        };
        if relationships.insert(id.into(), relationship).is_some() {
            return Err(CoreError::Validation(format!(
                "DOCX relationship ID is duplicated: {id}"
            )));
        }
    }
    Ok(relationships)
}

fn normalize_docx_part_target(target: &str) -> Option<String> {
    if target.is_empty()
        || target.starts_with('/')
        || target.contains('\\')
        || target.contains(':')
        || target.chars().any(char::is_control)
    {
        return None;
    }
    let mut components = vec!["word".to_owned()];
    for component in target.split('/') {
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

fn docx_warn(warnings: &mut BTreeSet<DocxWarning>, code: &'static str, message: impl Into<String>) {
    warnings.insert(DocxWarning {
        code,
        message: message.into(),
    });
}

fn validate_docx_content_types(xml: &str) -> Result<(), CoreError> {
    let document = parse_docx_xml(xml, "[Content_Types].xml")?;
    let valid = document.descendants().any(|node| {
        node.is_element()
            && node.tag_name().name() == "Override"
            && docx_attribute(node, "PartName") == Some("/word/document.xml")
            && docx_attribute(node, "ContentType").is_some_and(|value| {
                value
                    == "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
            })
    });
    if !valid {
        return Err(CoreError::Validation(
            "DOCX package does not declare a standard Word document part".into(),
        ));
    }
    Ok(())
}

fn docx_core_title(xml: &str) -> Result<Option<String>, CoreError> {
    let document = parse_docx_xml(xml, "docProps/core.xml")?;
    let title = document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "title")
        .and_then(|node| node.text())
        .map(|value| value.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|value| !value.is_empty());
    Ok(title)
}

fn docx_heading_styles(xml: Option<&str>) -> Result<BTreeMap<String, usize>, CoreError> {
    let Some(xml) = xml else {
        return Ok(BTreeMap::new());
    };
    let document = parse_docx_xml(xml, "word/styles.xml")?;
    let mut styles = BTreeMap::new();
    for style in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "style")
    {
        if docx_attribute(style, "type") != Some("paragraph") {
            continue;
        }
        let Some(style_id) = docx_attribute(style, "styleId") else {
            continue;
        };
        let outline = docx_child(style, "pPr")
            .and_then(|properties| docx_child(properties, "outlineLvl"))
            .and_then(|node| docx_attribute(node, "val"))
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|value| *value < 6)
            .map(|value| value + 1);
        let name = docx_child(style, "name")
            .and_then(|node| docx_attribute(node, "val"))
            .and_then(docx_heading_level);
        if let Some(level) = outline.or(name) {
            styles.insert(style_id.into(), level);
        }
    }
    Ok(styles)
}

fn docx_heading_level(value: &str) -> Option<usize> {
    let compact = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    compact
        .strip_prefix("heading")
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .filter(|level| (1..=6).contains(level))
}

fn docx_numbering(xml: Option<&str>) -> Result<BTreeMap<(String, usize), bool>, CoreError> {
    let Some(xml) = xml else {
        return Ok(BTreeMap::new());
    };
    let document = parse_docx_xml(xml, "word/numbering.xml")?;
    let mut abstract_levels = BTreeMap::<(String, usize), bool>::new();
    for abstract_numbering in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "abstractNum")
    {
        let Some(abstract_id) = docx_attribute(abstract_numbering, "abstractNumId") else {
            continue;
        };
        for level in abstract_numbering
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "lvl")
        {
            let index = docx_attribute(level, "ilvl")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let format = docx_child(level, "numFmt")
                .and_then(|node| docx_attribute(node, "val"))
                .unwrap_or("bullet");
            abstract_levels.insert(
                (abstract_id.into(), index),
                !matches!(format, "bullet" | "none"),
            );
        }
    }
    let mut result = BTreeMap::new();
    for numbering in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "num")
    {
        let Some(number_id) = docx_attribute(numbering, "numId") else {
            continue;
        };
        let Some(abstract_id) =
            docx_child(numbering, "abstractNumId").and_then(|node| docx_attribute(node, "val"))
        else {
            continue;
        };
        for ((candidate, level), ordered) in &abstract_levels {
            if candidate == abstract_id {
                result.insert((number_id.into(), *level), *ordered);
            }
        }
    }
    Ok(result)
}

fn docx_escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '|' | '<' | '>' | '#' | '+' | '-' | '!'
        ) {
            escaped.push('\\');
        }
        escaped.push(match character {
            '\r' | '\n' => ' ',
            value => value,
        });
    }
    escaped
}

fn markdown_inline_code(value: &str) -> String {
    let delimiter = "`".repeat(longest_character_run(value, '`').saturating_add(1).max(1));
    let pad = value.starts_with(['`', ' ']) || value.ends_with(['`', ' ']);
    if pad {
        format!("{delimiter} {value} {delimiter}")
    } else {
        format!("{delimiter}{value}{delimiter}")
    }
}

fn docx_toggle(properties: roxmltree::Node<'_, '_>, name: &str) -> bool {
    docx_child(properties, name).is_some_and(|node| {
        !docx_attribute(node, "val").is_some_and(|value| {
            matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "off")
        })
    })
}

struct DocxMarkdownWriter<'a> {
    output: String,
    relationships: &'a BTreeMap<String, DocxRelationship>,
    image_targets: &'a BTreeMap<String, String>,
    heading_styles: &'a BTreeMap<String, usize>,
    numbering: &'a BTreeMap<(String, usize), bool>,
    warnings: BTreeSet<DocxWarning>,
}

impl<'a> DocxMarkdownWriter<'a> {
    fn render(
        mut self,
        document: &roxmltree::Document<'_>,
    ) -> Result<(String, Vec<DocxWarning>), CoreError> {
        let body = document
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "body")
            .ok_or_else(|| CoreError::Validation("DOCX document XML has no body".into()))?;
        self.render_blocks(body)?;
        if self.output.len() > MAX_DOCX_MARKDOWN_BYTES {
            return Err(CoreError::Validation(
                "converted DOCX exceeds the Markdown output limit".into(),
            ));
        }
        Ok((
            format!("{}\n", self.output.trim()),
            self.warnings.into_iter().collect(),
        ))
    }

    fn render_blocks(&mut self, container: roxmltree::Node<'_, '_>) -> Result<(), CoreError> {
        for child in container.children().filter(roxmltree::Node::is_element) {
            match child.tag_name().name() {
                "p" => self.render_paragraph(child),
                "tbl" => self.render_table(child),
                "sdt" => {
                    if let Some(content) = child
                        .descendants()
                        .find(|node| node.is_element() && node.tag_name().name() == "sdtContent")
                    {
                        self.render_blocks(content)?;
                    }
                }
                "altChunk" => docx_warn(
                    &mut self.warnings,
                    "docx_content_omitted",
                    "An external DOCX altChunk could not be converted and was omitted.",
                ),
                "sectPr" => {}
                name => docx_warn(
                    &mut self.warnings,
                    "docx_content_omitted",
                    format!("Unsupported DOCX body element <{name}> was omitted."),
                ),
            }
        }
        Ok(())
    }

    fn render_paragraph(&mut self, paragraph: roxmltree::Node<'_, '_>) {
        let properties = docx_child(paragraph, "pPr");
        let style = properties
            .and_then(|node| docx_child(node, "pStyle"))
            .and_then(|node| docx_attribute(node, "val"));
        let heading = style
            .and_then(docx_heading_level)
            .or_else(|| style.and_then(|style| self.heading_styles.get(style).copied()));
        let list = properties
            .and_then(|node| docx_child(node, "numPr"))
            .and_then(|numbering| {
                let number_id =
                    docx_child(numbering, "numId").and_then(|node| docx_attribute(node, "val"))?;
                let level = docx_child(numbering, "ilvl")
                    .and_then(|node| docx_attribute(node, "val"))
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0);
                let ordered = self
                    .numbering
                    .get(&(number_id.into(), level))
                    .copied()
                    .unwrap_or(false);
                Some((level, ordered))
            });
        let inline = self.render_inline_children(paragraph).trim().to_owned();
        if inline.is_empty() {
            self.ensure_blank_line();
            return;
        }
        if let Some(level) = heading {
            self.ensure_blank_line();
            self.output.push_str(&"#".repeat(level));
            self.output.push(' ');
            self.output.push_str(&inline);
            self.ensure_blank_line();
        } else if let Some((level, ordered)) = list {
            self.ensure_line_break();
            self.output.push_str(&"  ".repeat(level));
            self.output.push_str(if ordered { "1. " } else { "- " });
            self.output.push_str(&inline);
            self.ensure_line_break();
        } else {
            self.ensure_blank_line();
            self.output.push_str(&inline);
            self.ensure_blank_line();
        }
    }

    fn render_table(&mut self, table: roxmltree::Node<'_, '_>) {
        self.ensure_blank_line();
        let rows = table
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "tr")
            .collect::<Vec<_>>();
        for (row_index, row) in rows.iter().enumerate() {
            self.output.push_str("| ");
            let cells = row
                .children()
                .filter(|node| node.is_element() && node.tag_name().name() == "tc")
                .collect::<Vec<_>>();
            for cell in &cells {
                let value = cell
                    .children()
                    .filter(|node| node.is_element() && node.tag_name().name() == "p")
                    .map(|paragraph| self.render_inline_children(paragraph).trim().to_owned())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · ");
                self.output.push_str(&value);
                self.output.push_str(" | ");
            }
            self.ensure_line_break();
            if row_index == 0 && !cells.is_empty() {
                self.output.push('|');
                for _ in &cells {
                    self.output.push_str(" --- |");
                }
                self.ensure_line_break();
            }
            if row.descendants().any(|node| {
                node.is_element() && matches!(node.tag_name().name(), "gridSpan" | "vMerge")
            }) {
                docx_warn(
                    &mut self.warnings,
                    "docx_table_simplified",
                    "A merged DOCX table cell was flattened during Markdown conversion.",
                );
            }
        }
        self.ensure_blank_line();
    }

    fn render_inline_children(&mut self, node: roxmltree::Node<'_, '_>) -> String {
        let mut output = String::new();
        for child in node.children().filter(roxmltree::Node::is_element) {
            match child.tag_name().name() {
                "pPr" | "bookmarkStart" | "bookmarkEnd" | "proofErr" => {}
                "r" => output.push_str(&self.render_run(child)),
                "hyperlink" => output.push_str(&self.render_hyperlink(child)),
                "smartTag" | "sdt" | "ins" | "moveTo" | "fldSimple" => {
                    if child.tag_name().name() == "fldSimple" {
                        docx_warn(
                            &mut self.warnings,
                            "docx_field_simplified",
                            "A DOCX field was reduced to its displayed text.",
                        );
                    }
                    output.push_str(&self.render_inline_children(child));
                }
                "del" | "moveFrom" => docx_warn(
                    &mut self.warnings,
                    "docx_revision_omitted",
                    "Deleted or moved-from revision text was omitted.",
                ),
                _ => output.push_str(&self.render_inline_children(child)),
            }
        }
        output
    }

    fn render_hyperlink(&mut self, hyperlink: roxmltree::Node<'_, '_>) -> String {
        let label = self.render_inline_children(hyperlink);
        let target = docx_attribute(hyperlink, "anchor")
            .map(|anchor| format!("#{anchor}"))
            .or_else(|| {
                let id = docx_attribute(hyperlink, "id")?;
                let relationship = self.relationships.get(id)?;
                relationship
                    .relationship_type
                    .ends_with("/hyperlink")
                    .then(|| relationship.target.clone())
            });
        let Some(target) = target else {
            return label;
        };
        let Some(target) = safe_html_target(&target) else {
            docx_warn(
                &mut self.warnings,
                "docx_unsafe_target_removed",
                "Removed an unsafe DOCX hyperlink target.",
            );
            return label;
        };
        let label = if label.trim().is_empty() {
            docx_escape_text(target)
        } else {
            label
        };
        format!("[{label}]({})", markdown_destination(target))
    }

    fn render_run(&mut self, run: roxmltree::Node<'_, '_>) -> String {
        let properties = docx_child(run, "rPr");
        let run_style = properties
            .and_then(|properties| docx_child(properties, "rStyle"))
            .and_then(|node| docx_attribute(node, "val"))
            .unwrap_or_default()
            .to_ascii_lowercase();
        let code_style = matches!(run_style.as_str(), "code" | "verbatim" | "htmlcode");
        let mut content = String::new();
        for child in run.children().filter(roxmltree::Node::is_element) {
            match child.tag_name().name() {
                "rPr" => {}
                "t" | "delText" => {
                    if child.tag_name().name() == "t" {
                        let text = child.text().unwrap_or_default();
                        if code_style {
                            content.push_str(&text.replace(['\r', '\n'], " "));
                        } else {
                            content.push_str(&docx_escape_text(text));
                        }
                    }
                }
                "tab" => content.push('\t'),
                "br" | "cr" => content.push_str("  \n"),
                "noBreakHyphen" => content.push('-'),
                "softHyphen" => content.push('\u{00ad}'),
                "drawing" | "pict" | "object" => {
                    content.push_str(&self.render_drawing(child));
                }
                "footnoteReference" | "endnoteReference" => docx_warn(
                    &mut self.warnings,
                    "docx_note_omitted",
                    "A DOCX footnote or endnote reference was omitted.",
                ),
                "instrText" => docx_warn(
                    &mut self.warnings,
                    "docx_field_simplified",
                    "A DOCX field instruction was omitted while retaining displayed text.",
                ),
                "sym" => docx_warn(
                    &mut self.warnings,
                    "docx_symbol_omitted",
                    "A symbol-font DOCX character could not be converted reliably.",
                ),
                _ => {}
            }
        }
        if content.is_empty() {
            return content;
        }
        let Some(properties) = properties else {
            return content;
        };
        if code_style {
            content = markdown_inline_code(content.trim());
        }
        if docx_toggle(properties, "strike") || docx_toggle(properties, "dstrike") {
            content = format!("~~{content}~~");
        }
        if docx_toggle(properties, "i") {
            content = format!("*{content}*");
        }
        if docx_toggle(properties, "b") {
            content = format!("**{content}**");
        }
        content
    }

    fn render_drawing(&mut self, drawing: roxmltree::Node<'_, '_>) -> String {
        let alt = drawing
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "docPr")
            .and_then(|node| {
                docx_attribute(node, "descr")
                    .or_else(|| docx_attribute(node, "title"))
                    .or_else(|| docx_attribute(node, "name"))
            })
            .unwrap_or("Image");
        let relationship_id = drawing
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "blip")
            .and_then(|node| {
                docx_attribute(node, "embed").or_else(|| docx_attribute(node, "link"))
            });
        let Some(target) = relationship_id.and_then(|id| self.image_targets.get(id)) else {
            docx_warn(
                &mut self.warnings,
                "docx_image_omitted",
                "A DOCX drawing had no safe, supported image payload and was omitted.",
            );
            return String::new();
        };
        format!(
            "![{}]({})",
            docx_escape_text(alt),
            markdown_destination(target)
        )
    }

    fn ensure_line_break(&mut self) {
        while self.output.ends_with(' ') {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn ensure_blank_line(&mut self) {
        self.ensure_line_break();
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
    }
}

fn convert_docx_to_markdown(bytes: &[u8], source_path: &str) -> Result<DocxConversion, CoreError> {
    if bytes.len() as u64 > MAX_ARCHIVE_COMPRESSED_BYTES {
        return Err(CoreError::Validation(
            "DOCX package exceeds the compressed-size limit".into(),
        ));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| CoreError::Validation(format!("invalid DOCX package: {error}")))?;
    let entries = preflight_docx_package(&mut archive)?;
    let content_types_bytes = read_docx_entry(&mut archive, &entries, "[Content_Types].xml")?;
    let content_types = decode_docx_xml(&content_types_bytes, "[Content_Types].xml")?;
    validate_docx_content_types(&content_types)?;
    let document_bytes = read_docx_entry(&mut archive, &entries, "word/document.xml")?;
    let document_xml = decode_docx_xml(&document_bytes, "word/document.xml")?;
    let document = parse_docx_xml(&document_xml, "word/document.xml")?;

    let relationships_xml = if entries.contains_key("word/_rels/document.xml.rels") {
        let bytes = read_docx_entry(&mut archive, &entries, "word/_rels/document.xml.rels")?;
        Some(decode_docx_xml(&bytes, "word/_rels/document.xml.rels")?)
    } else {
        None
    };
    let relationships = relationships_xml
        .as_deref()
        .map(docx_relationships)
        .transpose()?
        .unwrap_or_default();
    let styles_xml = if entries.contains_key("word/styles.xml") {
        let bytes = read_docx_entry(&mut archive, &entries, "word/styles.xml")?;
        Some(decode_docx_xml(&bytes, "word/styles.xml")?)
    } else {
        None
    };
    let numbering_xml = if entries.contains_key("word/numbering.xml") {
        let bytes = read_docx_entry(&mut archive, &entries, "word/numbering.xml")?;
        Some(decode_docx_xml(&bytes, "word/numbering.xml")?)
    } else {
        None
    };
    let heading_styles = docx_heading_styles(styles_xml.as_deref())?;
    let numbering = docx_numbering(numbering_xml.as_deref())?;

    let core_properties = if entries.contains_key("docProps/core.xml") {
        let bytes = read_docx_entry(&mut archive, &entries, "docProps/core.xml")?;
        Some(decode_docx_xml(&bytes, "docProps/core.xml")?)
    } else {
        None
    };
    let mut warnings = BTreeSet::new();
    let title = match core_properties
        .as_deref()
        .map(docx_core_title)
        .transpose()?
    {
        Some(Some(title)) if title.chars().count() <= 512 => Some(title),
        Some(Some(_)) => {
            docx_warn(
                &mut warnings,
                "docx_title_ignored",
                "Ignored a DOCX title longer than the 512-character import limit.",
            );
            None
        }
        _ => None,
    };

    for (path, code, message) in [
        (
            "word/comments.xml",
            "docx_comments_omitted",
            "DOCX comments are not converted in this import iteration.",
        ),
        (
            "word/footnotes.xml",
            "docx_notes_omitted",
            "DOCX footnote bodies are not converted in this import iteration.",
        ),
        (
            "word/endnotes.xml",
            "docx_notes_omitted",
            "DOCX endnote bodies are not converted in this import iteration.",
        ),
    ] {
        if entries.contains_key(path) {
            docx_warn(&mut warnings, code, message);
        }
    }
    if entries
        .keys()
        .any(|path| path.starts_with("word/header") || path.starts_with("word/footer"))
    {
        docx_warn(
            &mut warnings,
            "docx_headers_omitted",
            "DOCX headers and footers are not converted in this import iteration.",
        );
    }
    if entries.keys().any(|path| {
        path.starts_with("word/embeddings/")
            || path.starts_with("word/activeX/")
            || path.ends_with("vbaProject.bin")
    }) {
        docx_warn(
            &mut warnings,
            "docx_active_content_removed",
            "Embedded objects or active DOCX package content were not imported.",
        );
    }
    if entries.keys().any(|path| {
        path.starts_with("customXml/")
            || path.starts_with("word/glossary/")
            || path.starts_with("word/charts/")
            || path.starts_with("word/diagrams/")
    }) {
        docx_warn(
            &mut warnings,
            "docx_package_content_unconverted",
            "Additional DOCX package parts are listed in staged raw metadata but were not converted.",
        );
    }

    let container_name = source_path.rsplit('/').next().unwrap_or(source_path);
    let mut image_targets = BTreeMap::new();
    let mut assets_by_entry = BTreeMap::<String, DocxAsset>::new();
    for (relationship_id, relationship) in &relationships {
        if !relationship.relationship_type.ends_with("/image") {
            continue;
        }
        if relationship.external {
            let target = relationship.target.trim();
            let lower = target.to_ascii_lowercase();
            if (lower.starts_with("http://")
                || lower.starts_with("https://")
                || target.starts_with("//"))
                && safe_html_target(target).is_some()
            {
                image_targets.insert(relationship_id.clone(), target.into());
            } else {
                docx_warn(
                    &mut warnings,
                    "docx_unsafe_target_removed",
                    "Removed an unsafe external DOCX image target.",
                );
            }
            continue;
        }
        let Some(entry_path) = normalize_docx_part_target(&relationship.target) else {
            docx_warn(
                &mut warnings,
                "docx_unsafe_target_removed",
                "Removed a DOCX image relationship that escaped the package.",
            );
            continue;
        };
        let Some(mime_type) =
            asset_mime_type(&entry_path).filter(|value| value.starts_with("image/"))
        else {
            docx_warn(
                &mut warnings,
                "docx_image_omitted",
                format!("Unsupported DOCX image format was omitted: {entry_path}"),
            );
            continue;
        };
        if !entries.contains_key(&entry_path) {
            docx_warn(
                &mut warnings,
                "docx_image_missing",
                format!("DOCX image relationship target is missing: {entry_path}"),
            );
            continue;
        }
        if !assets_by_entry.contains_key(&entry_path) {
            let image_bytes = read_docx_entry(&mut archive, &entries, &entry_path)?;
            if !asset_signature_matches(mime_type, &image_bytes) {
                docx_warn(
                    &mut warnings,
                    "docx_image_invalid",
                    format!("DOCX image bytes do not match their format: {entry_path}"),
                );
                continue;
            }
            let filename = entry_path.rsplit('/').next().unwrap_or("image").to_owned();
            assets_by_entry.insert(
                entry_path.clone(),
                DocxAsset {
                    entry_path: entry_path.clone(),
                    filename,
                    mime_type,
                    bytes: image_bytes,
                },
            );
        }
        image_targets.insert(
            relationship_id.clone(),
            format!("{container_name}!/{entry_path}"),
        );
    }

    let (markdown, render_warnings) = DocxMarkdownWriter {
        output: String::new(),
        relationships: &relationships,
        image_targets: &image_targets,
        heading_styles: &heading_styles,
        numbering: &numbering,
        warnings,
    }
    .render(&document)?;

    Ok(DocxConversion {
        markdown,
        title,
        assets: assets_by_entry.into_values().collect(),
        warnings: render_warnings,
        core_properties,
        package_entry_count: entries.len(),
        package_entries: entries.keys().cloned().collect(),
    })
}

#[derive(Debug)]
struct HtmlConversion {
    markdown: String,
    title: Option<String>,
    warnings: Vec<HtmlConversionWarning>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HtmlConversionWarning {
    code: &'static str,
    message: String,
}

struct HtmlMarkdownWriter {
    output: String,
    warnings: BTreeSet<HtmlConversionWarning>,
    visited_nodes: usize,
    pending_space: bool,
}

impl HtmlMarkdownWriter {
    fn render(mut self, document: &Document) -> Result<HtmlConversion, CoreError> {
        for child in document.root().children() {
            self.render_node(child, 0, 0, false, false)?;
        }
        if self.output.len() > MAX_HTML_MARKDOWN_BYTES {
            return Err(CoreError::Validation(
                "converted HTML exceeds the Markdown output limit".into(),
            ));
        }
        let markdown = self.output.trim().to_owned() + "\n";
        let title = document
            .try_select("title")
            .map(|selection| {
                selection
                    .text()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .filter(|title| !title.is_empty());
        let title = if title
            .as_deref()
            .is_some_and(|title| title.chars().count() > 512)
        {
            self.warn(
                "html_title_ignored",
                "Ignored an HTML title longer than the 512-character import limit.",
            );
            None
        } else {
            title
        };
        if !document.errors.borrow().is_empty() {
            self.warn(
                "html_parser_recovered",
                format!(
                    "The HTML5 parser recovered from {} malformed construct(s).",
                    document.errors.borrow().len()
                ),
            );
        }
        Ok(HtmlConversion {
            markdown,
            title,
            warnings: self.warnings.into_iter().collect(),
        })
    }

    fn render_node(
        &mut self,
        node: NodeRef<'_>,
        depth: usize,
        list_depth: usize,
        ordered_list: bool,
        preformatted: bool,
    ) -> Result<(), CoreError> {
        self.visited_nodes = self.visited_nodes.saturating_add(1);
        if self.visited_nodes > MAX_HTML_DOM_NODES || depth > MAX_HTML_DOM_DEPTH {
            return Err(CoreError::Validation(
                "HTML document exceeds the DOM complexity limit".into(),
            ));
        }
        if node.is_text() {
            let text = node.immediate_text();
            if preformatted {
                self.output.push_str(&text);
            } else {
                self.push_normalized_text(&text);
            }
            return Ok(());
        }
        let Some(name) = node.node_name().map(|name| name.to_string()) else {
            return self.render_children(node, depth, list_depth, ordered_list, preformatted);
        };
        if matches!(
            name.as_str(),
            "script"
                | "style"
                | "iframe"
                | "object"
                | "embed"
                | "applet"
                | "template"
                | "noscript"
                | "svg"
                | "math"
        ) {
            self.warn(
                "html_content_removed",
                format!("Removed active or non-document <{name}> content."),
            );
            return Ok(());
        }
        match name.as_str() {
            "head" | "title" | "meta" | "link" | "base" => Ok(()),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.ensure_blank_line();
                let level = name[1..].parse::<usize>().unwrap_or(1);
                self.output.push_str(&"#".repeat(level));
                self.output.push(' ');
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "p" | "article" | "section" | "main" | "header" | "footer" | "aside" | "nav"
            | "div" | "figure" | "figcaption" | "address" => {
                self.ensure_blank_line();
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "br" => {
                self.output.push_str("  \n");
                Ok(())
            }
            "hr" => {
                self.ensure_blank_line();
                self.output.push_str("---");
                self.ensure_blank_line();
                Ok(())
            }
            "strong" | "b" => {
                self.flush_pending_space();
                self.output.push_str("**");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push_str("**");
                Ok(())
            }
            "em" | "i" => {
                self.flush_pending_space();
                self.output.push('*');
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push('*');
                Ok(())
            }
            "del" | "s" | "strike" => {
                self.flush_pending_space();
                self.output.push_str("~~");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push_str("~~");
                Ok(())
            }
            "code" if !preformatted => {
                self.flush_pending_space();
                self.push_inline_code(node.text().trim());
                Ok(())
            }
            "pre" => {
                self.ensure_blank_line();
                self.push_fenced_code(node.text().trim_matches('\n'));
                self.ensure_blank_line();
                Ok(())
            }
            "a" => {
                let href = node.attr("href").map(|value| value.to_string());
                if let Some(href) = href.as_deref().and_then(safe_html_target) {
                    self.flush_pending_space();
                    self.output.push('[');
                    let before = self.output.len();
                    self.render_children(node, depth, list_depth, ordered_list, false)?;
                    if self.output.len() == before {
                        self.output.push_str(&escape_markdown_text(href));
                    }
                    self.output.push_str("](");
                    self.output.push_str(&markdown_destination(href));
                    self.output.push(')');
                } else {
                    if href.is_some() {
                        self.warn(
                            "html_unsafe_target_removed",
                            "Removed an unsafe HTML link target.",
                        );
                    }
                    self.render_children(node, depth, list_depth, ordered_list, false)?;
                }
                Ok(())
            }
            "img" => {
                let source = node.attr("src").map(|value| value.to_string());
                if let Some(source) = source.as_deref().and_then(safe_html_target) {
                    let alt = node
                        .attr("alt")
                        .map(|value| value.to_string())
                        .unwrap_or_default();
                    self.flush_pending_space();
                    self.output.push_str("![");
                    self.output.push_str(&escape_markdown_text(&alt));
                    self.output.push_str("](");
                    self.output.push_str(&markdown_destination(source));
                    self.output.push(')');
                } else if source.is_some() {
                    self.warn(
                        "html_unsafe_target_removed",
                        "Removed an unsafe HTML image target.",
                    );
                }
                Ok(())
            }
            "ul" | "ol" => {
                self.ensure_line_break();
                self.render_children(node, depth, list_depth + 1, name == "ol", false)?;
                self.ensure_line_break();
                Ok(())
            }
            "li" => {
                self.ensure_line_break();
                self.output
                    .push_str(&"  ".repeat(list_depth.saturating_sub(1)));
                self.output
                    .push_str(if ordered_list { "1. " } else { "- " });
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_line_break();
                Ok(())
            }
            "blockquote" => {
                self.ensure_blank_line();
                self.output.push_str("> ");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "table" | "thead" | "tbody" | "tfoot" => {
                self.ensure_blank_line();
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_blank_line();
                Ok(())
            }
            "tr" => {
                let header_cells = node
                    .children()
                    .into_iter()
                    .filter(|child| {
                        child
                            .node_name()
                            .is_some_and(|name| name.to_string() == "th")
                    })
                    .count();
                self.ensure_line_break();
                self.output.push_str("| ");
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.ensure_line_break();
                if header_cells > 0 {
                    self.output.push('|');
                    for _ in 0..header_cells {
                        self.output.push_str(" --- |");
                    }
                    self.ensure_line_break();
                }
                Ok(())
            }
            "th" | "td" => {
                self.render_children(node, depth, list_depth, ordered_list, false)?;
                self.output.push_str(" | ");
                Ok(())
            }
            _ => self.render_children(node, depth, list_depth, ordered_list, preformatted),
        }
    }

    fn render_children(
        &mut self,
        node: NodeRef<'_>,
        depth: usize,
        list_depth: usize,
        ordered_list: bool,
        preformatted: bool,
    ) -> Result<(), CoreError> {
        for child in node.children() {
            self.render_node(child, depth + 1, list_depth, ordered_list, preformatted)?;
        }
        Ok(())
    }

    fn push_normalized_text(&mut self, value: &str) {
        if value.chars().next().is_some_and(char::is_whitespace) {
            self.pending_space = true;
        }
        let mut emitted_word = false;
        for word in value.split_whitespace() {
            if emitted_word {
                self.pending_space = true;
            }
            self.flush_pending_space();
            self.output.push_str(&escape_markdown_text(word));
            emitted_word = true;
        }
        if emitted_word {
            self.pending_space = value.chars().last().is_some_and(char::is_whitespace);
        } else if value.chars().any(char::is_whitespace) {
            self.pending_space = true;
        }
    }

    fn flush_pending_space(&mut self) {
        if self.pending_space
            && !self.output.is_empty()
            && !self.output.ends_with(char::is_whitespace)
        {
            self.output.push(' ');
        }
        self.pending_space = false;
    }

    fn warn(&mut self, code: &'static str, message: impl Into<String>) {
        self.warnings.insert(HtmlConversionWarning {
            code,
            message: message.into(),
        });
    }

    fn push_inline_code(&mut self, value: &str) {
        let delimiter = "`".repeat(longest_character_run(value, '`').saturating_add(1).max(1));
        let pad = value.starts_with(['`', ' ']) || value.ends_with(['`', ' ']);
        self.output.push_str(&delimiter);
        if pad {
            self.output.push(' ');
        }
        self.output.push_str(value);
        if pad {
            self.output.push(' ');
        }
        self.output.push_str(&delimiter);
    }

    fn push_fenced_code(&mut self, value: &str) {
        let delimiter = "`".repeat(longest_character_run(value, '`').saturating_add(1).max(3));
        self.output.push_str(&delimiter);
        self.output.push('\n');
        self.output.push_str(value);
        self.output.push('\n');
        self.output.push_str(&delimiter);
    }

    fn ensure_line_break(&mut self) {
        self.pending_space = false;
        while self.output.ends_with(' ') {
            self.output.pop();
        }
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }
    }

    fn ensure_blank_line(&mut self) {
        self.ensure_line_break();
        if !self.output.is_empty() && !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
    }
}

fn convert_html_to_markdown(html: &str) -> Result<HtmlConversion, CoreError> {
    HtmlMarkdownWriter {
        output: String::new(),
        warnings: BTreeSet::new(),
        visited_nodes: 0,
        pending_space: false,
    }
    .render(&Document::from(html))
}

fn safe_html_target(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty()
        || value.contains('\\')
        || value.contains(['<', '>', '"', '\''])
        || value.chars().any(char::is_control)
    {
        return None;
    }
    if value.starts_with("//") || value.starts_with('#') {
        return Some(value);
    }
    if value.starts_with('/') {
        return None;
    }
    if let Some((scheme, _)) = value.split_once(':') {
        return matches!(
            scheme.to_ascii_lowercase().as_str(),
            "http" | "https" | "mailto"
        )
        .then_some(value);
    }
    Some(value)
}

fn escape_markdown_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '[' | ']' | '|' | '<' | '>' | '#' | '+' | '-' | '!'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn longest_character_run(value: &str, target: char) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for character in value.chars() {
        if character == target {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

fn markdown_destination(value: &str) -> String {
    value
        .replace(' ', "%20")
        .replace('(', "%28")
        .replace(')', "%29")
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
    } else if extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm") {
        Some("html")
    } else if extension.eq_ignore_ascii_case("docx") {
        Some("docx")
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
    use std::io::Write;
    use std::path::PathBuf;
    use uuid::Uuid;
    use zip::write::SimpleFileOptions;

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

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);
        for (name, bytes) in entries {
            archive.start_file(*name, options).unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap();
    }

    const DOCX_CONTENT_TYPES: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

    const DOCX_DOCUMENT_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
 <w:body>
  <w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Field Guide</w:t></w:r></w:p>
  <w:p><w:r><w:t xml:space="preserve">A </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>bold</w:t></w:r><w:r><w:t xml:space="preserve"> and </w:t></w:r><w:r><w:rPr><w:i/></w:rPr><w:t>careful</w:t></w:r><w:r><w:t> note.</w:t></w:r></w:p>
  <w:p><w:hyperlink r:id="rLink"><w:r><w:t>Web</w:t></w:r></w:hyperlink></w:p>
  <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>First item</w:t></w:r></w:p>
  <w:p><w:r><w:drawing><wp:docPr id="1" name="Picture" descr="Map"/><a:blip r:embed="rImage"/></w:drawing></w:r></w:p>
  <w:tbl><w:tr><w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Value</w:t></w:r></w:p></w:tc></w:tr><w:tr><w:tc><w:p><w:r><w:t>North</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Cold</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
 </w:body>
</w:document>"#;

    const DOCX_RELATIONSHIPS_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Id="rLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
 <Relationship Id="rImage" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>
</Relationships>"#;

    const DOCX_NUMBERING_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
 <w:abstractNum w:abstractNumId="9"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum>
 <w:num w:numId="1"><w:abstractNumId w:val="9"/></w:num>
</w:numbering>"#;

    const DOCX_CORE_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Imported Field Guide</dc:title></cp:coreProperties>"#;

    fn write_docx_fixture(path: &Path) {
        write_zip(
            path,
            &[
                ("[Content_Types].xml", DOCX_CONTENT_TYPES),
                ("word/document.xml", DOCX_DOCUMENT_XML),
                ("word/_rels/document.xml.rels", DOCX_RELATIONSHIPS_XML),
                ("word/numbering.xml", DOCX_NUMBERING_XML),
                ("docProps/core.xml", DOCX_CORE_XML),
                ("word/media/image1.png", b"\x89PNG\r\n\x1a\nfixture"),
            ],
        );
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
    fn html_analysis_converts_structure_and_resolves_links_and_assets() {
        let source = TestDirectory::new();
        fs::create_dir_all(source.path().join("Notes")).unwrap();
        fs::create_dir_all(source.path().join("assets")).unwrap();
        let html = r#"<!doctype html>
<html><head><title>Field Guide</title></head><body>
<h1>Field Guide</h1>
<p>A <strong>bold</strong> and <em>careful</em> <code>note</code>.</p>
<ul><li>First</li><li>Second</li></ul>
<blockquote>Quoted passage</blockquote>
<table><tr><th>Name</th><th>Value</th></tr><tr><td>North</td><td>Cold</td></tr></table>
<p><a href="Other.html">Other note</a> <a href="https://example.com">Web</a></p>
<img src="../assets/map.png" alt="Map">
</body></html>"#;
        fs::write(source.path().join("Notes/Guide.html"), html).unwrap();
        fs::write(
            source.path().join("Notes/Other.html"),
            "<!doctype html><title>Other</title><p>Another note.</p>",
        )
        .unwrap();
        fs::write(
            source.path().join("assets/map.png"),
            b"\x89PNG\r\n\x1a\nfixture",
        )
        .unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let guide = staged
            .objects
            .iter()
            .find(|object| object.source_path == "Notes/Guide.html")
            .unwrap();
        let body = guide.body.as_ref().unwrap();

        assert_eq!(guide.source_kind, "html");
        assert_eq!(guide.title, "Field Guide");
        assert_eq!(body.format, "markdown");
        assert_eq!(guide.metadata["converted_from"], "html");
        assert_eq!(guide.raw_source_data["html"], html);
        assert!(body.body.contains("# Field Guide"));
        assert!(
            body.body.contains("A **bold** and *careful* `note`."),
            "{}",
            body.body
        );
        assert!(body.body.contains("- First"));
        assert!(body.body.contains("> Quoted passage"));
        assert!(body.body.contains("| Name | Value |"));
        assert!(body.body.contains("| --- | --- |"));
        assert!(guide.links.iter().any(|link| {
            link.target == "Other.html" && link.resolution == StagedLinkResolution::Resolved
        }));
        assert!(guide.links.iter().any(|link| {
            link.target == "https://example.com"
                && link.resolution == StagedLinkResolution::NotApplicable
        }));
        assert!(guide.links.iter().any(|link| {
            link.target == "../assets/map.png"
                && link.resolution == StagedLinkResolution::NotApplicable
        }));
        assert_eq!(staged.assets.len(), 1);
        assert_eq!(
            staged.assets[0].owner_object_id.as_deref(),
            Some(guide.id.as_str())
        );
    }

    #[test]
    fn html_analysis_removes_active_content_and_unsafe_targets() {
        let source = TestDirectory::new();
        fs::write(
            source.path().join("unsafe.html"),
            r#"<!doctype html><html><body>
<p onclick="steal()">Keep this text.</p>
<script>script_payload()</script><style>style_payload{}</style>
<iframe src="https://example.com">iframe_payload</iframe>
<svg><text>svg_payload</text></svg>
<p>&lt;script&gt;encoded_payload()&lt;/script&gt;</p>
<a href="javascript:alert(1)">Unsafe link</a>
<img src="data:text/html,unsafe" alt="unsafe image">
<a href="/absolute/path">Absolute link</a>
</body></html>"#,
        )
        .unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let object = &staged.objects[0];
        let body = &object.body.as_ref().unwrap().body;

        assert!(body.contains("Keep this text."));
        assert!(body.contains("Unsafe link"));
        assert!(body.contains("Absolute link"));
        assert!(!body.contains("script_payload"));
        assert!(!body.contains("style_payload"));
        assert!(!body.contains("iframe_payload"));
        assert!(!body.contains("svg_payload"));
        assert!(!body.contains("javascript:"));
        assert!(!body.contains("data:text/html"));
        assert!(!body.contains("/absolute/path"));
        assert!(!Parser::new_ext(body, Options::all())
            .any(|event| matches!(event, Event::Html(_) | Event::InlineHtml(_))));
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "html_content_removed"));
        assert!(staged.summary.warning_count > 0);
    }

    #[test]
    fn malformed_html_recovers_with_a_visible_warning() {
        let source = TestDirectory::new();
        fs::write(
            source.path().join("broken.htm"),
            "<!doctype html><title>Recovered</title><p>First <b>bold<p>Second</div>",
        )
        .unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();

        assert_eq!(staged.objects[0].title, "Recovered");
        assert!(staged.objects[0]
            .body
            .as_ref()
            .unwrap()
            .body
            .contains("Second"));
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "html_parser_recovered"));
    }

    #[test]
    fn html_conversion_enforces_dom_depth_limit() {
        let mut html = String::from("<!doctype html><body>");
        html.push_str(&"<div>".repeat(MAX_HTML_DOM_DEPTH + 8));
        html.push_str("too deep");
        html.push_str(&"</div>".repeat(MAX_HTML_DOM_DEPTH + 8));

        let error = convert_html_to_markdown(&html).unwrap_err();
        assert!(error.to_string().contains("DOM complexity limit"));
    }

    #[test]
    fn html_commit_preserves_converted_markdown_after_clean_rebuild() {
        let source = TestDirectory::new();
        let source_path = source.path().join("Guide.html");
        fs::write(
            &source_path,
            "<!doctype html><title>Guide</title><h1>Guide</h1><p>Converted <strong>body</strong>.</p>",
        )
        .unwrap();
        let staged =
            analyze_generic_documents(&source_path, GenericDocumentImportLimits::default())
                .unwrap();
        let expected_body = staged.objects[0].body.as_ref().unwrap().body.clone();
        let project = TestDirectory::new();
        let store = ProjectStore::open_directory(project.path()).unwrap();
        let generation = store.content_generation().unwrap();
        let mut mappings = ImportMappingOverrides::default();
        mappings.global.entity_type = Some("note".into());
        let candidate = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "html-session".into(),
                importer: staged.importer.clone(),
                source: staged.source.clone(),
                captured_content_generation: generation,
                current_content_generation: generation,
                manifest_fingerprint: "manifest-v1".into(),
                objects: staged.objects.clone(),
                unsupported_count: staged.unsupported.len(),
                diagnostics: staged.diagnostics.clone(),
            },
            &mappings,
        )
        .unwrap();
        let validated = validate_import_candidate_plan(ImportValidationBuild {
            candidate,
            staged_objects: staged.objects,
            staged_assets: staged.assets,
            catalog: ImportMappingCatalog {
                fingerprint: "manifest-v1".into(),
                entity_types: BTreeSet::from(["note".into()]),
                fields: BTreeMap::new(),
                relationship_types: BTreeSet::new(),
            },
            decisions: BTreeMap::new(),
            existing_targets: BTreeMap::new(),
            duplicate_targets: BTreeMap::new(),
        })
        .unwrap()
        .plan
        .unwrap();
        let report = store
            .commit_external_import(
                &validated,
                Some(&source_path),
                true,
                "00000000-0000-4000-8000-000000000003",
            )
            .unwrap();

        store.flush_checkpoint("HTML import test").unwrap();
        drop(store);
        fs::remove_dir_all(project.path().join(".daena")).unwrap();
        let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
        let documents = rebuilt
            .list_documents(report.created[0].entity_id.clone())
            .unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].format, "markdown");
        assert_eq!(documents[0].body, expected_body);
    }

    #[test]
    fn docx_analysis_preserves_structure_links_and_embedded_images() {
        let source = TestDirectory::new();
        let source_path = source.path().join("Guide.docx");
        write_docx_fixture(&source_path);

        let staged =
            analyze_generic_documents(&source_path, GenericDocumentImportLimits::default())
                .unwrap();
        let object = &staged.objects[0];
        let body = &object.body.as_ref().unwrap().body;

        assert_eq!(object.source_kind, "docx");
        assert_eq!(object.title, "Imported Field Guide");
        assert_eq!(object.body.as_ref().unwrap().format, "markdown");
        assert_eq!(object.metadata["converted_from"], "docx");
        assert!(object.raw_source_data["core_properties_xml"]
            .as_str()
            .unwrap()
            .contains("Imported Field Guide"));
        assert!(body.contains("# Field Guide"));
        assert!(body.contains("A **bold** and *careful* note."));
        assert!(body.contains("[Web](https://example.com)"));
        assert!(body.contains("1. First item"));
        assert!(body.contains("![Map](Guide.docx!/word/media/image1.png)"));
        assert!(body.contains("| Name | Value |"));
        assert!(body.contains("| --- | --- |"));
        assert!(object.links.iter().any(|link| {
            link.target == "https://example.com"
                && link.resolution == StagedLinkResolution::NotApplicable
        }));
        assert!(object.links.iter().any(|link| {
            link.target == "Guide.docx!/word/media/image1.png"
                && link.resolution == StagedLinkResolution::NotApplicable
        }));
        assert_eq!(staged.assets.len(), 1);
        assert_eq!(
            staged.assets[0].source_path,
            "Guide.docx!/word/media/image1.png"
        );
        assert_eq!(
            staged.assets[0].owner_object_id.as_deref(),
            Some(object.id.as_str())
        );
        assert_eq!(staged.summary.unresolved_link_count, 0);
    }

    #[test]
    fn docx_analysis_rejects_unsafe_packages_and_xml() {
        let source = TestDirectory::new();
        let traversal = source.path().join("traversal.docx");
        write_zip(
            &traversal,
            &[
                ("[Content_Types].xml", DOCX_CONTENT_TYPES),
                ("word/document.xml", DOCX_DOCUMENT_XML),
                ("../outside.xml", b"outside"),
            ],
        );
        assert!(
            analyze_generic_documents(&traversal, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("escapes or is not normalized")
        );

        let dtd = source.path().join("dtd.docx");
        write_zip(
            &dtd,
            &[
                ("[Content_Types].xml", DOCX_CONTENT_TYPES),
                (
                    "word/document.xml",
                    br#"<?xml version="1.0"?><!DOCTYPE w:document [<!ENTITY x "expanded">]><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>&x;</w:t></w:r></w:p></w:body></w:document>"#,
                ),
            ],
        );
        assert!(
            analyze_generic_documents(&dtd, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("DTD")
        );

        let malformed = source.path().join("malformed.docx");
        write_zip(
            &malformed,
            &[
                ("[Content_Types].xml", DOCX_CONTENT_TYPES),
                ("word/document.xml", b"<w:document>"),
            ],
        );
        assert!(
            analyze_generic_documents(&malformed, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("invalid DOCX XML")
        );

        let bomb = source.path().join("bomb.docx");
        let repeated = vec![b' '; 256 * 1024];
        write_zip(
            &bomb,
            &[
                ("[Content_Types].xml", DOCX_CONTENT_TYPES),
                ("word/document.xml", repeated.as_slice()),
            ],
        );
        assert!(
            analyze_generic_documents(&bomb, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("compression-ratio limit")
        );
    }

    #[test]
    fn docx_analysis_reports_omitted_active_and_unsupported_content() {
        let source = TestDirectory::new();
        let source_path = source.path().join("warnings.docx");
        write_zip(
            &source_path,
            &[
                ("[Content_Types].xml", DOCX_CONTENT_TYPES),
                ("word/document.xml", DOCX_DOCUMENT_XML),
                ("word/comments.xml", b"<comments/>"),
                ("word/vbaProject.bin", b"not executed"),
            ],
        );

        let staged =
            analyze_generic_documents(&source_path, GenericDocumentImportLimits::default())
                .unwrap();

        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "docx_comments_omitted"));
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "docx_active_content_removed"));
        assert!(staged.summary.warning_count >= 2);
    }

    #[test]
    fn docx_commit_preserves_markdown_and_image_after_clean_rebuild() {
        let source = TestDirectory::new();
        let source_path = source.path().join("Guide.docx");
        write_docx_fixture(&source_path);
        let staged =
            analyze_generic_documents(&source_path, GenericDocumentImportLimits::default())
                .unwrap();
        let expected_body = staged.objects[0].body.as_ref().unwrap().body.clone();
        let project = TestDirectory::new();
        let store = ProjectStore::open_directory(project.path()).unwrap();
        let generation = store.content_generation().unwrap();
        let mut mappings = ImportMappingOverrides::default();
        mappings.global.entity_type = Some("note".into());
        let candidate = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "docx-session".into(),
                importer: staged.importer.clone(),
                source: staged.source.clone(),
                captured_content_generation: generation,
                current_content_generation: generation,
                manifest_fingerprint: "manifest-v1".into(),
                objects: staged.objects.clone(),
                unsupported_count: staged.unsupported.len(),
                diagnostics: staged.diagnostics.clone(),
            },
            &mappings,
        )
        .unwrap();
        let validated = validate_import_candidate_plan(ImportValidationBuild {
            candidate,
            staged_objects: staged.objects,
            staged_assets: staged.assets,
            catalog: ImportMappingCatalog {
                fingerprint: "manifest-v1".into(),
                entity_types: BTreeSet::from(["note".into()]),
                fields: BTreeMap::new(),
                relationship_types: BTreeSet::new(),
            },
            decisions: BTreeMap::new(),
            existing_targets: BTreeMap::new(),
            duplicate_targets: BTreeMap::new(),
        })
        .unwrap()
        .plan
        .unwrap();
        let report = store
            .commit_external_import(
                &validated,
                Some(&source_path),
                true,
                "00000000-0000-4000-8000-000000000004",
            )
            .unwrap();

        assert_eq!(report.assets.len(), 1);
        assert_eq!(
            store
                .asset_bytes(report.assets[0].asset_id.clone())
                .unwrap(),
            b"\x89PNG\r\n\x1a\nfixture"
        );
        store.flush_checkpoint("DOCX import test").unwrap();
        drop(store);
        fs::remove_dir_all(project.path().join(".daena")).unwrap();
        let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
        let documents = rebuilt
            .list_documents(report.created[0].entity_id.clone())
            .unwrap();
        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].format, "markdown");
        assert_eq!(documents[0].body, expected_body);
        let assets = rebuilt
            .list_assets(report.created[0].entity_id.clone())
            .unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(
            rebuilt.asset_bytes(assets[0].id.clone()).unwrap(),
            b"\x89PNG\r\n\x1a\nfixture"
        );
    }

    #[test]
    fn docx_asset_reader_supports_file_folder_and_archive_sources() {
        let source = TestDirectory::new();
        fs::create_dir_all(source.path().join("Docs")).unwrap();
        let docx_path = source.path().join("Docs/Guide.docx");
        write_docx_fixture(&docx_path);
        let expected = b"\x89PNG\r\n\x1a\nfixture";

        assert_eq!(
            read_docx_import_asset_bytes(
                &docx_path,
                &ImportSourceKind::File,
                "Guide.docx!/word/media/image1.png",
                expected.len() as u64,
            )
            .unwrap(),
            expected
        );
        assert_eq!(
            read_docx_import_asset_bytes(
                source.path(),
                &ImportSourceKind::Folder,
                "Docs/Guide.docx!/word/media/image1.png",
                expected.len() as u64,
            )
            .unwrap(),
            expected
        );

        let docx_bytes = fs::read(&docx_path).unwrap();
        let archive_path = source.path().join("documents.zip");
        write_zip(&archive_path, &[("Docs/Guide.docx", &docx_bytes)]);
        let staged =
            analyze_generic_documents(&archive_path, GenericDocumentImportLimits::default())
                .unwrap();
        assert_eq!(staged.objects[0].source_kind, "docx");
        assert_eq!(
            staged.assets[0].source_path,
            "Docs/Guide.docx!/word/media/image1.png"
        );
        assert_eq!(
            read_docx_import_asset_bytes(
                &archive_path,
                &ImportSourceKind::Archive,
                "Docs/Guide.docx!/word/media/image1.png",
                expected.len() as u64,
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn zip_analysis_matches_folder_structure_and_content() {
        let folder = TestDirectory::new();
        fs::create_dir_all(folder.path().join("Notes")).unwrap();
        fs::create_dir_all(folder.path().join("assets")).unwrap();
        let note = b"# Note\n\n[Other](Other.md)\n![Map](../assets/map.png)\n";
        let other = b"# Other\n";
        let image = b"\x89PNG\r\n\x1a\nfixture";
        fs::write(folder.path().join("Notes/Note.md"), note).unwrap();
        fs::write(folder.path().join("Notes/Other.md"), other).unwrap();
        fs::write(folder.path().join("assets/map.png"), image).unwrap();
        let archive_directory = TestDirectory::new();
        let archive_path = archive_directory.path().join("fixture.zip");
        write_zip(
            &archive_path,
            &[
                ("Notes/Note.md", note),
                ("Notes/Other.md", other),
                ("assets/map.png", image),
            ],
        );

        let folder_result =
            analyze_generic_documents(folder.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let archive_result =
            analyze_generic_documents(&archive_path, GenericDocumentImportLimits::default())
                .unwrap();

        assert_eq!(archive_result.source.kind, ImportSourceKind::Archive);
        assert_eq!(
            archive_result
                .objects
                .iter()
                .map(|object| (&object.source_path, &object.title, &object.body))
                .collect::<Vec<_>>(),
            folder_result
                .objects
                .iter()
                .map(|object| (&object.source_path, &object.title, &object.body))
                .collect::<Vec<_>>()
        );
        assert_eq!(archive_result.assets.len(), 1);
        assert_eq!(archive_result.assets[0].source_path, "assets/map.png");
        assert_eq!(
            archive_result.assets[0].content_hash,
            folder_result.assets[0].content_hash
        );
        assert_eq!(
            archive_result
                .objects
                .iter()
                .flat_map(|object| object.links.iter())
                .map(|link| (&link.kind, &link.target, &link.resolution))
                .collect::<Vec<_>>(),
            folder_result
                .objects
                .iter()
                .flat_map(|object| object.links.iter())
                .map(|link| (&link.kind, &link.target, &link.resolution))
                .collect::<Vec<_>>()
        );
        assert_eq!(archive_result.summary, folder_result.summary);
    }

    #[test]
    fn zip_analysis_rejects_traversal_bombs_and_malformed_archives() {
        let source = TestDirectory::new();
        let traversal = source.path().join("traversal.zip");
        write_zip(&traversal, &[("../outside.md", b"outside")]);
        assert!(
            analyze_generic_documents(&traversal, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("escapes or is not normalized")
        );

        let symlink = source.path().join("symlink.zip");
        let file = fs::File::create(&symlink).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        archive
            .add_symlink(
                "linked.md",
                "target.md",
                SimpleFileOptions::default().unix_permissions(0o777),
            )
            .unwrap();
        archive.finish().unwrap();
        assert!(
            analyze_generic_documents(&symlink, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("links and special files")
        );

        let collision = source.path().join("collision.zip");
        write_zip(&collision, &[("Note.md", b"one"), ("note.md", b"two")]);
        assert!(
            analyze_generic_documents(&collision, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("duplicate or case-colliding")
        );

        let bomb = source.path().join("bomb.zip");
        let repeated = vec![0_u8; 256 * 1024];
        write_zip(&bomb, &[("bomb.md", repeated.as_slice())]);
        assert!(
            analyze_generic_documents(&bomb, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("compression ratio")
        );

        let malformed = source.path().join("malformed.zip");
        fs::write(&malformed, b"not a ZIP archive").unwrap();
        assert!(
            analyze_generic_documents(&malformed, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("invalid ZIP archive")
        );
    }

    #[test]
    fn zip_central_directory_preflight_can_be_cancelled() {
        let source = TestDirectory::new();
        let archive_path = source.path().join("cancel.zip");
        write_zip(&archive_path, &[("one.md", b"one"), ("two.md", b"two")]);

        let error = analyze_generic_documents_with_progress(
            &archive_path,
            GenericDocumentImportLimits::default(),
            |progress| {
                if progress.source_path.is_some() {
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
    }

    #[test]
    fn zip_attachment_commit_survives_checkpoint_rebuild() {
        let source = TestDirectory::new();
        let archive_path = source.path().join("fixture.zip");
        let note = b"# Note\n\n![Map](assets/map.png)\n";
        let image = b"\x89PNG\r\n\x1a\nfixture";
        write_zip(
            &archive_path,
            &[("Note.md", note), ("assets/map.png", image)],
        );
        let staged =
            analyze_generic_documents(&archive_path, GenericDocumentImportLimits::default())
                .unwrap();
        let project = TestDirectory::new();
        let store = ProjectStore::open_directory(project.path()).unwrap();
        let generation = store.content_generation().unwrap();
        let mut mappings = ImportMappingOverrides::default();
        mappings.global.entity_type = Some("note".into());
        let candidate = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "zip-session".into(),
                importer: staged.importer.clone(),
                source: staged.source.clone(),
                captured_content_generation: generation,
                current_content_generation: generation,
                manifest_fingerprint: "manifest-v1".into(),
                objects: staged.objects.clone(),
                unsupported_count: staged.unsupported.len(),
                diagnostics: staged.diagnostics.clone(),
            },
            &mappings,
        )
        .unwrap();
        let validated = validate_import_candidate_plan(ImportValidationBuild {
            candidate,
            staged_objects: staged.objects,
            staged_assets: staged.assets,
            catalog: ImportMappingCatalog {
                fingerprint: "manifest-v1".into(),
                entity_types: BTreeSet::from(["note".into()]),
                fields: BTreeMap::new(),
                relationship_types: BTreeSet::new(),
            },
            decisions: BTreeMap::new(),
            existing_targets: BTreeMap::new(),
            duplicate_targets: BTreeMap::new(),
        })
        .unwrap()
        .plan
        .unwrap();
        let report = store
            .commit_external_import(
                &validated,
                Some(&archive_path),
                true,
                "00000000-0000-4000-8000-000000000002",
            )
            .unwrap();
        assert_eq!(report.created.len(), 1);
        assert_eq!(report.assets.len(), 1);
        assert_eq!(
            store
                .asset_bytes(report.assets[0].asset_id.clone())
                .unwrap(),
            image
        );
        store.flush_checkpoint("ZIP import test").unwrap();
        drop(store);
        fs::remove_dir_all(project.path().join(".daena")).unwrap();
        let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
        let assets = rebuilt
            .list_assets(report.created[0].entity_id.clone())
            .unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(rebuilt.asset_bytes(assets[0].id.clone()).unwrap(), image);
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
