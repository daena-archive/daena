use super::*;

#[test]
fn physical_jobs_are_session_scoped_and_expire() {
    let mut manager = PhysicalJobManager::default();
    let first_session = manager.begin_session("project-a".into());
    let cancelled = Arc::new(AtomicBool::new(false));
    manager.jobs.insert(
        "job-a".into(),
        PhysicalJob {
            project_id: "project-a".into(),
            session_id: first_session.clone(),
            expires_at: Instant::now() - Duration::from_secs(1),
            cancel: cancelled.clone(),
            status: PhysicalJobStatus {
                job_id: "job-a".into(),
                request_id: "request-a".into(),
                state: "running".into(),
                stage: "Building terrain".into(),
                completed: 0,
                total: 1,
                error: None,
                error_code: None,
                physical_identity: None,
            },
            result: None,
        },
    );

    manager.reap_expired();
    assert!(cancelled.load(Ordering::Relaxed));
    assert!(!manager.jobs.contains_key("job-a"));

    let second_session = manager.begin_session("project-b".into());
    assert_ne!(first_session, second_session);
    assert!(manager.active_session_matches("project-b", &second_session));
    assert!(!manager.active_session_matches("project-a", &second_session));
}

#[test]
fn physical_climate_products_expose_derived_fields_without_source_data() {
    let grid = daena_physical::Grid::new(8, 4, daena_physical::DEFAULT_RADIUS_METRES).unwrap();
    let mut elevations = vec![500; grid.sample_count()];
    elevations[0] = -2_000;
    let field = daena_physical::PhysicalField {
        grid,
        seed: 831_429,
        retry_index: 0,
        target_land_fraction_ppm: 300_000,
        sea_level_mm: 0,
        elevations_mm: elevations,
    };
    let mut progress = daena_physical::NoopProgress;
    let climate = daena_physical::climate::derive_current_climate(
        &field,
        daena_physical::climate::ClimateSettings::default_for(grid),
        field.seed,
        field.retry_index,
        &mut progress,
    )
    .unwrap();
    let products = physical_climate_products(&climate);
    assert_eq!(products["derivationVersion"], 1);
    assert_eq!(products["width"], 8);
    assert_eq!(products["height"], 4);
    for key in [
        "temperatureCentiC",
        "moistureMmPerYear",
        "precipitationMmPerYear",
        "runoffMmPerYear",
        "runoffVolumeM3PerYear",
    ] {
        assert_eq!(products[key].as_array().unwrap().len(), grid.sample_count());
    }
    assert!(
        products["metrics"]["precipitationVolumeM3PerYear"]
            .as_u64()
            .unwrap()
            > 0
    );
}

