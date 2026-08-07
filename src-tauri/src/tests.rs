use super::*;

fn core_migration(manifest: &PluginManifest) -> Result<Option<Migration>, String> {
    Ok(core_migrations(manifest, "")?.into_iter().next())
}

#[test]
fn sanitize_mutation_request_id_keeps_only_uuids() {
    assert_eq!(sanitize_mutation_request_id("maps-fmg-1"), None);
    assert_eq!(sanitize_mutation_request_id(""), None);
    assert_eq!(sanitize_mutation_request_id("not-a-uuid"), None);
    let uuid = "f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11";
    assert_eq!(sanitize_mutation_request_id(uuid), Some(uuid));
}

#[test]
fn bundled_manifests_supply_generic_migrations() {
    let host = bundled_plugin_host(Arc::new(Mutex::new(CoreService::new()))).unwrap();
    let lore = host.catalog.get("daena.lore").unwrap();
    let timeline = host.catalog.get("daena.timeline").unwrap();
    let writing = host.catalog.get("daena.writing").unwrap();
    assert_eq!(
        core_migration(&lore.manifest).unwrap().unwrap().id,
        "lore-v1"
    );
    assert_eq!(
        core_migration(&timeline.manifest).unwrap().unwrap().id,
        "timeline-v1"
    );
    assert_eq!(
        core_migration(&writing.manifest).unwrap().unwrap().id,
        "writing-v1"
    );
}

#[test]
fn bundled_workspace_manifests_do_not_declare_duplicate_sidebar_views() {
    let host = bundled_plugin_host(Arc::new(Mutex::new(CoreService::new()))).unwrap();
    for plugin_id in ["daena.lore", "daena.timeline", "daena.writing"] {
        assert!(
            host.catalog
                .get(plugin_id)
                .unwrap()
                .manifest
                .views
                .is_empty(),
            "{plugin_id} must use host-owned workspace navigation"
        );
    }
}

#[test]
fn plugin_webview_bounds_scale_with_native_viewport() {
    let bounds = PluginWebviewBounds {
        x: 248.0,
        y: 58.0,
        width: 800.0,
        height: 624.0,
        viewport_width: 1440.0,
        viewport_height: 900.0,
    };
    let scaled = scale_plugin_bounds(bounds, 1200.0, 750.0);
    assert!((scaled.x - 248.0 * 1200.0 / 1440.0).abs() < 1e-9);
    assert!((scaled.y - 58.0 * 750.0 / 900.0).abs() < 1e-9);
    assert!((scaled.width - 800.0 * 1200.0 / 1440.0).abs() < 1e-9);
    assert!((scaled.height - 624.0 * 750.0 / 900.0).abs() < 1e-9);
    assert_eq!(scaled.viewport_width, 1200.0);
    assert_eq!(scaled.viewport_height, 750.0);
}

#[test]
fn maps_webview_url_overrides_hidden_bootstrap_dimensions() {
    let manifest: PluginManifest =
        serde_json::from_str(include_str!("../../packages/modules/maps/manifest.json")).unwrap();
    let policy = webview_policy(&manifest).unwrap();
    let url = plugin_webview_url(
        &policy,
        "project",
        None,
        None,
        None,
        PluginWebviewBounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
        },
    )
    .unwrap();
    let tauri::WebviewUrl::External(url) = url else {
        panic!("plugin webview must use an external custom-protocol URL");
    };
    let query = url.query().unwrap();
    assert!(query.contains("width=800"));
    assert!(query.contains("height=600"));
    assert!(query.contains("daena=1"));

    let map_url = plugin_webview_url(
        &policy,
        "project",
        Some("map-workspace"),
        Some("018f89df-b93e-7ad0-a07f-08b1441d1550"),
        Some("f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11"),
        PluginWebviewBounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            viewport_width: 800.0,
            viewport_height: 600.0,
        },
    )
    .unwrap();
    let tauri::WebviewUrl::External(map_url) = map_url else {
        panic!("plugin webview must use an external custom-protocol URL");
    };
    let map_query = map_url.query().unwrap();
    assert!(map_query.contains("view=map-workspace"));
    assert!(map_query.contains("mapEntityId=018f89df-b93e-7ad0-a07f-08b1441d1550"));
    assert!(map_query.contains("linkId=f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11"));
}

