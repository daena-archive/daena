use super::*;
use daena_plugin_api::{Entrypoints, PluginKind};
use std::time::Duration;

fn manifest(id: &str, namespace: &str) -> PluginManifest {
    PluginManifest {
        manifest_version: 1,
        id: id.into(),
        name: id.into(),
        version: "1.0.0".into(),
        publisher: "example".into(),
        enabled_by_default: None,
        stability: None,
        host_api: ">=1.0.0 <2.0.0".into(),
        kind: PluginKind::Sandboxed,
        entrypoints: Entrypoints {
            ui: Some("dist/index.html".into()),
            wasm: None,
        },
        capabilities: vec![
            "entity.read".into(),
            "entity.write".into(),
            "document.write".into(),
            "field.read:self".into(),
            "field.write:self".into(),
            "asset.read:self".into(),
            "event.publish:<type>".into(),
            "service.call:<name>".into(),
        ],
        dependencies: BTreeMap::new(),
        namespaces: vec![namespace.into()],
        schemas: vec![daena_plugin_api::SchemaContribution {
            namespace: namespace.into(),
            entity_types: vec!["person".into()],
            fields: vec![daena_plugin_api::FieldDefinition {
                key: "summary".into(),
                label: "Summary".into(),
                field_type: "text".into(),
                required: None,
                options: None,
                entity_types: None,
                relationship_type: None,
                target_entity_types: None,
                shared: false,
            }],
        }],
        templates: vec![],
        views: vec![daena_plugin_api::View {
            id: "overview".into(),
            title: "Overview".into(),
            components: vec![],
        }],
        commands: vec![daena_plugin_api::Command {
            id: "refresh".into(),
            title: "Refresh".into(),
            action: None,
            input: None,
            output: None,
            capabilities: vec![],
            exposure: vec![],
        }],
        services: daena_plugin_api::Services {
            provides: vec![],
            consumes: vec![daena_plugin_api::Service {
                name: "com.example.calculate".into(),
                major: 1,
            }],
        },
        events: daena_plugin_api::Events {
            publishes: vec![daena_plugin_api::Event {
                name: "daena.core/event".into(),
                version: 1,
            }],
            subscribes: vec![],
        },
        migrations: vec![],
    }
}
fn host() -> PluginHost {
    let mut host = PluginHost::new();
    let entry = CatalogEntry {
        manifest: manifest("com.example.one", "one"),
        package_root: PathBuf::new(),
        digest: "a".repeat(64),
        embedded_wasm: None,
    };
    host.catalog.insert_for_test(entry.clone()).unwrap();
    host.namespaces.register_manifest(&entry.manifest).unwrap();
    host.grants
        .set(
            "project",
            "com.example.one",
            &entry.manifest.capabilities,
            ["entity.read".into(), "field.read:self".into()]
                .into_iter()
                .collect(),
        )
        .unwrap();
    host
}