#[test]
fn physical_evolution_products_expose_routing_and_before_after_fields() {
    let settings = daena_physical::GenerationSettings {
        width: 8,
        height: 4,
        radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    let mut progress = daena_physical::NoopProgress;
    let world = daena_physical::generate_world(settings, 831_429, 0, &mut progress).unwrap();
    let products = physical_evolution_products(&world.evolution);
    assert_eq!(
        products["derivationVersion"],
        daena_physical::evolution::EVOLUTION_DERIVATION_VERSION
    );
    assert_eq!(products["preset"], "mature");
    for key in [
        "beforeElevationsMm",
        "elevationsMm",
        "routingElevationMm",
        "fillDepthMm",
        "slopePpm",
        "accumulationM3PerYear",
    ] {
        assert_eq!(
            products[key].as_array().unwrap().len(),
            settings.width as usize * settings.height as usize
        );
    }
    assert_eq!(products["drainageMetrics"]["convergencePpm"], 1_000_000);
    assert!(!products["edges"].as_array().unwrap().is_empty());
}

#[test]
fn physical_hydrology_products_expose_current_water_and_renderer_fields() {
    let settings = daena_physical::GenerationSettings {
        width: 8,
        height: 4,
        radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    let mut progress = daena_physical::NoopProgress;
    let world = daena_physical::generate_world(settings, 831_429, 0, &mut progress).unwrap();
    let products = physical_hydrology_products(&world.hydrology);
    assert_eq!(
        products["derivationVersion"],
        daena_physical::hydrology::HYDROLOGY_DERIVATION_VERSION
    );
    for key in [
        "waterLevelMm",
        "lakeLevelMm",
        "hillshadePpm",
        "bathymetryMm",
        "watershedId",
        "lakeCells",
        "iceCells",
        "iceThicknessMm",
        "islandId",
    ] {
        assert_eq!(
            products[key].as_array().unwrap().len(),
            settings.width as usize * settings.height as usize
        );
    }
    assert_eq!(products["metrics"]["converged"], true);
    assert!(products["metrics"]["toleranceM3"].as_u64().unwrap() > 0);
    assert!(products["metrics"]["watershedCount"].as_u64().unwrap() > 0);
    assert!(products["metrics"]["islandCount"].as_u64().unwrap() > 0);
    let geojson: serde_json::Value = serde_json::from_str(&world.derived_geojson).unwrap();
    assert_eq!(geojson["type"], "FeatureCollection");
    fn walk(value: &serde_json::Value) {
        if let Some(pair) = value.as_array() {
            if pair.len() >= 2 && pair[0].is_number() && pair[1].is_number() {
                let lon = pair[0].as_f64().unwrap();
                let lat = pair[1].as_f64().unwrap();
                assert!((-180.0..=180.0).contains(&lon));
                assert!((-90.0..=90.0).contains(&lat));
            } else {
                for item in pair {
                    walk(item);
                }
            }
        }
    }
    for feature in geojson["features"].as_array().unwrap() {
        walk(&feature["geometry"]["coordinates"]);
    }
}

#[test]
fn reopened_physical_hydrology_matches_the_generated_fixture() {
    let settings = daena_physical::GenerationSettings {
        width: 8,
        height: 4,
        radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    let mut progress = daena_physical::NoopProgress;
    let generated = daena_physical::generate_world(settings, 831_429, 0, &mut progress).unwrap();
    let generation = serde_json::json!({"settings": {
        "evolutionPreset": "mature",
        "historicalForcing": historical_forcing_products(
            daena_physical::history::HistoricalForcingParameters::default_for(831_429, 0)
        )
    }});
    let (derived_geojson, reopened) = derive_reopened_hydrology(
        &generated.tectonics,
        &generation,
        generated.report.reference_water_inventory_m3,
    )
    .unwrap();
    assert_eq!(derived_geojson, generated.derived_geojson);
    assert_eq!(reopened, generated.hydrology);
}

#[test]
fn reopened_historical_products_preserve_terrain_and_report_water_balance() {
    let settings = daena_physical::GenerationSettings {
        width: 8,
        height: 4,
        radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    let mut progress = daena_physical::NoopProgress;
    let generated = daena_physical::generate_world(settings, 831_429, 0, &mut progress).unwrap();
    let before = generated.tectonics.physical_field();
    let parameters = daena_physical::history::HistoricalForcingParameters::default_for(831_429, 0);
    let generation = serde_json::json!({
        "id": daena_core::maps::PHYSICAL_GENERATOR_ID,
        "version": daena_core::maps::PHYSICAL_GENERATOR_VERSION,
        "seed": 831_429,
        "retryIndex": 0,
        "settings": {
            "width": settings.width,
            "height": settings.height,
            "radiusMetres": settings.radius_metres,
            "targetLandFractionPpm": settings.target_land_fraction_ppm,
            "referenceWaterInventoryM3": generated.report.reference_water_inventory_m3,
            "plateCount": generated.tectonics.settings.plate_count,
            "continentalPlateCount": generated.tectonics.settings.continental_plate_count,
            "tectonicActivityPpm": generated.tectonics.settings.tectonic_activity_ppm,
            "islandActivityPpm": generated.tectonics.settings.island_activity_ppm,
            "evolutionPreset": "mature",
            "historicalForcing": historical_forcing_products(parameters)
        }
    });
    let mut history_progress = daena_physical::NoopProgress;
    let (historical, persisted, geojson) = derive_reopened_historical(
        &generated.tectonics,
        &generation,
        generated.report.reference_water_inventory_m3,
        parameters.period_years() / 4,
        &mut history_progress,
    )
    .unwrap();
    assert_eq!(persisted, parameters);
    assert!(!geojson.is_empty());
    assert_eq!(generated.tectonics.physical_field(), before);
    assert!(historical.hydrology.metrics.converged);
    assert!(historical.metrics.balance_error_m3 <= historical.hydrology.metrics.tolerance_m3);
    let source = daena_physical::tectonics::encode_source_v2(&generated.tectonics).unwrap();
    let source_hash = format!("sha256:{:x}", Sha256::digest(&source));
    let physical_identity = daena_core::maps::physical::validate_source(&source, &generation)
        .unwrap()
        .identity;
    let cache_key = historical_cache_key(
        &physical_identity,
        persisted,
        historical.metrics.normalized_epoch,
    );
    let response = historical_response(
        &generated.tectonics,
        &source_hash,
        &physical_identity,
        cache_key,
        historical.metrics.epoch_offset_years,
        historical.metrics.normalized_epoch,
        &historical,
        persisted,
        geojson.clone(),
    );
    assert_eq!(response["derivedHashes"]["canonicalSource"], source_hash);
    assert_eq!(response["derivedHashes"]["tectonics"], source_hash);
    assert_eq!(
        response["hazards"]["derivationVersion"],
        daena_physical::hazards::HAZARD_DERIVATION_VERSION
    );
    assert_eq!(response["hazards"]["model"], "relative-generated-v1");
    let cache_key = response["cacheKey"].as_str().unwrap();
    assert!(cache_key.contains(&physical_identity));
    assert!(cache_key.contains("history-v2"));
    assert!(cache_key.contains("hazards-v3"));
    assert!(cache_key.contains(&format!("epoch:{}", historical.metrics.normalized_epoch)));
    assert_eq!(response["chronology"]["contractVersion"], 1);
    assert_eq!(response["chronology"]["kind"], "physical-offset-years");
    assert_eq!(response["chronology"]["reference"], "accepted-source");
    assert_eq!(
        response["derivedHashes"]["geography"]
            .as_str()
            .unwrap()
            .len(),
        71
    );
    let repeated = historical_response(
        &generated.tectonics,
        response["sourceHash"].as_str().unwrap(),
        response["physicalIdentity"].as_str().unwrap(),
        response["cacheKey"].as_str().unwrap().to_string(),
        historical.metrics.epoch_offset_years,
        historical.metrics.normalized_epoch,
        &historical,
        persisted,
        geojson,
    );
    assert_eq!(response["derivedHashes"], repeated["derivedHashes"]);
    let mut cold_progress = daena_physical::NoopProgress;
    let (cold, cold_parameters, cold_geojson) = derive_reopened_historical(
        &generated.tectonics,
        &generation,
        generated.report.reference_water_inventory_m3,
        -persisted.period_years() / 4,
        &mut cold_progress,
    )
    .unwrap();
    let cold_response = historical_response(
        &generated.tectonics,
        &source_hash,
        &physical_identity,
        historical_cache_key(
            &physical_identity,
            cold_parameters,
            cold.metrics.normalized_epoch,
        ),
        cold.metrics.epoch_offset_years,
        cold.metrics.normalized_epoch,
        &cold,
        cold_parameters,
        cold_geojson,
    );
    assert_eq!(
        response["derivedHashes"]["finalElevation"],
        cold_response["derivedHashes"]["finalElevation"]
    );
    assert_eq!(
        response["derivedHashes"]["tectonics"],
        cold_response["derivedHashes"]["tectonics"]
    );
    assert!(
        cold_response["history"]["landIceM3"].as_u64().unwrap()
            > response["history"]["landIceM3"].as_u64().unwrap()
    );
}

#[test]
fn clearing_historical_cache_is_disposable_and_does_not_touch_sources() {
    historical_epoch_cache().lock().unwrap().insert(
        "test-cache-entry".into(),
        serde_json::json!({"derived": true}),
    );
    clear_historical_epoch_cache().unwrap();
    assert!(historical_epoch_cache().lock().unwrap().is_empty());
}

#[test]
fn superseded_historical_requests_cancel_at_progress_checkpoints() {
    let generation = Arc::new(AtomicU64::new(1));
    let progress = HistoricalProgress {
        generation: generation.clone(),
        expected: 1,
        reporter: None,
    };
    generation.store(2, Ordering::Release);
    assert!(matches!(
        daena_physical::ProgressSink::check_cancelled(&progress),
        Err(daena_physical::PhysicalError::Cancelled)
    ));
}

#[test]
fn stopping_project_watcher_releases_resources_without_joining() {
    let (stop, _receiver) = mpsc::channel();
    let watcher = Arc::new(Mutex::new(ProjectWatcher {
        stop: Some(stop),
        filesystem: None,
    }));

    stop_project_watcher(&watcher).unwrap();

    let watcher = watcher.lock().unwrap();
    assert!(watcher.stop.is_none());
    assert!(watcher.filesystem.is_none());
}

#[test]
fn watcher_filters_runtime_and_editor_paths_without_reconciliation() {
    let root = std::path::Path::new("/tmp/daena-watcher-test");
    assert_eq!(
        watched_portable_path(root, &root.join("entities/one/entity.json")),
        Some("entities/one/entity.json".into())
    );
    for path in [
        ".daena/index.sqlite",
        ".git/index",
        "entities/.DS_Store",
        "entities/one/.entity.json.swp",
        "notes.txt",
    ] {
        assert_eq!(
            watched_portable_path(root, &root.join(path)),
            None,
            "{path}"
        );
    }
}

#[test]
fn watcher_startup_snapshot_distinguishes_initial_state_from_external_changes() {
    let root =
        std::env::temp_dir().join(format!("daena-watcher-snapshot-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("entities")).unwrap();
    std::fs::write(root.join("project.json"), b"initial").unwrap();

    let snapshot = portable_tree_snapshot(&root).unwrap();
    assert_eq!(
        portable_path_fingerprint(&root, "project.json").unwrap(),
        snapshot.get("project.json").cloned()
    );

    std::fs::write(root.join("project.json"), b"external").unwrap();
    assert_ne!(
        portable_path_fingerprint(&root, "project.json").unwrap(),
        snapshot.get("project.json").cloned()
    );
    assert_eq!(
        portable_path_fingerprint(&root, "entities/new/entity.json").unwrap(),
        None
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn watcher_accepts_the_app_owned_checkpoint_but_flags_external_edits() {
    let root =
        std::env::temp_dir().join(format!("daena-watcher-checkpoint-{}", uuid::Uuid::new_v4()));
    let project = ProjectStore::open_directory(&root).unwrap();
    project
        .create_entity(CreateEntity {
            name: "Checkpoint owner".into(),
            entity_type: None,
        })
        .unwrap();
    project.flush_checkpoint("watcher test").unwrap();
    assert!(portable_checkpoint_is_current(&root));

    std::fs::write(root.join("project.json"), b"external edit").unwrap();
    assert!(!portable_checkpoint_is_current(&root));

    drop(project);
    std::fs::remove_dir_all(root).unwrap();
}

fn ai_test_host() -> SharedPluginHost {
    Arc::new(Mutex::new(bundled_plugin_host(new_shared_core()).unwrap()))
}

#[test]
fn ai_index_lifecycle_is_project_bound_and_non_blocking() {
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::default()));
    assert!(!ai::index_status(&runtime).available);
    let root =
        std::env::temp_dir().join(format!("daena-ai-index-lifecycle-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    ai::attach_project_index(&runtime, root.to_str().unwrap());
    let status = ai::index_status(&runtime);
    assert!(status.available);
    assert_eq!(status.state, Some(daena_ai::index::IndexState::Absent));
    assert!(root.join(".daena/ai/index.sqlite").is_file());
    ai::detach_project_index(&runtime);
    assert!(!ai::index_status(&runtime).available);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ai_gate_fails_closed_until_project_opt_in() {
    // Missing manifest → fail closed.
    let missing =
        std::env::temp_dir().join(format!("daena-ai-gate-missing-{}", uuid::Uuid::new_v4()));
    assert!(ensure_project_ai_enabled(missing.to_str().unwrap()).is_err());

    // Fresh project defaults to disabled.
    let root = std::env::temp_dir().join(format!("daena-ai-gate-{}", uuid::Uuid::new_v4()));
    {
        let project = ProjectStore::open_directory(&root).unwrap();
        assert!(ensure_project_ai_enabled(root.to_str().unwrap()).is_err());
        drop(project);
        let project = ProjectStore::open_directory(&root).unwrap();
        project.set_ai_enabled(true).unwrap();
    }
    assert!(ensure_project_ai_enabled(root.to_str().unwrap()).is_ok());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ai_toggle_works_through_read_only_store_like_the_command_path() {
    // with_read_project hands commands an open_read_only store; the toggle
    // must still persist through it because project.json is a plain file.
    let root = std::env::temp_dir().join(format!("daena-ai-toggle-ro-{}", uuid::Uuid::new_v4()));
    ProjectStore::open_directory(&root).unwrap();
    let reader = ProjectStore::open_read_only(&root).unwrap();
    assert!(!reader.info().unwrap().ai_enabled);
    let info = reader.set_ai_enabled(true).unwrap();
    assert!(info.ai_enabled);
    drop(reader);
    let reader = ProjectStore::open_read_only(&root).unwrap();
    assert!(reader.info().unwrap().ai_enabled);
    std::fs::remove_dir_all(root).unwrap();
}

fn ai_test_context(runtime: ai::SharedAiRuntime) -> AiBrokerContext {
    // Broker tests need a real, AI-enabled project: the ai.request.start gate
    // reads the canonical manifest and fails closed without an opt-in.
    static PROJECT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let root = PROJECT.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("daena-ai-broker-{}", uuid::Uuid::new_v4()));
        ProjectStore::open_directory(&root)
            .unwrap()
            .set_ai_enabled(true)
            .unwrap();
        root
    });
    AiBrokerContext {
        app: None,
        core: None,
        settings: None,
        ai_runtime: runtime,
        session_id: "test-session".into(),
        caller: daena_ai::AiCaller::authorized_plugin(
            "daena.lore",
            root.to_str().unwrap(),
            Vec::new(),
            vec![format!("project:{}", root.to_string_lossy())],
            1,
            "pending",
        ),
    }
}

fn wait_ai_terminal(runtime: &ai::SharedAiRuntime, request_id: &str) -> Vec<ai::AiStreamEvent> {
    for _ in 0..100 {
        let events = ai::poll_ai_events(runtime, request_id).unwrap();
        if events.iter().any(|event| {
            matches!(
                event.phase.as_str(),
                "completed" | "failed" | "cancelled" | "deadline_exceeded"
            )
        }) {
            return events;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("AI test provider did not reach a terminal state")
}

struct CancellationProvider;

impl ai::AiProvider for CancellationProvider {
    fn generate(
        &self,
        request: ai::ProviderRequest,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Vec<ai::AiStreamEvent> {
        for _ in 0..100 {
            if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                return vec![ai::AiStreamEvent {
                    sequence: 0,
                    request_id: request.request_id,
                    phase: "cancelled".into(),
                    delta: None,
                    output: None,
                    error: None,
                }];
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        vec![ai::AiStreamEvent {
            sequence: 0,
            request_id: request.request_id.clone(),
            phase: "completed".into(),
            delta: None,
            output: Some("late".into()),
            error: None,
        }]
    }
}

struct OversizedProvider;

impl ai::AiProvider for OversizedProvider {
    fn generate(
        &self,
        request: ai::ProviderRequest,
        _cancelled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Vec<ai::AiStreamEvent> {
        vec![ai::AiStreamEvent {
            sequence: 0,
            request_id: request.request_id,
            phase: "completed".into(),
            delta: None,
            output: Some("x".repeat(daena_ai::DEFAULT_LIMITS.max_output_bytes + 1)),
            error: None,
        }]
    }
}

#[test]
fn broker_ai_lifecycle_supports_text_and_structured_requests_without_provider() {
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        ai::FakeLoopbackProvider,
    ))));
    let plugins = ai_test_host();
    let text = dispatch_host_rpc(&plugins, "daena.lore", "project", "ai.request.start", serde_json::json!({
        "operation": "generate_text", "taskId": "text", "userInstruction": "rewrite", "immediateContext": {"selection": "hello"}
    }), ai_test_context(runtime.clone())).unwrap();
    let text_id = text["requestId"].as_str().unwrap();
    let text_events = wait_ai_terminal(&runtime, text_id);
    assert_eq!(text_events.last().unwrap().phase, "completed");
    let text_result = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.result",
        serde_json::json!({"requestId": text_id}),
        ai_test_context(runtime.clone()),
    )
    .unwrap();
    assert!(text_result["output"].as_str().unwrap().contains("hello"));

    let contract = serde_json::json!({"type":"object","properties":{"summary":{"type":"string","maxLength":4000}},"required":["summary"],"additionalProperties":false});
    let structured = dispatch_host_rpc(&plugins, "daena.lore", "project", "ai.request.start", serde_json::json!({
        "operation": "generate_structured", "taskId": "structured", "userInstruction": "draft", "immediateContext": {"entity":"Ada"}, "outputContract": contract
    }), ai_test_context(runtime.clone())).unwrap();
    let structured_id = structured["requestId"].as_str().unwrap();
    assert_eq!(
        wait_ai_terminal(&runtime, structured_id)
            .last()
            .unwrap()
            .phase,
        "completed"
    );
    let structured_result = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.result",
        serde_json::json!({"requestId": structured_id}),
        ai_test_context(runtime),
    )
    .unwrap();
    assert!(structured_result["summary"]
        .as_str()
        .unwrap()
        .contains("Ada"));
}

#[test]
fn broker_ai_request_ids_are_session_bound() {
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        ai::FakeLoopbackProvider,
    ))));
    let plugins = ai_test_host();
    let started = dispatch_host_rpc(&plugins, "daena.lore", "project", "ai.request.start", serde_json::json!({
        "operation": "generate_text", "taskId": "text", "userInstruction": "rewrite", "immediateContext": {"selection": "hello"}
    }), ai_test_context(runtime.clone())).unwrap();
    let request_id = started["requestId"].as_str().unwrap();
    let mut other = ai_test_context(runtime);
    other.session_id = "other-session".into();
    let error = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.poll",
        serde_json::json!({"requestId": request_id}),
        other,
    )
    .unwrap_err();
    assert!(error.contains("not bound"));
}

#[test]
fn broker_ai_rejects_invalid_schema_and_cancels_through_lifecycle() {
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        CancellationProvider,
    ))));
    let plugins = ai_test_host();
    let invalid = dispatch_host_rpc(&plugins, "daena.lore", "project", "ai.request.start", serde_json::json!({
        "operation": "generate_structured", "taskId": "bad", "userInstruction": "draft", "immediateContext": {}, "outputContract": {"type":"object"}
    }), ai_test_context(runtime.clone())).unwrap_err();
    assert!(invalid.contains("requires properties"));

    let started = dispatch_host_rpc(&plugins, "daena.lore", "project", "ai.request.start", serde_json::json!({
        "operation": "generate_text", "taskId": "cancel", "userInstruction": "rewrite", "immediateContext": {"selection": "hello"}
    }), ai_test_context(runtime.clone())).unwrap();
    let request_id = started["requestId"].as_str().unwrap();
    dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.cancel",
        serde_json::json!({"requestId": request_id}),
        ai_test_context(runtime.clone()),
    )
    .unwrap();
    assert_eq!(
        wait_ai_terminal(&runtime, request_id).last().unwrap().phase,
        "cancelled"
    );
    let result = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.result",
        serde_json::json!({"requestId": request_id}),
        ai_test_context(runtime),
    );
    assert!(result.is_err());
}