#[test]
fn broker_dispatch_uses_plugin_project_authority() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let created = dispatch_module_rpc(
        &mut core,
        "entity.create",
        serde_json::json!({"name": "Broker Entity", "type": "person"}),
        None,
    )
    .unwrap();
    assert_eq!(created["name"], "Broker Entity");
    let entities =
        dispatch_module_rpc(&mut core, "entity.list", serde_json::json!({}), None).unwrap();
    assert_eq!(entities.as_array().unwrap().len(), 1);
    let missing_revision = dispatch_module_rpc(
        &mut core,
        "entity.update",
        serde_json::json!({"id": created["id"]}),
        None,
    )
    .unwrap_err();
    assert!(missing_revision.to_string().contains("expectedRevision"));
}

#[test]
fn binary_transfer_handles_are_bound_one_use_and_chunk_ordered() {
    let mut manager = BinaryTransferManager::default();
    let expired = manager.token(BinaryTransfer::Read {
        plugin_id: "maps".into(),
        session_id: "session".into(),
        bytes: b"expired".to_vec(),
        mime_type: "application/octet-stream".into(),
        expires_at: Instant::now() - Duration::from_secs(1),
    });
    assert!(manager.take_read(&expired, "maps", "session").is_err());
    let read = manager.token(BinaryTransfer::Read {
        plugin_id: "maps".into(),
        session_id: "session".into(),
        bytes: b"map".to_vec(),
        mime_type: "application/octet-stream".into(),
        expires_at: Instant::now() + ASSET_TRANSFER_TTL,
    });
    assert!(manager.take_read(&read, "other", "session").is_err());
    assert!(manager.take_read(&read, "maps", "other-session").is_err());
    assert_eq!(
        manager.take_read(&read, "maps", "session").unwrap().0,
        b"map"
    );
    assert!(manager.take_read(&read, "maps", "session").is_err());

    let upload = manager.token(BinaryTransfer::Upload {
        plugin_id: "maps".into(),
        session_id: "session".into(),
        project_id: "project".into(),
        asset_id: "asset".into(),
        expected_revision: "revision".into(),
        mime_type: "application/octet-stream".into(),
        declared_size: 3,
        next_chunk: 0,
        bytes: Vec::new(),
        expires_at: Instant::now() + ASSET_TRANSFER_TTL,
    });
    assert!(manager
        .append_upload(&upload, "other", "session", 0, b"a")
        .is_err());
    assert!(manager
        .append_upload(&upload, "maps", "other-session", 0, b"a")
        .is_err());
    assert!(manager
        .append_upload(&upload, "maps", "session", 1, b"a")
        .is_err());
    assert_eq!(
        manager
            .append_upload(&upload, "maps", "session", 0, b"ab")
            .unwrap(),
        2
    );
    assert!(manager
        .append_upload(&upload, "maps", "session", 1, b"cd")
        .is_err());
    assert_eq!(
        manager
            .append_upload(&upload, "maps", "session", 1, b"c")
            .unwrap(),
        3
    );
    let (input, bytes, revision) = manager
        .prepare_upload(&upload, "maps", "session", "project", "sha256:placeholder")
        .unwrap();
    assert_eq!(input.asset_id, "asset");
    assert_eq!(bytes, b"abc");
    assert_eq!(revision, "revision");
    assert!(manager
        .prepare_upload(
            &upload,
            "maps",
            "session",
            "other-project",
            "sha256:placeholder"
        )
        .is_err());
    assert!(manager
        .complete_upload(&upload, "maps", "other-session")
        .is_err());
    assert!(manager.complete_upload(&upload, "maps", "session").is_ok());
    assert!(manager.complete_upload(&upload, "maps", "session").is_err());
}

