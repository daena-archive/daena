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