#[test]
fn broker_ai_deadline_emits_terminal_deadline_event() {
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        CancellationProvider,
    ))));
    let plugins = ai_test_host();
    let started = dispatch_host_rpc(&plugins, "daena.lore", "project", "ai.request.start", serde_json::json!({
        "operation": "generate_text", "taskId": "deadline", "userInstruction": "rewrite", "immediateContext": {"selection": "hello"}, "deadlineMs": 5
    }), ai_test_context(runtime.clone())).unwrap();
    let request_id = started["requestId"].as_str().unwrap();
    assert_eq!(
        wait_ai_terminal(&runtime, request_id).last().unwrap().phase,
        "deadline_exceeded"
    );
    assert!(ai::ai_request_result(&runtime, request_id).is_err());
}

#[test]
fn retrieval_context_is_provenance_bearing_and_denies_missing_data_grants() {
    let project = ProjectStore::in_memory().unwrap();
    let entity = project
        .create_entity(CreateEntity {
            name: "Retrieval target".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    project
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "The archivist guards the salt library.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let policy = daena_plugin_api::AiRetrievalPolicyPayload {
        mode: daena_plugin_api::AiRetrievalMode::ExplicitOnly,
        query: None,
        seed_ids: vec![entity.id.clone()],
        allowed_source_kinds: vec!["document".into()],
        relationship_depth: 0,
        passage_count: 4,
        include_shared_fields: false,
    };
    let caller = daena_ai::AiCaller::authorized_plugin(
        "daena.lore",
        "project",
        vec!["document.read".into()],
        vec!["project:project".into()],
        1,
        "request",
    );
    let (context, citations) = ai::build_retrieval_context(&project, &caller, &policy).unwrap();
    assert!(context.contains("[UNTRUSTED_PROJECT_DATA]"));
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].entity_id.as_deref(), Some(entity.id.as_str()));
    assert!(citations[0].document_id.is_some());

    let denied = daena_ai::AiCaller::authorized_plugin(
        "daena.lore",
        "project",
        Vec::new(),
        Vec::new(),
        1,
        "request",
    );
    assert_eq!(
        ai::build_retrieval_context(&project, &denied, &policy).unwrap_err(),
        "RemoteContextDenied"
    );

    let related_policy = daena_plugin_api::AiRetrievalPolicyPayload {
        mode: daena_plugin_api::AiRetrievalMode::Related,
        query: None,
        seed_ids: vec![entity.id],
        allowed_source_kinds: vec!["document".into()],
        relationship_depth: 1,
        passage_count: 4,
        include_shared_fields: false,
    };
    assert_eq!(
        ai::build_retrieval_context(&project, &caller, &related_policy).unwrap_err(),
        "RemoteContextDenied"
    );
}