#[test]
fn recovery_uploads_are_bound_chunk_ordered_and_hash_verified() {
    let mut manager = BinaryTransferManager::default();
    let token = manager.token(BinaryTransfer::RecoveryUpload {
        plugin_id: "maps".into(),
        session_id: "session".into(),
        project_id: "project".into(),
        entity_id: "map-entity".into(),
        declared_size: 3,
        next_chunk: 0,
        bytes: Vec::new(),
        expires_at: Instant::now() + ASSET_TRANSFER_TTL,
    });
    assert!(manager
        .append_upload(&token, "other", "session", 0, b"a")
        .is_err());
    assert!(manager
        .append_upload(&token, "maps", "other-session", 0, b"a")
        .is_err());
    assert!(manager
        .append_upload(&token, "maps", "session", 1, b"a")
        .is_err());
    assert_eq!(
        manager
            .append_upload(&token, "maps", "session", 0, b"ab")
            .unwrap(),
        2
    );
    assert!(manager
        .append_upload(&token, "maps", "session", 1, b"cd")
        .is_err());
    assert_eq!(
        manager
            .append_upload(&token, "maps", "session", 1, b"c")
            .unwrap(),
        3
    );
    let good_hash = format!("sha256:{:x}", Sha256::digest(b"abc"));
    assert!(manager
        .prepare_recovery_upload(&token, "maps", "session", "other-project", &good_hash)
        .is_err());
    assert!(manager
        .prepare_recovery_upload(&token, "maps", "session", "project", "sha256:not-the-bytes")
        .is_err());
    let (entity_id, bytes) = manager
        .prepare_recovery_upload(&token, "maps", "session", "project", &good_hash)
        .unwrap();
    assert_eq!(entity_id, "map-entity");
    assert_eq!(bytes, b"abc");
    assert!(manager
        .complete_upload(&token, "maps", "other-session")
        .is_err());
    assert!(manager.complete_upload(&token, "maps", "session").is_ok());
    assert!(manager.complete_upload(&token, "maps", "session").is_err());

    let incomplete = manager.token(BinaryTransfer::RecoveryUpload {
        plugin_id: "maps".into(),
        session_id: "session".into(),
        project_id: "project".into(),
        entity_id: "map-entity".into(),
        declared_size: 4,
        next_chunk: 0,
        bytes: Vec::new(),
        expires_at: Instant::now() + ASSET_TRANSFER_TTL,
    });
    manager
        .append_upload(&incomplete, "maps", "session", 0, b"ab")
        .unwrap();
    assert!(manager
        .prepare_recovery_upload(
            &incomplete,
            "maps",
            "session",
            "project",
            &format!("sha256:{:x}", Sha256::digest(b"ab"))
        )
        .is_err());
}

#[test]
fn bundled_plugin_protocol_serves_only_embedded_assets() {
    let request = tauri::http::Request::builder()
        .uri("plugin://daena.lore/dist/ui/index.html")
        .body(Vec::new())
        .unwrap();
    let response = plugin_asset_response("daena.lore", &request, None, None);
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("Content-Type").unwrap(), "text/html");
    assert!(String::from_utf8_lossy(response.body()).contains("plugin.js"));

    let traversal = tauri::http::Request::builder()
        .uri("plugin://daena.lore/../manifest.json")
        .body(Vec::new())
        .unwrap();
    assert_eq!(
        plugin_asset_response("daena.lore", &traversal, None, None).status(),
        404
    );
}

