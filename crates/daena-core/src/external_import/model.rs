// External import plan model types.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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
    pub categories: BTreeMap<String, ImportMappingDecision>,
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
    #[serde(default)]
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub multiple: bool,
    #[serde(default)]
    pub options: BTreeSet<String>,
    #[serde(default)]
    pub one_of: Vec<ImportFieldVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportFieldVariant {
    pub field_type: String,
    #[serde(default)]
    pub options: BTreeSet<String>,
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
    pub source_key: String,
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
    #[serde(default)]
    pub unsupported: Vec<UnsupportedSourceData>,
    #[serde(default)]
    pub diagnostics: Vec<ImportDiagnostic>,
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
pub struct ImportedFieldReport {
    pub staged_object_id: String,
    pub source_path: String,
    pub entity_id: String,
    pub source_key: String,
    pub namespace: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportDecisionReport {
    pub staged_object_id: String,
    pub source_path: String,
    pub decision: String,
    #[serde(default)]
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportMissingReferenceReport {
    pub staged_object_id: String,
    pub source_path: String,
    pub target: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    #[serde(default)]
    pub fields: Vec<ImportedFieldReport>,
    #[serde(default)]
    pub decisions: Vec<ImportDecisionReport>,
    #[serde(default)]
    pub unsupported: Vec<UnsupportedSourceData>,
    #[serde(default)]
    pub missing_references: Vec<ImportMissingReferenceReport>,
    #[serde(default)]
    pub diagnostics: Vec<ImportDiagnostic>,
    pub skipped_source_paths: Vec<String>,
    pub warnings: Vec<ImportValidationIssue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportValidationBuild {
    pub candidate: ImportCandidatePlan,
    pub staged_objects: Vec<StagedObject>,
    pub staged_assets: Vec<StagedAsset>,
    pub staged_unsupported: Vec<UnsupportedSourceData>,
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