#[test]
fn related_retrieval_limits_lexical_matches_to_the_entity_neighborhood() {
    let project = ProjectStore::in_memory().unwrap();
    let seed = project
        .create_entity(CreateEntity {
            name: "John".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let related = project
        .create_entity(CreateEntity {
            name: "Mars".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let unrelated = project
        .create_entity(CreateEntity {
            name: "Venus".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    project
        .create_relationship(RelationshipInput {
            source_id: seed.id.clone(),
            target_id: related.id.clone(),
            relationship_type: "lives_on".into(),
            metadata: None,
        })
        .unwrap();
    project
        .save_document(SaveDocument {
            entity_id: related.id.clone(),
            body: "Many Mars residents are farmers.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    project
        .save_document(SaveDocument {
            entity_id: unrelated.id,
            body: "Many Venus residents are farmers.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();

    let caller = daena_ai::AiCaller::authorized_plugin(
        "daena.lore",
        "project",
        vec![
            "document.read".into(),
            "search.query".into(),
            "relationship.read".into(),
        ],
        vec!["project:project".into()],
        1,
        "related-query",
    );
    let policy = daena_plugin_api::AiRetrievalPolicyPayload {
        mode: daena_plugin_api::AiRetrievalMode::Related,
        query: Some("farmers".into()),
        seed_ids: vec![seed.id],
        allowed_source_kinds: vec!["document".into()],
        relationship_depth: 1,
        passage_count: 8,
        include_shared_fields: false,
    };
    let (context, citations) = ai::build_retrieval_context(&project, &caller, &policy).unwrap();
    assert!(context.contains("Many Mars residents are farmers."));
    assert!(!context.contains("Many Venus residents are farmers."));
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].entity_id.as_deref(), Some(related.id.as_str()));
}

#[test]
fn related_retrieval_supports_two_hops_and_preserves_relationship_context() {
    let project = ProjectStore::in_memory().unwrap();
    let john = project
        .create_entity(CreateEntity {
            name: "John".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let city = project
        .create_entity(CreateEntity {
            name: "City A".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let country = project
        .create_entity(CreateEntity {
            name: "Country B".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    project
        .create_relationship(RelationshipInput {
            source_id: john.id.clone(),
            target_id: city.id.clone(),
            relationship_type: "lives_in".into(),
            metadata: None,
        })
        .unwrap();
    project
        .create_relationship(RelationshipInput {
            source_id: city.id.clone(),
            target_id: country.id.clone(),
            relationship_type: "located_in".into(),
            metadata: None,
        })
        .unwrap();
    project
        .save_document(SaveDocument {
            entity_id: country.id,
            body: "English is commonly spoken here.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();

    let caller = daena_ai::AiCaller::authorized_plugin(
        "daena.lore",
        "project",
        vec!["document.read".into(), "relationship.read".into()],
        vec!["project:project".into()],
        1,
        "two-hop-query",
    );
    let policy = daena_plugin_api::AiRetrievalPolicyPayload {
        mode: daena_plugin_api::AiRetrievalMode::Related,
        query: None,
        seed_ids: vec![john.id],
        allowed_source_kinds: vec!["document".into(), "relationship".into()],
        relationship_depth: 2,
        passage_count: 8,
        include_shared_fields: false,
    };
    let (context, citations) = ai::build_retrieval_context(&project, &caller, &policy).unwrap();
    assert!(context.contains("John --lives_in--> City A"));
    assert!(context.contains("City A --located_in--> Country B"));
    assert!(context.contains("English is commonly spoken here."));
    assert!(citations
        .iter()
        .any(|citation| citation.source_kind == "relationship"));
}

#[test]
fn relationship_citations_use_metadata_labels_without_removing_raw_context() {
    let mut project = ProjectStore::in_memory().unwrap();
    let source = project
        .create_entity(CreateEntity {
            name: "Artifact".into(),
            entity_type: Some("artifact".into()),
        })
        .unwrap();
    let target = project
        .create_entity(CreateEntity {
            name: "Alice".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    project
        .set_relationship_metadata_schemas(BTreeMap::from([(
            "owned_by".into(),
            vec![
                MetadataFieldDefinition {
                    key: "validFrom".into(),
                    label: "Valid from".into(),
                    field_type: "date".into(),
                    required: None,
                    options: None,
                    one_of: None,
                },
                MetadataFieldDefinition {
                    key: "validTo".into(),
                    label: "Valid to".into(),
                    field_type: "date".into(),
                    required: None,
                    options: None,
                    one_of: None,
                },
            ],
        )]))
        .unwrap();
    project
        .create_relationship(RelationshipInput {
            source_id: source.id.clone(),
            target_id: target.id,
            relationship_type: "owned_by".into(),
            metadata: Some(
                serde_json::json!({
                    "validFrom": "2024-01-01",
                    "validTo": "2024-06-01"
                })
                .to_string(),
            ),
        })
        .unwrap();

    let caller = daena_ai::AiCaller::authorized_plugin(
        "daena.lore",
        "project",
        vec!["document.read".into(), "relationship.read".into()],
        vec!["project:project".into()],
        1,
        "relationship-citation-summary",
    );
    let policy = daena_plugin_api::AiRetrievalPolicyPayload {
        mode: daena_plugin_api::AiRetrievalMode::Related,
        query: None,
        seed_ids: vec![source.id],
        allowed_source_kinds: vec!["relationship".into()],
        relationship_depth: 1,
        passage_count: 4,
        include_shared_fields: false,
    };
    let (context, citations) = ai::build_retrieval_context(&project, &caller, &policy).unwrap();
    assert!(context.contains("Metadata:"));
    assert!(context.contains("validFrom"));
    let citation = citations
        .iter()
        .find(|citation| citation.source_kind == "relationship")
        .unwrap();
    assert_eq!(
        citation.summary.as_deref(),
        Some("Artifact --owned_by--> Alice · Valid from: 2024-01-01 · Valid to: 2024-06-01")
    );
}

#[test]
fn broker_retrieval_attaches_citations_until_inspected() {
    let core = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_memory(trusted_shell())
        .unwrap();
    let entity = core
        .lock()
        .unwrap()
        .core
        .lock()
        .unwrap()
        .project(trusted_shell())
        .unwrap()
        .create_entity(CreateEntity {
            name: "Broker retrieval".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    core.lock()
        .unwrap()
        .core
        .lock()
        .unwrap()
        .project(trusted_shell())
        .unwrap()
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "A cited archive passage.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        ai::FakeLoopbackProvider,
    ))));
    let plugins = ai_test_host();
    let mut context = ai_test_context(runtime.clone());
    context.core = Some(core);
    context.caller.capabilities = vec!["document.read".into()];
    let started = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.start",
        serde_json::json!({
            "operation": "generate_text",
            "taskId": "cited",
            "userInstruction": "rewrite",
            "immediateContext": {"selection": "hello"},
            "retrievalPolicy": {
                "mode": "explicit_only",
                "seedIds": [entity.id],
                "allowedSourceKinds": ["document"],
                "relationshipDepth": 0,
                "passageCount": 4,
                "includeSharedFields": false
            }
        }),
        context,
    )
    .unwrap();
    let request_id = started["requestId"].as_str().unwrap();
    assert_eq!(
        wait_ai_terminal(&runtime, request_id).last().unwrap().phase,
        "completed"
    );
    let mut result_context = ai_test_context(runtime.clone());
    let core_for_result = new_shared_core();
    result_context.core = Some(core_for_result);
    let result = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.result",
        serde_json::json!({"requestId": request_id}),
        result_context,
    )
    .unwrap();
    assert!(result["output"].as_str().unwrap().contains("cited archive"));
    let citations = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.citations",
        serde_json::json!({"requestId": request_id}),
        ai_test_context(runtime),
    )
    .unwrap();
    assert_eq!(citations.as_array().unwrap().len(), 1);
    assert_eq!(citations[0]["sourceKind"], "document");
}

#[test]
fn broker_structured_retrieval_passes_context_to_provider() {
    let core = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_memory(trusted_shell())
        .unwrap();
    let entity = core
        .lock()
        .unwrap()
        .core
        .lock()
        .unwrap()
        .project(trusted_shell())
        .unwrap()
        .create_entity(CreateEntity {
            name: "Structured retrieval".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    core.lock()
        .unwrap()
        .core
        .lock()
        .unwrap()
        .project(trusted_shell())
        .unwrap()
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "The grounded passage must reach structured generation.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        ai::FakeLoopbackProvider,
    ))));
    let plugins = ai_test_host();
    let mut context = ai_test_context(runtime.clone());
    context.core = Some(core);
    context.caller.capabilities = vec!["document.read".into()];
    let started = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.start",
        serde_json::json!({
            "operation": "generate_structured",
            "taskId": "structured-cited",
            "userInstruction": "draft",
            "immediateContext": {"entity": "subject"},
            "outputContract": {"type":"object","properties":{"summary":{"type":"string","maxLength":4000}},"required":["summary"],"additionalProperties":false},
            "retrievalPolicy": {"mode":"explicit_only","seedIds":[entity.id],"allowedSourceKinds":["document"],"relationshipDepth":0,"passageCount":4,"includeSharedFields":false}
        }),
        context,
    )
    .unwrap();
    let request_id = started["requestId"].as_str().unwrap();
    assert_eq!(
        wait_ai_terminal(&runtime, request_id).last().unwrap().phase,
        "completed"
    );
    let result = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.result",
        serde_json::json!({"requestId": request_id}),
        ai_test_context(runtime),
    )
    .unwrap();
    assert!(result["summary"]
        .as_str()
        .unwrap()
        .contains("grounded passage must reach structured generation"));
}

#[test]
fn retrieval_evaluation_corpus_matches_expected_sources_without_forbidden_markers() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../crates/daena-ai/fixtures/retrieval-evaluation.json"
    ))
    .unwrap();
    let project = ProjectStore::in_memory().unwrap();
    let mut ids = std::collections::BTreeMap::new();
    for entity in corpus["entities"].as_array().unwrap() {
        let created = project
            .create_entity(CreateEntity {
                name: entity["id"].as_str().unwrap().into(),
                entity_type: Some("fixture".into()),
            })
            .unwrap();
        project
            .save_document(SaveDocument {
                entity_id: created.id.clone(),
                body: entity["document"].as_str().unwrap().into(),
                format: Some("markdown".into()),
            })
            .unwrap();
        for field in entity["fields"].as_array().into_iter().flatten() {
            project
                .set_field(FieldValue {
                    entity_id: created.id.clone(),
                    namespace: field["namespace"].as_str().unwrap().into(),
                    key: field["key"].as_str().unwrap().into(),
                    value: serde_json::Value::String(field["value"].as_str().unwrap().into()),
                    revision: String::new(),
                })
                .unwrap();
        }
        ids.insert(entity["id"].as_str().unwrap().to_string(), created.id);
    }
    for query in corpus["queries"].as_array().unwrap() {
        let passages = project
            .search_passages(query["query"].as_str().unwrap().into(), 8)
            .unwrap();
        let actual = passages
            .iter()
            .filter(|passage| passage.source_kind == "document")
            .map(|passage| passage.entity_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let expected = query["expectedSourceIds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| ids[id.as_str().unwrap()].clone())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected);
        assert!(passages
            .iter()
            .filter(|passage| passage.source_kind == "document")
            .all(|passage| !passage.content.contains("PRIVATE_NAMESPACE_MARKER")));
    }
    let caller = daena_ai::AiCaller::authorized_plugin(
        "daena.lore",
        "project",
        vec!["document.read".into(), "search.query".into()],
        vec!["project:project".into()],
        1,
        "project-query",
    );
    let policy = daena_plugin_api::AiRetrievalPolicyPayload {
        mode: daena_plugin_api::AiRetrievalMode::Project,
        query: Some("moonstone".into()),
        seed_ids: Vec::new(),
        allowed_source_kinds: vec!["document".into()],
        relationship_depth: 0,
        passage_count: 1,
        include_shared_fields: false,
    };
    let (context, citations) = ai::build_retrieval_context(&project, &caller, &policy).unwrap();
    assert!(context.contains("The hero carries the moonstone."));
    assert_eq!(citations.len(), 1);
    assert_eq!(
        citations[0].entity_id.as_deref(),
        Some(ids["hero"].as_str())
    );
    let forbidden_policy = daena_plugin_api::AiRetrievalPolicyPayload {
        mode: daena_plugin_api::AiRetrievalMode::Project,
        query: Some("PRIVATE_NAMESPACE_MARKER".into()),
        seed_ids: Vec::new(),
        allowed_source_kinds: vec!["document".into()],
        relationship_depth: 0,
        passage_count: 4,
        include_shared_fields: false,
    };
    let (forbidden_context, forbidden_citations) =
        ai::build_retrieval_context(&project, &caller, &forbidden_policy).unwrap();
    assert!(forbidden_context.is_empty());
    assert!(forbidden_citations.is_empty());
}

#[test]
fn broker_ai_oversized_provider_output_fails_closed() {
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        OversizedProvider,
    ))));
    let plugins = ai_test_host();
    let started = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.start",
        serde_json::json!({
            "operation": "generate_text",
            "taskId": "oversized",
            "userInstruction": "rewrite",
            "immediateContext": {"selection": "hello"}
        }),
        ai_test_context(runtime.clone()),
    )
    .unwrap();
    let request_id = started["requestId"].as_str().unwrap();
    let events = wait_ai_terminal(&runtime, request_id);
    assert_eq!(events.last().unwrap().phase, "failed");
    assert_eq!(
        events.last().unwrap().error.as_deref(),
        Some("OutputValidationFailed")
    );
    assert!(dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.result",
        serde_json::json!({"requestId": request_id}),
        ai_test_context(runtime),
    )
    .is_err());
}

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
    let host = bundled_plugin_host(new_shared_core()).unwrap();
    let lore = host.catalog.get("daena.lore").unwrap();
    let timeline = host.catalog.get("daena.timeline").unwrap();
    let writing = host.catalog.get("daena.writing").unwrap();
    let language = host.catalog.get("daena.language").unwrap();
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
    assert_eq!(
        core_migration(&language.manifest).unwrap().unwrap().id,
        "language-v1"
    );
}

#[test]
fn bundled_workspace_manifests_do_not_declare_duplicate_sidebar_views() {
    let host = bundled_plugin_host(new_shared_core()).unwrap();
    for plugin_id in [
        "daena.lore",
        "daena.timeline",
        "daena.writing",
        "daena.language",
    ] {
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
fn physical_events_survive_module_lifecycle_and_clean_rebuild() {
    let root = std::env::temp_dir().join(format!("daena-maps-startup-{}", uuid::Uuid::new_v4()));
    let project = ProjectStore::open_directory(&root).unwrap();
    let mut host = bundled_plugin_host(new_shared_core()).unwrap();

    sync_project_usage(&project, &mut host).unwrap();

    let project_id = root.to_string_lossy().to_string();
    assert!(!host
        .declarations
        .views(&project_id, "daena.maps")
        .is_empty());
    assert!(project.is_module_enabled("daena.maps").unwrap());

    let map = project.create_map("Materialization map".into()).unwrap();
    let event = project
        .create_entries_with_request(
            vec![daena_core::CreateEntry {
                name: "Earthquake · year 12".into(),
                entity_type: Some(daena_core::maps::PHYSICAL_EVENT_ENTITY_TYPE.into()),
                document: Some(daena_core::CreateEntryDocument {
                    body: "# Earthquake".into(),
                    format: Some("markdown".into()),
                }),
                fields: vec![
                    daena_core::CreateEntryField {
                        namespace: daena_core::maps::PHYSICAL_EVENT_NAMESPACE.into(),
                        key: "provenance".into(),
                        value: serde_json::json!({"prediction": false}),
                    },
                    daena_core::CreateEntryField {
                        namespace: daena_core::maps::MAP_NAMESPACE.into(),
                        key: daena_core::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY.into(),
                        value: serde_json::json!({
                            "contractVersion": 1,
                            "kind": "physical-offset-years",
                            "reference": "accepted-source",
                            "startOffsetYears": 12,
                            "endOffsetYears": 12
                        }),
                    },
                ],
                relationships: vec![daena_core::CreateEntryRelationship {
                    relationship_type: daena_core::maps::PHYSICAL_EVENT_ON_MAP_RELATIONSHIP.into(),
                    target_ids: vec![map.id.clone()],
                }],
            }],
            Some("00000000-0000-4000-8000-000000000061"),
        )
        .unwrap()
        .remove(0);
    project
        .flush_checkpoint("persist physical event lifecycle")
        .unwrap();

    project
        .set_module_enabled("daena.maps".into(), false)
        .unwrap();
    sync_project_usage(&project, &mut host).unwrap();
    assert!(host
        .declarations
        .views(&project_id, "daena.maps")
        .is_empty());
    assert!(project
        .list_entities()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == event.id));

    project
        .set_module_enabled("daena.maps".into(), true)
        .unwrap();
    sync_project_usage(&project, &mut host).unwrap();
    assert!(!host
        .declarations
        .views(&project_id, "daena.maps")
        .is_empty());
    drop(project);
    std::fs::remove_file(root.join(".daena/index.sqlite")).unwrap();

    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert!(reopened.is_module_enabled("daena.maps").unwrap());
    assert!(reopened
        .list_entities()
        .unwrap()
        .iter()
        .any(|candidate| candidate.id == event.id));
    assert_eq!(reopened.map_locations(event.id.clone()).unwrap().len(), 0);
    assert_eq!(
        reopened.list_relationships(event.id.clone()).unwrap().len(),
        1
    );
    assert!(reopened.list_fields(event.id).unwrap().iter().any(|field| {
        field.namespace == daena_core::maps::MAP_NAMESPACE
            && field.key == daena_core::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY
    }));
    let mut reopened_host = bundled_plugin_host(new_shared_core()).unwrap();
    sync_project_usage(&reopened, &mut reopened_host).unwrap();
    assert!(!reopened_host
        .declarations
        .views(&project_id, "daena.maps")
        .is_empty());
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
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
        Some(("daena.maps/editor", 1)),
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
        Some(("daena.maps/editor", 1)),
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
    assert!(!map_query.contains("mapMode="));
}

#[test]
fn broker_dispatch_uses_plugin_project_authority() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let created = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "Broker Entity", "type": "person"}),
        None,
    )
    .unwrap();
    assert_eq!(created["name"], "Broker Entity");
    let entities = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.list",
        serde_json::json!({}),
        None,
    )
    .unwrap();
    assert_eq!(entities.as_array().unwrap().len(), 1);
    let missing_revision = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.update",
        serde_json::json!({"id": created["id"]}),
        None,
    )
    .unwrap_err();
    assert!(missing_revision.to_string().contains("expectedRevision"));
}

