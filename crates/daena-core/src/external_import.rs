use crate::CoreError;
use dom_query::{Document, NodeRef};
use pulldown_cmark::{Event, Options, Parser, Tag};
use quick_xml::encoding::Decoder as XmlDecoder;
use quick_xml::events::{BytesStart, Event as XmlEvent};
use quick_xml::Reader as XmlReader;
use quick_xml::XmlVersion;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use zip::ZipArchive;

pub const STAGED_IMPORT_SCHEMA_VERSION: u32 = 1;
pub const IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION: u32 = 1;
pub const VALIDATED_IMPORT_PLAN_SCHEMA_VERSION: u32 = 1;
pub const GENERIC_DOCUMENT_IMPORTER_ID: &str = "daena.generic-documents";
pub const GENERIC_DOCUMENT_IMPORTER_VERSION: &str = "1";
pub const OBSIDIAN_IMPORTER_ID: &str = "daena.obsidian-vault";
pub const OBSIDIAN_IMPORTER_VERSION: &str = "1";
pub const MEDIAWIKI_IMPORTER_ID: &str = "daena.mediawiki-xml";
pub const MEDIAWIKI_IMPORTER_VERSION: &str = "1";
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
const MAX_MEDIAWIKI_SOURCE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_MEDIAWIKI_XML_DEPTH: usize = 128;
const MAX_MEDIAWIKI_TEMPLATES_PER_PAGE: usize = 512;
const MAX_MEDIAWIKI_TEMPLATE_DEPTH: usize = 64;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedImportSourceContext {
    #[serde(default)]
    pub source_kind: String,
    #[serde(default)]
    pub parent_source_path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub unmapped_fields: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub links: Vec<StagedLink>,
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
    #[serde(default)]
    pub source_context: ValidatedImportSourceContext,
    pub decision: ImportObjectDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedImportRelationship {
    pub source_staged_object_id: String,
    pub target_staged_object_id: String,
    pub relationship_type: String,
    pub source_kind: String,
    pub source_target: String,
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
    pub relationships: Vec<ValidatedImportRelationship>,
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
pub struct ImportedRelationshipReport {
    pub relationship_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub relationship_type: String,
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
    #[serde(default)]
    pub relationships: Vec<ImportedRelationshipReport>,
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
    let mut unmapped_field_count = 0_usize;
    let mut unmapped_field_objects = 0_usize;
    let mut unmapped_source_link_count = 0_usize;
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
                        // Folder/global mappings mean "map this key when present". Wiki
                        // infoboxes and Obsidian frontmatter are intentionally sparse.
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
                let object_unmapped = object
                    .fields
                    .keys()
                    .filter(|source_key| {
                        !candidate_object
                            .mapping
                            .field_mappings
                            .contains_key(*source_key)
                    })
                    .count();
                if object_unmapped > 0 {
                    unmapped_field_count += object_unmapped;
                    unmapped_field_objects += 1;
                }
                unmapped_source_link_count += object
                    .links
                    .iter()
                    .filter(|link| {
                        !candidate_object
                            .mapping
                            .relationship_mappings
                            .contains_key(staged_link_kind_key(&link.kind))
                    })
                    .count();
            }
        }
        let unmapped_fields = object
            .fields
            .iter()
            .filter(|(source_key, _)| {
                !matches!(&decision, ImportObjectDecision::Create)
                    || !candidate_object
                        .mapping
                        .field_mappings
                        .contains_key(*source_key)
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        validated.push(ValidatedImportObject {
            staged_object_id: object.id.clone(),
            source_id: object.source_id.clone(),
            source_path: object.source_path.clone(),
            content_hash: object.content_hash.clone(),
            title: object.title.clone(),
            entity_type,
            document: object.body.clone(),
            fields,
            source_context: ValidatedImportSourceContext {
                source_kind: object.source_kind.clone(),
                parent_source_path: object.parent_source_path.clone(),
                tags: object.tags.clone(),
                aliases: object.aliases.clone(),
                metadata: object.metadata.clone(),
                unmapped_fields,
                links: object.links.clone(),
            },
            decision,
        });
    }
    if unmapped_field_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "unmapped_source_fields_preserved",
            &format!(
                "{unmapped_field_count} unmapped source field(s) across {unmapped_field_objects} item(s) will remain in import source metadata."
            ),
            None,
            None,
            None,
        ));
    }
    if unmapped_source_link_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "source_links_preserved",
            &format!(
                "{unmapped_source_link_count} unmapped source link(s) will remain in the document and import source metadata; only explicitly mapped, resolved links become relationships."
            ),
            None,
            None,
            None,
        ));
    }

    let decisions_by_object = validated
        .iter()
        .map(|object| (object.staged_object_id.as_str(), &object.decision))
        .collect::<BTreeMap<_, _>>();
    let mut relationship_keys = BTreeSet::new();
    let mut validated_relationships = Vec::new();
    let mut unresolved_mapped_link_count = 0_usize;
    let mut skipped_target_link_count = 0_usize;
    for candidate_object in &candidate.objects {
        let object = staged
            .get(&candidate_object.staged_object_id)
            .expect("candidate staged object was checked above");
        if matches!(
            decisions_by_object.get(object.id.as_str()),
            Some(ImportObjectDecision::Skip)
        ) {
            continue;
        }
        for (source_kind, relationship_type) in &candidate_object.mapping.relationship_mappings {
            if !catalog.relationship_types.contains(relationship_type) {
                issues.push(validation_issue(
                    ImportValidationSeverity::Error,
                    "target_relationship_unavailable",
                    &format!("The mapped relationship type {relationship_type} is not available."),
                    Some(object.source_path.clone()),
                    Some(object.id.clone()),
                    None,
                ));
                continue;
            }
            for link in object
                .links
                .iter()
                .filter(|link| staged_link_kind_key(&link.kind) == source_kind)
            {
                if link.resolution != StagedLinkResolution::Resolved {
                    unresolved_mapped_link_count += 1;
                    continue;
                }
                let target_id = link
                    .resolved_object_id
                    .as_deref()
                    .expect("resolved links were checked by staged import validation");
                if matches!(
                    decisions_by_object.get(target_id),
                    Some(ImportObjectDecision::Skip)
                ) {
                    skipped_target_link_count += 1;
                    continue;
                }
                if !decisions_by_object.contains_key(target_id) {
                    return Err(CoreError::Validation(format!(
                        "resolved import relationship target is missing: {target_id}"
                    )));
                }
                let key = (
                    object.id.clone(),
                    target_id.to_owned(),
                    relationship_type.clone(),
                );
                if relationship_keys.insert(key.clone()) {
                    validated_relationships.push(ValidatedImportRelationship {
                        source_staged_object_id: key.0,
                        target_staged_object_id: key.1,
                        relationship_type: key.2,
                        source_kind: source_kind.clone(),
                        source_target: link.target.clone(),
                    });
                }
            }
        }
    }
    if unresolved_mapped_link_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "mapped_links_unresolved",
            &format!(
                "{unresolved_mapped_link_count} mapped link(s) were ambiguous, missing, or external and will not become relationships."
            ),
            None,
            None,
            None,
        ));
    }
    if skipped_target_link_count > 0 {
        issues.push(validation_issue(
            ImportValidationSeverity::Warning,
            "mapped_link_targets_skipped",
            &format!(
                "{skipped_target_link_count} mapped link(s) target skipped items and will not become relationships."
            ),
            None,
            None,
            None,
        ));
    }
    let has_errors = issues
        .iter()
        .any(|issue| issue.severity == ImportValidationSeverity::Error);
    if has_errors {
        return Ok(ImportValidationOutcome { plan: None, issues });
    }
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
        relationships: validated_relationships,
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

fn staged_link_kind_key(kind: &StagedLinkKind) -> &'static str {
    match kind {
        StagedLinkKind::Internal => "internal",
        StagedLinkKind::External => "external",
        StagedLinkKind::Embed => "embed",
    }
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
    progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    analyze_documents_with_progress(source, limits, ImportProfile::Generic, progress)
}

pub fn analyze_obsidian_vault(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
) -> Result<StagedImport, CoreError> {
    analyze_obsidian_vault_with_progress(source, limits, |_| Ok(()))
}

pub fn analyze_obsidian_vault_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    analyze_documents_with_progress(source, limits, ImportProfile::Obsidian, progress)
}

pub fn analyze_mediawiki_xml(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
) -> Result<StagedImport, CoreError> {
    analyze_mediawiki_xml_with_progress(source, limits, |_| Ok(()))
}

pub fn analyze_mediawiki_xml_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    mut progress: impl FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
) -> Result<StagedImport, CoreError> {
    validate_limits(&limits)?;
    let source = source.as_ref();
    let metadata = fs::symlink_metadata(source).map_err(|source| CoreError::Io {
        operation: "read MediaWiki import source metadata",
        source,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CoreError::Validation(
            "MediaWiki import requires a regular XML file".into(),
        ));
    }
    if metadata.len() > MAX_MEDIAWIKI_SOURCE_BYTES {
        return Err(CoreError::Validation(format!(
            "MediaWiki XML exceeds the maximum source size of {MAX_MEDIAWIKI_SOURCE_BYTES} bytes"
        )));
    }
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| CoreError::Validation("MediaWiki XML filename is not valid UTF-8".into()))?
        .to_owned();
    let canonical_source = fs::canonicalize(source).map_err(|source| CoreError::Io {
        operation: "resolve MediaWiki import source path",
        source,
    })?;
    let source_id = hex_digest(canonical_source.to_string_lossy().as_bytes());
    let file = fs::File::open(source).map_err(|source| CoreError::Io {
        operation: "open MediaWiki import source",
        source,
    })?;
    let mut analyzer = MediaWikiAnalyzer {
        limits,
        import: StagedImport {
            schema_version: STAGED_IMPORT_SCHEMA_VERSION,
            importer: ImporterIdentity {
                id: MEDIAWIKI_IMPORTER_ID.into(),
                version: MEDIAWIKI_IMPORTER_VERSION.into(),
                name: "MediaWiki XML".into(),
            },
            source: ImportSource {
                id: source_id,
                kind: ImportSourceKind::WikiDump,
                display_name: source_name.clone(),
            },
            objects: Vec::new(),
            assets: Vec::new(),
            unsupported: Vec::new(),
            diagnostics: Vec::new(),
            summary: ImportAnalysisSummary::default(),
        },
        source_name,
        namespaces: BTreeMap::new(),
        site_metadata: BTreeMap::new(),
        folders: BTreeSet::new(),
        processed_pages: 0,
        total_wikitext_bytes: 0,
        total_diagnostics: 0,
        omitted_revisions: 0,
        progress: &mut progress,
    };
    analyzer.report_progress(0, None)?;
    analyzer.parse(file)?;
    analyzer.resolve_links_and_redirects()?;
    if analyzer.omitted_revisions > 0 {
        analyzer.import.unsupported.push(UnsupportedSourceData {
            source_path: analyzer.source_name.clone(),
            source_kind: "mediawiki_revision_history".into(),
            reason: "older page revisions were intentionally omitted".into(),
            raw_metadata: BTreeMap::from([(
                "omitted_revision_count".into(),
                serde_json::Value::from(analyzer.omitted_revisions),
            )]),
        });
        analyzer.record_diagnostic(ImportDiagnostic {
            severity: ImportDiagnosticSeverity::Warning,
            code: "mediawiki_revision_history_omitted".into(),
            message: format!(
                "{} older page revisions were omitted; only each latest revision was staged.",
                analyzer.omitted_revisions
            ),
            source_path: Some(analyzer.source_name.clone()),
            object_id: None,
        })?;
    }
    analyzer
        .import
        .objects
        .sort_by(|left, right| left.source_path.cmp(&right.source_path));
    analyzer
        .import
        .refresh_summary(analyzer.folders.len(), metadata.len());
    analyzer.import.validate()?;
    Ok(analyzer.import)
}

