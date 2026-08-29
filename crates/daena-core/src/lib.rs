mod authority;
mod error;
mod external_import;
pub mod maps;
mod migrations;
mod project;
mod storage;
mod sync;

pub use authority::{Authority, AuthorityContext};
pub use error::CoreError;
pub use external_import::{
    analyze_generic_documents, analyze_generic_documents_with_progress, analyze_mediawiki_xml,
    analyze_mediawiki_xml_with_progress, analyze_obsidian_vault,
    analyze_obsidian_vault_with_progress, build_import_candidate_plan,
    validate_import_candidate_plan, ExternalImportCommitReport, GenericDocumentImportLimits,
    ImportAnalysisProgress, ImportAnalysisSummary, ImportCandidateIssue, ImportCandidateMapping,
    ImportCandidateObject, ImportCandidatePlan, ImportCandidatePlanBuild, ImportDecisionReport,
    ImportDiagnostic, ImportDiagnosticSeverity, ImportExistingTarget, ImportFieldTarget,
    ImportFieldVariant, ImportMappingCatalog, ImportMappingDecision, ImportMappingOverrides,
    ImportMissingReferenceReport, ImportObjectDecision, ImportSource, ImportSourceKind,
    ImportValidationBuild, ImportValidationIssue, ImportValidationOutcome,
    ImportValidationSeverity, ImportedAssetReport, ImportedFieldReport, ImportedObjectReport,
    ImportedRelationshipReport, ImporterIdentity, MappingHintKind, StagedAsset, StagedDocument,
    StagedImport, StagedLink, StagedLinkKind, StagedLinkResolution, StagedMappingHint,
    StagedObject, UnsupportedSourceData, ValidatedImportAsset, ValidatedImportField,
    ValidatedImportObject, ValidatedImportPlan, ValidatedImportRelationship,
    ValidatedImportSourceContext, EXTERNAL_IMPORT_ANALYSIS_CANCELLED, GENERIC_DOCUMENT_IMPORTER_ID,
    GENERIC_DOCUMENT_IMPORTER_VERSION, IMPORT_CANDIDATE_PLAN_SCHEMA_VERSION, MEDIAWIKI_IMPORTER_ID,
    MEDIAWIKI_IMPORTER_VERSION, OBSIDIAN_IMPORTER_ID, OBSIDIAN_IMPORTER_VERSION,
    STAGED_IMPORT_SCHEMA_VERSION, VALIDATED_IMPORT_PLAN_SCHEMA_VERSION,
};
pub use migrations::{FieldDefinition, Migration, Operation};
pub use project::{
    set_checkpoint_export_status_listener, AcceptedPhysicalMap, AcceptedVectorMap, Asset,
    AssetFileInput, AssetFileReplaceInput, AssetInput, AssetMetadataUpdate, AssetReplaceInput,
    AttachedMapRaster, CheckpointHandle, CreateEntity, CreateEntry, CreateEntryDocument,
    CreateEntryField, CreateEntryRelationship, Document, Entity, EntityListQuery, EntityPage,
    EntitySortDirection, EntitySortField, EntityTypeCount, ExternalChangeReport, FieldValue,
    Generation, GitChange, GitLogEntry, GitPreflight, GitRemote, GitResetResult, GitStatus,
    GitToolInfo, GitUpstream, ImportedImageMap, MapEditApply, MapFeatureSearchResult,
    MapLinkMutation, MigrationHistoryEntry, ModuleField, ModuleNamespace, ModuleRecordListParams,
    ModuleState, PluginBackup, ProjectInfo, ProjectSnapshot, ProjectStore, RasterLayerChange,
    RasterLayerUpdate, Relationship, RelationshipInput, RelationshipPage, RelationshipQuery,
    RelationshipQueryDirection, RelationshipUpdate, SaveDocument, SaveEntry, SearchPassage,
    SyncSummary, VectorLayerDelete, VectorSourceReplace, WikiPageExportFormat,
    DEFAULT_ENTITY_QUERY_LIMIT, DEFAULT_RELATIONSHIP_QUERY_LIMIT, MAX_ENTITY_GET_MANY,
    MAX_ENTITY_QUERY_LIMIT, MAX_RELATIONSHIP_QUERY_ENTITIES, MAX_RELATIONSHIP_QUERY_LIMIT,
};
pub use storage::{
    build_checkpoint_manifest, canonical_json_bytes, canonical_markdown, canonical_markdown_bytes,
    normalized_project_path, parse_json, read_canonical_project, read_json, validate_checkpoint,
    write_canonical_project, write_checkpoint_manifest, write_json, AssetsFile, CanonicalAsset,
    CanonicalMigration, CanonicalProject, CanonicalRelationship, CheckpointFile,
    CheckpointManifest, EntityDocumentRef, EntityFile, FieldsFile, FilesystemRepository,
    PluginStateFile, ProjectManifest, RelationshipsFile, CHECKPOINT_MANIFEST_FILE, CORE_PLUGIN_ID,
    PROJECT_FORMAT_VERSION,
};