#[test]
fn timeline_shared_field_list_returns_lore_birth_and_death() {
    let mut host = PluginHost::new();
    host.register_bundled_json(include_str!("../../packages/modules/lore/manifest.json"))
        .unwrap();
    let payload = serde_json::json!({
        "entityId": "person-id",
        "namespace": "lore",
        "sharedOnly": true,
    });
    let shared_keys =
        shared_field_keys_for_request(&host, "daena.timeline", "field.list", &payload)
            .unwrap()
            .unwrap();
    assert!(shared_keys.contains("birth"));
    assert!(shared_keys.contains("death"));
    assert!(!shared_keys.contains("occupation"));

    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let person = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({
            "name": "Aven",
            "type": "person",
            "fields": [
                {"namespace": "lore", "key": "birth", "value": "42"},
                {"namespace": "lore", "key": "death", "value": "81"},
                {"namespace": "lore", "key": "occupation", "value": "Archivist"}
            ]
        }),
        None,
    )
    .unwrap();
    let fields = dispatch_module_rpc(
        &mut core,
        Some("daena.timeline"),
        Some(shared_keys),
        None,
        "field.list",
        serde_json::json!({
            "entityId": person["id"],
            "namespace": "lore",
            "sharedOnly": true,
        }),
        None,
    )
    .unwrap();
    let listed = fields
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|field| field["key"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(listed, ["birth", "death"].into_iter().collect());
}

