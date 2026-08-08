mod authority;
mod error;
pub mod maps;
mod migrations;
mod project;
mod storage;
mod sync;

pub use authority::{Authority, AuthorityContext};
pub use error::CoreError;
pub use migrations::{FieldDefinition, Migration, Operation};
pub use project::{
    Asset, AssetFileInput, AssetInput, AssetReplaceInput, CreateEntity, CreateEntry,
    CreateEntryDocument, CreateEntryField, Document, Entity, ExternalChangeReport, FieldValue,
    GitLogEntry, GitPreflight, GitRemote, GitResetResult, GitStatus, GitToolInfo, GitUpstream,
    MigrationHistoryEntry, ModuleField, ModuleNamespace, ModuleState, PluginBackup, ProjectInfo,
    ProjectSnapshot, ProjectStore, Relationship, RelationshipInput, SaveDocument, SaveEntry,
    SearchPassage,
};
pub use storage::{
    canonical_json_bytes, canonical_markdown, canonical_markdown_bytes, normalized_project_path,
    parse_json, read_canonical_project, read_json, write_canonical_project, write_json, AssetsFile,
    CanonicalAsset, CanonicalMigration, CanonicalProject, CanonicalRelationship, CanonicalSource,
    EntityDocumentRef, EntityFile, FieldsFile, FilesystemRepository, PluginStateFile,
    ProjectManifest, RelationshipsFile, CORE_PLUGIN_ID, PROJECT_FORMAT_VERSION,
};

#[derive(Default)]
pub struct CoreService {
    project: Option<ProjectStore>,
}

impl CoreService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(
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
        let project = ProjectStore::open_directory(path)?;
        let info = project
            .info()
            .ok_or_else(|| CoreError::Validation("project has no directory root".into()))?;
        self.project = Some(project);
        Ok(info)
    }

    pub fn open_memory(&mut self, context: AuthorityContext) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "open in-memory project")?;
        self.project = Some(ProjectStore::in_memory()?);
        Ok(())
    }

    pub fn close(&mut self, context: AuthorityContext) -> Result<(), CoreError> {
        self.require_trusted_shell(context, "close project")?;
        self.project = None;
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
