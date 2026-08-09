use super::*;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

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
fn service_close_flushes_queued_directory_exports() {
    let root = std::env::temp_dir().join(format!("daena-service-close-{}", Uuid::new_v4()));
    let mut service = CoreService::new();
    let context = AuthorityContext::trusted_shell();
    service.open_directory(context, &root).unwrap();
    let entity = service
        .project(context)
        .unwrap()
        .create_entity(CreateEntity {
            name: "Close flush owner".into(),
            entity_type: None,
        })
        .unwrap();
    service
        .project(context)
        .unwrap()
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Flushed on close\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    service.close(context).unwrap();

    let document = root.join("entities").join(entity.id).join("document.md");
    assert_eq!(
        std::fs::read_to_string(document).unwrap(),
        "Flushed on close\n"
    );
    std::fs::remove_dir_all(root).unwrap();
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