#[test]
fn broker_asset_metadata_update_matches_the_owned_namespace() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let entity = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "Asset owner", "type": "person"}),
        None,
    )
    .unwrap();
    let asset = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "asset.register",
        serde_json::json!({
            "entity_id": entity["id"],
            "namespace": "lore",
            "filename": "portrait.png",
            "content_hash": "sha256:portrait",
            "size": 1,
            "mime_type": "image/png",
            "path": "assets/images/portrait.png",
            "expectedRevision": entity["revision"],
        }),
        None,
    )
    .unwrap();

    let denied = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "asset.update",
        serde_json::json!({
            "assetId": asset["id"],
            "namespace": "foreign",
            "role": "profile",
            "expectedRevision": asset["revision"],
        }),
        None,
    )
    .unwrap_err();
    assert!(matches!(denied, CoreError::Unauthorized { .. }));

    let updated = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "asset.update",
        serde_json::json!({
            "assetId": asset["id"],
            "namespace": "lore",
            "role": "profile",
            "expectedRevision": asset["revision"],
        }),
        None,
    )
    .unwrap();
    assert_eq!(updated["role"], "profile");
}

#[test]
fn broker_dispatch_enforces_module_record_owner_entity_types() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let person = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "Person", "type": "person"}),
        None,
    )
    .unwrap();
    let denied = dispatch_module_rpc(
        &mut core,
        Some("daena.language"),
        None,
        Some(vec!["language".into()]),
        "record.create",
        serde_json::json!({
            "collection": "lexemes",
            "ownerEntityId": person["id"],
            "value": {"lemma": "sol", "meanings": ["sun"]}
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(denied, Err(CoreError::Unauthorized { .. })));

    let language = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "Asteri", "type": "language"}),
        None,
    )
    .unwrap();
    let created = dispatch_module_rpc(
        &mut core,
        Some("daena.language"),
        None,
        Some(vec!["language".into()]),
        "record.create",
        serde_json::json!({
            "collection": "lexemes",
            "ownerEntityId": language["id"],
            "value": {"lemma": "sol", "meanings": ["sun"]}
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert_eq!(created["value"]["lemma"], "sol");
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
    assert!(!body.contains("daena-inline.css"));
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
    assert_eq!(response.headers().get("Content-Security-Policy").unwrap(), "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; style-src-elem 'self' 'unsafe-inline'; style-src-attr 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self'; manifest-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'; frame-ancestors 'none'");

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
    assert!(bridge_body.contains("waitForUploadedPack"));
    assert!(bridge_body.contains(r#"asset.list", { entityId: map.id, namespace: "maps" }"#));
    assert!(bridge_body.contains("Daena Maps provider startup failed"));
    assert!(bridge_body.contains("document.body.appendChild(overlayRoot)"));
    assert!(bridge_body.contains("position:fixed;inset:0"));
    assert!(!bridge_body.contains("host.style.position"));
    assert!(bridge_body.contains("daena-save-chrome"));
    assert!(bridge_body.contains("commitFirstSave"));
    assert!(bridge_body.contains("data-daena-link-open"));
    assert!(bridge_body.contains("data-daena-fullscreen"));
    assert!(bridge_body.contains("publishUiState(\"fullscreen\""));
    assert!(bridge_body.contains("data-daena-back"));
    assert!(bridge_body.contains("publishUiState(\"back\""));
    assert!(bridge_body.contains("data-daena-back-confirm"));
    assert!(bridge_body.contains("Discard unsaved progress?"));
    assert!(bridge_body.contains(">Discard</button>"));
    assert!(bridge_body.contains(">Cancel</button>"));
    assert!(bridge_body.contains("linkArming"));
    assert!(bridge_body.contains("data-daena-link-x"));
    assert!(bridge_body.contains("daena-link-chrome"));
    assert!(bridge_body.contains("maps.locations.upsert"));
    assert!(bridge_body.contains("maps.locations.create_and_link"));
    assert!(bridge_body.contains("startPick"));
    assert!(bridge_body.contains("showNameForm"));
    assert!(bridge_body.contains("data-daena-name-form"));
    assert!(!bridge_body.contains("promptMapName"));
    assert!(bridge_body.contains(r#"rpc("entity.create""#));
    assert!(!bridge_body.contains(r#"fields: [{ namespace: "maps", key: "map""#));
    assert!(!bridge_body.contains("if (!mapAsset) { await window.generateMapOnLoad?.(); return; }"));

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
    assert!(main_body.contains(r#"!window.DAENA_HOST && "serviceWorker""#));
    assert!(main_body.contains("hideLoading();\n    await checkLoadParameters();"));
    assert!(main_body.contains("!location.hostname && !window.DAENA_HOST"));
    assert!(!main_body.contains("openwidget.min.js"));
    assert!(!main_body.contains("if (!window.DAENA_HOST) hideLoading();"));

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

    let module = tauri::http::Request::builder()
        .uri("plugin://daena.maps/dist/ui/fmg/index-B5l1uyn4.js")
        .body(Vec::new())
        .unwrap();
    let module_response = plugin_asset_response("daena.maps", &module, None, None);
    let module_body = String::from_utf8_lossy(module_response.body());
    assert!(!module_body.contains("fonts.gstatic.com"));
    assert!(module_body
        .contains("if(window.DAENA_HOST)return;throw new Error(\"Pack cells not found\")"));
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
    let core: SharedCore = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
    let map_id = {
        let core = current_session(&core).unwrap();
        let core = core.core.lock().unwrap();
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
        let core = current_session(&core).unwrap();
        let core = core.core.lock().unwrap();
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
    assert!(begin["url"]
        .as_str()
        .unwrap()
        .starts_with(&format!("/__asset/{handle}/0?sessionId=session")));
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
    flush_checkpoint_for_shared_core(&core, "maps asset create").unwrap();
    let saved_asset: Asset = serde_json::from_value(saved).unwrap();
    assert_eq!(saved_asset.namespace, daena_core::maps::MAP_NAMESPACE);
    assert_eq!(saved_asset.size, 5);

    {
        let core = current_session(&core).unwrap();
        let core = core.core.lock().unwrap();
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
            serde_json::Value::String(saved_asset.id.clone()),
            "first-save commit must link sourceAssetId so the map appears in Saved Maps"
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

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn maps_image_import_rpc_round_trips_and_cancel_leaves_no_entity() {
    let root =
        std::env::temp_dir().join(format!("daena-image-import-rpc-{}", uuid::Uuid::new_v4()));
    let core: SharedCore = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
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
    let png = daena_core::maps::encode_transparent_png(4, 3).unwrap();
    let begin = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.image.import.begin",
        serde_json::json!({
            "name": "Atlas",
            "size": png.len(),
            "mimeType": "image/png",
            "filename": "atlas.png"
        }),
        None,
    )
    .unwrap();
    let cancelled = begin["handle"].as_str().unwrap().to_string();
    {
        let mut manager = transfers.lock().unwrap();
        assert_eq!(
            manager
                .append_upload(&cancelled, "daena.maps", "session", 0, &png)
                .unwrap(),
            png.len()
        );
    }
    dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "asset.transfer.cancel",
        serde_json::json!({"handle": cancelled}),
        None,
    )
    .unwrap();
    assert!(dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.image.import.commit",
        serde_json::json!({
            "handle": cancelled,
            "contentHash": format!("sha256:{:x}", Sha256::digest(&png))
        }),
        None,
    )
    .is_err());
    {
        let core = current_session(&core).unwrap();
        let core = core.core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        assert!(
            project.list_entities().unwrap().is_empty(),
            "cancelled image import must not create a map entity"
        );
    }

    let begin = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.image.import.begin",
        serde_json::json!({
            "name": "Atlas",
            "size": png.len(),
            "mimeType": "image/png",
            "filename": "atlas.png"
        }),
        None,
    )
    .unwrap();
    let handle = begin["handle"].as_str().unwrap().to_string();
    {
        let mut manager = transfers.lock().unwrap();
        assert_eq!(
            manager
                .append_upload(&handle, "daena.maps", "session", 0, &png)
                .unwrap(),
            png.len()
        );
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let imported = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.image.import.commit",
        serde_json::json!({
            "handle": handle,
            "contentHash": format!("sha256:{:x}", Sha256::digest(&png))
        }),
        Some(&request_id),
    )
    .unwrap();
    let retried = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.image.import.commit",
        serde_json::json!({
            "handle": handle,
            "contentHash": format!("sha256:{:x}", Sha256::digest(&png))
        }),
        Some(&request_id),
    )
    .unwrap();
    assert_eq!(imported["entity"]["id"], retried["entity"]["id"]);
    flush_checkpoint_for_shared_core(&core, "maps image import").unwrap();
    let map_id = imported["entity"]["id"].as_str().unwrap().to_string();
    assert_eq!(
        imported["entity"]["entity_type"],
        daena_core::maps::MAP_ENTITY_TYPE
    );
    let source_id = imported["source"]["id"].as_str().unwrap().to_string();
    {
        let core = current_session(&core).unwrap();
        let core = core.core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        let descriptor = project
            .list_fields(map_id.clone())
            .unwrap()
            .into_iter()
            .find(|field| field.key == "map")
            .unwrap();
        assert_eq!(
            descriptor.value["provider"]["id"],
            daena_core::maps::VECTOR_PROVIDER
        );
        assert_eq!(descriptor.value["sourceAssetId"], source_id);
        assert_eq!(
            descriptor.value["previewAssetId"],
            imported["preview"]["id"]
        );
        assert_eq!(
            imported["source"]["mime_type"],
            daena_core::maps::VECTOR_MIME
        );
        assert_eq!(imported["preview"]["mime_type"], "image/png");
    }

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn maps_vector_create_rpc_round_trips_and_cancel_leaves_no_entity() {
    let root =
        std::env::temp_dir().join(format!("daena-vector-create-rpc-{}", uuid::Uuid::new_v4()));
    let core: SharedCore = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
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
    let candidate = serde_json::to_vec(&serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]
            }
        }]
    }))
    .unwrap();
    let generation = serde_json::json!({
        "id": "daena-landmass",
        "version": 1,
        "seed": 831429,
        "settings": {
            "landPercent": 40,
            "continentCount": 3,
            "coastlineRoughness": "medium",
            "islandFrequency": "medium"
        }
    });
    let begin = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.vector.create.begin",
        serde_json::json!({
            "name": "World",
            "size": candidate.len(),
            "generation": generation
        }),
        None,
    )
    .unwrap();
    let cancelled = begin["handle"].as_str().unwrap().to_string();
    {
        let mut manager = transfers.lock().unwrap();
        assert_eq!(
            manager
                .append_upload(&cancelled, "daena.maps", "session", 0, &candidate)
                .unwrap(),
            candidate.len()
        );
    }
    dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "asset.transfer.cancel",
        serde_json::json!({"handle": cancelled}),
        None,
    )
    .unwrap();
    assert!(dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.vector.create.commit",
        serde_json::json!({
            "handle": cancelled,
            "contentHash": format!("sha256:{:x}", Sha256::digest(&candidate))
        }),
        None,
    )
    .is_err());
    {
        let core = current_session(&core).unwrap();
        let core = core.core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        assert!(
            project.list_entities().unwrap().is_empty(),
            "cancelled vector create must not create a map entity"
        );
    }

    let begin = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.vector.create.begin",
        serde_json::json!({
            "name": "World",
            "size": candidate.len(),
            "generation": generation
        }),
        None,
    )
    .unwrap();
    let handle = begin["handle"].as_str().unwrap().to_string();
    {
        let mut manager = transfers.lock().unwrap();
        assert_eq!(
            manager
                .append_upload(&handle, "daena.maps", "session", 0, &candidate)
                .unwrap(),
            candidate.len()
        );
    }
    let request_id = uuid::Uuid::new_v4().to_string();
    let accepted = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.vector.create.commit",
        serde_json::json!({
            "handle": handle,
            "contentHash": format!("sha256:{:x}", Sha256::digest(&candidate))
        }),
        Some(&request_id),
    )
    .unwrap();
    let retried = dispatch_binary_asset_rpc(
        &core,
        &transfers,
        &session,
        "maps.vector.create.commit",
        serde_json::json!({
            "handle": handle,
            "contentHash": format!("sha256:{:x}", Sha256::digest(&candidate))
        }),
        Some(&request_id),
    )
    .unwrap();
    assert_eq!(accepted["entity"]["id"], retried["entity"]["id"]);
    flush_checkpoint_for_shared_core(&core, "maps vector create").unwrap();
    let map_id = accepted["entity"]["id"].as_str().unwrap().to_string();
    let layers_revision = {
        let core = current_session(&core).unwrap();
        let core = core.core.lock().unwrap();
        let project = core.project(trusted_shell()).unwrap();
        project
            .list_fields(map_id.clone())
            .unwrap()
            .into_iter()
            .find(|field| field.key == "layers")
            .unwrap()
            .revision
    };
    let created = {
        let session = current_session(&core).unwrap();
        let mut core = session.core.lock().unwrap();
        dispatch_module_rpc(
            &mut core,
            Some("daena.maps"),
            None,
            None,
            "maps.layer.create",
            serde_json::json!({
                "mapEntityId": map_id,
                "name": "Countries",
                "expectedRevision": layers_revision
            }),
            None,
        )
        .unwrap()
    };
    assert_eq!(created["layers"]["value"]["layers"][0]["kind"], "vector");
    assert!(created["asset"].is_null());

    std::fs::remove_dir_all(root).ok();
}
