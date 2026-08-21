//! Trusted-shell source selection and bounded external-import analysis sessions.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use daena_core::{
    analyze_generic_documents_with_progress, analyze_obsidian_vault_with_progress,
    build_import_candidate_plan, validate_import_candidate_plan, CoreError,
    ExternalImportCommitReport, GenericDocumentImportLimits, ImportAnalysisProgress,
    ImportAnalysisSummary, ImportCandidatePlan, ImportCandidatePlanBuild, ImportDiagnostic,
    ImportFieldTarget, ImportMappingCatalog, ImportMappingOverrides, ImportObjectDecision,
    ImportSource, ImportValidationBuild, ImportValidationIssue, ImportValidationSeverity,
    ImporterIdentity, StagedAsset, StagedImport, StagedObject, UnsupportedSourceData,
    ValidatedImportPlan, EXTERNAL_IMPORT_ANALYSIS_CANCELLED, GENERIC_DOCUMENT_IMPORTER_ID,
    GENERIC_DOCUMENT_IMPORTER_VERSION, OBSIDIAN_IMPORTER_ID, OBSIDIAN_IMPORTER_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use super::{
    current_info, effective_module_manifests, with_core, with_read_project, SharedCore,
    SharedExternalImports, SharedPluginHost,
};

pub const EXTERNAL_IMPORT_PROGRESS_EVENT: &str = "external-import-progress";

const SOURCE_HANDLE_TTL: Duration = Duration::from_mins(10);
const ANALYSIS_SESSION_TTL: Duration = Duration::from_mins(30);
const MAX_SOURCE_HANDLES: usize = 16;
const MAX_ANALYSIS_SESSIONS: usize = 8;
const MAX_PAGE_ITEMS: usize = 200;
const SPILL_SOURCE_BYTES: u64 = 8 * 1024 * 1024;
const SPILL_ITEM_COUNT: usize = 1_000;