#[derive(Debug, Default)]
struct MediaWikiRevision {
    id: String,
    parent_id: String,
    timestamp: String,
    model: String,
    format: String,
    sha1: String,
    contributor: String,
    text: String,
}

#[derive(Debug, Default)]
struct MediaWikiPage {
    title: String,
    namespace_id: String,
    id: String,
    redirect_target: Option<String>,
    revision: Option<MediaWikiRevision>,
    current_revision: Option<MediaWikiRevision>,
    revision_count: usize,
}

struct MediaWikiAnalyzer<'a> {
    limits: GenericDocumentImportLimits,
    import: StagedImport,
    source_name: String,
    namespaces: BTreeMap<String, String>,
    site_metadata: BTreeMap<String, String>,
    folders: BTreeSet<String>,
    processed_pages: usize,
    total_wikitext_bytes: u64,
    total_diagnostics: usize,
    omitted_revisions: usize,
    progress: &'a mut dyn FnMut(ImportAnalysisProgress) -> Result<(), CoreError>,
}

impl MediaWikiAnalyzer<'_> {
    fn parse(&mut self, file: fs::File) -> Result<(), CoreError> {
        let mut reader = XmlReader::from_reader(BufReader::new(file));
        reader.config_mut().trim_text(false);
        reader.config_mut().check_end_names = true;
        let mut buffer = Vec::with_capacity(64 * 1024);
        let mut stack = Vec::<Vec<u8>>::new();
        let mut page = None::<MediaWikiPage>;
        let mut pending_namespace = None::<(String, String)>;
        let mut root_seen = false;
        let mut event_count = 0_u64;
        loop {
            let event = reader.read_event_into(&mut buffer).map_err(|error| {
                CoreError::Validation(format!(
                    "invalid MediaWiki XML near byte {}: {error}",
                    reader.error_position()
                ))
            })?;
            event_count = event_count.saturating_add(1);
            if event_count.is_multiple_of(512) {
                self.report_progress(reader.buffer_position(), None)?;
            }
            match event {
                XmlEvent::Start(start) => {
                    let name = xml_local_name(start.name().as_ref()).to_vec();
                    if !root_seen {
                        if name.as_slice() != b"mediawiki" {
                            return Err(CoreError::Validation(
                                "XML source root must be a MediaWiki export".into(),
                            ));
                        }
                        root_seen = true;
                    }
                    if stack.len() >= MAX_MEDIAWIKI_XML_DEPTH {
                        return Err(CoreError::Validation(format!(
                            "MediaWiki XML exceeds the maximum nesting depth of {MAX_MEDIAWIKI_XML_DEPTH}"
                        )));
                    }
                    if name.as_slice() == b"page" {
                        if page.is_some() {
                            return Err(CoreError::Validation(
                                "MediaWiki XML contains nested page elements".into(),
                            ));
                        }
                        page = Some(MediaWikiPage::default());
                    } else if name.as_slice() == b"revision" {
                        let current_page = page.as_mut().ok_or_else(|| {
                            CoreError::Validation(
                                "MediaWiki revision appeared outside a page".into(),
                            )
                        })?;
                        if current_page.current_revision.is_some() {
                            return Err(CoreError::Validation(
                                "MediaWiki XML contains nested revision elements".into(),
                            ));
                        }
                        current_page.current_revision = Some(MediaWikiRevision::default());
                        current_page.revision_count = current_page.revision_count.saturating_add(1);
                    } else if name.as_slice() == b"redirect" {
                        if let Some(current_page) = page.as_mut() {
                            current_page.redirect_target =
                                mediawiki_xml_attribute(&start, b"title", reader.decoder())?;
                        }
                    } else if name.as_slice() == b"namespace" && page.is_none() {
                        let key = mediawiki_xml_attribute(&start, b"key", reader.decoder())?
                            .unwrap_or_default();
                        pending_namespace = Some((key, String::new()));
                    }
                    stack.push(name);
                }
                XmlEvent::Empty(start) => {
                    let name = xml_local_name(start.name().as_ref()).to_vec();
                    if name.as_slice() == b"redirect" {
                        if let Some(current_page) = page.as_mut() {
                            current_page.redirect_target =
                                mediawiki_xml_attribute(&start, b"title", reader.decoder())?;
                        }
                    }
                }
                XmlEvent::Text(text) => {
                    let decoded = text.decode().map_err(|error| {
                        CoreError::Validation(format!("invalid MediaWiki XML text: {error}"))
                    })?;
                    let value = quick_xml::escape::unescape(&decoded).map_err(|error| {
                        CoreError::Validation(format!(
                            "MediaWiki XML contains an unsupported entity reference: {error}"
                        ))
                    })?;
                    append_mediawiki_xml_text(
                        &stack,
                        &value,
                        &mut page,
                        &mut pending_namespace,
                        &mut self.site_metadata,
                    );
                    self.validate_current_revision_size(&page)?;
                }
                XmlEvent::CData(text) => {
                    let value = text.decode().map_err(|error| {
                        CoreError::Validation(format!("invalid MediaWiki XML CDATA: {error}"))
                    })?;
                    append_mediawiki_xml_text(
                        &stack,
                        &value,
                        &mut page,
                        &mut pending_namespace,
                        &mut self.site_metadata,
                    );
                    self.validate_current_revision_size(&page)?;
                }
                XmlEvent::End(end) => {
                    let name = xml_local_name(end.name().as_ref()).to_vec();
                    if stack.last().map(Vec::as_slice) != Some(name.as_slice()) {
                        return Err(CoreError::Validation(
                            "MediaWiki XML element nesting is invalid".into(),
                        ));
                    }
                    if name.as_slice() == b"revision" {
                        let current_page = page.as_mut().expect("revision requires page");
                        let revision = current_page
                            .current_revision
                            .take()
                            .expect("revision state is present");
                        if current_page
                            .revision
                            .as_ref()
                            .is_none_or(|current| mediawiki_revision_is_newer(&revision, current))
                        {
                            current_page.revision = Some(revision);
                        }
                    } else if name.as_slice() == b"namespace" && page.is_none() {
                        if let Some((key, value)) = pending_namespace.take() {
                            self.namespaces.insert(key, value.trim().to_owned());
                        }
                    } else if name.as_slice() == b"page" {
                        let current_page = page.take().expect("page state is present");
                        self.finish_page(current_page, reader.buffer_position())?;
                    }
                    stack.pop();
                }
                XmlEvent::DocType(_) => {
                    return Err(CoreError::Validation(
                        "MediaWiki XML DTD and entity declarations are not allowed".into(),
                    ));
                }
                XmlEvent::Decl(declaration) => {
                    if declaration
                        .encoding()
                        .transpose()
                        .map_err(|error| {
                            CoreError::Validation(format!(
                                "invalid MediaWiki XML encoding declaration: {error}"
                            ))
                        })?
                        .is_some_and(|encoding| !encoding.eq_ignore_ascii_case(b"utf-8"))
                    {
                        return Err(CoreError::Validation(
                            "MediaWiki XML must use UTF-8 encoding".into(),
                        ));
                    }
                }
                XmlEvent::GeneralRef(reference) => {
                    let value = if let Some(character) =
                        reference.resolve_char_ref().map_err(|error| {
                            CoreError::Validation(format!(
                                "invalid MediaWiki XML character reference: {error}"
                            ))
                        })? {
                        character.to_string()
                    } else {
                        match reference
                            .decode()
                            .map_err(|error| {
                                CoreError::Validation(format!(
                                    "invalid MediaWiki XML entity reference: {error}"
                                ))
                            })?
                            .as_ref()
                        {
                            "amp" => "&".into(),
                            "lt" => "<".into(),
                            "gt" => ">".into(),
                            "apos" => "'".into(),
                            "quot" => "\"".into(),
                            entity => {
                                return Err(CoreError::Validation(format!(
                                    "MediaWiki XML entity reference '&{entity};' is not allowed"
                                )))
                            }
                        }
                    };
                    append_mediawiki_xml_text(
                        &stack,
                        &value,
                        &mut page,
                        &mut pending_namespace,
                        &mut self.site_metadata,
                    );
                    self.validate_current_revision_size(&page)?;
                }
                XmlEvent::Eof => break,
                XmlEvent::Comment(_) | XmlEvent::PI(_) => {}
            }
            buffer.clear();
        }
        if !root_seen || !stack.is_empty() || page.is_some() {
            return Err(CoreError::Validation(
                "MediaWiki XML ended before all elements were closed".into(),
            ));
        }
        self.report_progress(reader.buffer_position(), None)
    }
}