#[test]
fn bundled_maps_shell_is_deterministic_and_provider_fail_closed() {
    let request = tauri::http::Request::builder()
        .uri("plugin://daena.maps/dist/ui/index.html")
        .body(Vec::new())
        .unwrap();
    let response = plugin_asset_response("daena.maps", &request, None, None);
    let body = String::from_utf8(response.body().clone()).unwrap();
    assert_eq!(response.status(), 200);
    assert!(body.contains("Azgaar's Fantasy Map Generator"));
    assert!(body.contains("daena-bridge.js"));
    assert!(!body.contains("googletagmanager.com"));
    assert!(!body.contains("dataLayer"));
    assert!(
        body.find("<script defer src=\"daena-bridge.js\">").unwrap()
            < body
                .find("<script defer src=\"daena-inline-bootstrap.js\">")
                .unwrap()
    );
    assert!(
        body.find("<script defer src=\"daena-inline-bootstrap.js\">")
            .unwrap()
            < body.find("<script type=\"module\"").unwrap()
    );
    assert!(
        body.find("<base href=\"/dist/ui/fmg/").unwrap()
            < body.find("<script defer src=\"daena-bridge.js\">").unwrap()
    );
    assert!(body.contains("rel=\"stylesheet\"\n      href=\"index.css?v=1.113.1\""));
    assert!(!body.contains("rel=\"preload\""));
    assert_eq!(response.headers().get("Content-Security-Policy").unwrap(), "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; style-src-elem 'self' 'unsafe-inline'; style-src-attr 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; manifest-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'; frame-ancestors 'none'");

    let bridge = tauri::http::Request::builder()
        .uri("plugin://daena.maps/dist/ui/fmg/daena-bridge.js")
        .body(Vec::new())
        .unwrap();
    let bridge_response = plugin_asset_response("daena.maps", &bridge, None, None);
    assert_eq!(bridge_response.status(), 200);
    let bridge_body = String::from_utf8_lossy(bridge_response.body());
    assert!(bridge_body.contains("asset.replace.begin"));
    assert!(bridge_body.contains("requestedMapEntityId"));
    assert!(bridge_body.contains("requested map is unavailable"));
    assert!(bridge_body.contains("metadata.size === 0"));
    assert!(bridge_body.contains("daena-map-diagnostic"));
    assert!(bridge_body.contains("asset.replace.commit"));
    assert!(bridge_body.contains("if (!mapAsset) { await window.generateMapOnLoad?.(); return; }"));
    assert!(bridge_body.contains("Daena Maps provider startup failed"));

    let bootstrap = tauri::http::Request::builder()
        .uri("plugin://daena.maps/dist/ui/fmg/daena-inline-bootstrap.js")
        .body(Vec::new())
        .unwrap();
    let bootstrap_response = plugin_asset_response("daena.maps", &bootstrap, None, None);
    assert_eq!(bootstrap_response.status(), 200);
    let bootstrap_body = String::from_utf8_lossy(bootstrap_response.body());
    assert!(bootstrap_body.contains("element.style.cssText"));
    assert!(bootstrap_body.contains("data-daena-event"));
    assert!(bootstrap_body.contains("element.addEventListener"));
    assert!(bootstrap_body.contains("decodeURIComponent(element.dataset.daenaStyle)"));
    assert!(bootstrap_body.contains("querySelectorAll(\"[data-daena-style]\")"));

    let main = tauri::http::Request::builder()
        .uri("plugin://daena.maps/dist/ui/fmg/main.js")
        .body(Vec::new())
        .unwrap();
    let main_response = plugin_asset_response("daena.maps", &main, None, None);
    assert_eq!(main_response.status(), 200);
    let main_body = String::from_utf8_lossy(main_response.body());
    assert!(main_body.contains("function toggleAssistant()"));
    assert!(main_body.contains("if (DAENA_HOST) return;"));
    assert!(!main_body.contains("openwidget.min.js"));

    let missing = tauri::http::Request::builder()
        .uri("plugin://daena.maps/dist/ui/fmg/not-present.js")
        .body(Vec::new())
        .unwrap();
    assert_eq!(
        plugin_asset_response("daena.maps", &missing, None, None).status(),
        404
    );

    for path in [
        "/dist/ui/index.css",
        "/dist/ui/manifest.webmanifest",
        "/Fantasy-Map-Generator/index-B5l1uyn4.js",
    ] {
        let request = tauri::http::Request::builder()
            .uri(format!("plugin://daena.maps{path}"))
            .body(Vec::new())
            .unwrap();
        assert_eq!(
            plugin_asset_response("daena.maps", &request, None, None).status(),
            200,
            "{path}"
        );
    }
}