#[derive(Default)]
pub struct CoreService {
    project: Option<ProjectStore>,
}

impl CoreService {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(
        &mut self,
        context: AuthorityContext,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "open project")?;
        self.flush_current_project()?;
        self.open_without_flush(context, path)
    }

    pub fn open_without_flush(
        &mut self,
        context: AuthorityContext,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "open project")?;
        self.project = Some(ProjectStore::open(path)?);
        Ok(())
    }

    pub fn open_directory(
        &mut self,
        context: AuthorityContext,
        path: impl AsRef<std::path::Path>,
    ) -> Result<ProjectInfo, CoreError> {
        self.require_trusted_shell(context, "open project directory")?;
        self.flush_current_project()?;
        self.open_directory_without_flush(context, path)
    }

    pub fn open_directory_without_flush(
        &mut self,
        context: AuthorityContext,
        path: impl AsRef<std::path::Path>,
    ) -> Result<ProjectInfo, CoreError> {
        self.require_trusted_shell(context, "open project directory")?;
        let project = ProjectStore::open_directory(path)?;
        let info = project
            .info()
            .ok_or_else(|| CoreError::Validation("project has no directory root".into()))?;
        self.project = Some(project);
        Ok(info)
    }

    pub fn open_memory(&mut self, context: AuthorityContext) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "open in-memory project")?;
        self.flush_current_project()?;
        self.open_memory_without_flush(context)
    }

    pub fn open_memory_without_flush(
        &mut self,
        context: AuthorityContext,
    ) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "open in-memory project")?;
        self.project = Some(ProjectStore::in_memory()?);
        Ok(())
    }

    pub fn close(&mut self, context: AuthorityContext) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "close project")?;
        self.flush_current_project()?;
        self.close_without_flush(context)
    }

    pub fn close_without_flush(&mut self, context: AuthorityContext) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "close project")?;
        self.project = None;
        Ok(())
    }

    fn flush_current_project(&self) -> Result<(), CoreError> {
        if let Some(project) = &self.project {
            project.flush_checkpoint("project lifecycle transition")?;
        }
        Ok(())
    }

    pub fn info(&self) -> Option<ProjectInfo> {
        self.project.as_ref().and_then(ProjectStore::info)
    }

    pub fn project(&self, context: AuthorityContext) -> Result<&ProjectStore, CoreError> {
        self.require_project_access(context, "access project")?;
        self.project.as_ref().ok_or(CoreError::ProjectNotOpen)
    }

    pub fn project_mut(
        &mut self,
        context: AuthorityContext,
    ) -> Result<&mut ProjectStore, CoreError> {
        self.require_project_access(context, "mutate project")?;
        self.project.as_mut().ok_or(CoreError::ProjectNotOpen)
    }

    fn require_trusted_shell(
        &self,
        context: AuthorityContext,
        operation: &'static str,
    ) -> Result<(), CoreError> {
        if context.authority() == Authority::TrustedShell {
            Ok(())
        } else {
            Err(CoreError::Unauthorized { operation })
        }
    }

    fn require_project_access(
        &self,
        context: AuthorityContext,
        operation: &'static str,
    ) -> Result<(), CoreError> {
        if matches!(
            context.authority(),
            Authority::TrustedShell | Authority::Plugin
        ) {
            Ok(())
        } else {
            Err(CoreError::Unauthorized { operation })
        }
    }
}

#[cfg(test)]
mod tests;