impl MediaWikiAnalyzer<'_> {
    fn finish_page(&mut self, mut page: MediaWikiPage, source_bytes: u64) -> Result<(), CoreError> {
        self.processed_pages = self.processed_pages.saturating_add(1);
        if self.processed_pages > self.limits.max_files
            || self.processed_pages > self.limits.max_entries
        {
            return Err(CoreError::Validation(format!(
                "MediaWiki XML exceeds the maximum page count of {}",
                self.limits.max_files.min(self.limits.max_entries)
            )));
        }
        page.title = page.title.trim().to_owned();
        if page.title.is_empty() {
            return Err(CoreError::Validation(
                "MediaWiki page title cannot be empty".into(),
            ));
        }
        let revision = page.revision.take().unwrap_or_default();
        let wikitext_bytes = revision.text.len() as u64;
        if wikitext_bytes > self.limits.max_file_bytes {
            return Err(CoreError::Validation(format!(
                "MediaWiki page '{}' exceeds the maximum page size of {} bytes",
                page.title, self.limits.max_file_bytes
            )));
        }
        self.total_wikitext_bytes = self
            .total_wikitext_bytes
            .checked_add(wikitext_bytes)
            .ok_or_else(|| CoreError::Validation("MediaWiki content size overflowed".into()))?;
        if self.total_wikitext_bytes > self.limits.max_total_bytes {
            return Err(CoreError::Validation(format!(
                "MediaWiki pages exceed the maximum staged content size of {} bytes",
                self.limits.max_total_bytes
            )));
        }
        self.omitted_revisions = self
            .omitted_revisions
            .saturating_add(page.revision_count.saturating_sub(1));
        let markup = analyze_mediawiki_markup(&revision.text, page.redirect_target.as_deref());
        let namespace_id = page.namespace_id.trim();
        let namespace_id = if namespace_id.is_empty() {
            0
        } else {
            namespace_id.parse::<i64>().map_err(|_| {
                CoreError::Validation(format!(
                    "MediaWiki page '{}' has an invalid namespace id",
                    page.title
                ))
            })?
        };
        let namespace_id = namespace_id.to_string();
        let namespace_name = self
            .namespaces
            .get(&namespace_id)
            .cloned()
            .unwrap_or_default();
        let parent_source_path = format!("namespaces/{namespace_id}");
        self.folders.insert(parent_source_path.clone());
        let native_identity = if page.id.trim().is_empty() {
            format!("title:{}", normalize_mediawiki_title(&page.title))
        } else {
            format!("page:{}", page.id.trim())
        };
        let object_id = hex_digest(
            format!(
                "{}\0{}\0{}",
                self.import.importer.id, self.import.source.id, native_identity
            )
            .as_bytes(),
        );
        let source_path = format!(
            "{parent_source_path}/pages/{}.wiki",
            &hex_digest(native_identity.as_bytes())[..24]
        );
        let mut fields = BTreeMap::from([
            (
                "namespace_id".into(),
                serde_json::Value::String(namespace_id.clone()),
            ),
            (
                "page_id".into(),
                serde_json::Value::String(page.id.trim().into()),
            ),
        ]);
        if !namespace_name.is_empty() {
            fields.insert(
                "namespace".into(),
                serde_json::Value::String(namespace_name.clone()),
            );
        }
        for (key, value) in [
            ("revision_id", revision.id.trim()),
            ("revision_timestamp", revision.timestamp.trim()),
            ("content_model", revision.model.trim()),
            ("source_format", revision.format.trim()),
            (
                "redirect_target",
                markup.redirect_target.as_deref().unwrap_or(""),
            ),
        ] {
            if !value.is_empty() {
                fields.insert(key.into(), serde_json::Value::String(value.into()));
            }
        }
        if !markup.categories.is_empty() {
            fields.insert(
                "categories".into(),
                serde_json::Value::Array(
                    markup
                        .categories
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        if !markup.template_names.is_empty() {
            fields.insert(
                "templates".into(),
                serde_json::Value::Array(
                    markup
                        .template_names
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }
        for (key, value) in &markup.infobox_fields {
            fields.insert(format!("infobox.{key}"), value.clone());
        }
        let mut mapping_hints = vec![StagedMappingHint {
            kind: MappingHintKind::Hierarchy,
            source_key: Some("namespace".into()),
            suggested_value: serde_json::Value::String(namespace_id.clone()),
            confidence: Some(1.0),
            reason: Some("MediaWiki namespace".into()),
        }];
        for category in &markup.categories {
            mapping_hints.push(StagedMappingHint {
                kind: MappingHintKind::SourceCategory,
                source_key: Some("categories".into()),
                suggested_value: serde_json::Value::String(category.clone()),
                confidence: Some(1.0),
                reason: Some("MediaWiki category".into()),
            });
            mapping_hints.push(StagedMappingHint {
                kind: MappingHintKind::Hierarchy,
                source_key: Some("categories".into()),
                suggested_value: serde_json::Value::String(category.clone()),
                confidence: Some(0.7),
                reason: Some("MediaWiki category hierarchy candidate".into()),
            });
        }
        for key in markup.infobox_fields.keys() {
            mapping_hints.push(StagedMappingHint {
                kind: MappingHintKind::Field,
                source_key: Some(format!("infobox.{key}")),
                suggested_value: serde_json::Value::String(key.clone()),
                confidence: Some(0.65),
                reason: Some("MediaWiki infobox parameter".into()),
            });
        }
        let mut metadata = BTreeMap::from([
            (
                "source_format".into(),
                serde_json::Value::String("mediawiki".into()),
            ),
            (
                "document_format".into(),
                serde_json::Value::String("wikitext".into()),
            ),
            (
                "namespace_id".into(),
                serde_json::Value::String(namespace_id),
            ),
            (
                "revision_count".into(),
                serde_json::Value::from(page.revision_count),
            ),
        ]);
        if !namespace_name.is_empty() {
            metadata.insert(
                "namespace".into(),
                serde_json::Value::String(namespace_name),
            );
        }
        for (key, value) in &self.site_metadata {
            metadata.insert(
                format!("wiki_{key}"),
                serde_json::Value::String(value.clone()),
            );
        }
        if let Some(target) = &markup.redirect_target {
            metadata.insert(
                "mediawiki_redirect".into(),
                serde_json::Value::String(target.clone()),
            );
        }
        let mut latest_revision = serde_json::Map::new();
        for (key, value) in [
            ("id", revision.id),
            ("parent_id", revision.parent_id),
            ("timestamp", revision.timestamp),
            ("model", revision.model),
            ("format", revision.format),
            ("sha1", revision.sha1),
            ("contributor", revision.contributor),
        ] {
            if !value.trim().is_empty() {
                latest_revision.insert(key.into(), serde_json::Value::String(value));
            }
        }
        let mut object_diagnostics = Vec::new();
        for warning in markup.warnings {
            self.reserve_diagnostic()?;
            object_diagnostics.push(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: "mediawiki_wikitext_partial".into(),
                message: warning,
                source_path: Some(source_path.clone()),
                object_id: Some(object_id.clone()),
            });
        }
        self.import.objects.push(StagedObject {
            id: object_id.clone(),
            source_id: object_id,
            source_kind: "mediawiki_page".into(),
            source_path: source_path.clone(),
            content_hash: hex_digest(revision.text.as_bytes()),
            title: page.title,
            body: Some(StagedDocument {
                format: "markdown".into(),
                body: revision.text.clone(),
            }),
            parent_source_path: Some(parent_source_path),
            tags: markup.categories,
            aliases: Vec::new(),
            fields,
            metadata,
            raw_source_data: BTreeMap::from([
                ("wikitext".into(), serde_json::Value::String(revision.text)),
                (
                    "latest_revision".into(),
                    serde_json::Value::Object(latest_revision),
                ),
                (
                    "templates".into(),
                    serde_json::Value::Array(markup.templates),
                ),
            ]),
            links: markup.links,
            mapping_hints,
            diagnostics: object_diagnostics,
        });
        self.report_progress(source_bytes, Some(source_path))
    }

    fn resolve_links_and_redirects(&mut self) -> Result<(), CoreError> {
        let mut objects_by_title = BTreeMap::<String, BTreeSet<String>>::new();
        for object in &self.import.objects {
            objects_by_title
                .entry(normalize_mediawiki_title(&object.title))
                .or_default()
                .insert(object.id.clone());
        }
        struct PendingDiagnostic {
            code: &'static str,
            message: String,
            source_path: String,
            object_id: String,
        }
        let mut diagnostics = Vec::new();
        let mut redirect_aliases = Vec::<(String, String)>::new();
        for object in &mut self.import.objects {
            let redirect_target = object
                .metadata
                .get("mediawiki_redirect")
                .and_then(serde_json::Value::as_str)
                .map(normalize_mediawiki_title);
            for link in &mut object.links {
                if link.resolution == StagedLinkResolution::NotApplicable {
                    continue;
                }
                let target_key =
                    normalize_mediawiki_title(mediawiki_link_page_target(&link.target));
                if target_key.is_empty() {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(object.id.clone());
                    continue;
                }
                let candidates = objects_by_title
                    .get(&target_key)
                    .cloned()
                    .unwrap_or_default();
                if candidates.len() == 1 {
                    let target_id = candidates.into_iter().next().expect("one candidate");
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(target_id.clone());
                    if redirect_target.as_deref() == Some(target_key.as_str()) {
                        redirect_aliases.push((target_id.clone(), object.title.clone()));
                        object.mapping_hints.push(StagedMappingHint {
                            kind: MappingHintKind::Relationship,
                            source_key: Some("redirect_target".into()),
                            suggested_value: serde_json::Value::String(target_id),
                            confidence: Some(1.0),
                            reason: Some("unique MediaWiki redirect target".into()),
                        });
                    }
                } else if candidates.len() > 1 {
                    link.resolution = StagedLinkResolution::Ambiguous;
                    link.candidate_object_ids = candidates.into_iter().collect();
                    diagnostics.push(PendingDiagnostic {
                        code: "mediawiki_target_ambiguous",
                        message: format!(
                            "MediaWiki target '{}' matches multiple staged pages.",
                            link.target
                        ),
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                } else {
                    link.resolution = StagedLinkResolution::Missing;
                    diagnostics.push(PendingDiagnostic {
                        code: "mediawiki_target_missing",
                        message: format!(
                            "MediaWiki target '{}' was not found in the selected dump.",
                            link.target
                        ),
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                }
            }
        }
        let object_indexes = self
            .import
            .objects
            .iter()
            .enumerate()
            .map(|(index, object)| (object.id.clone(), index))
            .collect::<BTreeMap<_, _>>();
        for (target_id, alias) in redirect_aliases {
            if let Some(index) = object_indexes.get(&target_id) {
                let target = &mut self.import.objects[*index];
                if alias != target.title && !target.aliases.contains(&alias) {
                    target.aliases.push(alias);
                    target.aliases.sort();
                }
            }
        }
        for diagnostic in diagnostics {
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: diagnostic.code.into(),
                message: diagnostic.message,
                source_path: Some(diagnostic.source_path),
                object_id: Some(diagnostic.object_id),
            })?;
        }
        Ok(())
    }

    fn reserve_diagnostic(&mut self) -> Result<(), CoreError> {
        if self.total_diagnostics >= self.limits.max_diagnostics {
            return Err(CoreError::Validation(format!(
                "MediaWiki analysis exceeds the maximum diagnostic count of {}",
                self.limits.max_diagnostics
            )));
        }
        self.total_diagnostics += 1;
        Ok(())
    }

    fn validate_current_revision_size(
        &self,
        page: &Option<MediaWikiPage>,
    ) -> Result<(), CoreError> {
        if page
            .as_ref()
            .and_then(|page| page.current_revision.as_ref())
            .is_some_and(|revision| revision.text.len() as u64 > self.limits.max_file_bytes)
        {
            return Err(CoreError::Validation(format!(
                "MediaWiki revision exceeds the maximum page size of {} bytes",
                self.limits.max_file_bytes
            )));
        }
        Ok(())
    }

    fn record_diagnostic(&mut self, diagnostic: ImportDiagnostic) -> Result<(), CoreError> {
        self.reserve_diagnostic()?;
        self.import.diagnostics.push(diagnostic);
        Ok(())
    }

    fn report_progress(
        &mut self,
        source_bytes: u64,
        source_path: Option<String>,
    ) -> Result<(), CoreError> {
        (self.progress)(ImportAnalysisProgress {
            processed_entries: self.processed_pages,
            staged_object_count: self.import.objects.len(),
            unsupported_count: self.import.unsupported.len(),
            source_bytes,
            source_path,
        })
    }
}

fn xml_local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn mediawiki_xml_attribute(
    start: &BytesStart<'_>,
    key: &[u8],
    decoder: XmlDecoder,
) -> Result<Option<String>, CoreError> {
    for attribute in start.attributes().with_checks(true) {
        let attribute = attribute.map_err(|error| {
            CoreError::Validation(format!("invalid MediaWiki XML attribute: {error}"))
        })?;
        if xml_local_name(attribute.key.as_ref()) == key {
            return attribute
                .decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)
                .map(|value| Some(value.into_owned()))
                .map_err(|error| {
                    CoreError::Validation(format!("invalid MediaWiki XML attribute value: {error}"))
                });
        }
    }
    Ok(None)
}

fn append_mediawiki_xml_text(
    stack: &[Vec<u8>],
    value: &str,
    page: &mut Option<MediaWikiPage>,
    pending_namespace: &mut Option<(String, String)>,
    site_metadata: &mut BTreeMap<String, String>,
) {
    let element = stack.last().map(Vec::as_slice).unwrap_or_default();
    let parent = stack
        .get(stack.len().saturating_sub(2))
        .map(Vec::as_slice)
        .unwrap_or_default();
    if let Some(page) = page.as_mut() {
        if let Some(revision) = page.current_revision.as_mut() {
            match (parent, element) {
                (b"revision", b"id") => revision.id.push_str(value),
                (b"revision", b"parentid") => revision.parent_id.push_str(value),
                (b"revision", b"timestamp") => revision.timestamp.push_str(value),
                (b"revision", b"model") => revision.model.push_str(value),
                (b"revision", b"format") => revision.format.push_str(value),
                (b"revision", b"sha1") => revision.sha1.push_str(value),
                (_, b"username" | b"ip") => revision.contributor.push_str(value),
                (_, b"text") => revision.text.push_str(value),
                _ => {}
            }
        } else {
            match (parent, element) {
                (b"page", b"title") => page.title.push_str(value),
                (b"page", b"ns") => page.namespace_id.push_str(value),
                (b"page", b"id") => page.id.push_str(value),
                _ => {}
            }
        }
    } else if element == b"namespace" {
        if let Some((_, namespace)) = pending_namespace.as_mut() {
            namespace.push_str(value);
        }
    } else if parent == b"siteinfo"
        && matches!(
            element,
            b"sitename" | b"dbname" | b"base" | b"generator" | b"case"
        )
    {
        site_metadata
            .entry(String::from_utf8_lossy(element).into_owned())
            .or_default()
            .push_str(value);
    }
}

fn mediawiki_revision_is_newer(candidate: &MediaWikiRevision, current: &MediaWikiRevision) -> bool {
    let candidate_timestamp = candidate.timestamp.trim();
    let current_timestamp = current.timestamp.trim();
    if candidate_timestamp != current_timestamp {
        return candidate_timestamp > current_timestamp;
    }
    let candidate_id = candidate.id.trim().parse::<u64>().unwrap_or_default();
    let current_id = current.id.trim().parse::<u64>().unwrap_or_default();
    candidate_id >= current_id
}

#[derive(Debug, Default)]
struct MediaWikiMarkup {
    categories: Vec<String>,
    links: Vec<StagedLink>,
    redirect_target: Option<String>,
    template_names: Vec<String>,
    templates: Vec<serde_json::Value>,
    infobox_fields: BTreeMap<String, serde_json::Value>,
    warnings: Vec<String>,
}

fn analyze_mediawiki_markup(wikitext: &str, xml_redirect: Option<&str>) -> MediaWikiMarkup {
    let mut markup = MediaWikiMarkup::default();
    let mut categories = BTreeSet::new();
    let mut index = 0;
    while let Some(relative_start) = wikitext[index..].find("[[") {
        let start = index + relative_start;
        let Some(relative_end) = wikitext[start + 2..].find("]]") else {
            markup
                .warnings
                .push("Preserved an unclosed MediaWiki internal link.".into());
            break;
        };
        let end = start + 2 + relative_end;
        let raw = &wikitext[start..end + 2];
        let content = wikitext[start + 2..end].trim();
        let (target, label) = content
            .split_once('|')
            .map(|(target, label)| (target.trim(), Some(label.trim())))
            .unwrap_or((content, None));
        if !target.is_empty() {
            let semantic_target = target.trim_start_matches(':');
            let (prefix, suffix) = semantic_target
                .split_once(':')
                .map(|(prefix, suffix)| (prefix.trim(), suffix.trim()))
                .unwrap_or(("", semantic_target));
            let is_category = !target.starts_with(':') && prefix.eq_ignore_ascii_case("category");
            let is_file = matches_ignore_ascii_case(prefix, &["file", "image"]);
            if is_category {
                let category = mediawiki_link_page_target(suffix).trim();
                if !category.is_empty() {
                    categories.insert(category.to_owned());
                }
            }
            markup.links.push(StagedLink {
                kind: if is_file {
                    StagedLinkKind::Embed
                } else {
                    StagedLinkKind::Internal
                },
                target: semantic_target.into(),
                label: label.filter(|label| !label.is_empty()).map(str::to_owned),
                resolution: if is_category || is_file {
                    StagedLinkResolution::NotApplicable
                } else {
                    StagedLinkResolution::Unresolved
                },
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: Some(raw.into()),
            });
        }
        index = end + 2;
    }
    markup.categories = categories.into_iter().collect();

    let (templates, template_warnings) = discover_mediawiki_templates(wikitext);
    markup.warnings.extend(template_warnings);
    let mut template_names = BTreeSet::new();
    for template in templates {
        template_names.insert(template.name.clone());
        if template.name.to_ascii_lowercase().starts_with("infobox") {
            for (key, value) in &template.parameters {
                let key = normalize_mediawiki_field_key(key);
                if key.is_empty() {
                    continue;
                }
                insert_mediawiki_field_value(
                    &mut markup.infobox_fields,
                    key,
                    serde_json::Value::String(value.clone()),
                );
            }
        }
        markup.templates.push(serde_json::json!({
            "name": template.name,
            "parameters": template.parameters,
            "raw": template.raw,
        }));
    }
    markup.template_names = template_names.into_iter().collect();
    markup.redirect_target = xml_redirect
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            let trimmed = wikitext.trim_start();
            let prefix = trimmed.get(..9)?;
            prefix.eq_ignore_ascii_case("#redirect").then(|| {
                markup
                    .links
                    .iter()
                    .find(|link| link.kind == StagedLinkKind::Internal)
                    .map(|link| link.target.clone())
            })?
        });
    if let Some(target) = &markup.redirect_target {
        let normalized = normalize_mediawiki_title(mediawiki_link_page_target(target));
        if !markup.links.iter().any(|link| {
            link.kind == StagedLinkKind::Internal
                && normalize_mediawiki_title(mediawiki_link_page_target(&link.target)) == normalized
        }) {
            markup.links.push(StagedLink {
                kind: StagedLinkKind::Internal,
                target: target.clone(),
                label: None,
                resolution: StagedLinkResolution::Unresolved,
                resolved_object_id: None,
                candidate_object_ids: Vec::new(),
                raw: None,
            });
        }
    }
    markup.warnings.sort();
    markup.warnings.dedup();
    markup
}

#[derive(Debug)]
struct MediaWikiTemplate {
    name: String,
    parameters: BTreeMap<String, String>,
    raw: String,
}

fn discover_mediawiki_templates(wikitext: &str) -> (Vec<MediaWikiTemplate>, Vec<String>) {
    let bytes = wikitext.as_bytes();
    let mut templates = Vec::new();
    let mut warnings = Vec::new();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] != b'{' || bytes[index + 1] != b'{' || bytes.get(index + 2) == Some(&b'{') {
            index += 1;
            continue;
        }
        let start = index;
        let mut depth = 1_usize;
        index += 2;
        while index + 1 < bytes.len() && depth > 0 {
            if bytes[index] == b'{' && bytes[index + 1] == b'{' {
                depth = depth.saturating_add(1);
                if depth > MAX_MEDIAWIKI_TEMPLATE_DEPTH {
                    warnings.push(format!(
                        "Template nesting exceeded the maximum depth of {MAX_MEDIAWIKI_TEMPLATE_DEPTH}."
                    ));
                    return (templates, warnings);
                }
                index += 2;
            } else if bytes[index] == b'}' && bytes[index + 1] == b'}' {
                depth -= 1;
                index += 2;
            } else {
                index += 1;
            }
        }
        if depth != 0 {
            warnings.push("Preserved an unclosed MediaWiki template invocation.".into());
            break;
        }
        if templates.len() >= MAX_MEDIAWIKI_TEMPLATES_PER_PAGE {
            warnings.push(format!(
                "Only the first {MAX_MEDIAWIKI_TEMPLATES_PER_PAGE} template invocations were analyzed."
            ));
            break;
        }
        let raw = &wikitext[start..index];
        let inner = &raw[2..raw.len() - 2];
        let parts = split_mediawiki_template_parts(inner);
        let Some(name) = parts
            .first()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
        else {
            continue;
        };
        if name.starts_with('{') {
            continue;
        }
        let mut parameters = BTreeMap::new();
        let mut positional = 0_usize;
        for part in parts.into_iter().skip(1) {
            let (key, value) = if let Some((key, value)) = split_mediawiki_parameter(&part) {
                (key.trim().to_owned(), value.trim().to_owned())
            } else {
                positional += 1;
                (positional.to_string(), part.trim().to_owned())
            };
            if !key.is_empty() {
                parameters.insert(key, value);
            }
        }
        templates.push(MediaWikiTemplate {
            name: name.replace('_', " ").trim().to_owned(),
            parameters,
            raw: raw.to_owned(),
        });
    }
    (templates, warnings)
}

