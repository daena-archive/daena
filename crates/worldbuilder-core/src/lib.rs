mod authority;
mod error;
mod migrations;
mod project;

pub use authority::{Authority, AuthorityContext};
pub use error::CoreError;
pub use migrations::{FieldDefinition, Migration, Operation};
pub use project::{
    Asset, AssetFileInput, AssetInput, CreateEntity, Document, Entity, FieldValue, GitLogEntry,
    GitStatus, ModuleState, ProjectInfo, ProjectSnapshot, ProjectStore, Relationship,
    RelationshipInput, SaveDocument, SaveEntry,
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
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn service_lifecycle_preserves_project_state_boundaries() {
        let mut service = CoreService::new();
        let context = AuthorityContext::trusted_shell();
        assert!(matches!(
            service.project(context),
            Err(CoreError::ProjectNotOpen)
        ));
        service.open_memory(context).unwrap();
        assert!(service.project(context).is_ok());
        service.close(context).unwrap();
        assert!(matches!(
            service.project(context),
            Err(CoreError::ProjectNotOpen)
        ));
    }

    #[test]
    fn service_replaces_open_projects_atomically() {
        let mut service = CoreService::new();
        let context = AuthorityContext::trusted_shell();
        service.open_memory(context).unwrap();
        let first = service.project(context).unwrap().info();
        service.open_memory(context).unwrap();
        assert_eq!(service.project(context).unwrap().info(), first);
    }

    #[test]
    fn shared_service_serializes_blocking_project_access() {
        let service = Arc::new(Mutex::new(CoreService::new()));
        let workers = (0..4)
            .map(|_| {
                let service = Arc::clone(&service);
                thread::spawn(move || {
                    let mut service = service.lock().unwrap();
                    let context = AuthorityContext::trusted_shell();
                    service.open_memory(context).unwrap();
                    service.project(context).unwrap().list_entities().unwrap()
                })
            })
            .collect::<Vec<_>>();

        for worker in workers {
            assert!(worker.join().unwrap().is_empty());
        }
    }

    #[test]
    fn plugin_authority_can_use_an_open_project_but_cannot_control_lifecycle() {
        let mut service = CoreService::new();
        service
            .open_memory(AuthorityContext::trusted_shell())
            .unwrap();
        assert!(service.project(AuthorityContext::plugin()).is_ok());
        assert!(matches!(
            service.open_memory(AuthorityContext::plugin()),
            Err(CoreError::Unauthorized { .. })
        ));
        assert!(matches!(
            service.close(AuthorityContext::plugin()),
            Err(CoreError::Unauthorized { .. })
        ));
    }
}
