use crate::CoreError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub const STAGED_IMPORT_SCHEMA_VERSION: u32 = 1;
pub const IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION: u32 = 1;
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
        discovered_entries: if metadata.is_file() { 1 } else { 0 },
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
    analyzer
        .import
        .objects
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
        let body = match String::from_utf8(bytes) {
            Ok(body) => body,
            Err(_) => {
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
            }
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
            fields: BTreeMap::new(),
            metadata: BTreeMap::new(),
            raw_source_data: BTreeMap::new(),
            links: Vec::new(),
            mapping_hints: Vec::new(),
            diagnostics: Vec::new(),
        });
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
        let mut limits = GenericDocumentImportLimits::default();
        limits.max_total_bytes = 7;

        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum total size of 7 bytes"));

        let mut limits = GenericDocumentImportLimits::default();
        limits.max_files = 1;
        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum file count of 1"));

        let mut limits = GenericDocumentImportLimits::default();
        limits.max_file_bytes = 3;
        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum size of 3 bytes"));

        fs::create_dir(source.path().join("nested")).unwrap();
        fs::write(source.path().join("nested/deep.md"), "deep").unwrap();
        let mut limits = GenericDocumentImportLimits::default();
        limits.max_depth = 0;
        let error = analyze_generic_documents(source.path(), limits).unwrap_err();
        assert!(error
            .to_string()
            .contains("exceeds the maximum folder depth of 0"));

        let mut limits = GenericDocumentImportLimits::default();
        limits.max_entries = 1;
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