fn split_mediawiki_template_parts(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut template_depth = 0_usize;
    let mut link_depth = 0_usize;
    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"{{") {
            template_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"}}") {
            template_depth = template_depth.saturating_sub(1);
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"[[") {
            link_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"]]") {
            link_depth = link_depth.saturating_sub(1);
            index += 2;
        } else if bytes[index] == b'|' && template_depth == 0 && link_depth == 0 {
            parts.push(value[start..index].to_owned());
            start = index + 1;
            index += 1;
        } else {
            index += 1;
        }
    }
    parts.push(value[start..].to_owned());
    parts
}

fn split_mediawiki_parameter(value: &str) -> Option<(&str, &str)> {
    let bytes = value.as_bytes();
    let mut template_depth = 0_usize;
    let mut link_depth = 0_usize;
    let mut index = 0;
    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"{{") {
            template_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"}}") {
            template_depth = template_depth.saturating_sub(1);
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"[[") {
            link_depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"]]") {
            link_depth = link_depth.saturating_sub(1);
            index += 2;
        } else if bytes[index] == b'=' && template_depth == 0 && link_depth == 0 {
            return Some((&value[..index], &value[index + 1..]));
        } else {
            index += 1;
        }
    }
    None
}

fn insert_mediawiki_field_value(
    fields: &mut BTreeMap<String, serde_json::Value>,
    key: String,
    value: serde_json::Value,
) {
    match fields.remove(&key) {
        None => {
            fields.insert(key, value);
        }
        Some(serde_json::Value::Array(mut values)) => {
            values.push(value);
            fields.insert(key, serde_json::Value::Array(values));
        }
        Some(previous) => {
            fields.insert(key, serde_json::Value::Array(vec![previous, value]));
        }
    }
}