type CandidateMaterial = (Vec<StagedObject>, Vec<StagedAsset>, Vec<ImportDiagnostic>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImporterDescriptor {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub source_kinds: Vec<String>,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportSourceHandle {
    pub source_handle: String,
    pub source_kind: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportResultMetadata {
    pub schema_version: u32,
    pub importer: ImporterIdentity,
    pub source: ImportSource,
    pub summary: ImportAnalysisSummary,
    pub total_items: usize,
    pub spilled_to_local_storage: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportAnalysisStatus {
    pub session_id: String,
    pub importer_id: String,
    pub state: String,
    pub stage: String,
    pub processed_entries: usize,
    pub staged_object_count: usize,
    pub unsupported_count: usize,
    pub source_bytes: u64,
    pub sequence: u64,
    pub current_source_path: Option<String>,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub captured_content_generation: i64,
    pub current_content_generation: Option<i64>,
    pub result: Option<ExternalImportResultMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ExternalImportPageItem {
    Object(StagedObject),
    Asset(StagedAsset),
    Unsupported(UnsupportedSourceData),
    Diagnostic(ImportDiagnostic),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportPage {
    pub session_id: String,
    pub offset: usize,
    pub limit: usize,
    pub total_items: usize,
    pub items: Vec<ExternalImportPageItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalImportLimitsInput {
    pub max_entries: usize,
    pub max_files: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_depth: usize,
    pub max_diagnostics: usize,
}

impl From<ExternalImportLimitsInput> for GenericDocumentImportLimits {
    fn from(input: ExternalImportLimitsInput) -> Self {
        Self {
            max_entries: input.max_entries,
            max_files: input.max_files,
            max_file_bytes: input.max_file_bytes,
            max_total_bytes: input.max_total_bytes,
            max_depth: input.max_depth,
            max_diagnostics: input.max_diagnostics,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalImportBeginInput {
    pub source_handle: String,
    pub importer_id: String,
    #[serde(default)]
    pub limits: Option<ExternalImportLimitsInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalImportCandidatePlanInput {
    pub session_id: String,
    pub manifest_fingerprint: String,
    #[serde(default)]
    pub mappings: ImportMappingOverrides,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalImportValidateInput {
    pub session_id: String,
    #[serde(default)]
    pub mappings: ImportMappingOverrides,
    #[serde(default)]
    pub decisions: BTreeMap<String, ImportObjectDecision>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalImportValidationSummary {
    pub validation_id: Option<String>,
    pub plan_id: Option<String>,
    pub create_count: usize,
    pub skip_count: usize,
    pub map_count: usize,
    pub asset_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
    pub issues: Vec<ImportValidationIssue>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalImportCommitInput {
    pub session_id: String,
    pub validation_id: String,
    pub request_id: String,
    #[serde(default)]
    pub acknowledge_warnings: bool,
}

#[derive(Debug)]
struct SourceSelection {
    project_id: String,
    source_kind: String,
    path: PathBuf,
    expires_at: Instant,
}

enum ImportResultStorage {
    Memory {
        metadata: ExternalImportResultMetadata,
        items: Vec<ExternalImportPageItem>,
    },
    Spill {
        metadata: ExternalImportResultMetadata,
        path: PathBuf,
    },
}

impl ImportResultStorage {
    fn metadata(&self) -> &ExternalImportResultMetadata {
        match self {
            Self::Memory { metadata, .. } | Self::Spill { metadata, .. } => metadata,
        }
    }

    fn page(&self, offset: usize, limit: usize) -> Result<Vec<ExternalImportPageItem>, String> {
        let total = self.metadata().total_items;
        if limit == 0 || limit > MAX_PAGE_ITEMS {
            return Err(format!(
                "external_import.invalid_page: limit must be between 1 and {MAX_PAGE_ITEMS}"
            ));
        }
        if offset > total {
            return Err("external_import.invalid_page: offset exceeds the result size".into());
        }
        match self {
            Self::Memory { items, .. } => {
                let end = offset.saturating_add(limit).min(items.len());
                Ok(items[offset..end].to_vec())
            }
            Self::Spill { path, .. } => read_spill_page(path, offset, limit),
        }
    }

    fn cleanup(&self) -> Result<(), String> {
        if let Self::Spill { path, .. } = self {
            remove_spill_file(path)?;
        }
        Ok(())
    }

    fn candidate_material(&self) -> Result<CandidateMaterial, String> {
        match self {
            Self::Memory { items, .. } => Ok(candidate_material_from_items(items.iter())),
            Self::Spill { path, .. } => read_spill_candidate_material(path),
        }
    }
}

struct ExternalImportJob {
    project_id: String,
    source_root: PathBuf,
    expires_at: Instant,
    cancel: Arc<AtomicBool>,
    status: ExternalImportAnalysisStatus,
    result: Option<ImportResultStorage>,
    validated: Option<ValidatedImportPlan>,
}

#[derive(Default)]
pub struct ExternalImportJobManager {
    sources: BTreeMap<String, SourceSelection>,
    jobs: BTreeMap<String, ExternalImportJob>,
}

impl ExternalImportJobManager {
    fn reap(&mut self) -> Result<(), String> {
        let now = Instant::now();
        self.sources.retain(|_, source| source.expires_at > now);
        let expired = self
            .jobs
            .iter()
            .filter(|(_, job)| job.expires_at <= now)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in expired {
            if let Some(job) = self.jobs.get(&id) {
                job.cancel.store(true, Ordering::Relaxed);
                if let Some(result) = &job.result {
                    result.cleanup()?;
                }
            }
            self.jobs.remove(&id);
        }
        Ok(())
    }

    pub fn cancel_all(&mut self) -> Result<(), String> {
        for job in self.jobs.values() {
            job.cancel.store(true, Ordering::Relaxed);
            if let Some(result) = &job.result {
                result.cleanup()?;
            }
        }
        self.sources.clear();
        self.jobs.clear();
        Ok(())
    }

    fn register_source(
        &mut self,
        project_id: String,
        source_kind: String,
        path: PathBuf,
    ) -> Result<ExternalImportSourceHandle, String> {
        self.reap()?;
        if self.sources.len() >= MAX_SOURCE_HANDLES {
            return Err("external_import.resource_limit: too many pending source handles".into());
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("external_import.source_unavailable: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("external_import.invalid_source: source cannot be a symlink".into());
        }
        let valid_kind = match source_kind.as_str() {
            "file" => metadata.is_file(),
            "folder" => metadata.is_dir(),
            _ => false,
        };
        if !valid_kind {
            return Err(
                "external_import.invalid_source: source kind does not match selection".into(),
            );
        }
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Selected source")
            .to_owned();
        let source_handle = Uuid::new_v4().to_string();
        self.sources.insert(
            source_handle.clone(),
            SourceSelection {
                project_id,
                source_kind: source_kind.clone(),
                path,
                expires_at: Instant::now() + SOURCE_HANDLE_TTL,
            },
        );
        Ok(ExternalImportSourceHandle {
            source_handle,
            source_kind,
            display_name,
        })
    }

    fn ensure_can_begin(&mut self, project_id: &str) -> Result<(), String> {
        self.reap()?;
        if self.jobs.len() >= MAX_ANALYSIS_SESSIONS {
            return Err("external_import.resource_limit: too many analysis sessions".into());
        }
        if self.jobs.values().any(|job| {
            job.project_id == project_id
                && matches!(job.status.state.as_str(), "queued" | "analyzing")
        }) {
            return Err("external_import.resource_limit: one analysis may run per project".into());
        }
        Ok(())
    }

    fn take_source(
        &mut self,
        project_id: &str,
        source_handle: &str,
    ) -> Result<SourceSelection, String> {
        let source = self.sources.get(source_handle).ok_or_else(|| {
            "external_import.source_not_found: source handle expired or was not found".to_string()
        })?;
        if source.project_id != project_id {
            return Err(
                "external_import.source_not_found: source handle is not valid for this project"
                    .into(),
            );
        }
        self.sources
            .remove(source_handle)
            .ok_or_else(|| "external_import.source_not_found: source handle was not found".into())
    }
}

#[tauri::command]
pub fn project_external_importers() -> Vec<ExternalImporterDescriptor> {
    vec![
        ExternalImporterDescriptor {
            id: GENERIC_DOCUMENT_IMPORTER_ID.into(),
            version: GENERIC_DOCUMENT_IMPORTER_VERSION.into(),
            name: "Generic documents".into(),
            description: "Markdown, HTML, DOCX, plain-text, ZIP, and recursive folder analysis"
                .into(),
            source_kinds: vec!["file".into(), "folder".into()],
            extensions: vec![
                "md".into(),
                "markdown".into(),
                "html".into(),
                "htm".into(),
                "docx".into(),
                "txt".into(),
                "zip".into(),
            ],
        },
        ExternalImporterDescriptor {
            id: OBSIDIAN_IMPORTER_ID.into(),
            version: OBSIDIAN_IMPORTER_VERSION.into(),
            name: "Obsidian vault".into(),
            description: "Markdown vaults with YAML, wikilinks, embeds, and attachments".into(),
            source_kinds: vec!["folder".into()],
            extensions: vec!["md".into(), "markdown".into()],
        },
    ]
}

#[tauri::command]
pub async fn project_external_import_select_source(
    app: AppHandle,
    core: tauri::State<'_, SharedCore>,
    imports: tauri::State<'_, SharedExternalImports>,
    source_kind: String,
) -> Result<Option<ExternalImportSourceHandle>, String> {
    let project_id = current_project_id(core.inner())?;
    let selected = match source_kind.as_str() {
        "file" => app
            .dialog()
            .file()
            .add_filter(
                "Documents and archives",
                &["md", "markdown", "html", "htm", "docx", "txt", "zip"],
            )
            .blocking_pick_file(),
        "folder" => app.dialog().file().blocking_pick_folder(),
        _ => {
            return Err("external_import.invalid_source: source kind must be file or folder".into())
        }
    };
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|error| format!("external_import.invalid_source: {error}"))?;
    let mut manager = imports
        .lock()
        .map_err(|_| "external import state is unavailable".to_string())?;
    if current_project_id(core.inner())? != project_id {
        return Err(
            "external_import.project_changed: project changed while selecting the source".into(),
        );
    }
    manager
        .register_source(project_id, source_kind, path)
        .map(Some)
}

#[tauri::command]
pub async fn project_external_import_analyze_begin(
    app: AppHandle,
    core: tauri::State<'_, SharedCore>,
    imports: tauri::State<'_, SharedExternalImports>,
    input: ExternalImportBeginInput,
) -> Result<ExternalImportAnalysisStatus, String> {
    if !matches!(
        input.importer_id.as_str(),
        GENERIC_DOCUMENT_IMPORTER_ID | OBSIDIAN_IMPORTER_ID
    ) {
        return Err("external_import.importer_not_found: importer is not available".into());
    }
    let project_id = current_project_id(core.inner())?;
    let captured_content_generation =
        with_read_project(core.clone(), daena_core::ProjectStore::content_generation).await?;
    let limits = input.limits.map(Into::into).unwrap_or_default();
    let (session_id, source, cancel, status) = {
        let mut manager = imports
            .lock()
            .map_err(|_| "external import state is unavailable".to_string())?;
        if current_project_id(core.inner())? != project_id {
            return Err(
                "external_import.project_changed: project changed before analysis started".into(),
            );
        }
        manager.ensure_can_begin(&project_id)?;
        let source = manager.take_source(&project_id, &input.source_handle)?;
        if input.importer_id == OBSIDIAN_IMPORTER_ID && source.source_kind != "folder" {
            return Err(
                "external_import.invalid_source: Obsidian import requires a vault folder".into(),
            );
        }
        let session_id = Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));
        let status = ExternalImportAnalysisStatus {
            session_id: session_id.clone(),
            importer_id: input.importer_id.clone(),
            state: "queued".into(),
            stage: "queued".into(),
            processed_entries: 0,
            staged_object_count: 0,
            unsupported_count: 0,
            source_bytes: 0,
            sequence: 0,
            current_source_path: None,
            error: None,
            error_code: None,
            captured_content_generation,
            current_content_generation: Some(captured_content_generation),
            result: None,
        };
        manager.jobs.insert(
            session_id.clone(),
            ExternalImportJob {
                project_id: project_id.clone(),
                source_root: source.path.clone(),
                expires_at: Instant::now() + ANALYSIS_SESSION_TTL,
                cancel: cancel.clone(),
                status: status.clone(),
                result: None,
                validated: None,
            },
        );
        (session_id, source, cancel, status)
    };
    spawn_analysis(
        app,
        imports.inner().clone(),
        session_id,
        project_id,
        source,
        input.importer_id,
        limits,
        cancel,
    );
    Ok(status)
}

#[tauri::command]
pub async fn project_external_import_analysis_status(
    core: tauri::State<'_, SharedCore>,
    imports: tauri::State<'_, SharedExternalImports>,
    session_id: String,
) -> Result<ExternalImportAnalysisStatus, String> {
    let project_id = current_project_id(core.inner())?;
    let current_content_generation =
        with_read_project(core, daena_core::ProjectStore::content_generation).await?;
    let mut manager = imports
        .lock()
        .map_err(|_| "external import state is unavailable".to_string())?;
    manager.reap()?;
    let job = project_job(&manager, &project_id, &session_id)?;
    let mut status = job.status.clone();
    status.current_content_generation = Some(current_content_generation);
    Ok(status)
}

#[tauri::command]
pub fn project_external_import_analysis_cancel(
    core: tauri::State<'_, SharedCore>,
    imports: tauri::State<'_, SharedExternalImports>,
    session_id: String,
) -> Result<ExternalImportAnalysisStatus, String> {
    let project_id = current_project_id(core.inner())?;
    let mut manager = imports
        .lock()
        .map_err(|_| "external import state is unavailable".to_string())?;
    manager.reap()?;
    let job = manager
        .jobs
        .get_mut(&session_id)
        .filter(|job| job.project_id == project_id)
        .ok_or_else(|| {
            "external_import.session_not_found: analysis session was not found".to_string()
        })?;
    job.cancel.store(true, Ordering::Relaxed);
    if let Some(result) = &job.result {
        result.cleanup()?;
    }
    job.result = None;
    job.validated = None;
    job.status.state = "cancelled".into();
    job.status.stage = "cancelled".into();
    job.status.error = None;
    job.status.error_code = Some("external_import.cancelled".into());
    job.status.result = None;
    job.expires_at = Instant::now() + ANALYSIS_SESSION_TTL;
    Ok(job.status.clone())
}

#[tauri::command]
pub async fn project_external_import_analysis_page(
    core: tauri::State<'_, SharedCore>,
    imports: tauri::State<'_, SharedExternalImports>,
    session_id: String,
    offset: usize,
    limit: usize,
) -> Result<ExternalImportPage, String> {
    let project_id = current_project_id(core.inner())?;
    let imports = imports.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut manager = imports
            .lock()
            .map_err(|_| "external import state is unavailable".to_string())?;
        manager.reap()?;
        let job = project_job(&manager, &project_id, &session_id)?;
        if job.status.state != "ready" {
            return Err("external_import.not_ready: analysis result is not ready".into());
        }
        let result = job
            .result
            .as_ref()
            .ok_or_else(|| "external_import.not_ready: analysis result is missing".to_string())?;
        Ok(ExternalImportPage {
            session_id,
            offset,
            limit,
            total_items: result.metadata().total_items,
            items: result.page(offset, limit)?,
        })
    })
    .await
    .map_err(|error| format!("external import paging worker failed: {error}"))?
}

#[tauri::command]
pub async fn project_external_import_candidate_plan(
    core: tauri::State<'_, SharedCore>,
    imports: tauri::State<'_, SharedExternalImports>,
    input: ExternalImportCandidatePlanInput,
) -> Result<ImportCandidatePlan, String> {
    let project_id = current_project_id(core.inner())?;
    let current_content_generation =
        with_read_project(core, daena_core::ProjectStore::content_generation).await?;
    let imports = imports.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut manager = imports
            .lock()
            .map_err(|_| "external import state is unavailable".to_string())?;
        manager.reap()?;
        let job = project_job(&manager, &project_id, &input.session_id)?;
        if job.status.state != "ready" {
            return Err("external_import.not_ready: analysis result is not ready".into());
        }
        let result = job
            .result
            .as_ref()
            .ok_or_else(|| "external_import.not_ready: analysis result is missing".to_string())?;
        let metadata = result.metadata().clone();
        let (objects, _, diagnostics) = result.candidate_material()?;
        build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: input.session_id,
                importer: metadata.importer,
                source: metadata.source,
                captured_content_generation: job.status.captured_content_generation,
                current_content_generation,
                manifest_fingerprint: input.manifest_fingerprint,
                objects,
                unsupported_count: metadata.summary.unsupported_count,
                diagnostics,
            },
            &input.mappings,
        )
        .map_err(|error| format!("external_import.invalid_candidate_plan: {error}"))
    })
    .await
    .map_err(|error| format!("external import planning worker failed: {error}"))?
}

#[tauri::command]
pub async fn project_external_import_validate(
    core: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    imports: tauri::State<'_, SharedExternalImports>,
    input: ExternalImportValidateInput,
) -> Result<ExternalImportValidationSummary, String> {
    let project_id = current_project_id(core.inner())?;
    let validation_session_id = input.session_id;
    let mappings = input.mappings;
    let decisions = input.decisions;
    let imports_shared = imports.inner().clone();
    let material_project_id = project_id.clone();
    let material_session_id = validation_session_id.clone();
    let (captured_generation, metadata, objects, assets, diagnostics) =
        tauri::async_runtime::spawn_blocking(move || {
            let mut manager = imports_shared
                .lock()
                .map_err(|_| "external import state is unavailable".to_string())?;
            manager.reap()?;
            let job = project_job(&manager, &material_project_id, &material_session_id)?;
            if job.status.state != "ready" {
                return Err(String::from(
                    "external_import.not_ready: analysis result is not ready",
                ));
            }
            let result = job.result.as_ref().ok_or_else(|| {
                "external_import.not_ready: analysis result is missing".to_string()
            })?;
            let (objects, assets, diagnostics) = result.candidate_material()?;
            Ok((
                job.status.captured_content_generation,
                result.metadata().clone(),
                objects,
                assets,
                diagnostics,
            ))
        })
        .await
        .map_err(|error| format!("external import validation worker failed: {error}"))??;
    let plugins = plugins.inner().clone();
    let candidate_session_id = validation_session_id.clone();
    let outcome = with_read_project(core, move |project| {
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let catalog = import_mapping_catalog(project, &host)?;
        let current_generation = project.content_generation()?;
        let candidate = build_import_candidate_plan(
            ImportCandidatePlanBuild {
                session_id: candidate_session_id,
                importer: metadata.importer.clone(),
                source: metadata.source.clone(),
                captured_content_generation: captured_generation,
                current_content_generation: current_generation,
                manifest_fingerprint: catalog.fingerprint.clone(),
                objects: objects.clone(),
                unsupported_count: metadata.summary.unsupported_count,
                diagnostics,
            },
            &mappings,
        )?;
        let existing_ids = decisions
            .values()
            .filter_map(|decision| match decision {
                ImportObjectDecision::MapToExisting { entity_id, .. } => Some(entity_id.clone()),
                ImportObjectDecision::Create | ImportObjectDecision::Skip => None,
            })
            .collect::<BTreeSet<_>>();
        let existing_targets = project.external_import_existing_targets(&existing_ids)?;
        let duplicate_sources = objects
            .iter()
            .map(|object| (object.id.clone(), object.source_id.clone()))
            .collect::<Vec<_>>();
        let duplicate_targets =
            project.external_import_duplicate_targets(&metadata.importer.id, &duplicate_sources)?;
        validate_import_candidate_plan(ImportValidationBuild {
            candidate,
            staged_objects: objects,
            staged_assets: assets,
            catalog,
            decisions,
            existing_targets,
            duplicate_targets,
        })
    })
    .await?;
    let plan = outcome.plan;
    let (create_count, skip_count, map_count) = plan.as_ref().map_or((0, 0, 0), |plan| {
        plan.objects
            .iter()
            .fold((0, 0, 0), |counts, object| match &object.decision {
                ImportObjectDecision::Create => (counts.0 + 1, counts.1, counts.2),
                ImportObjectDecision::Skip => (counts.0, counts.1 + 1, counts.2),
                ImportObjectDecision::MapToExisting { .. } => (counts.0, counts.1, counts.2 + 1),
            })
    });
    let asset_count = plan.as_ref().map_or(0, |plan| plan.assets.len());
    let warning_count = outcome
        .issues
        .iter()
        .filter(|issue| issue.severity == ImportValidationSeverity::Warning)
        .count();
    let error_count = outcome.issues.len().saturating_sub(warning_count);
    let validation_id = plan.as_ref().map(|plan| plan.plan_id.clone());
    {
        let mut manager = imports
            .lock()
            .map_err(|_| "external import state is unavailable".to_string())?;
        let job = manager
            .jobs
            .get_mut(&validation_session_id)
            .filter(|job| job.project_id == project_id)
            .ok_or_else(|| {
                "external_import.session_not_found: analysis session was not found".to_string()
            })?;
        job.validated.clone_from(&plan);
    }
    Ok(ExternalImportValidationSummary {
        validation_id: validation_id.clone(),
        plan_id: validation_id,
        create_count,
        skip_count,
        map_count,
        asset_count,
        warning_count,
        error_count,
        issues: outcome.issues,
    })
}

#[tauri::command]
pub async fn project_external_import_commit(
    core: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    imports: tauri::State<'_, SharedExternalImports>,
    input: ExternalImportCommitInput,
) -> Result<ExternalImportCommitReport, String> {
    let project_id = current_project_id(core.inner())?;
    let (plan, source_root) = {
        let mut manager = imports
            .lock()
            .map_err(|_| "external import state is unavailable".to_string())?;
        manager.reap()?;
        let job = project_job(&manager, &project_id, &input.session_id)?;
        let plan = job.validated.as_ref().ok_or_else(|| {
            "external_import.validation_required: validate the import plan before commit"
                .to_string()
        })?;
        if plan.plan_id != input.validation_id {
            return Err("external_import.validation_stale: validation ID does not match".into());
        }
        (plan.clone(), job.source_root.clone())
    };
    let plugins = plugins.inner().clone();
    let acknowledge_warnings = input.acknowledge_warnings;
    let request_id = input.request_id.clone();
    let report = with_core(core, move |core| {
        let project = core.project(super::trusted_shell())?;
        let host = plugins
            .lock()
            .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?;
        let catalog = import_mapping_catalog(project, &host)?;
        if catalog.fingerprint != plan.manifest_fingerprint {
            return Err(CoreError::Conflict(
                "enabled schema contributions changed after validation".into(),
            ));
        }
        project.commit_external_import(&plan, Some(&source_root), acknowledge_warnings, &request_id)
    })
    .await?;
    let mut manager = imports
        .lock()
        .map_err(|_| "external import state is unavailable".to_string())?;
    if let Some(job) = manager.jobs.get(&input.session_id) {
        let cleanup_succeeded = job
            .result
            .as_ref()
            .is_none_or(|result| result.cleanup().is_ok());
        if cleanup_succeeded {
            manager.jobs.remove(&input.session_id);
        }
    }
    Ok(report)
}

fn import_mapping_catalog(
    project: &daena_core::ProjectStore,
    host: &daena_plugin_host::PluginHost,
) -> Result<ImportMappingCatalog, CoreError> {
    let mut manifests = effective_module_manifests(project, host)?
        .into_iter()
        .filter_map(|(manifest, enabled)| enabled.then_some(manifest))
        .collect::<Vec<_>>();
    manifests.sort_by(|left, right| left.id.cmp(&right.id));
    let bytes = serde_json::to_vec(&manifests)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
    let fingerprint = format!("sha256:{:x}", Sha256::digest(bytes));
    let mut entity_types = BTreeSet::new();
    let mut fields = BTreeMap::new();
    let mut relationship_types = BTreeSet::new();
    for manifest in manifests {
        for schema in manifest.schemas {
            entity_types.extend(schema.entity_types);
            for field in schema.fields {
                let id = format!("{}:{}", schema.namespace, field.key);
                if let Some(relationship_type) = &field.relationship_type {
                    relationship_types.insert(relationship_type.clone());
                }
                fields.insert(
                    id,
                    ImportFieldTarget {
                        namespace: schema.namespace.clone(),
                        key: field.key,
                        entity_types: field.entity_types.unwrap_or_default().into_iter().collect(),
                    },
                );
            }
        }
        entity_types.extend(
            manifest
                .templates
                .into_iter()
                .map(|template| template.entity_type),
        );
    }
    Ok(ImportMappingCatalog {
        fingerprint,
        entity_types,
        fields,
        relationship_types,
    })
}

pub fn cancel_external_imports(imports: &SharedExternalImports) -> Result<(), String> {
    imports
        .lock()
        .map_err(|_| "external import state is unavailable".to_string())?
        .cancel_all()
}

fn current_project_id(core: &SharedCore) -> Result<String, String> {
    current_info(core)?
        .map(|info| info.root)
        .ok_or_else(|| "open a directory project before importing external content".into())
}

fn project_job<'a>(
    manager: &'a ExternalImportJobManager,
    project_id: &str,
    session_id: &str,
) -> Result<&'a ExternalImportJob, String> {
    manager
        .jobs
        .get(session_id)
        .filter(|job| job.project_id == project_id)
        .ok_or_else(|| "external_import.session_not_found: analysis session was not found".into())
}

fn spawn_analysis(
    app: AppHandle,
    imports: SharedExternalImports,
    session_id: String,
    project_id: String,
    source: SourceSelection,
    importer_id: String,
    limits: GenericDocumentImportLimits,
    cancel: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn_blocking(move || {
        update_progress(
            &app,
            &imports,
            &session_id,
            ImportAnalysisProgress::default(),
        );
        let staged = if importer_id == OBSIDIAN_IMPORTER_ID {
            let progress_app = app.clone();
            let progress_imports = imports.clone();
            let progress_session_id = session_id.clone();
            let progress_cancel = cancel.clone();
            analyze_obsidian_vault_with_progress(&source.path, limits, move |progress| {
                if progress_cancel.load(Ordering::Relaxed) {
                    return Err(CoreError::Conflict(
                        EXTERNAL_IMPORT_ANALYSIS_CANCELLED.into(),
                    ));
                }
                update_progress(
                    &progress_app,
                    &progress_imports,
                    &progress_session_id,
                    progress,
                );
                Ok(())
            })
        } else {
            let progress_app = app.clone();
            let progress_imports = imports.clone();
            let progress_session_id = session_id.clone();
            let progress_cancel = cancel.clone();
            analyze_generic_documents_with_progress(&source.path, limits, move |progress| {
                if progress_cancel.load(Ordering::Relaxed) {
                    return Err(CoreError::Conflict(
                        EXTERNAL_IMPORT_ANALYSIS_CANCELLED.into(),
                    ));
                }
                update_progress(
                    &progress_app,
                    &progress_imports,
                    &progress_session_id,
                    progress,
                );
                Ok(())
            })
        };
        if cancel.load(Ordering::Relaxed) {
            finish_cancelled(&app, &imports, &session_id);
            return;
        }
        let staged = match staged {
            Ok(staged) => staged,
            Err(error) if error.to_string() == EXTERNAL_IMPORT_ANALYSIS_CANCELLED => {
                finish_cancelled(&app, &imports, &session_id);
                return;
            }
            Err(error) => {
                finish_failed(&app, &imports, &session_id, error.to_string());
                return;
            }
        };
        let result = match prepare_result(&project_id, &session_id, staged) {
            Ok(result) => result,
            Err(error) => {
                finish_failed(&app, &imports, &session_id, error);
                return;
            }
        };
        if cancel.load(Ordering::Relaxed) {
            let _ = result.cleanup();
            finish_cancelled(&app, &imports, &session_id);
            return;
        }
        let mut manager = if let Ok(manager) = imports.lock() {
            manager
        } else {
            let _ = result.cleanup();
            return;
        };
        let Some(job) = manager.jobs.get_mut(&session_id) else {
            let _ = result.cleanup();
            return;
        };
        if job.cancel.load(Ordering::Relaxed) {
            let _ = result.cleanup();
            return;
        }
        job.status.state = "ready".into();
        job.status.stage = "ready".into();
        job.status.sequence = job.status.sequence.saturating_add(1);
        job.status.current_source_path = None;
        job.status.result = Some(result.metadata().clone());
        job.expires_at = Instant::now() + ANALYSIS_SESSION_TTL;
        job.result = Some(result);
        let status = job.status.clone();
        drop(manager);
        let _ = app.emit(EXTERNAL_IMPORT_PROGRESS_EVENT, status);
    });
}

fn update_progress(
    app: &AppHandle,
    imports: &SharedExternalImports,
    session_id: &str,
    progress: ImportAnalysisProgress,
) {
    let status = {
        let mut manager = match imports.lock() {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let Some(job) = manager.jobs.get_mut(session_id) else {
            return;
        };
        if job.cancel.load(Ordering::Relaxed) {
            return;
        }
        job.status.state = "analyzing".into();
        job.status.stage = "analyzing".into();
        job.status.processed_entries = progress.processed_entries;
        job.status.staged_object_count = progress.staged_object_count;
        job.status.unsupported_count = progress.unsupported_count;
        job.status.source_bytes = progress.source_bytes;
        job.status.current_source_path = progress.source_path;
        job.status.sequence = job.status.sequence.saturating_add(1);
        job.status.clone()
    };
    let _ = app.emit(EXTERNAL_IMPORT_PROGRESS_EVENT, status);
}

fn finish_cancelled(app: &AppHandle, imports: &SharedExternalImports, session_id: &str) {
    let status = {
        let mut manager = match imports.lock() {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let Some(job) = manager.jobs.get_mut(session_id) else {
            return;
        };
        job.status.state = "cancelled".into();
        job.status.stage = "cancelled".into();
        job.status.error = None;
        job.status.error_code = Some("external_import.cancelled".into());
        job.status.current_source_path = None;
        job.status.sequence = job.status.sequence.saturating_add(1);
        job.expires_at = Instant::now() + ANALYSIS_SESSION_TTL;
        job.status.clone()
    };
    let _ = app.emit(EXTERNAL_IMPORT_PROGRESS_EVENT, status);
}

fn finish_failed(
    app: &AppHandle,
    imports: &SharedExternalImports,
    session_id: &str,
    error: String,
) {
    let status = {
        let mut manager = match imports.lock() {
            Ok(manager) => manager,
            Err(_) => return,
        };
        let Some(job) = manager.jobs.get_mut(session_id) else {
            return;
        };
        job.status.state = "failed".into();
        job.status.stage = "failed".into();
        job.status.error = Some(error);
        job.status.error_code = Some("external_import.analysis_failed".into());
        job.status.current_source_path = None;
        job.status.sequence = job.status.sequence.saturating_add(1);
        job.expires_at = Instant::now() + ANALYSIS_SESSION_TTL;
        job.status.clone()
    };
    let _ = app.emit(EXTERNAL_IMPORT_PROGRESS_EVENT, status);
}

fn prepare_result(
    project_id: &str,
    session_id: &str,
    staged: StagedImport,
) -> Result<ImportResultStorage, String> {
    let StagedImport {
        schema_version,
        importer,
        source,
        objects,
        assets,
        unsupported,
        diagnostics,
        summary,
    } = staged;
    let mut items =
        Vec::with_capacity(objects.len() + assets.len() + unsupported.len() + diagnostics.len());
    items.extend(objects.into_iter().map(ExternalImportPageItem::Object));
    items.extend(assets.into_iter().map(ExternalImportPageItem::Asset));
    items.extend(
        unsupported
            .into_iter()
            .map(ExternalImportPageItem::Unsupported),
    );
    items.extend(
        diagnostics
            .into_iter()
            .map(ExternalImportPageItem::Diagnostic),
    );
    let should_spill =
        summary.total_source_bytes > SPILL_SOURCE_BYTES || items.len() > SPILL_ITEM_COUNT;
    let metadata = ExternalImportResultMetadata {
        schema_version,
        importer,
        source,
        summary,
        total_items: items.len(),
        spilled_to_local_storage: should_spill,
    };
    if !should_spill {
        return Ok(ImportResultStorage::Memory { metadata, items });
    }
    spill_result(project_id, session_id, metadata, &items)
}

fn spill_result(
    project_id: &str,
    session_id: &str,
    metadata: ExternalImportResultMetadata,
    items: &[ExternalImportPageItem],
) -> Result<ImportResultStorage, String> {
    Uuid::parse_str(session_id)
        .map_err(|_| "external_import.invalid_session: session id is invalid".to_string())?;
    let directory = secure_spill_directory(Path::new(project_id))?;
    let path = directory.join(format!("{session_id}.jsonl"));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
    let mut writer = BufWriter::new(file);
    let result = (|| -> Result<(), String> {
        for item in items {
            serde_json::to_writer(&mut writer, item)
                .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
            writer
                .write_all(b"\n")
                .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
        }
        writer
            .flush()
            .map_err(|error| format!("external_import.local_storage_failed: {error}"))
    })();
    if let Err(error) = result {
        drop(writer);
        return match remove_spill_file(&path) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(format!("{error}; cleanup also failed: {cleanup_error}")),
        };
    }
    Ok(ImportResultStorage::Spill { metadata, path })
}

fn secure_spill_directory(project_root: &Path) -> Result<PathBuf, String> {
    let daena = project_root.join(".daena");
    ensure_real_directory(&daena, false)?;
    let local = daena.join("local");
    ensure_real_directory(&local, true)?;
    let imports = local.join("imports");
    ensure_real_directory(&imports, true)?;
    Ok(imports)
}

fn ensure_real_directory(path: &Path, create: bool) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(
            "external_import.local_storage_failed: staging directory is not a real directory"
                .into(),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
            fs::create_dir(path)
                .map_err(|error| format!("external_import.local_storage_failed: {error}"))
        }
        Err(error) => Err(format!("external_import.local_storage_failed: {error}")),
    }
}

fn read_spill_page(
    path: &Path,
    offset: usize,
    limit: usize,
) -> Result<Vec<ExternalImportPageItem>, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(
            "external_import.local_storage_failed: staging result is not a real file".into(),
        );
    }
    let file = fs::File::open(path)
        .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
    BufReader::new(file)
        .lines()
        .skip(offset)
        .take(limit)
        .map(|line| {
            let line =
                line.map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
            serde_json::from_str(&line)
                .map_err(|error| format!("external_import.local_storage_failed: {error}"))
        })
        .collect()
}

fn candidate_material_from_items<'a>(
    items: impl Iterator<Item = &'a ExternalImportPageItem>,
) -> (Vec<StagedObject>, Vec<StagedAsset>, Vec<ImportDiagnostic>) {
    let mut objects = Vec::new();
    let mut assets = Vec::new();
    let mut diagnostics = Vec::new();
    for item in items {
        match item {
            ExternalImportPageItem::Object(object) => objects.push(object.clone()),
            ExternalImportPageItem::Asset(asset) => assets.push(asset.clone()),
            ExternalImportPageItem::Diagnostic(diagnostic) => {
                diagnostics.push(diagnostic.clone());
            }
            ExternalImportPageItem::Unsupported(_) => {}
        }
    }
    (objects, assets, diagnostics)
}