#[test]
fn installed_plugin_assets_are_served_from_the_verified_ui_root() {
    let root = std::env::temp_dir().join(format!("daena-protocol-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("dist/ui")).unwrap();
    std::fs::write(root.join("dist/ui/index.html"), b"installed plugin").unwrap();
    let mut manifest: PluginManifest =
        serde_json::from_str(include_str!("../../packages/modules/lore/manifest.json")).unwrap();
    manifest.id = "com.example.third-party".into();
    let request = tauri::http::Request::builder()
        .uri("plugin://com.example.third-party/dist/ui/index.html")
        .body(Vec::new())
        .unwrap();
    let response = plugin_asset_response(
        "com.example.third-party",
        &request,
        Some(&root),
        Some(&manifest),
    );
    assert_eq!(response.status(), 200);
    assert_eq!(response.body(), b"installed plugin");

    let traversal = tauri::http::Request::builder()
        .uri("plugin://com.example.third-party/dist/ui/../../manifest.json")
        .body(Vec::new())
        .unwrap();
    assert_eq!(
        plugin_asset_response(
            "com.example.third-party",
            &traversal,
            Some(&root),
            Some(&manifest),
        )
        .status(),
        404
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn plugin_bootstrap_uses_camel_case_wire_fields() {
    let value = serde_json::to_value(PluginBootstrap {
        rpc_version: daena_plugin_api::RPC_VERSION,
        session_id: "session".into(),
        plugin_id: "daena.lore".into(),
        project_id: "project".into(),
        version: "0.1.0".into(),
        host_api: ">=1.0.0 <2.0.0".into(),
        granted_capabilities: Vec::new(),
        optional_features: Vec::new(),
        package_digest: "digest".into(),
        manifest: serde_json::from_str(include_str!("../../packages/modules/lore/manifest.json"))
            .unwrap(),
    })
    .unwrap();
    assert_eq!(value["sessionId"], "session");
    assert!(value.get("session_id").is_none());
}

#[test]
fn plugin_view_selection_requires_manifest_declaration() {
    let manifest: PluginManifest =
        serde_json::from_str(include_str!("../../examples/plugins/ui/manifest.json")).unwrap();
    assert!(validate_plugin_view(&manifest, None).is_ok());
    assert!(validate_plugin_view(&manifest, Some("ink-tools")).is_ok());
    assert!(validate_plugin_view(&manifest, Some("missing-view")).is_err());
}

#[test]
fn embedded_plugin_bounds_are_finite_and_bounded() {
    let valid = PluginWebviewBounds {
        x: 0.0,
        y: 58.0,
        width: 900.0,
        height: 700.0,
        viewport_width: 1440.0,
        viewport_height: 900.0,
    };
    assert!(valid.validate().is_ok());

    for invalid in [
        PluginWebviewBounds { x: -1.0, ..valid },
        PluginWebviewBounds {
            y: f64::NAN,
            ..valid
        },
        PluginWebviewBounds {
            width: 0.0,
            ..valid
        },
        PluginWebviewBounds {
            height: 10_001.0,
            ..valid
        },
    ] {
        assert!(invalid.validate().is_err());
    }
}

#[test]
fn show_results_navigation_emits_event_payload() {
    let root =
        std::env::temp_dir().join(format!("daena-show-results-test-{}", uuid::Uuid::new_v4()));
    let mut core = CoreService::new();
    core.open_directory(trusted_shell(), &root).unwrap();
    let (map_id, place_id) = {
        let project = core.project(trusted_shell()).unwrap();
        let map = project.create_map("Test Map".into()).unwrap();
        let place = project
            .create_entity(daena_core::CreateEntity {
                name: "Place".into(),
                entity_type: Some("place".into()),
            })
            .unwrap();
        project
            .set_field(daena_core::FieldValue {
                entity_id: place.id.clone(),
                namespace: daena_core::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({
                    "schemaVersion": 1,
                    "locations": [{
                        "id": uuid::Uuid::new_v4().to_string(),
                        "mapEntityId": map.id.clone(),
                        "role": "location",
                        "label": "Test Location",
                        "anchor": {"kind": "point", "point": [0.5, 0.5]},
                        "validity": {"from": null, "to": null}
                    }]
                }),
                revision: String::new(),
            })
            .unwrap();
        (map.id, place.id)
    };
    let request = MapsNavigationRequest {
        operation: "showResults".into(),
        map_entity_id: None,
        entity_id: None,
        link_id: None,
        date: None,
        entity_ids: Some(vec![place_id]),
    };
    let outcome = resolve_maps_navigation(&mut core, &request).unwrap();
    assert_eq!(outcome.emit, Some((map_id, None)));
    assert!(outcome.result.is_ok());

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn maps_asset_create_rpc_round_trips_source_asset() {
    let root = std::env::temp_dir().join(format!("daena-map-create-rpc-{}", uuid::Uuid::new_v4()));
    let core: SharedCore = Arc::new(Mutex::new(CoreService::new()));
    core.lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
    let map_id = {
        let core = core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        project.create_map("Test Map".into()).unwrap().id
    };
    let transfers: SharedBinaryTransfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let session = Session {
        id: "session".into(),
        plugin_id: "daena.maps".into(),
        package_digest: "digest".into(),
        plugin_version: "0.1.0".into(),
        host_api: ">=1.0.0 <2.0.0".into(),
        project_id: "project".into(),
        origin: "plugin:daena.maps".into(),
        grants: std::collections::BTreeSet::new(),
        generation: 1,
        expires_at: std::time::SystemTime::now() + ASSET_TRANSFER_TTL,
        revoked: false,
    };
    let place_id = {
        let core = core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        project
            .create_entity(CreateEntity {
                name: "Place".into(),
                entity_type: Some("place".into()),
            })
            .unwrap()
            .id
    };
    assert!(
        dispatch_binary_asset_rpc(
            &core,
            &transfers,
            &session,
            "maps.asset.create.begin",
            serde_json::json!({"mapEntityId": place_id, "size": 5}),
            None,
        )
        .is_err(),
        "a non-map entity must be rejected"
    );

    let begin = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.asset.create.begin",
        serde_json::json!({"mapEntityId": map_id, "size": 5}),
        None,
    )
    .unwrap();
    let handle = begin["handle"].as_str().unwrap().to_string();
    assert!(begin["url"].as_str().unwrap().starts_with(&format!(
        "plugin://daena.maps/__asset/{handle}/0?sessionId=session"
    )));
    {
        let mut manager = transfers.lock().unwrap();
        assert_eq!(
            manager
                .append_upload(&handle, "daena.maps", "session", 0, b"fmg-!")
                .unwrap(),
            5
        );
    }

    let saved = dispatch_binary_asset_rpc(
            &core,
            &transfers,
            &session,
            "maps.asset.create.commit",
            serde_json::json!({"handle": handle, "contentHash": format!("sha256:{:x}", Sha256::digest(b"fmg-!"))}),
            None,
        )
        .unwrap();
    let saved_asset: Asset = serde_json::from_value(saved).unwrap();
    assert_eq!(saved_asset.namespace, daena_core::maps::MAP_NAMESPACE);
    assert_eq!(saved_asset.size, 5);

    {
        let core = core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        let asset = project.asset(saved_asset.id.clone()).unwrap();
        assert_eq!(asset.size, 5);
        let info = project.info().unwrap();
        let path = daena_core::normalized_project_path(Path::new(&info.root), &asset.path).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"fmg-!");
        let descriptor = project
            .list_fields(map_id.clone())
            .unwrap()
            .into_iter()
            .find(|field| field.namespace == daena_core::maps::MAP_NAMESPACE && field.key == "map")
            .unwrap();
        assert_eq!(
            descriptor.value["sourceAssetId"],
            serde_json::Value::Null,
            "commit must not touch the descriptor; only the first save may"
        );
    }

    let read = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "asset.read.begin",
        serde_json::json!({"assetId": saved_asset.id, "namespace": "maps"}),
        None,
    )
    .unwrap();
    assert_eq!(read["size"], 5);

    {
        let core = core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        project
                .set_field(FieldValue {
                    entity_id: map_id,
                    namespace: daena_core::maps::MAP_NAMESPACE.into(),
                    key: "map".into(),
                    value: serde_json::json!({
                        "schemaVersion": 1,
                        "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                        "sourceAssetId": saved_asset.id,
                        "previewAssetId": null,
                        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
                    }),
                    revision: String::new(),
                })
                .unwrap();
    }

    std::fs::remove_dir_all(root).ok();
}