fn normalize_mediawiki_field_key(value: &str) -> String {
    let mut normalized = String::new();
    let mut pending_separator = false;
    for character in value.trim().chars().take(128) {
        if character.is_alphanumeric() {
            if pending_separator && !normalized.is_empty() {
                normalized.push('_');
            }
            normalized.extend(character.to_lowercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    normalized
}

fn normalize_mediawiki_title(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(':')
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn mediawiki_link_page_target(value: &str) -> &str {
    value
        .split_once('#')
        .map(|(page, _)| page)
        .unwrap_or(value)
        .trim()
}

fn matches_ignore_ascii_case(value: &str, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportProfile {
    Generic,
    Obsidian,
}

fn analyze_documents_with_progress(
    source: impl AsRef<Path>,
    limits: GenericDocumentImportLimits,
    profile: ImportProfile,
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
    if profile == ImportProfile::Obsidian && !metadata.is_dir() {
        return Err(CoreError::Validation(
            "Obsidian import requires a vault folder".into(),
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
    let source_kind = if profile == ImportProfile::Obsidian {
        ImportSourceKind::Vault
    } else if metadata.is_dir() {
        ImportSourceKind::Folder
    } else if is_zip_archive {
        ImportSourceKind::Archive
    } else {
        ImportSourceKind::File
    };
    let (importer_id, importer_version, importer_name) = match profile {
        ImportProfile::Generic => (
            GENERIC_DOCUMENT_IMPORTER_ID,
            GENERIC_DOCUMENT_IMPORTER_VERSION,
            "Generic documents",
        ),
        ImportProfile::Obsidian => (
            OBSIDIAN_IMPORTER_ID,
            OBSIDIAN_IMPORTER_VERSION,
            "Obsidian vault",
        ),
    };
    let mut analyzer = GenericDocumentAnalyzer {
        profile,
        limits,
        import: StagedImport {
            schema_version: STAGED_IMPORT_SCHEMA_VERSION,
            importer: ImporterIdentity {
                id: importer_id.into(),
                version: importer_version.into(),
                name: importer_name.into(),
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
    if profile == ImportProfile::Obsidian {
        analyzer.resolve_obsidian_references()?;
    } else {
        analyzer.resolve_markdown_references()?;
    }
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
    profile: ImportProfile,
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
            } else if metadata.is_dir()
                && self.profile == ImportProfile::Obsidian
                && child_parts.len() == 1
                && matches!(child_parts[0].as_str(), ".obsidian" | ".trash")
            {
                self.record_unsupported(
                    relative_path.clone(),
                    "obsidian_configuration",
                    "Obsidian configuration and trash folders are intentionally excluded",
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
        if asset_mime_type(source_path).is_none() && !self.supports_document(source_path) {
            return self.record_unsupported(
                source_path.to_owned(),
                "file",
                if self.profile == ImportProfile::Obsidian {
                    "file type is not supported by the Obsidian vault importer"
                } else {
                    "file type is not supported by the generic document importer"
                },
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

    fn supports_document(&self, source_path: &str) -> bool {
        match self.profile {
            ImportProfile::Generic => document_format(source_path).is_some(),
            ImportProfile::Obsidian => document_format(source_path) == Some("markdown"),
        }
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
                    self.import.importer.id, self.import.source.id, source_path
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
                self.import.importer.id, self.import.source.id, source_path
            )
            .as_bytes(),
        );
        let mut title = document_title(source_path);
        let parent_source_path = source_path
            .rsplit_once('/')
            .map(|(parent, _)| parent.to_owned());
        let mut body_format = source_format;
        let frontmatter = (source_format == "markdown")
            .then(|| markdown_frontmatter(&body).map(str::to_owned))
            .flatten();
        let mut fields = BTreeMap::new();
        let mut raw_source_data = BTreeMap::new();
        let mut aliases = Vec::new();
        let mut tags = Vec::new();
        let mut mapping_hints = Vec::new();
        if let Some(frontmatter) = &frontmatter {
            raw_source_data.insert(
                "frontmatter".into(),
                serde_json::Value::String(frontmatter.clone()),
            );
            if self.profile == ImportProfile::Obsidian {
                let parsed = parse_obsidian_frontmatter(frontmatter);
                fields = parsed.fields;
                aliases = parsed.aliases;
                tags = parsed.tags;
                if let Some(entity_type) = parsed.entity_type_hint {
                    mapping_hints.push(StagedMappingHint {
                        kind: MappingHintKind::EntityType,
                        source_key: Some("type".into()),
                        suggested_value: serde_json::Value::String(entity_type),
                        confidence: Some(0.85),
                        reason: Some("Obsidian YAML frontmatter type".into()),
                    });
                }
                for message in parsed.warnings {
                    self.record_diagnostic(ImportDiagnostic {
                        severity: ImportDiagnosticSeverity::Warning,
                        code: "obsidian_frontmatter_partial".into(),
                        message,
                        source_path: Some(source_path.to_owned()),
                        object_id: Some(source_id.clone()),
                    })?;
                }
            } else {
                fields.insert(
                    "frontmatter".into(),
                    serde_json::Value::String(frontmatter.clone()),
                );
            }
        }
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
            let link_body = if self.profile == ImportProfile::Obsidian {
                markdown_body_after_frontmatter(&body)
            } else {
                &body
            };
            let mut links = discover_markdown_links(link_body);
            if self.profile == ImportProfile::Obsidian {
                links.extend(discover_obsidian_links(link_body));
            }
            links
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
            source_kind: if self.profile == ImportProfile::Obsidian {
                "obsidian_markdown".into()
            } else {
                source_format.to_owned()
            },
            source_path: source_path.to_owned(),
            content_hash,
            title,
            body: Some(StagedDocument {
                format: body_format.to_owned(),
                body,
            }),
            parent_source_path,
            tags,
            aliases,
            fields,
            metadata,
            raw_source_data,
            links,
            mapping_hints,
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
                self.import.importer.id, self.import.source.id, source_path
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
                    self.import.importer.id, self.import.source.id, asset_source_path
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

    fn resolve_obsidian_references(&mut self) -> Result<(), CoreError> {
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
        let mut object_keys = BTreeMap::<String, BTreeSet<String>>::new();
        for object in &self.import.objects {
            for key in std::iter::once(object.source_path.as_str())
                .chain(std::iter::once(obsidian_path_without_markdown_extension(
                    &object.source_path,
                )))
                .chain(
                    Path::new(&object.source_path)
                        .file_stem()
                        .and_then(|value| value.to_str()),
                )
                .chain(std::iter::once(object.title.as_str()))
                .chain(object.aliases.iter().map(String::as_str))
            {
                object_keys
                    .entry(obsidian_lookup_key(key))
                    .or_default()
                    .insert(object.id.clone());
            }
        }
        let mut asset_keys = BTreeMap::<String, BTreeSet<usize>>::new();
        for (index, asset) in self.import.assets.iter().enumerate() {
            for key in [asset.source_path.as_str(), asset.filename.as_str()] {
                asset_keys
                    .entry(obsidian_lookup_key(key))
                    .or_default()
                    .insert(index);
            }
        }

        struct PendingDiagnostic {
            code: &'static str,
            message: String,
            source_path: String,
            object_id: String,
        }
        let mut diagnostics = Vec::new();
        let (objects, assets) = (&mut self.import.objects, &mut self.import.assets);
        for object in objects {
            for link in &mut object.links {
                if is_external_markdown_target(&link.target) {
                    link.resolution = StagedLinkResolution::NotApplicable;
                    continue;
                }
                if link.raw.is_none() {
                    let Some(target_path) =
                        resolve_relative_source_path(&object.source_path, &link.target)
                    else {
                        link.resolution = StagedLinkResolution::Missing;
                        diagnostics.push(PendingDiagnostic {
                            code: "markdown_target_missing",
                            message: format!(
                                "Markdown target '{}' was not found in the selected vault.",
                                link.target
                            ),
                            source_path: object.source_path.clone(),
                            object_id: object.id.clone(),
                        });
                        continue;
                    };
                    if let Some(target_id) = objects_by_path.get(&target_path) {
                        link.resolution = StagedLinkResolution::Resolved;
                        link.resolved_object_id = Some(target_id.clone());
                    } else if let Some(asset_index) = assets_by_path.get(&target_path) {
                        attach_obsidian_asset(
                            &object.id,
                            &mut object.mapping_hints,
                            link,
                            &target_path,
                            &mut assets[*asset_index],
                        );
                    } else {
                        link.resolution = StagedLinkResolution::Missing;
                        diagnostics.push(PendingDiagnostic {
                            code: "markdown_target_missing",
                            message: format!(
                                "Markdown target '{}' was not found in the selected vault.",
                                link.target
                            ),
                            source_path: object.source_path.clone(),
                            object_id: object.id.clone(),
                        });
                    }
                    continue;
                }

                let target = obsidian_target_path(&link.target);
                if target.is_empty() {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = Some(object.id.clone());
                    continue;
                }
                let candidate_paths = obsidian_candidate_paths(&object.source_path, target);
                let mut object_candidates = BTreeSet::new();
                for path in &candidate_paths {
                    if let Some(candidates) = object_keys.get(&obsidian_lookup_key(path)) {
                        object_candidates.extend(candidates.iter().cloned());
                    }
                    if Path::new(path).extension().is_none() {
                        let markdown_path = format!("{path}.md");
                        if let Some(candidates) =
                            object_keys.get(&obsidian_lookup_key(&markdown_path))
                        {
                            object_candidates.extend(candidates.iter().cloned());
                        }
                    }
                }
                if object_candidates.is_empty() {
                    for key in obsidian_fallback_keys(target) {
                        if let Some(candidates) = object_keys.get(&key) {
                            object_candidates.extend(candidates.iter().cloned());
                        }
                    }
                }
                let mut asset_candidates = BTreeSet::new();
                for path in &candidate_paths {
                    if let Some(candidates) = asset_keys.get(&obsidian_lookup_key(path)) {
                        asset_candidates.extend(candidates.iter().copied());
                    }
                }
                if asset_candidates.is_empty() {
                    for key in obsidian_fallback_keys(target) {
                        if let Some(candidates) = asset_keys.get(&key) {
                            asset_candidates.extend(candidates.iter().copied());
                        }
                    }
                }

                let prefer_asset =
                    link.kind == StagedLinkKind::Embed && obsidian_target_looks_like_asset(target);
                if prefer_asset && asset_candidates.len() == 1 {
                    let index = *asset_candidates.iter().next().expect("one candidate");
                    let target_path = assets[index].source_path.clone();
                    attach_obsidian_asset(
                        &object.id,
                        &mut object.mapping_hints,
                        link,
                        &target_path,
                        &mut assets[index],
                    );
                } else if object_candidates.len() == 1 {
                    link.resolution = StagedLinkResolution::Resolved;
                    link.resolved_object_id = object_candidates.into_iter().next();
                } else if object_candidates.len() > 1 {
                    link.resolution = StagedLinkResolution::Ambiguous;
                    link.candidate_object_ids = object_candidates.into_iter().collect();
                    diagnostics.push(PendingDiagnostic {
                        code: "obsidian_target_ambiguous",
                        message: format!(
                            "Obsidian target '{}' matches multiple notes.",
                            link.target
                        ),
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                } else if asset_candidates.len() == 1 {
                    let index = *asset_candidates.iter().next().expect("one candidate");
                    let target_path = assets[index].source_path.clone();
                    attach_obsidian_asset(
                        &object.id,
                        &mut object.mapping_hints,
                        link,
                        &target_path,
                        &mut assets[index],
                    );
                } else {
                    link.resolution = StagedLinkResolution::Missing;
                    let (code, message) = if asset_candidates.len() > 1 {
                        (
                            "obsidian_asset_ambiguous",
                            format!(
                                "Obsidian target '{}' matches multiple attachments.",
                                link.target
                            ),
                        )
                    } else {
                        (
                            "obsidian_target_missing",
                            format!(
                                "Obsidian target '{}' was not found in the vault.",
                                link.target
                            ),
                        )
                    };
                    diagnostics.push(PendingDiagnostic {
                        code,
                        message,
                        source_path: object.source_path.clone(),
                        object_id: object.id.clone(),
                    });
                }
            }
        }
        for diagnostic in diagnostics {
            self.record_diagnostic(ImportDiagnostic {
                severity: ImportDiagnosticSeverity::Warning,
                code: diagnostic.code.into(),
                message: diagnostic.message,
                source_path: Some(diagnostic.source_path),
                object_id: Some(diagnostic.object_id),
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

fn attach_obsidian_asset(
    object_id: &str,
    mapping_hints: &mut Vec<StagedMappingHint>,
    link: &mut StagedLink,
    target_path: &str,
    asset: &mut StagedAsset,
) {
    link.resolution = StagedLinkResolution::NotApplicable;
    if asset.owner_object_id.is_none() {
        asset.owner_object_id = Some(object_id.into());
    }
    asset.raw_metadata.insert(
        "resolved_from".into(),
        serde_json::Value::String(link.target.clone()),
    );
    mapping_hints.push(StagedMappingHint {
        kind: MappingHintKind::AssetRelationship,
        source_key: Some(target_path.into()),
        suggested_value: serde_json::Value::String("attachment".into()),
        confidence: Some(1.0),
        reason: Some("Obsidian attachment or embed".into()),
    });
}

fn obsidian_path_without_markdown_extension(path: &str) -> &str {
    path.get(path.len().saturating_sub(3)..)
        .filter(|suffix| suffix.eq_ignore_ascii_case(".md"))
        .map_or(path, |_| &path[..path.len() - 3])
}

fn obsidian_lookup_key(value: &str) -> String {
    value.trim().trim_start_matches('/').to_lowercase()
}

fn obsidian_target_path(target: &str) -> &str {
    let target = target
        .split_once('#')
        .map(|(path, _)| path)
        .unwrap_or(target);
    target
        .split_once('^')
        .map(|(path, _)| path)
        .unwrap_or(target)
        .trim()
}

fn obsidian_candidate_paths(source_path: &str, target: &str) -> Vec<String> {
    if target.contains('\\') || target.chars().any(char::is_control) {
        return Vec::new();
    }
    let target = target.trim();
    let root_target = target.trim_start_matches('/');
    let mut paths = BTreeSet::new();
    if !target.starts_with('/') {
        if let Some(relative) = resolve_relative_source_path(source_path, target) {
            paths.insert(relative);
        }
    }
    if !root_target.is_empty() {
        paths.insert(root_target.into());
    }
    paths.into_iter().collect()
}

fn obsidian_fallback_keys(target: &str) -> Vec<String> {
    let target = target.trim().trim_start_matches('/');
    let mut keys = BTreeSet::new();
    keys.insert(obsidian_lookup_key(target));
    keys.insert(obsidian_lookup_key(
        obsidian_path_without_markdown_extension(target),
    ));
    if let Some(filename) = Path::new(target)
        .file_name()
        .and_then(|value| value.to_str())
    {
        keys.insert(obsidian_lookup_key(filename));
    }
    if let Some(stem) = Path::new(target)
        .file_stem()
        .and_then(|value| value.to_str())
    {
        keys.insert(obsidian_lookup_key(stem));
    }
    keys.into_iter().filter(|key| !key.is_empty()).collect()
}

fn obsidian_target_looks_like_asset(target: &str) -> bool {
    Path::new(target)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| !extension.eq_ignore_ascii_case("md"))
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

fn discover_obsidian_links(body: &str) -> Vec<StagedLink> {
    let mut links = Vec::new();
    let mut fence = None::<(u8, usize)>;
    let mut frontmatter = markdown_frontmatter(body).is_some();
    let mut first_line = true;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if frontmatter {
            if !first_line && matches!(trimmed, "---" | "...") {
                frontmatter = false;
            }
            first_line = false;
            continue;
        }
        first_line = false;
        if let Some((marker, opening_run)) = fence {
            let bytes = trimmed.as_bytes();
            let run = bytes.iter().take_while(|byte| **byte == marker).count();
            if run >= opening_run && bytes[run..].iter().all(u8::is_ascii_whitespace) {
                fence = None;
            }
            continue;
        }
        if let Some(marker @ (0x60 | b'~')) = trimmed.as_bytes().first().copied() {
            let run = trimmed
                .as_bytes()
                .iter()
                .take_while(|byte| **byte == marker)
                .count();
            if run >= 3 {
                fence = Some((marker, run));
                continue;
            }
        }
        let bytes = line.as_bytes();
        let mut index = 0;
        let mut inline_code = None::<usize>;
        while index < bytes.len() {
            if bytes[index] == 0x60 {
                let run = bytes[index..]
                    .iter()
                    .take_while(|byte| **byte == 0x60)
                    .count();
                if !obsidian_syntax_is_escaped(bytes, index) {
                    match inline_code {
                        Some(opening_run) if opening_run == run => inline_code = None,
                        None => inline_code = Some(run),
                        _ => {}
                    }
                }
                index += run;
                continue;
            }
            if inline_code.is_some() {
                index += 1;
                continue;
            }
            let (embed, open) = if bytes[index] == b'!'
                && bytes.get(index + 1) == Some(&b'[')
                && bytes.get(index + 2) == Some(&b'[')
            {
                (true, index + 1)
            } else if bytes[index] == b'[' && bytes.get(index + 1) == Some(&b'[') {
                (false, index)
            } else {
                index += 1;
                continue;
            };
            let raw_start = if embed { open - 1 } else { open };
            if obsidian_syntax_is_escaped(bytes, raw_start) {
                index = open + 2;
                continue;
            }
            let content_start = open + 2;
            let Some(relative_end) = line[content_start..].find("]]") else {
                break;
            };
            let content_end = content_start + relative_end;
            let content = line[content_start..content_end].trim();
            let raw_end = content_end + 2;
            let raw = line[raw_start..raw_end].to_owned();
            let (target, label) = content
                .split_once('|')
                .map(|(target, label)| (target.trim(), Some(label.trim())))
                .unwrap_or((content, None));
            if !target.is_empty() {
                links.push(StagedLink {
                    kind: if embed {
                        StagedLinkKind::Embed
                    } else {
                        StagedLinkKind::Internal
                    },
                    target: target.into(),
                    label: label.filter(|label| !label.is_empty()).map(str::to_owned),
                    resolution: StagedLinkResolution::Unresolved,
                    resolved_object_id: None,
                    candidate_object_ids: Vec::new(),
                    raw: Some(raw),
                });
            }
            index = raw_end;
        }
    }
    links
}

fn obsidian_syntax_is_escaped(bytes: &[u8], index: usize) -> bool {
    let preceding_backslashes = bytes[..index]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count();
    preceding_backslashes % 2 == 1
}

#[derive(Debug, Default)]
struct ObsidianFrontmatter {
    fields: BTreeMap<String, serde_json::Value>,
    aliases: Vec<String>,
    tags: Vec<String>,
    entity_type_hint: Option<String>,
    warnings: Vec<String>,
}

fn parse_obsidian_frontmatter(frontmatter: &str) -> ObsidianFrontmatter {
    let lines = frontmatter.lines().collect::<Vec<_>>();
    let mut parsed = ObsidianFrontmatter::default();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        if line.starts_with(char::is_whitespace) {
            parsed
                .warnings
                .push("Ignored an unattached indented YAML frontmatter line.".into());
            index += 1;
            continue;
        }
        let Some((key, remainder)) = line.split_once(':') else {
            parsed
                .warnings
                .push("Ignored a YAML frontmatter line without a key/value separator.".into());
            index += 1;
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || !key.chars().all(|character| {
                character.is_alphanumeric() || matches!(character, '_' | '-' | '.')
            })
        {
            parsed
                .warnings
                .push(format!("Ignored unsupported YAML frontmatter key: {key}"));
            index += 1;
            continue;
        }
        let remainder = remainder.trim();
        let mut consumed_until = index + 1;
        let value = if remainder.is_empty() || matches!(remainder, "|" | ">") {
            let mut block = Vec::new();
            while consumed_until < lines.len()
                && (lines[consumed_until].starts_with(char::is_whitespace)
                    || lines[consumed_until].trim().is_empty())
            {
                block.push(lines[consumed_until]);
                consumed_until += 1;
            }
            let non_empty = block
                .iter()
                .filter(|line| !line.trim().is_empty())
                .collect::<Vec<_>>();
            if !non_empty.is_empty()
                && non_empty
                    .iter()
                    .all(|line| line.trim_start().starts_with("- "))
            {
                serde_json::Value::Array(
                    non_empty
                        .into_iter()
                        .map(|line| parse_obsidian_yaml_scalar(line.trim_start()[2..].trim()))
                        .collect(),
                )
            } else if matches!(remainder, "|" | ">") {
                let separator = if remainder == ">" { " " } else { "\n" };
                serde_json::Value::String(
                    block
                        .into_iter()
                        .map(|line| line.trim_start())
                        .collect::<Vec<_>>()
                        .join(separator),
                )
            } else if block.is_empty() {
                serde_json::Value::Null
            } else {
                parsed.warnings.push(format!(
                    "Preserved unsupported nested YAML for '{key}' as text."
                ));
                serde_json::Value::String(
                    block
                        .into_iter()
                        .map(str::trim_end)
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            }
        } else {
            parse_obsidian_yaml_value(remainder, &mut parsed.warnings)
        };
        if parsed.fields.insert(key.into(), value).is_some() {
            parsed.warnings.push(format!(
                "A duplicate YAML frontmatter key was replaced: {key}"
            ));
        }
        index = consumed_until;
    }

    parsed.aliases = ["aliases", "alias"]
        .into_iter()
        .filter_map(|key| parsed.fields.get(key))
        .flat_map(obsidian_string_values)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    parsed.tags = parsed
        .fields
        .get("tags")
        .or_else(|| parsed.fields.get("tag"))
        .into_iter()
        .flat_map(obsidian_string_values)
        .flat_map(|value| {
            value
                .split([',', ' '])
                .map(|tag| tag.trim().trim_start_matches('#').to_owned())
                .filter(|tag| !tag.is_empty())
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    parsed.entity_type_hint = parsed
        .fields
        .get("type")
        .or_else(|| parsed.fields.get("entity_type"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    parsed.warnings.sort();
    parsed.warnings.dedup();
    parsed
}

fn parse_obsidian_yaml_value(value: &str, warnings: &mut Vec<String>) -> serde_json::Value {
    let value = value.trim();
    if value.starts_with('[') && value.ends_with(']') {
        return serde_json::Value::Array(
            split_obsidian_inline_list(&value[1..value.len() - 1])
                .into_iter()
                .map(|value| parse_obsidian_yaml_scalar(&value))
                .collect(),
        );
    }
    if value.starts_with('{') && value.ends_with('}') {
        if let Ok(value) = serde_json::from_str(value) {
            return value;
        }
        warnings.push("Preserved a non-JSON inline YAML mapping as text.".into());
    }
    parse_obsidian_yaml_scalar(value)
}

fn parse_obsidian_yaml_scalar(value: &str) -> serde_json::Value {
    let value = value.trim();
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        return serde_json::from_str(value)
            .unwrap_or_else(|_| serde_json::Value::String(value[1..value.len() - 1].into()));
    }
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        return serde_json::Value::String(value[1..value.len() - 1].replace("''", "'"));
    }
    match value.to_ascii_lowercase().as_str() {
        "true" => return serde_json::Value::Bool(true),
        "false" => return serde_json::Value::Bool(false),
        "null" | "~" => return serde_json::Value::Null,
        _ => {}
    }
    if let Ok(integer) = value.parse::<i64>() {
        return serde_json::Value::from(integer);
    }
    if let Ok(float) = value.parse::<f64>() {
        if float.is_finite() {
            return serde_json::Value::from(float);
        }
    }
    serde_json::Value::String(value.into())
}

fn split_obsidian_inline_list(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && quote == Some('"') {
            current.push(character);
            escaped = true;
            continue;
        }
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            }
            current.push(character);
            continue;
        }
        if character == ',' && quote.is_none() {
            values.push(current.trim().to_owned());
            current.clear();
        } else {
            current.push(character);
        }
    }
    if !current.trim().is_empty() {
        values.push(current.trim().to_owned());
    }
    values
}

fn obsidian_string_values(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(|value| match value {
                serde_json::Value::String(value) => Some(value.clone()),
                serde_json::Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
            .collect(),
        serde_json::Value::String(value) => vec![value.clone()],
        serde_json::Value::Number(value) => vec![value.to_string()],
        _ => Vec::new(),
    }
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

fn markdown_body_after_frontmatter(body: &str) -> &str {
    let Some(remainder) = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))
    else {
        return body;
    };
    let mut offset = 0;
    for line in remainder.split_inclusive('\n') {
        let value = line.trim_end_matches(['\r', '\n']);
        offset += line.len();
        if value == "---" || value == "..." {
            return &remainder[offset..];
        }
    }
    body
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
    fn obsidian_vault_preserves_frontmatter_and_resolves_vault_links_and_embeds() {
        let vault = TestDirectory::new();
        fs::create_dir_all(vault.path().join(".obsidian")).unwrap();
        fs::create_dir_all(vault.path().join("Characters")).unwrap();
        fs::create_dir_all(vault.path().join("Places")).unwrap();
        fs::create_dir_all(vault.path().join("assets")).unwrap();
        fs::write(vault.path().join(".obsidian/app.json"), "{}").unwrap();
        let home_body = r#"---
aliases:
  - The Grey
  - Mithrandir
tags: [wizard, fellowship]
type: person
species: Maia
rank: 7
homepage: "[Metadata](Missing.md)"
---
# Gandalf

Travel to [[Places/Middle Earth|Middle Earth]] or [[Middle Earth]].
![[assets/map.png|Map]]
![[Places/Middle Earth#North]]
`[[Ignored inline]]`
```text
[[Ignored fenced]]
```
> [!note] Unsupported plugin syntax remains verbatim.
"#;
        fs::write(vault.path().join("Characters/Gandalf.md"), home_body).unwrap();
        fs::write(
            vault.path().join("Places/Middle Earth.md"),
            "# Middle Earth\n\n## North\n",
        )
        .unwrap();
        fs::write(
            vault.path().join("assets/map.png"),
            b"\x89PNG\r\n\x1a\nfixture",
        )
        .unwrap();

        let staged =
            analyze_obsidian_vault(vault.path(), GenericDocumentImportLimits::default()).unwrap();
        let gandalf = staged
            .objects
            .iter()
            .find(|object| object.source_path == "Characters/Gandalf.md")
            .unwrap();

        assert_eq!(staged.importer.id, OBSIDIAN_IMPORTER_ID);
        assert_eq!(staged.source.kind, ImportSourceKind::Vault);
        assert_eq!(gandalf.source_kind, "obsidian_markdown");
        assert_eq!(gandalf.aliases, vec!["Mithrandir", "The Grey"]);
        assert_eq!(gandalf.tags, vec!["fellowship", "wizard"]);
        assert_eq!(gandalf.fields["species"], "Maia");
        assert_eq!(gandalf.fields["rank"], 7);
        assert!(!gandalf.links.iter().any(|link| link.target == "Missing.md"));
        assert_eq!(gandalf.body.as_ref().unwrap().body, home_body);
        assert!(gandalf.raw_source_data["frontmatter"]
            .as_str()
            .unwrap()
            .contains("type: person"));
        assert!(gandalf.mapping_hints.iter().any(|hint| {
            hint.kind == MappingHintKind::EntityType && hint.suggested_value == "person"
        }));
        assert_eq!(
            gandalf
                .links
                .iter()
                .filter(|link| link.raw.is_some())
                .count(),
            4
        );
        assert!(gandalf.links.iter().any(|link| {
            link.target == "Middle Earth" && link.resolution == StagedLinkResolution::Resolved
        }));
        assert!(gandalf.links.iter().any(|link| {
            link.target == "Places/Middle Earth#North"
                && link.kind == StagedLinkKind::Embed
                && link.resolution == StagedLinkResolution::Resolved
        }));
        assert!(gandalf.links.iter().any(|link| {
            link.target == "assets/map.png"
                && link.kind == StagedLinkKind::Embed
                && link.resolution == StagedLinkResolution::NotApplicable
        }));
        assert_eq!(staged.assets.len(), 1);
        assert_eq!(
            staged.assets[0].owner_object_id.as_deref(),
            Some(gandalf.id.as_str())
        );
        assert!(staged.unsupported.iter().any(|item| {
            item.source_path == ".obsidian" && item.source_kind == "obsidian_configuration"
        }));
    }

    #[test]
    fn obsidian_vault_reports_ambiguous_missing_and_partial_frontmatter() {
        let vault = TestDirectory::new();
        fs::create_dir_all(vault.path().join("A")).unwrap();
        fs::create_dir_all(vault.path().join("B")).unwrap();
        fs::write(vault.path().join("A/Twin.md"), "# First").unwrap();
        fs::write(vault.path().join("B/Twin.md"), "# Second").unwrap();
        fs::write(
            vault.path().join("Home.md"),
            "---\ncustom:\n  nested: value\n---\n[[Twin]] [[Missing]]",
        )
        .unwrap();

        let staged =
            analyze_obsidian_vault(vault.path(), GenericDocumentImportLimits::default()).unwrap();
        let home = staged
            .objects
            .iter()
            .find(|object| object.source_path == "Home.md")
            .unwrap();
        let twin = home
            .links
            .iter()
            .find(|link| link.target == "Twin")
            .unwrap();
        let missing = home
            .links
            .iter()
            .find(|link| link.target == "Missing")
            .unwrap();

        assert_eq!(twin.resolution, StagedLinkResolution::Ambiguous);
        assert_eq!(twin.candidate_object_ids.len(), 2);
        assert_eq!(missing.resolution, StagedLinkResolution::Missing);
        assert_eq!(home.fields["custom"], "  nested: value");
        for code in [
            "obsidian_target_ambiguous",
            "obsidian_target_missing",
            "obsidian_frontmatter_partial",
        ] {
            assert!(staged
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code));
        }
    }

    #[test]
    fn generic_markdown_does_not_apply_obsidian_semantics() {
        let source = TestDirectory::new();
        fs::write(
            source.path().join("Note.md"),
            "---\naliases: [Alias]\n---\n[[Other]]",
        )
        .unwrap();

        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let object = &staged.objects[0];

        assert!(object.aliases.is_empty());
        assert!(object.links.is_empty());
        assert_eq!(object.fields.len(), 1);
        assert!(object.fields["frontmatter"]
            .as_str()
            .unwrap()
            .contains("aliases"));
    }

    #[test]
    fn obsidian_frontmatter_preserves_a_lone_double_quote_without_panicking() {
        let parsed = parse_obsidian_frontmatter("malformed: \"");

        assert_eq!(parsed.fields["malformed"], "\"");
    }

    #[test]
    fn obsidian_link_scanner_ignores_code_spans_fences_and_escaped_embeds() {
        let marker = char::from(0x60);
        let inline_fence = marker.to_string().repeat(2);
        let inner_fence = marker.to_string().repeat(3);
        let block_fence = marker.to_string().repeat(4);
        let body = format!(
            "{inline_fence}[[Hidden inline]]{inline_fence} [[Visible]]\n\
             {block_fence}text\n\
             {inner_fence} [[Still fenced]]\n\
             {block_fence}\n\
             \\![[Escaped embed]] \\\\![[Visible embed]]"
        );

        let links = discover_obsidian_links(&body);

        assert_eq!(
            links
                .iter()
                .map(|link| link.target.as_str())
                .collect::<Vec<_>>(),
            vec!["Visible", "Visible embed"]
        );
        assert_eq!(links[1].kind, StagedLinkKind::Embed);
    }

    #[test]
    fn obsidian_vault_commit_preserves_markdown_and_attachment_after_clean_rebuild() {
        let vault = TestDirectory::new();
        fs::create_dir_all(vault.path().join("assets")).unwrap();
        let body = "---\naliases: [Start]\ntags: [lore]\n---\n# Home\n\n[[Target]]\n\n![[assets/map.png]]\n";
        fs::write(vault.path().join("Home.md"), body).unwrap();
        fs::write(vault.path().join("Target.md"), "# Target\n").unwrap();
        fs::write(
            vault.path().join("assets/map.png"),
            b"\x89PNG\r\n\x1a\nfixture",
        )
        .unwrap();
        let staged =
            analyze_obsidian_vault(vault.path(), GenericDocumentImportLimits::default()).unwrap();
        let project = TestDirectory::new();
        let store = ProjectStore::open_directory(project.path()).unwrap();
        let generation = store.content_generation().unwrap();
        let mut mappings = ImportMappingOverrides::default();
        mappings.global.entity_type = Some("note".into());
        mappings
            .global
            .relationship_mappings
            .insert("internal".into(), "references".into());
        let candidate = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "obsidian-session".into(),
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
                relationship_types: BTreeSet::from(["references".into()]),
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
                Some(vault.path()),
                true,
                "00000000-0000-4000-8000-000000000006",
            )
            .unwrap();

        assert_eq!(report.created.len(), 2);
        assert_eq!(report.assets.len(), 1);
        assert_eq!(report.relationships.len(), 1);
        store.flush_checkpoint("Obsidian import test").unwrap();
        drop(store);
        fs::remove_dir_all(project.path().join(".daena")).unwrap();
        let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
        let entity_id = report
            .created
            .iter()
            .find(|item| item.source_path == "Home.md")
            .unwrap()
            .entity_id
            .clone();
        let documents = rebuilt.list_documents(entity_id.clone()).unwrap();
        assert_eq!(documents[0].body, body);
        assert_eq!(
            rebuilt.list_relationships(entity_id.clone()).unwrap().len(),
            1
        );
        let source_fields = rebuilt.list_fields(entity_id.clone()).unwrap();
        let source_context = source_fields
            .iter()
            .find(|field| field.key.starts_with("externalImportSource."))
            .and_then(|field| field.value.get("sourceContext"))
            .unwrap();
        assert_eq!(source_context["aliases"], serde_json::json!(["Start"]));
        assert_eq!(source_context["tags"], serde_json::json!(["lore"]));
        let assets = rebuilt.list_assets(entity_id).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(
            rebuilt.asset_bytes(assets[0].id.clone()).unwrap(),
            b"\x89PNG\r\n\x1a\nfixture"
        );
    }

    #[test]
    fn obsidian_import_rejects_single_files() {
        let source = TestDirectory::new();
        let note = source.path().join("Note.md");
        fs::write(&note, "# Note").unwrap();

        assert!(
            analyze_obsidian_vault(&note, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("requires a vault folder")
        );
    }

    fn write_mediawiki_fixture(path: &Path) {
        fs::write(
            path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<mediawiki xmlns="http://www.mediawiki.org/xml/export-0.11/">
  <siteinfo>
    <sitename>Example Wiki</sitename><dbname>example</dbname>
    <base>https://example.test/wiki/Main_Page</base>
    <generator>MediaWiki 1.45</generator><case>first-letter</case>
    <namespaces><namespace key="0" case="first-letter"></namespace><namespace key="10" case="first-letter">Template</namespace></namespaces>
  </siteinfo>
  <page>
    <title>Gandalf</title><ns>0</ns><id>1</id>
    <revision><id>11</id><timestamp>2025-02-01T00:00:00Z</timestamp>
      <contributor><username>Archivist</username></contributor>
      <model>wikitext</model><format>text/x-wiki</format>
      <text xml:space="preserve"><![CDATA[{{Infobox person
| born = Before the First Age
| location = [[Middle Earth]]
}}
'''Gandalf''' travels through [[Middle_Earth|Middle Earth]].
[[Category:Characters]]
[[File:Gandalf.png|thumb]]
]]></text><sha1>new-hash</sha1>
    </revision>
    <revision><id>10</id><timestamp>2025-01-01T00:00:00Z</timestamp>
      <model>wikitext</model><format>text/x-wiki</format><text xml:space="preserve">Older revision</text>
    </revision>
  </page>
  <page><title>Middle Earth</title><ns>0</ns><id>2</id>
    <revision><id>20</id><timestamp>2025-02-02T00:00:00Z</timestamp>
      <model>wikitext</model><format>text/x-wiki</format><text xml:space="preserve">A world &amp; realm.</text>
    </revision>
  </page>
  <page><title>Mithrandir</title><ns>0</ns><id>3</id><redirect title="Gandalf" />
    <revision><id>30</id><timestamp>2025-02-03T00:00:00Z</timestamp>
      <model>wikitext</model><format>text/x-wiki</format><text xml:space="preserve">#REDIRECT [[Gandalf]]</text>
    </revision>
  </page>
</mediawiki>"#,
        )
        .unwrap();
    }

    #[test]
    fn mediawiki_analysis_streams_latest_pages_and_preserves_wikitext_metadata() {
        let source = TestDirectory::new();
        let source_path = source.path().join("wiki.xml");
        write_mediawiki_fixture(&source_path);

        let staged =
            analyze_mediawiki_xml(&source_path, GenericDocumentImportLimits::default()).unwrap();
        let gandalf = staged
            .objects
            .iter()
            .find(|object| object.title == "Gandalf")
            .unwrap();
        let middle_earth = staged
            .objects
            .iter()
            .find(|object| object.title == "Middle Earth")
            .unwrap();

        assert_eq!(staged.importer.id, MEDIAWIKI_IMPORTER_ID);
        assert_eq!(staged.source.kind, ImportSourceKind::WikiDump);
        assert_eq!(staged.objects.len(), 3);
        assert_eq!(gandalf.source_kind, "mediawiki_page");
        assert!(gandalf
            .body
            .as_ref()
            .unwrap()
            .body
            .contains("Before the First Age"));
        assert!(!gandalf
            .body
            .as_ref()
            .unwrap()
            .body
            .contains("Older revision"));
        assert_eq!(
            gandalf.raw_source_data["wikitext"],
            gandalf.body.as_ref().unwrap().body
        );
        assert_eq!(gandalf.raw_source_data["latest_revision"]["id"], "11");
        assert_eq!(gandalf.metadata["wiki_generator"], "MediaWiki 1.45");
        assert_eq!(gandalf.fields["source_format"], "text/x-wiki");
        assert_eq!(gandalf.fields["infobox.born"], "Before the First Age");
        assert_eq!(gandalf.tags, vec!["Characters"]);
        assert_eq!(middle_earth.body.as_ref().unwrap().body, "A world & realm.");
        assert!(gandalf.mapping_hints.iter().any(|hint| {
            hint.kind == MappingHintKind::Field
                && hint.source_key.as_deref() == Some("infobox.born")
        }));
        assert!(gandalf.links.iter().any(|link| {
            link.target == "Middle_Earth"
                && link.resolution == StagedLinkResolution::Resolved
                && link.resolved_object_id.as_deref() == Some(middle_earth.id.as_str())
        }));
        assert!(gandalf.links.iter().any(|link| {
            link.target == "File:Gandalf.png"
                && link.kind == StagedLinkKind::Embed
                && link.resolution == StagedLinkResolution::NotApplicable
        }));
        assert!(gandalf.aliases.contains(&"Mithrandir".into()));
        assert_eq!(staged.unsupported.len(), 1);
        assert_eq!(
            staged.unsupported[0].source_kind,
            "mediawiki_revision_history"
        );
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "mediawiki_revision_history_omitted"));
    }

    #[test]
    fn mediawiki_analysis_rejects_dtd_malformed_xml_and_page_limits() {
        let source = TestDirectory::new();
        let dtd = source.path().join("dtd.xml");
        fs::write(
            &dtd,
            r#"<?xml version="1.0"?><!DOCTYPE mediawiki [<!ENTITY x "expanded">]><mediawiki><page><title>&x;</title></page></mediawiki>"#,
        )
        .unwrap();
        assert!(
            analyze_mediawiki_xml(&dtd, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("DTD")
        );

        let malformed = source.path().join("malformed.xml");
        fs::write(&malformed, "<mediawiki><page></mediawiki>").unwrap();
        assert!(
            analyze_mediawiki_xml(&malformed, GenericDocumentImportLimits::default())
                .unwrap_err()
                .to_string()
                .contains("invalid MediaWiki XML")
        );

        let limited = source.path().join("limited.xml");
        write_mediawiki_fixture(&limited);
        let limits = GenericDocumentImportLimits {
            max_files: 2,
            ..Default::default()
        };
        assert!(analyze_mediawiki_xml(&limited, limits)
            .unwrap_err()
            .to_string()
            .contains("maximum page count"));

        let limits = GenericDocumentImportLimits {
            max_file_bytes: 8,
            ..Default::default()
        };
        assert!(analyze_mediawiki_xml(&limited, limits)
            .unwrap_err()
            .to_string()
            .contains("maximum page size"));
    }

    #[test]
    fn mediawiki_links_keep_ambiguous_and_missing_targets_reviewable() {
        let source = TestDirectory::new();
        let source_path = source.path().join("links.xml");
        fs::write(
            &source_path,
            r#"<mediawiki>
<page><title>Home</title><ns>0</ns><id>1</id><revision><id>1</id><text>[[Twin]] [[Missing]]</text></revision></page>
<page><title>Twin</title><ns>0</ns><id>2</id><revision><id>2</id><text>First</text></revision></page>
<page><title>Twin</title><ns>1</ns><id>3</id><revision><id>3</id><text>Second</text></revision></page>
</mediawiki>"#,
        )
        .unwrap();

        let staged =
            analyze_mediawiki_xml(&source_path, GenericDocumentImportLimits::default()).unwrap();
        let home = staged
            .objects
            .iter()
            .find(|object| object.title == "Home")
            .unwrap();
        let twin = home
            .links
            .iter()
            .find(|link| link.target == "Twin")
            .unwrap();
        let missing = home
            .links
            .iter()
            .find(|link| link.target == "Missing")
            .unwrap();

        assert_eq!(twin.resolution, StagedLinkResolution::Ambiguous);
        assert_eq!(twin.candidate_object_ids.len(), 2);
        assert_eq!(missing.resolution, StagedLinkResolution::Missing);
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "mediawiki_target_ambiguous"));
        assert!(staged
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "mediawiki_target_missing"));
    }

    #[test]
    fn mediawiki_streaming_progress_can_cancel_before_the_dump_completes() {
        let source = TestDirectory::new();
        let source_path = source.path().join("large.xml");
        let mut xml = String::from("<mediawiki>");
        for page in 0..500 {
            use std::fmt::Write as _;
            write!(
                xml,
                "<page><title>Page {page}</title><ns>0</ns><id>{page}</id><revision><id>{page}</id><text>Body {page}</text></revision></page>"
            )
            .unwrap();
        }
        xml.push_str("</mediawiki>");
        fs::write(&source_path, xml).unwrap();
        let mut callbacks = 0;

        let error = analyze_mediawiki_xml_with_progress(
            &source_path,
            GenericDocumentImportLimits::default(),
            |progress| {
                callbacks += 1;
                if progress.processed_entries >= 25 {
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
        assert!(callbacks > 1);
        assert!(callbacks < 500);
    }

    #[test]
    fn mediawiki_commit_preserves_latest_wikitext_after_clean_rebuild() {
        let source = TestDirectory::new();
        let source_path = source.path().join("wiki.xml");
        write_mediawiki_fixture(&source_path);
        let staged =
            analyze_mediawiki_xml(&source_path, GenericDocumentImportLimits::default()).unwrap();
        let gandalf_staged_id = staged
            .objects
            .iter()
            .find(|object| object.title == "Gandalf")
            .unwrap()
            .id
            .clone();
        let expected = staged
            .objects
            .iter()
            .find(|object| object.id == gandalf_staged_id)
            .unwrap()
            .body
            .as_ref()
            .unwrap()
            .body
            .clone();
        let project = TestDirectory::new();
        let store = ProjectStore::open_directory(project.path()).unwrap();
        let generation = store.content_generation().unwrap();
        let mut mappings = ImportMappingOverrides::default();
        mappings.global.entity_type = Some("note".into());
        mappings
            .global
            .relationship_mappings
            .insert("internal".into(), "references".into());
        mappings
            .global
            .field_mappings
            .insert("infobox.born".into(), "wiki:born".into());
        let candidate = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: "mediawiki-session".into(),
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
                fields: BTreeMap::from([(
                    "wiki:born".into(),
                    ImportFieldTarget {
                        namespace: "wiki".into(),
                        key: "born".into(),
                        entity_types: BTreeSet::from(["note".into()]),
                    },
                )]),
                relationship_types: BTreeSet::from(["references".into()]),
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
                "00000000-0000-4000-8000-000000000007",
            )
            .unwrap();

        let gandalf_entity_id = report
            .created
            .iter()
            .find(|created| created.staged_object_id == gandalf_staged_id)
            .unwrap()
            .entity_id
            .clone();
        assert_eq!(report.relationships.len(), 2);
        store.flush_checkpoint("MediaWiki import test").unwrap();
        drop(store);
        fs::remove_dir_all(project.path().join(".daena")).unwrap();
        let rebuilt = ProjectStore::open_directory(project.path()).unwrap();
        let documents = rebuilt.list_documents(gandalf_entity_id.clone()).unwrap();
        assert_eq!(documents[0].body, expected);
        assert_eq!(
            rebuilt
                .list_relationships(gandalf_entity_id.clone())
                .unwrap()
                .into_iter()
                .filter(|relationship| relationship.source_id == gandalf_entity_id)
                .count(),
            1
        );
        let source_fields = rebuilt.list_fields(gandalf_entity_id).unwrap();
        let source_context = source_fields
            .iter()
            .find(|field| field.key.starts_with("externalImportSource."))
            .and_then(|field| field.value.get("sourceContext"))
            .unwrap();
        assert_eq!(source_context["tags"], serde_json::json!(["Characters"]));
        assert_eq!(
            source_context["unmappedFields"]["templates"][0],
            "Infobox person"
        );
        assert_eq!(
            source_fields
                .iter()
                .find(|field| field.namespace == "wiki" && field.key == "born")
                .unwrap()
                .value,
            "Before the First Age"
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