fn read_spill_candidate_material(path: &Path) -> Result<CandidateMaterial, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(
            "external_import.local_storage_failed: staging result is not a real file".into(),
        );
    }
    let file = fs::File::open(path)
        .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
    let mut objects = Vec::new();
    let mut assets = Vec::new();
    let mut diagnostics = Vec::new();
    for line in BufReader::new(file).lines() {
        let line =
            line.map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
        let item: ExternalImportPageItem = serde_json::from_str(&line)
            .map_err(|error| format!("external_import.local_storage_failed: {error}"))?;
        match item {
            ExternalImportPageItem::Object(object) => objects.push(object),
            ExternalImportPageItem::Asset(asset) => assets.push(asset),
            ExternalImportPageItem::Diagnostic(diagnostic) => diagnostics.push(diagnostic),
            ExternalImportPageItem::Unsupported(_) => {}
        }
    }
    Ok((objects, assets, diagnostics))
}

fn remove_spill_file(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            "external_import.local_storage_failed: refused to remove a non-file staging result"
                .into(),
        ),
        Ok(_) => fs::remove_file(path)
            .map_err(|error| format!("external_import.local_storage_failed: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("external_import.local_storage_failed: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use daena_core::analyze_generic_documents;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("daena-external-import-job-test-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
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

    fn test_status(session_id: &str) -> ExternalImportAnalysisStatus {
        ExternalImportAnalysisStatus {
            session_id: session_id.into(),
            importer_id: GENERIC_DOCUMENT_IMPORTER_ID.into(),
            state: "ready".into(),
            stage: "ready".into(),
            processed_entries: 2,
            staged_object_count: 2,
            unsupported_count: 0,
            source_bytes: 6,
            sequence: 1,
            current_source_path: None,
            error: None,
            error_code: None,
            captured_content_generation: 1,
            current_content_generation: Some(1),
            result: None,
        }
    }

    #[test]
    fn built_in_importers_advertise_obsidian_as_folder_only() {
        let importers = project_external_importers();
        let generic = importers
            .iter()
            .find(|importer| importer.id == GENERIC_DOCUMENT_IMPORTER_ID)
            .unwrap();
        let obsidian = importers
            .iter()
            .find(|importer| importer.id == OBSIDIAN_IMPORTER_ID)
            .unwrap();

        assert_eq!(importers.len(), 2);
        assert_eq!(generic.source_kinds, vec!["file", "folder"]);
        assert_eq!(obsidian.source_kinds, vec!["folder"]);
        assert_eq!(obsidian.extensions, vec!["md", "markdown"]);
    }

    #[test]
    fn source_handles_are_project_bound_and_single_use() {
        let directory = TestDirectory::new();
        let source_path = directory.path().join("notes.md");
        fs::write(&source_path, "notes").unwrap();
        let mut manager = ExternalImportJobManager::default();
        let handle = manager
            .register_source("project-a".into(), "file".into(), source_path.clone())
            .unwrap();

        assert!(manager
            .take_source("project-b", &handle.source_handle)
            .unwrap_err()
            .contains("not valid for this project"));
        let selected = manager
            .take_source("project-a", &handle.source_handle)
            .unwrap();
        assert_eq!(selected.path, source_path);
        assert_eq!(selected.source_kind, "file");
        assert!(manager
            .take_source("project-a", &handle.source_handle)
            .is_err());
    }

    #[test]
    fn spilled_results_are_paged_and_removed_explicitly() {
        let project = TestDirectory::new();
        fs::create_dir(project.path().join(".daena")).unwrap();
        let source = TestDirectory::new();
        fs::write(source.path().join("one.md"), "one").unwrap();
        fs::write(source.path().join("two.md"), "two").unwrap();
        let staged =
            analyze_generic_documents(source.path(), GenericDocumentImportLimits::default())
                .unwrap();
        let StagedImport {
            schema_version,
            importer,
            source,
            objects,
            assets,
            unsupported,
            diagnostics,
            summary,
        } = staged;
        let items = objects
            .into_iter()
            .map(ExternalImportPageItem::Object)
            .collect::<Vec<_>>();
        assert!(assets.is_empty() && unsupported.is_empty() && diagnostics.is_empty());
        let metadata = ExternalImportResultMetadata {
            schema_version,
            importer,
            source,
            summary,
            total_items: items.len(),
            spilled_to_local_storage: true,
        };
        let session_id = Uuid::new_v4().to_string();
        let storage = spill_result(
            project.path().to_str().unwrap(),
            &session_id,
            metadata,
            &items,
        )
        .unwrap();

        let page = storage.page(1, 1).unwrap();
        assert_eq!(page.len(), 1);
        let spill_path = match &storage {
            ImportResultStorage::Spill { path, .. } => path.clone(),
            _ => panic!("expected spilled result"),
        };
        assert!(spill_path.is_file());
        let cancel = Arc::new(AtomicBool::new(false));
        let mut manager = ExternalImportJobManager::default();
        manager.jobs.insert(
            session_id.clone(),
            ExternalImportJob {
                project_id: project.path().to_string_lossy().into_owned(),
                source_root: project.path().to_path_buf(),
                expires_at: Instant::now() + ANALYSIS_SESSION_TTL,
                cancel: cancel.clone(),
                status: test_status(&session_id),
                result: Some(storage),
                validated: None,
            },
        );
        manager.cancel_all().unwrap();
        assert!(cancel.load(Ordering::Relaxed));
        assert!(manager.jobs.is_empty());
        assert!(!spill_path.exists());
    }

    #[test]
    fn guessed_or_cross_project_sessions_are_not_visible() {
        let session_id = Uuid::new_v4().to_string();
        let mut manager = ExternalImportJobManager::default();
        manager.jobs.insert(
            session_id.clone(),
            ExternalImportJob {
                project_id: "project-a".into(),
                source_root: PathBuf::from("source-a"),
                expires_at: Instant::now() + ANALYSIS_SESSION_TTL,
                cancel: Arc::new(AtomicBool::new(false)),
                status: test_status(&session_id),
                result: None,
                validated: None,
            },
        );

        assert!(project_job(&manager, "project-b", &session_id).is_err());
        assert!(project_job(&manager, "project-a", &Uuid::new_v4().to_string()).is_err());
        assert!(project_job(&manager, "project-a", &session_id).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn spill_directory_refuses_symlinks() {
        use std::os::unix::fs::symlink;

        let project = TestDirectory::new();
        let outside = TestDirectory::new();
        fs::create_dir(project.path().join(".daena")).unwrap();
        symlink(outside.path(), project.path().join(".daena/local")).unwrap();

        let error = secure_spill_directory(project.path()).unwrap_err();
        assert!(error.contains("not a real directory"));
    }
}