#[test]
fn relationship_delete_is_authorized_as_relationship_write() {
    let mut host = host();
    let entry = host.catalog.get("com.example.one").unwrap().clone();
    let session = host.sessions.issue(
        &entry,
        "project",
        "plugin://com.example.one",
        BTreeSet::new(),
        Duration::from_secs(60),
    );

    assert_eq!(
        required_capabilities(
            "relationship.delete",
            &serde_json::json!({ "id": "relationship-1" }),
            &session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["relationship.write".to_string()]
    );
}

#[test]
fn project_version_selection_is_scoped_and_revokes_sessions() {
    let mut host = host();
    host.select_project_version("project-a", "com.example.one", "1.0.0")
        .unwrap();
    assert_eq!(
        host.selected_project_version("project-a", "com.example.one"),
        Some("1.0.0".into())
    );
    assert_eq!(
        host.selected_project_version("project-b", "com.example.one"),
        Some("1.0.0".into())
    );
    assert!(host
        .project_versions
        .contains_key(&("project-a".into(), "com.example.one".into())));
    assert!(!host
        .project_versions
        .contains_key(&("project-b".into(), "com.example.one".into())));
}
#[test]
fn digest_changes_when_file_changes() {
    let root = std::env::temp_dir().join(format!(
        "wb-plugin-{}",
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("dist")).unwrap();
    fs::write(root.join("manifest.json"), b"{}").unwrap();
    fs::write(root.join("dist/index.html"), b"one").unwrap();
    let first = package_digest(&root).unwrap();
    fs::write(root.join("dist/index.html"), b"two").unwrap();
    assert_ne!(first, package_digest(&root).unwrap());
    let _ = fs::remove_dir_all(root);
}
#[test]
fn development_catalog_validates_referenced_files_and_duplicate_ids() {
    let root = std::env::temp_dir().join(format!(
        "wb-plugin-package-{}",
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("dist")).unwrap();
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&manifest("com.example.one", "one")).unwrap(),
    )
    .unwrap();
    fs::write(root.join("dist/index.html"), b"plugin").unwrap();
    let mut catalog = PluginCatalog::default();
    assert_eq!(
        catalog.install_development_dir(&root).unwrap().manifest.id,
        "com.example.one"
    );
    assert!(catalog.install_development_dir(&root).is_err());
    fs::remove_file(root.join("dist/index.html")).unwrap();
    assert!(catalog.install_development_dir(root.join(".")).is_err());
    let _ = fs::remove_dir_all(root);
}
#[cfg(unix)]
#[test]
fn development_catalog_rejects_symlinks() {
    use std::os::unix::fs::symlink;
    let root = std::env::temp_dir().join(format!(
        "wb-plugin-link-{}",
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(root.join("dist")).unwrap();
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&manifest("com.example.link", "link")).unwrap(),
    )
    .unwrap();
    fs::write(root.join("real.html"), b"plugin").unwrap();
    symlink(root.join("real.html"), root.join("dist/index.html")).unwrap();
    assert!(PluginCatalog::default()
        .install_development_dir(&root)
        .is_err());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_usage_survives_host_restart_for_uninstall_protection() {
    let directory = tempfile::tempdir().unwrap();
    let install_root = directory.path().join("plugins");
    let state_path = directory.path().join("plugin-state.json");
    let mut first = PluginHost::new();
    first
        .load_installed_packages(
            &install_root,
            &state_path,
            ArchiveLimits::default(),
            VerificationPolicy::default(),
        )
        .unwrap();
    first
        .record_project_usage("closed-project", "com.example.plugin", "1.0.0")
        .unwrap();

    let mut restarted = PluginHost::new();
    restarted
        .load_installed_packages(
            &install_root,
            &state_path,
            ArchiveLimits::default(),
            VerificationPolicy::default(),
        )
        .unwrap();
    assert!(restarted.project_uses_version("com.example.plugin", "1.0.0"));
}

#[test]
fn capability_grants_survive_in_project_local_file() {
    let directory = tempfile::tempdir().unwrap();
    let project_root = directory.path().join("project");
    fs::create_dir_all(project_root.join(".daena/local")).unwrap();
    let state_path = directory.path().join("plugin-state.json");
    let install_root = directory.path().join("plugins");
    let expected = ["entity.read".into(), "field.read:self".into()]
        .into_iter()
        .collect::<BTreeSet<_>>();

    let mut first = host();
    first.state_path = Some(state_path.clone());
    first.bind_project_grants(&project_root, "project").unwrap();
    first
        .grant_capabilities("project", "com.example.one", expected.clone())
        .unwrap();

    let mut restarted = PluginHost::new();
    restarted
        .load_installed_packages(
            &install_root,
            &state_path,
            ArchiveLimits::default(),
            VerificationPolicy::default(),
        )
        .unwrap();
    restarted
        .catalog
        .insert_for_test(CatalogEntry {
            manifest: manifest("com.example.one", "one"),
            package_root: PathBuf::new(),
            digest: "a".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    restarted
        .bind_project_grants(&project_root, "project")
        .unwrap();
    assert_eq!(restarted.grants.get("project", "com.example.one"), expected);
    assert!(!state_path
        .exists()
        .then(|| fs::read_to_string(&state_path).unwrap_or_default())
        .unwrap_or_default()
        .contains("entity.read"));
}

#[test]
fn first_party_bundled_bootstrap_grants_declared_capabilities() {
    let directory = tempfile::tempdir().unwrap();
    let project_root = directory.path().join("project");
    fs::create_dir_all(project_root.join(".daena/local")).unwrap();
    let mut host = PluginHost::new();
    host.register_bundled_json(include_str!("../../../packages/modules/maps/manifest.json"))
        .unwrap();
    host.bind_project_grants(&project_root, "project").unwrap();
    assert!(host.grants.is_empty("project", "daena.maps"));

    let session = host
        .bootstrap("daena.maps", "project", "plugin:daena.maps")
        .unwrap();
    assert!(
        session.grants.contains("entity.read"),
        "Maps bootstrap must grant entity.read for firstMapAsset"
    );
    assert!(session.grants.contains("asset.write:self"));
    assert_eq!(
        host.grants.get("project", "daena.maps"),
        host.catalog
            .get("daena.maps")
            .unwrap()
            .manifest
            .capabilities
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
    );

    // Third-party publishers still default to deny-all without consent.
    host.catalog
        .insert_for_test(CatalogEntry {
            manifest: manifest("com.example.other", "other"),
            package_root: PathBuf::new(),
            digest: "b".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    let third_party = host
        .bootstrap("com.example.other", "project", "plugin:com.example.other")
        .unwrap();
    assert!(third_party.grants.is_empty());
}

#[test]
fn legacy_global_grants_migrate_into_project_local_file() {
    let directory = tempfile::tempdir().unwrap();
    let project_root = directory.path().join("world");
    fs::create_dir_all(project_root.join(".daena/local")).unwrap();
    let state_path = directory.path().join("plugin-state.json");
    let install_root = directory.path().join("plugins");
    let expected = ["entity.read".into()].into_iter().collect::<BTreeSet<_>>();

    let mut seeded = PluginHost::new();
    seeded.state_path = Some(state_path.clone());
    seeded
        .legacy_grants
        .insert_loaded("world-id", "com.example.one", expected.clone());
    seeded.persist_state().unwrap();

    let mut host = PluginHost::new();
    host.load_installed_packages(
        &install_root,
        &state_path,
        ArchiveLimits::default(),
        VerificationPolicy::default(),
    )
    .unwrap();
    host.bind_project_grants(&project_root, "world-id").unwrap();
    assert_eq!(host.grants.get("world-id", "com.example.one"), expected);
    assert!(project_root
        .join(".daena/local/plugin-grants.json")
        .is_file());
    let rewritten: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    assert_eq!(rewritten["grants"]["grants"], serde_json::json!([]));
}

#[test]
fn empty_legacy_grants_are_accepted_during_host_restart() {
    let directory = tempfile::tempdir().unwrap();
    let state_path = directory.path().join("plugin-state.json");
    let install_root = directory.path().join("plugins");
    fs::write(
        &state_path,
        r#"{"packages":{"versions":{}},"grants":{},"project_usage":[]}"#,
    )
    .unwrap();

    let mut restarted = PluginHost::new();
    restarted
        .load_installed_packages(
            &install_root,
            &state_path,
            ArchiveLimits::default(),
            VerificationPolicy::default(),
        )
        .unwrap();

    assert!(restarted.grants.is_empty("project", "com.example.one"));
}

#[test]
fn clearing_project_usage_clears_the_live_project_selection() {
    let mut host = host();
    host.select_project_version("project", "com.example.one", "1.0.0")
        .unwrap();
    assert!(host
        .project_versions
        .contains_key(&("project".into(), "com.example.one".into())));

    host.clear_project_usage("project", "com.example.one")
        .unwrap();

    assert!(!host
        .project_versions
        .contains_key(&("project".into(), "com.example.one".into())));
    assert!(!host.project_uses_version("com.example.one", "1.0.0"));
}

#[test]
fn forged_identity_and_origin_are_rejected() {
    let mut host = host();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let request = RpcRequest {
        rpc_version: 1,
        session_id: session.id.clone(),
        request_id: "1".into(),
        method: "entity.list".into(),
        payload: serde_json::json!({}),
    };
    assert_eq!(
        host.rpc("plugin://other", &request).error.unwrap().code,
        "session.origin"
    );
    let forged = RpcRequest {
        session_id: "forged".into(),
        ..request
    };
    assert_eq!(
        host.rpc("plugin://one", &forged).error.unwrap().code,
        "session.invalid"
    );
}

#[test]
fn bundled_bootstrap_is_deny_by_default_without_consent() {
    let mut host = PluginHost::new();
    let entry = CatalogEntry {
        manifest: manifest("com.example.unconsented", "unconsented"),
        package_root: PathBuf::new(),
        digest: "b".repeat(64),
        embedded_wasm: None,
    };
    host.catalog.insert_for_test(entry.clone()).unwrap();
    host.namespaces.register_manifest(&entry.manifest).unwrap();
    let session = host
        .ensure_bundled_session("com.example.unconsented", "project")
        .unwrap();
    assert!(session.grants.is_empty());
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        session_id: session.id,
        request_id: "deny".into(),
        method: "entity.list".into(),
        payload: serde_json::json!({}),
    };
    assert_eq!(
        host.rpc("bundled:com.example.unconsented", &request)
            .error
            .unwrap()
            .code,
        "capability.denied"
    );
}

#[test]
fn undeclared_fields_and_relationship_types_fail_closed() {
    let mut host = host();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let field = RpcRequest {
        rpc_version: RPC_VERSION,
        session_id: session.id.clone(),
        request_id: "field".into(),
        method: "field.read".into(),
        payload: serde_json::json!({
            "namespace": "one",
            "key": "not_declared"
        }),
    };
    assert_eq!(
        host.rpc("plugin://one", &field).error.unwrap().code,
        "schema.undeclared"
    );
    let relationship = RpcRequest {
        request_id: "relationship".into(),
        method: "relationship.create".into(),
        payload: serde_json::json!({
            "source_id": "source",
            "target_id": "target",
            "relationship_type": "forged_type",
            "metadata": "{}"
        }),
        ..field
    };
    assert_eq!(
        host.rpc("plugin://one", &relationship).error.unwrap().code,
        "relationship.undeclared"
    );
}

#[test]
fn shared_fields_are_readable_cross_namespace_but_never_writable() {
    let mut owner = manifest("com.example.owner", "owner");
    owner.schemas[0].fields[0].shared = true;
    let mut reader = manifest("com.example.reader", "reader");
    reader.capabilities.push("field.read:shared".into());
    let mut host = PluginHost::new();
    for plugin in [owner.clone(), reader.clone()] {
        host.catalog
            .insert_for_test(CatalogEntry {
                manifest: plugin.clone(),
                package_root: PathBuf::new(),
                digest: plugin.id.repeat(64).chars().take(64).collect(),
                embedded_wasm: None,
            })
            .unwrap();
        host.namespaces.register_manifest(&plugin).unwrap();
    }
    host.grants
        .set(
            "project",
            &reader.id,
            &reader.capabilities,
            ["field.read:shared".into()].into_iter().collect(),
        )
        .unwrap();
    let session = host
        .bootstrap(&reader.id, "project", "plugin://reader")
        .unwrap();
    let read = RpcRequest {
        rpc_version: RPC_VERSION,
        session_id: session.id.clone(),
        request_id: "shared-read".into(),
        method: "field.read".into(),
        payload: serde_json::json!({
            "entityId": "entity",
            "namespace": "owner",
            "key": "summary"
        }),
    };
    assert!(host.rpc("plugin://reader", &read).ok);

    let write = RpcRequest {
        request_id: "shared-write".into(),
        method: "field.set".into(),
        payload: serde_json::json!({
            "entityId": "entity",
            "namespace": "owner",
            "key": "summary",
            "value": "forged"
        }),
        ..read
    };
    assert_eq!(
        host.rpc("plugin://reader", &write).error.unwrap().code,
        "namespace.denied"
    );
}

#[test]
fn relationship_delete_requires_the_stored_identity() {
    let mut plugin = manifest("com.example.relationship", "relationship");
    plugin.schemas[0].fields[0].relationship_type = Some("linked".into());
    plugin.capabilities.push("relationship.write".into());
    let mut host = PluginHost::new();
    host.catalog
        .insert_for_test(CatalogEntry {
            manifest: plugin.clone(),
            package_root: PathBuf::new(),
            digest: "a".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    host.namespaces.register_manifest(&plugin).unwrap();
    host.grants
        .set(
            "project",
            &plugin.id,
            &plugin.capabilities,
            ["relationship.write".into()].into_iter().collect(),
        )
        .unwrap();
    let session = host
        .bootstrap(&plugin.id, "project", "plugin://relationship")
        .unwrap();
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        session_id: session.id,
        request_id: "relationship-delete".into(),
        method: "relationship.delete".into(),
        payload: serde_json::json!({
            "id": "relationship-id",
            "relationship_type": "forged",
            "__stored_relationship_type": "linked"
        }),
    };
    assert_eq!(
        host.rpc("plugin://relationship", &request)
            .error
            .unwrap()
            .code,
        "relationship.identity"
    );
}

#[test]
fn explicit_grants_are_bound_to_the_manifest_request() {
    let mut host = host();
    host.grant_capabilities(
        "project",
        "com.example.one",
        ["entity.read".into()].into_iter().collect(),
    )
    .unwrap();
    assert_eq!(
        host.grants.get("project", "com.example.one"),
        ["entity.read".into()].into_iter().collect()
    );
    assert!(host
        .grant_capabilities(
            "project",
            "com.example.one",
            ["filesystem.write".into()].into_iter().collect(),
        )
        .is_err());
}

#[test]
fn wasm_service_provider_is_registered_and_invokable_after_activation() {
    let root = tempfile::tempdir().unwrap();
    let dist = root.path().join("dist");
    fs::create_dir_all(&dist).unwrap();
    fs::write(
        dist.join("service.wasm"),
        wat::parse_str("(module (func (export \"run\") (result i32) i32.const 7))").unwrap(),
    )
    .unwrap();
    let mut provider = manifest("com.example.wasm-provider", "wasm-provider");
    provider.entrypoints.wasm = Some("dist/service.wasm".into());
    provider.capabilities = vec!["service.provide:com.example.wasm.count@1".into()];
    provider.services.provides = vec![daena_plugin_api::Service {
        name: "com.example.wasm.count".into(),
        major: 1,
    }];
    let mut host = PluginHost::new();
    host.catalog
        .insert_for_test(CatalogEntry {
            manifest: provider.clone(),
            package_root: root.path().into(),
            digest: "c".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    host.namespaces.register_manifest(&provider).unwrap();
    host.grants
        .set(
            "project",
            &provider.id,
            &provider.capabilities,
            provider.capabilities.iter().cloned().collect(),
        )
        .unwrap();
    host.activate_bundled("project", &provider.id).unwrap();
    let value = host
        .services
        .call(
            "consumer",
            "com.example.wasm.count",
            1,
            serde_json::json!({}),
            Duration::from_millis(100),
        )
        .unwrap();
    assert_eq!(value["value"], 7);
}

#[test]
fn bundled_timeline_service_uses_the_generic_wasm_provider_path() {
    let mut host = PluginHost::new();
    host.register_bundled_json_with_wasm(
        include_str!("../../../packages/modules/timeline/manifest.json"),
        Some(BUNDLED_TIMELINE_SERVICE_WASM),
    )
    .unwrap();
    let manifest = host.catalog.get("daena.timeline").unwrap().manifest.clone();
    host.grants
        .set(
            "project",
            &manifest.id,
            &manifest.capabilities,
            manifest.capabilities.iter().cloned().collect(),
        )
        .unwrap();
    host.activate_bundled("project", &manifest.id).unwrap();
    let value = host
        .services
        .call(
            "com.example.consumer",
            "daena.timeline.resolve-date",
            1,
            serde_json::json!({"date": "0042-03-15"}),
            Duration::from_millis(100),
        )
        .unwrap();
    assert_eq!(value["date"], "0042-03-15");
    host.deactivate_bundled("project", &manifest.id);
    assert_eq!(
        host.services
            .provider_health("daena.timeline.resolve-date", 1),
        Some(ProviderHealth::Disabled)
    );
}
#[test]
fn undeclared_and_foreign_namespace_operations_are_rejected() {
    let mut host = host();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let denied = RpcRequest {
        rpc_version: 1,
        session_id: session.id.clone(),
        request_id: "1".into(),
        method: "entity.update".into(),
        payload: serde_json::json!({}),
    };
    assert_eq!(
        host.rpc("plugin://one", &denied).error.unwrap().code,
        "capability.denied"
    );
    let trusted = RpcRequest {
        method: "project.open".into(),
        ..denied.clone()
    };
    assert_eq!(
        host.rpc("plugin://one", &trusted).error.unwrap().code,
        "method.unknown"
    );
    let foreign = RpcRequest {
        method: "field.read".into(),
        payload: serde_json::json!({"namespace":"other"}),
        ..denied
    };
    assert_eq!(
        host.rpc("plugin://one", &foreign).error.unwrap().code,
        "namespace.denied"
    );
}

#[test]
fn templated_entity_creation_requires_all_declared_capabilities() {
    let mut host = host();
    host.grants
        .set(
            "project",
            "com.example.one",
            &[
                "entity.read".into(),
                "entity.write".into(),
                "document.write".into(),
                "field.write:self".into(),
            ],
            ["entity.write".into(), "document.write".into()]
                .into_iter()
                .collect::<std::collections::BTreeSet<String>>(),
        )
        .unwrap();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let request = RpcRequest {
        rpc_version: 1,
        session_id: session.id,
        request_id: "create".into(),
        method: "entity.create".into(),
        payload: serde_json::json!({
            "name": "Ash Court",
            "document": {"body": "A quiet power."},
            "fields": [{"namespace": "one", "key": "summary", "value": "A quiet power."}]
        }),
    };
    assert_eq!(
        host.rpc("plugin://one", &request).error.unwrap().code,
        "capability.denied"
    );
}
#[test]
fn asset_list_is_authorized_for_read_capability() {
    let mut host = host();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let request = RpcRequest {
        rpc_version: 1,
        session_id: session.id,
        request_id: "asset-list".into(),
        method: "asset.list".into(),
        payload: serde_json::json!({"namespace": "one", "entityId": "entity-1"}),
    };
    assert_ne!(
        host.rpc("plugin://one", &request)
            .error
            .map(|error| error.code),
        Some("method.unknown".into())
    );
}
#[test]
fn maps_asset_create_is_authorized_for_write_capability() {
    let mut host = host();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let denied = RpcRequest {
        rpc_version: 1,
        session_id: session.id,
        request_id: "maps-create".into(),
        method: "maps.asset.create.begin".into(),
        payload: serde_json::json!({"mapEntityId": "map-1", "size": 42}),
    };
    assert_eq!(
        host.rpc("plugin://one", &denied).error.unwrap().code,
        "capability.denied"
    );
    host.grants
        .set(
            "project",
            "com.example.one",
            &["asset.write:self".into()],
            ["asset.write:self".into()]
                .into_iter()
                .collect::<std::collections::BTreeSet<String>>(),
        )
        .unwrap();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let granted = RpcRequest {
        session_id: session.id,
        method: "maps.asset.create.commit".into(),
        payload: serde_json::json!({"handle": "handle-1", "contentHash": "sha256:abc"}),
        ..denied
    };
    let response = host.rpc("plugin://one", &granted);
    assert!(
        response.ok,
        "expected authorization to succeed, got: {:?}",
        response.error.as_ref().map(|error| error.code.clone())
    );
    assert_eq!(response.error, None);
}
#[test]
fn maps_locations_and_reconcile_are_authorized_for_read_capability() {
    let mut host = host();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    for method in ["maps.locations.list", "maps.reconcile.links"] {
        let denied = RpcRequest {
            rpc_version: 1,
            session_id: session.id.clone(),
            request_id: "maps-read".into(),
            method: method.into(),
            payload: serde_json::json!({ "mapEntityId": "map-1" }),
        };
        assert_eq!(
            host.rpc("plugin://one", &denied).error.unwrap().code,
            "capability.denied",
            "{method} should be denied without asset.read:self"
        );
    }
    host.grants
        .set(
            "project",
            "com.example.one",
            &["asset.read:self".into()],
            ["asset.read:self".into()]
                .into_iter()
                .collect::<std::collections::BTreeSet<String>>(),
        )
        .unwrap();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    for method in ["maps.locations.list", "maps.reconcile.links"] {
        let granted = RpcRequest {
            rpc_version: 1,
            session_id: session.id.clone(),
            request_id: "maps-read".into(),
            method: method.into(),
            payload: serde_json::json!({ "mapEntityId": "map-1" }),
        };
        let response = host.rpc("plugin://one", &granted);
        assert!(
            response.ok,
            "{method} should authorize with asset.read:self, got: {:?}",
            response.error.as_ref().map(|error| error.code.clone())
        );
        assert_eq!(response.error, None);
    }
}
#[test]
fn revoked_session_cannot_be_replayed() {
    let mut host = host();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    host.revoke_plugin("project", "com.example.one");
    let request = RpcRequest {
        rpc_version: 1,
        session_id: session.id,
        request_id: "1".into(),
        method: "entity.list".into(),
        payload: serde_json::json!({}),
    };
    assert_eq!(
        host.rpc("plugin://one", &request).error.unwrap().code,
        "session.revoked"
    );
}
#[test]
fn activation_generation_invalidates_previous_session() {
    let mut host = host();
    let first = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let second = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    assert_ne!(first.id, second.id);
    let request = RpcRequest {
        rpc_version: 1,
        session_id: first.id,
        request_id: "1".into(),
        method: "entity.list".into(),
        payload: serde_json::json!({}),
    };
    assert_eq!(
        host.rpc("plugin://one", &request).error.unwrap().code,
        "session.revoked"
    );
}
#[test]
fn dynamic_event_and_service_grants_are_checked() {
    let mut host = host();
    let entry = host
        .catalog
        .get("com.example.one")
        .unwrap()
        .manifest
        .clone();
    host.grants
        .set(
            "project",
            "com.example.one",
            &entry.capabilities,
            [
                "event.publish:daena.core/event@1".into(),
                "service.call:com.example.calculate".into(),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    for (method, payload) in [
        (
            "event.publish",
            serde_json::json!({"type":"daena.core/event@1"}),
        ),
        (
            "service.call",
            serde_json::json!({"name":"com.example.calculate","major":1}),
        ),
    ] {
        let request = RpcRequest {
            rpc_version: 1,
            session_id: session.id.clone(),
            request_id: method.into(),
            method: method.into(),
            payload,
        };
        assert!(host.rpc("plugin://one", &request).ok);
    }
    host.grants
        .set(
            "project",
            "com.example.one",
            &entry.capabilities,
            entry.capabilities.iter().cloned().collect(),
        )
        .unwrap();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let request = RpcRequest {
        rpc_version: 1,
        session_id: session.id,
        request_id: "wildcard-event".into(),
        method: "event.publish".into(),
        payload: serde_json::json!({"type":"daena.core/event@1"}),
    };
    assert!(host.rpc("plugin://one", &request).ok);
}
#[test]
fn authorized_event_and_service_calls_preserve_webview_session() {
    let mut host = host();
    let entry = host
        .catalog
        .get("com.example.one")
        .unwrap()
        .manifest
        .clone();
    host.grants
        .set(
            "project",
            "com.example.one",
            &entry.capabilities,
            entry.capabilities.iter().cloned().collect(),
        )
        .unwrap();
    host.services
        .register(
            "com.example.one",
            "com.example.calculate",
            1,
            Arc::new(|_request| Ok(serde_json::json!({"sum": 2}))),
        )
        .unwrap();
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    host.subscribe_event_authorized("com.example.one", "project", "daena.core/event", 1)
        .unwrap();
    assert!(host
        .publish_event_authorized(
            "com.example.one",
            "project",
            "daena.core/event",
            1,
            serde_json::json!({"ready": true}),
        )
        .is_ok());
    assert!(host
        .poll_events_authorized("com.example.one", "project", "daena.core/event", 1)
        .is_ok());
    assert!(host
        .call_service_authorized(
            "com.example.one",
            "project",
            "com.example.calculate",
            1,
            serde_json::json!({"a": 1, "b": 1}),
            Duration::from_millis(1000),
        )
        .is_ok());
    assert!(
        host.sessions.valid(&session.id, "plugin://one").is_ok(),
        "webview session must survive authorized event and service traffic"
    );
}
#[test]
fn namespace_collisions_are_rejected() {
    let mut ownership = NamespaceOwnership::default();
    ownership
        .register_manifest(&manifest("com.example.one", "shared"))
        .unwrap();
    assert!(ownership
        .register_manifest(&manifest("com.example.two", "shared"))
        .is_err());
}
#[test]
fn expired_session_is_rejected() {
    let mut host = host();
    host.session_ttl = Duration::ZERO;
    let session = host
        .bootstrap("com.example.one", "project", "plugin://one")
        .unwrap();
    let request = RpcRequest {
        rpc_version: 1,
        session_id: session.id,
        request_id: "1".into(),
        method: "entity.list".into(),
        payload: serde_json::json!({}),
    };
    assert_eq!(
        host.rpc("plugin://one", &request).error.unwrap().code,
        "session.expired"
    );
}

#[test]
fn canonical_bundled_manifests_register_without_handwritten_rust_copies() {
    let mut host = PluginHost::new();
    host.register_bundled_json(include_str!("../../../packages/modules/lore/manifest.json"))
        .unwrap();
    host.register_bundled_json(include_str!(
        "../../../packages/modules/timeline/manifest.json"
    ))
    .unwrap();
    assert_eq!(host.catalog.list().count(), 2);
    assert!(host.catalog.get("daena.lore").is_some());
    assert!(host.catalog.get("daena.timeline").is_some());
}

#[test]
fn dependencies_resolve_in_activation_order_and_reject_cycles() {
    let mut catalog = PluginCatalog::default();
    let mut app = manifest("com.example.app", "app");
    app.dependencies.insert(
        "com.example.service".into(),
        daena_plugin_api::Dependency {
            version: "^1.0.0".into(),
            required: true,
        },
    );
    catalog
        .insert_for_test(CatalogEntry {
            manifest: app,
            package_root: PathBuf::new(),
            digest: "a".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    catalog
        .insert_for_test(CatalogEntry {
            manifest: manifest("com.example.service", "service"),
            package_root: PathBuf::new(),
            digest: "b".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    assert_eq!(
        DependencyResolver::resolve(&catalog, "com.example.app")
            .unwrap()
            .order,
        vec!["com.example.service", "com.example.app"]
    );
    let mut cycle = catalog.get("com.example.service").unwrap().manifest.clone();
    cycle.dependencies.insert(
        "com.example.app".into(),
        daena_plugin_api::Dependency {
            version: "*".into(),
            required: true,
        },
    );
    let mut cyclic = PluginCatalog::default();
    cyclic
        .insert_for_test(CatalogEntry {
            manifest: catalog.get("com.example.app").unwrap().manifest.clone(),
            package_root: PathBuf::new(),
            digest: "a".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    cyclic
        .insert_for_test(CatalogEntry {
            manifest: cycle,
            package_root: PathBuf::new(),
            digest: "b".repeat(64),
            embedded_wasm: None,
        })
        .unwrap();
    assert!(DependencyResolver::resolve(&cyclic, "com.example.app").is_err());
}

#[test]
fn event_bus_is_at_most_once_and_bounded_for_slow_subscribers() {
    let mut bus = EventBus::new(1, 1024);
    bus.subscribe("project", "consumer", "daena.core/entity-changed", 1);
    assert_eq!(
        bus.publish(
            "project",
            "daena.core",
            "daena.core/entity-changed",
            1,
            serde_json::json!({"id": 1})
        )
        .unwrap(),
        PublishResult {
            delivered: 1,
            dropped: 0
        }
    );
    assert_eq!(
        bus.publish(
            "project",
            "daena.core",
            "daena.core/entity-changed",
            1,
            serde_json::json!({"id": 2})
        )
        .unwrap(),
        PublishResult {
            delivered: 1,
            dropped: 1
        }
    );
    let events = bus.drain("project", "consumer", "daena.core/entity-changed", 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].payload["id"], 2);
    let mut limited = EventBus::new(1, 4);
    assert!(limited
        .publish(
            "project",
            "core",
            "large",
            1,
            serde_json::json!("0123456789")
        )
        .is_err());
}

#[test]
fn services_enforce_provider_loss_deadlines_and_reentrancy() {
    let mut services = ServiceRegistry::new(1024);
    services
        .register(
            "timeline",
            "com.example.timeline/date",
            1,
            Arc::new(|request| Ok(request.payload)),
        )
        .unwrap();
    assert_eq!(
        services
            .call(
                "consumer",
                "com.example.timeline/date",
                1,
                serde_json::json!({"date":"1-1-1"}),
                Duration::from_millis(100),
            )
            .unwrap()["date"],
        "1-1-1"
    );
    assert!(services
        .call_with_stack(
            &["consumer".into(), "com.example.timeline/date@1".into()],
            "com.example.timeline/date",
            1,
            serde_json::json!({}),
            Duration::from_millis(100),
        )
        .is_err());
    services.unregister_plugin("timeline");
    assert!(services
        .call(
            "consumer",
            "com.example.timeline/date",
            1,
            serde_json::json!({}),
            Duration::from_millis(100),
        )
        .is_err());
    services
        .register(
            "slow",
            "com.example.slow",
            1,
            Arc::new(|request| {
                while !request.cancellation.is_cancelled() {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(HostError("cancelled".into()))
            }),
        )
        .unwrap();
    assert!(services
        .call(
            "consumer",
            "com.example.slow",
            1,
            serde_json::json!({}),
            Duration::from_millis(10),
        )
        .is_err());
}

#[test]
fn service_shutdown_quarantines_a_provider_that_ignores_cancellation() {
    let mut services = ServiceRegistry::new(1024);
    let started = Arc::new(AtomicBool::new(false));
    let handler_started = started.clone();
    services
        .register(
            "wedged",
            "com.example.wedged",
            1,
            Arc::new(move |_request| {
                handler_started.store(true, Ordering::Release);
                thread::sleep(Duration::from_millis(75));
                Ok(serde_json::json!({"done": true}))
            }),
        )
        .unwrap();
    let caller = services.clone();
    let worker = thread::spawn(move || {
        caller.call(
            "consumer",
            "com.example.wedged",
            1,
            serde_json::json!({}),
            Duration::from_millis(250),
        )
    });
    for _ in 0..100 {
        if started.load(Ordering::Acquire) {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(started.load(Ordering::Acquire));
    assert!(!services.deactivate_plugin("wedged", Duration::from_millis(5)));
    assert_eq!(
        services.provider_health("com.example.wedged", 1),
        Some(ProviderHealth::Quarantined)
    );
    let _ = worker.join();
}

#[test]
fn lifecycle_quarantines_after_three_failed_activations() {
    let mut lifecycle = LifecycleRegistry::default();
    for _ in 0..3 {
        lifecycle.begin_activation("project", "plugin").unwrap();
        lifecycle.activation_failed("project", "plugin", "startup failed");
    }
    let record = lifecycle.state("project", "plugin");
    assert_eq!(record.state, LifecycleState::Quarantined);
    assert!(lifecycle.begin_activation("project", "plugin").is_err());
}

#[test]
fn quarantine_is_cleared_only_by_explicit_retry() {
    let mut lifecycle = LifecycleRegistry::default();
    for _ in 0..3 {
        lifecycle.begin_activation("project", "plugin").unwrap();
        lifecycle.activation_failed("project", "plugin", "startup failed");
    }
    assert_eq!(
        lifecycle.state("project", "plugin").state,
        LifecycleState::Quarantined
    );
    lifecycle.clear_quarantine("project", "plugin");
    let record = lifecycle.state("project", "plugin");
    assert_eq!(record.state, LifecycleState::Resolved);
    assert_eq!(record.failures, 0);
    assert_eq!(record.last_error, None);
    lifecycle.begin_activation("project", "plugin").unwrap();
    lifecycle.activation_succeeded("project", "plugin");
    assert_eq!(
        lifecycle.state("project", "plugin").state,
        LifecycleState::Active
    );
}

#[test]
fn lifecycle_rolls_back_failed_startup_and_can_retry_before_quarantine() {
    let mut lifecycle = LifecycleRegistry::default();
    assert!(lifecycle
        .activate_with("project", "plugin", || {
            Err(HostError("startup failed".into()))
        })
        .is_err());
    assert_eq!(
        lifecycle.state("project", "plugin").state,
        LifecycleState::Failed
    );
    lifecycle
        .activate_with("project", "plugin", || Ok(()))
        .unwrap();
    assert_eq!(
        lifecycle.state("project", "plugin").state,
        LifecycleState::Active
    );
}

#[test]
fn optional_timeline_service_supports_a_declared_consumer() {
    let service_name = "com.example.timeline.resolve-date";
    let mut provider = manifest("com.example.timeline", "timeline");
    provider.services.provides.push(daena_plugin_api::Service {
        name: service_name.into(),
        major: 1,
    });
    let mut consumer = manifest("com.example.consumer", "consumer");
    consumer.capabilities.push("service.call:<name>".into());
    consumer.services.consumes.push(daena_plugin_api::Service {
        name: service_name.into(),
        major: 1,
    });
    let mut host = PluginHost::new();
    for plugin in [provider, consumer] {
        host.catalog
            .insert_for_test(CatalogEntry {
                manifest: plugin.clone(),
                package_root: PathBuf::new(),
                digest: plugin.id.repeat(64).chars().take(64).collect(),
                embedded_wasm: None,
            })
            .unwrap();
        host.namespaces.register_manifest(&plugin).unwrap();
    }
    host.grants
        .set(
            "project",
            "com.example.consumer",
            &host
                .catalog
                .get("com.example.consumer")
                .unwrap()
                .manifest
                .capabilities,
            ["entity.read".into(), format!("service.call:{service_name}")]
                .into_iter()
                .collect(),
        )
        .unwrap();
    host.services
        .register(
            "com.example.timeline",
            service_name,
            1,
            Arc::new(|request| Ok(serde_json::json!({"resolved": request.payload["date"]}))),
        )
        .unwrap();
    host.activate_bundled("project", "com.example.consumer")
        .unwrap();
    assert_eq!(
        host.call_service(
            "com.example.consumer",
            "project",
            service_name,
            1,
            serde_json::json!({"date": "0042-03-15"}),
            Duration::from_millis(100),
        )
        .unwrap()["resolved"],
        "0042-03-15"
    );
}

#[test]
fn activation_registers_and_deactivation_removes_manifest_declarations() {
    let mut host = host();
    host.activate_bundled("project", "com.example.one").unwrap();
    assert_eq!(
        host.declarations.views("project", "com.example.one"),
        vec![daena_plugin_api::View {
            id: "overview".into(),
            title: "Overview".into(),
            components: vec![],
        }]
    );
    assert_eq!(
        host.declarations.commands("project", "com.example.one"),
        vec![daena_plugin_api::Command {
            id: "refresh".into(),
            title: "Refresh".into(),
            action: None,
            input: None,
            output: None,
            capabilities: vec![],
            exposure: vec![],
        }]
    );
    host.deactivate_bundled("project", "com.example.one");
    assert!(host
        .declarations
        .views("project", "com.example.one")
        .is_empty());
    assert!(host
        .declarations
        .commands("project", "com.example.one")
        .is_empty());
}

#[test]
fn declared_host_commands_are_invokable_only_through_their_view() {
    let mut host = host();
    let entry = host.catalog.entries.get_mut("com.example.one").unwrap();
    entry.manifest.commands[0].action = Some(CommandAction::RefreshView);
    entry.manifest.commands[0].input = Some(daena_plugin_api::CommandSchema {
        schema_type: daena_plugin_api::CommandValueType::Object,
        properties: BTreeMap::from([(
            "reason".into(),
            daena_plugin_api::CommandProperty {
                value_type: daena_plugin_api::CommandValueType::String,
            },
        )]),
        required: vec!["reason".into()],
        additional_properties: false,
    });
    entry.manifest.commands[0].output = Some(daena_plugin_api::CommandSchema {
        schema_type: daena_plugin_api::CommandValueType::Object,
        properties: BTreeMap::from([(
            "type".into(),
            daena_plugin_api::CommandProperty {
                value_type: daena_plugin_api::CommandValueType::String,
            },
        )]),
        required: vec!["type".into()],
        additional_properties: false,
    });
    entry.manifest.commands[0].capabilities = vec!["entity.read".into()];
    entry.manifest.commands[0].exposure = vec![daena_plugin_api::CommandExposure::View];
    entry.manifest.views[0].components = vec![ViewComponent::Button {
        id: "refresh-button".into(),
        label: "Refresh".into(),
        command: "refresh".into(),
    }];
    host.activate_bundled("project", "com.example.one").unwrap();
    assert_eq!(
        host.invoke_command_with_payload(
            "project",
            "com.example.one",
            "overview",
            "refresh",
            serde_json::json!({"reason": "test"}),
        )
        .unwrap(),
        CommandAction::RefreshView
    );
    assert!(host
        .invoke_command_with_payload(
            "project",
            "com.example.one",
            "overview",
            "refresh",
            serde_json::json!({}),
        )
        .is_err());
    assert!(host
        .invoke_command_with_payload(
            "project",
            "com.example.one",
            "overview",
            "refresh",
            serde_json::json!({"reason": "test", "extra": true}),
        )
        .is_err());
    assert!(host
        .invoke_command("project", "com.example.one", "overview", "missing")
        .is_err());
}

#[test]
fn broker_commands_are_not_exposed_as_host_view_buttons() {
    let mut host = host();
    let entry = host.catalog.entries.get_mut("com.example.one").unwrap();
    entry.manifest.commands[0].action = Some(CommandAction::RefreshView);
    entry.manifest.commands[0].exposure = vec![daena_plugin_api::CommandExposure::Broker];
    host.activate_bundled("project", "com.example.one").unwrap();
    assert_eq!(
        host.invoke_broker_command(
            "project",
            "com.example.one",
            "refresh",
            serde_json::json!({}),
        )
        .unwrap(),
        CommandAction::RefreshView
    );
    assert!(host
        .invoke_command("project", "com.example.one", "overview", "refresh")
        .is_err());
}

#[test]
fn host_views_require_active_runtime_and_granted_data_capability() {
    let mut host = host();
    host.catalog
        .entries
        .get_mut("com.example.one")
        .unwrap()
        .manifest
        .views[0]
        .components = vec![daena_plugin_api::ViewComponent::EntityList {
        id: "people".into(),
        title: "People".into(),
        entity_type: "person".into(),
        limit: 10,
    }];

    host.activate_bundled("project", "com.example.one").unwrap();
    assert_eq!(
        host.host_view("project", "com.example.one", "overview")
            .unwrap()
            .components
            .len(),
        1
    );

    host.grants
        .set(
            "project",
            "com.example.one",
            &host
                .catalog
                .get("com.example.one")
                .unwrap()
                .manifest
                .capabilities,
            BTreeSet::new(),
        )
        .unwrap();
    assert!(host
        .host_view("project", "com.example.one", "overview")
        .is_err());
}

#[test]
fn host_field_forms_require_read_and_write_grants() {
    let mut host = host();
    host.catalog
        .entries
        .get_mut("com.example.one")
        .unwrap()
        .manifest
        .views[0]
        .components = vec![
        daena_plugin_api::ViewComponent::EntityList {
            id: "people".into(),
            title: "People".into(),
            entity_type: "person".into(),
            limit: 10,
        },
        daena_plugin_api::ViewComponent::FieldForm {
            id: "summary".into(),
            title: "Summary".into(),
            source: "people".into(),
            namespace: "one".into(),
            fields: vec!["summary".into()],
            editable: true,
        },
    ];
    host.activate_bundled("project", "com.example.one").unwrap();

    host.grants
        .set(
            "project",
            "com.example.one",
            &["entity.read".into(), "field.read:self".into()],
            BTreeSet::new(),
        )
        .unwrap();
    assert!(host
        .host_view("project", "com.example.one", "overview")
        .is_err());

    host.grants
        .set(
            "project",
            "com.example.one",
            &[
                "entity.read".into(),
                "field.read:self".into(),
                "field.write:self".into(),
            ],
            [
                "entity.read".to_string(),
                "field.read:self".to_string(),
                "field.write:self".to_string(),
            ]
            .into_iter()
            .collect(),
        )
        .unwrap();
    assert_eq!(
        host.grants.get("project", "com.example.one"),
        [
            "entity.read".to_string(),
            "field.read:self".to_string(),
            "field.write:self".to_string()
        ]
        .into_iter()
        .collect()
    );
    let result = host.host_view("project", "com.example.one", "overview");
    assert!(result.is_ok(), "host view failed: {:?}", result.err());
}

#[test]
fn capability_mappings_are_stable_for_static_methods() {
    let mut host = host();
    let entry = host.catalog.get("com.example.one").unwrap().clone();
    let session = host.sessions.issue(
        &entry,
        "project",
        "plugin://com.example.one",
        BTreeSet::new(),
        Duration::from_secs(60),
    );
    let empty = serde_json::json!({});
    let cases: &[(&str, &serde_json::Value, &[&str])] = &[
        ("entity.list", &empty, &["entity.read"]),
        ("entity.get", &empty, &["entity.read"]),
        ("entity.update", &empty, &["entity.write"]),
        ("entity.delete", &empty, &["entity.delete"]),
        ("document.list", &empty, &["document.read"]),
        ("document.save", &empty, &["document.write"]),
        ("relationship.list", &empty, &["relationship.read"]),
        ("relationship.create", &empty, &["relationship.write"]),
        ("relationship.delete", &empty, &["relationship.write"]),
        ("search.query", &empty, &["search.query"]),
        ("asset.replace.commit", &empty, &["asset.write:self"]),
        ("asset.transfer.cancel", &empty, &[]),
        ("maps.asset.create.begin", &empty, &["asset.write:self"]),
        ("maps.asset.create.commit", &empty, &["asset.write:self"]),
        ("maps.recovery.export.begin", &empty, &["asset.write:self"]),
        ("maps.recovery.export.commit", &empty, &["asset.write:self"]),
        ("maps.recovery.restore", &empty, &["asset.write:self"]),
        ("maps.recovery.list", &empty, &["asset.read:self"]),
        ("maps.locations.list", &empty, &["asset.read:self"]),
        ("maps.reconcile.links", &empty, &["asset.read:self"]),
    ];
    for (method, payload, expected) in cases {
        assert_eq!(
            required_capabilities(method, payload, &session, &host.namespaces).unwrap(),
            expected.iter().map(|c| c.to_string()).collect::<Vec<_>>(),
            "capability mapping for {method} drifted"
        );
    }
    assert_eq!(
        required_capabilities("project.open", &empty, &session, &host.namespaces)
            .unwrap_err()
            .code,
        "method.unknown"
    );
}

#[test]
fn entity_create_capability_rules_are_stable() {
    let mut host = host();
    let entry = host.catalog.get("com.example.one").unwrap().clone();
    let session = host.sessions.issue(
        &entry,
        "project",
        "plugin://com.example.one",
        BTreeSet::new(),
        Duration::from_secs(60),
    );
    let bare = required_capabilities(
        "entity.create",
        &serde_json::json!({}),
        &session,
        &host.namespaces,
    )
    .unwrap();
    assert_eq!(bare, vec!["entity.write".to_string()]);

    let with_document = required_capabilities(
        "entity.create",
        &serde_json::json!({ "document": { "body": "text" } }),
        &session,
        &host.namespaces,
    )
    .unwrap();
    assert_eq!(
        with_document,
        vec!["entity.write".to_string(), "document.write".to_string()]
    );

    let with_owned_fields = required_capabilities(
        "entity.create",
        &serde_json::json!({
            "fields": [ { "namespace": "one", "key": "summary", "value": "text" } ]
        }),
        &session,
        &host.namespaces,
    )
    .unwrap();
    assert_eq!(
        with_owned_fields,
        vec!["entity.write".to_string(), "field.write:self".to_string()]
    );

    let with_relationships = required_capabilities(
        "entity.create",
        &serde_json::json!({
            "relationships": [ { "relationship_type": "linked", "target_ids": ["e2"] } ]
        }),
        &session,
        &host.namespaces,
    )
    .unwrap();
    assert_eq!(
        with_relationships,
        vec!["entity.write".to_string(), "relationship.write".to_string()]
    );

    let foreign_fields = required_capabilities(
        "entity.create",
        &serde_json::json!({
            "fields": [ { "namespace": "other", "key": "summary", "value": "text" } ]
        }),
        &session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(foreign_fields.code, "namespace.denied");

    let malformed_fields = required_capabilities(
        "entity.create",
        &serde_json::json!({ "fields": "not-an-array" }),
        &session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(malformed_fields.code, "payload.invalid");
}

#[test]
fn field_and_asset_capability_rules_are_stable() {
    let mut owner = manifest("com.example.owner", "owner");
    owner.schemas[0].fields[0].shared = true;
    let mut reader = manifest("com.example.reader", "reader");
    reader.capabilities.push("field.read:shared".into());
    let mut host = PluginHost::new();
    for plugin in [owner.clone(), reader.clone()] {
        host.catalog
            .insert_for_test(CatalogEntry {
                manifest: plugin.clone(),
                package_root: PathBuf::new(),
                digest: plugin.id.repeat(64).chars().take(64).collect(),
                embedded_wasm: None,
            })
            .unwrap();
        host.namespaces.register_manifest(&plugin).unwrap();
    }
    let owner_session = host.sessions.issue(
        host.catalog.get("com.example.owner").unwrap(),
        "project",
        "plugin://owner",
        BTreeSet::new(),
        Duration::from_secs(60),
    );
    let reader_session = host.sessions.issue(
        host.catalog.get("com.example.reader").unwrap(),
        "project",
        "plugin://reader",
        BTreeSet::new(),
        Duration::from_secs(60),
    );

    assert_eq!(
        required_capabilities(
            "field.read",
            &serde_json::json!({ "namespace": "owner", "key": "summary" }),
            &owner_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["field.read:self".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "field.list",
            &serde_json::json!({ "namespace": "owner" }),
            &owner_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["field.read:self".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "field.set",
            &serde_json::json!({ "namespace": "owner", "key": "summary" }),
            &owner_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["field.write:self".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "field.read",
            &serde_json::json!({ "namespace": "owner", "key": "summary" }),
            &reader_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["field.read:shared".to_string()]
    );
    let unshared = required_capabilities(
        "field.read",
        &serde_json::json!({ "namespace": "owner", "key": "other" }),
        &reader_session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(unshared.code, "namespace.denied");
    let foreign_write = required_capabilities(
        "field.set",
        &serde_json::json!({ "namespace": "owner", "key": "summary" }),
        &reader_session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(foreign_write.code, "namespace.denied");

    assert_eq!(
        required_capabilities(
            "asset.list",
            &serde_json::json!({ "namespace": "owner" }),
            &owner_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["asset.read:self".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "asset.register",
            &serde_json::json!({ "namespace": "owner" }),
            &owner_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["asset.register".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "asset.read.begin",
            &serde_json::json!({ "namespace": "owner" }),
            &owner_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["asset.read:self".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "asset.replace.begin",
            &serde_json::json!({ "namespace": "owner" }),
            &owner_session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["asset.write:self".to_string()]
    );
    let foreign_asset = required_capabilities(
        "asset.list",
        &serde_json::json!({ "namespace": "reader" }),
        &owner_session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(foreign_asset.code, "namespace.denied");
    let missing_asset_namespace = required_capabilities(
        "asset.list",
        &serde_json::json!({}),
        &owner_session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(missing_asset_namespace.code, "payload.invalid");
}

#[test]
fn event_and_service_capability_rules_are_stable() {
    let mut host = host();
    let entry = host.catalog.get("com.example.one").unwrap().clone();
    let session = host.sessions.issue(
        &entry,
        "project",
        "plugin://com.example.one",
        BTreeSet::new(),
        Duration::from_secs(60),
    );
    assert_eq!(
        required_capabilities(
            "event.publish",
            &serde_json::json!({ "type": "daena.core/event@1" }),
            &session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["event.publish:daena.core/event@1".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "event.subscribe",
            &serde_json::json!({ "type": "daena.core/event@1" }),
            &session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["event.subscribe:daena.core/event@1".to_string()]
    );
    assert_eq!(
        required_capabilities(
            "event.poll",
            &serde_json::json!({ "type": "daena.core/event@1" }),
            &session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["event.subscribe:daena.core/event@1".to_string()]
    );
    let missing_event_type = required_capabilities(
        "event.publish",
        &serde_json::json!({}),
        &session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(missing_event_type.code, "payload.invalid");
    assert_eq!(
        required_capabilities(
            "service.call",
            &serde_json::json!({ "name": "com.example.calculate", "major": 1 }),
            &session,
            &host.namespaces,
        )
        .unwrap(),
        vec!["service.call:com.example.calculate@1".to_string()]
    );
    let missing_major = required_capabilities(
        "service.call",
        &serde_json::json!({ "name": "com.example.calculate" }),
        &session,
        &host.namespaces,
    )
    .unwrap_err();
    assert_eq!(missing_major.code, "payload.invalid");
}
