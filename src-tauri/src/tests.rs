use super::*;
use crate::app_close::{
    abandon_close, acknowledge_close, cancel_close, confirm_close, request_close,
    should_intercept_exit, CloseHandshake,
};
use crate::app_menu::{
    menu_shortcut, quit_action, window_close_action, QuitAction, WindowCloseAction,
};

#[test]
fn main_window_capability_allows_printing_wiki_pages() {
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
    let permissions = capability["permissions"].as_array().unwrap();
    assert!(permissions
        .iter()
        .any(|permission| permission == "core:webview:allow-print"));
}

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

fn engine_products(resolution: u32) -> daena_physical::engine::Products {
    let settings = daena_physical::engine::Settings::from_parts(
        831_429,
        daena_physical::engine::WorldPreset::Earthlike,
        resolution,
        daena_physical::DEFAULT_RADIUS_METRES,
        290_000,
        daena_physical::planetary::PlanetaryConfiguration::earth_like(),
    );
    daena_physical::engine::generate(settings, &mut daena_physical::NoopProgress).unwrap()
}

fn contract_shape(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::{json, Value};
    match value {
        Value::Object(map) => {
            let fields = map
                .iter()
                .map(|(key, value)| (key.clone(), contract_shape(value)))
                .collect::<serde_json::Map<_, _>>();
            json!({"kind": "object", "fields": fields})
        }
        Value::Array(items) => json!({
            "kind": "array",
            "items": items.first().map_or_else(|| json!({"kind": "any"}), contract_shape),
        }),
        Value::Null => json!({"kind": "null"}),
        Value::Bool(_) => json!({"kind": "boolean"}),
        Value::Number(_) => json!({"kind": "number"}),
        Value::String(_) => json!({"kind": "string"}),
    }
}

#[test]
fn physical_frontend_contract_fixture_matches_the_rust_json() {
    let products = engine_products(64);
    let generation = products.generation().unwrap().clone();

    let mut hydrology = products.hydrology().clone();
    if hydrology.basins.is_empty() {
        hydrology.basins.push(daena_physical::products::Basin {
            id: 1,
            minimum_cell: 2,
            minimum_elevation_mm: 3,
            cell_count: 4,
            spill_cell: 5,
            spill_elevation_mm: Some(6),
            water_level_mm: 7,
            water_volume_m3: 8,
            inflow_m3_per_year: 9,
            evaporation_m3_per_year: 10,
            outflow_m3_per_year: 11,
            status: daena_physical::products::BasinStatus::Active,
        });
    }
    if hydrology.rivers.is_empty() {
        hydrology
            .rivers
            .push(daena_physical::products::RiverSegment {
                id: 1,
                source_cell: 2,
                mouth_cell: 3,
                strahler_order: 4,
                destination: daena_physical::products::RiverDestination::Ocean,
                spill_outlet: false,
                coordinate_count: 0,
            });
    }
    let mut hazards = products.hazards().clone();
    if hazards.volcanic_sources.is_empty() {
        hazards
            .volcanic_sources
            .push(daena_physical::products::VolcanicSource {
                stable_id: 9,
                cell: 0,
                origin: daena_physical::products::VolcanicOrigin::Hotspot,
                activity_class: daena_physical::products::VolcanicActivity::Moderate,
                eruptions_per_model_year_nano: 1,
            });
    }

    let mut request = daena_atlas::request::AtlasRenderRequest::spike_png(64, 32).unwrap();
    request.level = daena_atlas::request::DetailLevel::Standard;
    let identity = daena_atlas::spike_identity_from_source(products.source());
    let prepared = daena_atlas::prepare_from_source(
        products.source(),
        &identity,
        &request,
        None,
        None,
        &mut daena_atlas::NoopProgress,
    )
    .unwrap();
    let sample = prepared.sample_surface(0, 0);
    let decoded = daena_physical::engine::decode_source(products.source()).unwrap();
    let event_request = daena_physical::events::EventMaterializationRequest {
        event_kind: daena_physical::events::NaturalEventKind::Earthquake,
        interval_start_years: -100_000,
        interval_end_years: 100_000,
        max_events: 1,
        hazard_seed: 1,
    };
    let mut event = decoded
        .sample_events(None, &event_request, &mut daena_physical::NoopProgress)
        .unwrap()
        .events
        .into_iter()
        .next()
        .expect("the small engine world samples at least one earthquake");
    // The earthquake event leaves this optional field zero; set it so the
    // fixture also pins the serialized key shape of the event wrapper.
    event.hydrology_derivation_version = 1;

    let mut fixture = serde_json::Map::new();
    fixture.insert("generation".into(), contract_shape(&generation));
    fixture.insert(
        "climate".into(),
        contract_shape(&engine_climate_products(products.climate())),
    );
    fixture.insert(
        "hydrology".into(),
        contract_shape(&engine_hydrology_products(&hydrology)),
    );
    fixture.insert(
        "hazards".into(),
        contract_shape(&engine_hazard_products(
            &hazards,
            products.resolution(),
            daena_physical::DEFAULT_RADIUS_METRES as f64,
        )),
    );
    fixture.insert(
        "surfaceSample".into(),
        contract_shape(&serde_json::to_value(sample).unwrap()),
    );
    fixture.insert(
        "materializedEvent".into(),
        contract_shape(
            &serde_json::to_value(MaterializedPhysicalEvent {
                entity_id: "entity".into(),
                event,
            })
            .unwrap(),
        ),
    );
    let fixture = serde_json::Value::Object(fixture);
    let mut text = serde_json::to_string_pretty(&fixture).unwrap();
    text.push('\n');
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src/lib/project/physical-contract.fixture.json");
    if std::env::var_os("DAENA_UPDATE_PHYSICAL_CONTRACT").is_some() {
        std::fs::write(&path, text).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    assert_eq!(
        text,
        expected,
        "{} is stale; regenerate it with DAENA_UPDATE_PHYSICAL_CONTRACT=1 cargo test -p daena physical_frontend_contract_fixture_matches_the_rust_json",
        path.display()
    );
}

#[test]
fn accepted_engine_source_rebuilds_and_rejects_an_external_checkpoint_edit() {
    let products = engine_products(32);
    let generation = products.generation().unwrap().clone();
    let root = std::env::temp_dir().join(format!("daena-engine-storage-{}", uuid::Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let accepted = store
        .accept_physical_map(
            "Engine storage".into(),
            products.source().to_vec(),
            generation.clone(),
            Some("00000000-0000-4000-8000-0000000000e3"),
        )
        .unwrap();
    let identity = accepted.physical_identity.clone();
    let source_path = accepted.source.path.clone();
    let original = daena_physical::engine::decode_source(products.source()).unwrap();
    let original_hashes = original.hashes();
    store.flush_checkpoint("engine storage test").unwrap();
    drop(store);

    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    let descriptor = rebuilt
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "map")
        .unwrap();
    let bytes = rebuilt.asset_bytes(accepted.source.id.clone()).unwrap();
    let validated =
        daena_core::maps::physical::validate_source(&bytes, &descriptor.value["generation"])
            .unwrap();
    assert_eq!(validated.identity, identity);
    assert_eq!(validated.source.hashes(), original_hashes);
    let physics = engine_static_derived(
        &validated.source,
        daena_physical::planetary::PlanetaryConfiguration::earth_like(),
    )
    .unwrap();
    assert_eq!(physics.hashes, original_hashes);
    assert_eq!(&physics.view, products.view());
    assert_eq!(&physics.climate, products.climate());
    assert_eq!(&physics.hydrology, products.hydrology());
    assert_eq!(&physics.hazards, products.hazards());
    assert_eq!(physics.geojson, products.derived_geojson());
    drop(rebuilt);

    let portable_source = root.join(source_path);
    let mut edited = std::fs::read(&portable_source).unwrap();
    let last = edited.len() - 1;
    edited[last] ^= 1;
    std::fs::write(&portable_source, edited).unwrap();
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    assert!(ProjectStore::open_directory(&root).is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "wall-clock: release-only deadline sweep"]
fn physical_command_cancellation_latency_is_under_budget() {
    struct DeadlineSink {
        deadline: Instant,
        armed: bool,
        last_phase: daena_physical::ProgressPhase,
    }

    impl DeadlineSink {
        fn new(deadline: Instant) -> Self {
            Self {
                deadline,
                armed: false,
                last_phase: daena_physical::ProgressPhase::BuildingTectonicStructure,
            }
        }

        fn never() -> Self {
            Self::new(Instant::now() + Duration::from_secs(86_400))
        }

        fn check(&mut self) {
            if Instant::now() >= self.deadline {
                self.armed = true;
            }
        }
    }

    impl daena_physical::ProgressSink for DeadlineSink {
        fn report(
            &mut self,
            phase: daena_physical::ProgressPhase,
            _completed: u32,
            _total: u32,
        ) -> Result<(), daena_physical::PhysicalError> {
            self.last_phase = phase;
            self.check();
            Ok(())
        }

        fn check_cancelled(&self) -> Result<(), daena_physical::PhysicalError> {
            if self.armed || Instant::now() >= self.deadline {
                Err(daena_physical::PhysicalError::Cancelled)
            } else {
                Ok(())
            }
        }
    }

    fn settings(resolution: u32) -> daena_physical::engine::Settings {
        daena_physical::engine::Settings::from_parts(
            7,
            daena_physical::engine::WorldPreset::Earthlike,
            resolution,
            daena_physical::DEFAULT_RADIUS_METRES,
            290_000,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        )
    }

    fn sweep(resolution: u32, step_ms: u64) -> (Duration, Duration, u64, String) {
        let settings = settings(resolution);
        let started = Instant::now();
        let full = daena_physical::engine::generate(settings, &mut DeadlineSink::never())
            .expect("uncancelled generation succeeds");
        let total = started.elapsed();
        drop(full);
        let mut worst = Duration::ZERO;
        let mut worst_at_ms = 0;
        let mut worst_phase = String::from("none");
        let mut deadline_ms = 50;
        while Duration::from_millis(deadline_ms) < total {
            let started = Instant::now();
            let mut sink = DeadlineSink::new(started + Duration::from_millis(deadline_ms));
            let result = daena_physical::engine::generate(settings, &mut sink);
            let elapsed = started.elapsed();
            if matches!(result, Err(daena_physical::PhysicalError::Cancelled)) {
                let late = elapsed.saturating_sub(Duration::from_millis(deadline_ms));
                if late > worst {
                    worst = late;
                    worst_at_ms = deadline_ms;
                    worst_phase = sink.last_phase.label().into();
                }
            }
            deadline_ms += step_ms;
        }
        (total, worst, worst_at_ms, worst_phase)
    }

    let (total_512, worst_512, at_512, phase_512) = sweep(512, 50);
    let (total_1024, worst_1024, at_1024, phase_1024) = sweep(1024, 50);
    let report = format!(
        "n=512 full {:?} worst late {:?} at {} ms ({})\nn=1024 full {:?} worst late {:?} at {} ms ({})",
        total_512, worst_512, at_512, phase_512,
        total_1024, worst_1024, at_1024, phase_1024
    );
    println!("{report}");
    assert!(
        worst_512.as_millis() < daena_physical::CANCELLATION_LATENCY_BUDGET_MS,
        "n=512 cancellation was {worst_512:?} late at {at_512} ms ({phase_512})"
    );
    assert!(
        worst_1024.as_millis() < daena_physical::CANCELLATION_LATENCY_BUDGET_MS,
        "n=1024 cancellation was {worst_1024:?} late at {at_1024} ms ({phase_1024})"
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
        assert!(ensure_project_ai_enabled(root.to_str().unwrap()).is_ok());
        project.flush_checkpoint("AI gate test").unwrap();
    }
    assert!(ensure_project_ai_enabled(root.to_str().unwrap()).is_ok());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ai_toggle_uses_the_writer_and_readers_observe_the_exported_setting() {
    let root = std::env::temp_dir().join(format!("daena-ai-toggle-ro-{}", uuid::Uuid::new_v4()));
    let writer = ProjectStore::open_directory(&root).unwrap();
    let reader = ProjectStore::open_read_only(&root).unwrap();
    assert!(!reader.info().unwrap().ai_enabled);
    assert!(reader.set_ai_enabled(true).is_err());
    drop(reader);
    assert!(writer.set_ai_enabled(true).unwrap().ai_enabled);
    writer.flush_checkpoint("AI command-path test").unwrap();
    let reader = ProjectStore::open_read_only(&root).unwrap();
    assert!(reader.info().unwrap().ai_enabled);
    drop(reader);
    drop(writer);
    std::fs::remove_dir_all(root).unwrap();
}

fn ai_test_context(runtime: ai::SharedAiRuntime) -> AiBrokerContext {
    // Broker tests need a real, AI-enabled project: the ai.request.start gate
    // reads the canonical manifest and fails closed without an opt-in.
    static PROJECT: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    static SETTINGS: std::sync::OnceLock<SharedSettings> = std::sync::OnceLock::new();
    let root = PROJECT.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("daena-ai-broker-{}", uuid::Uuid::new_v4()));
        let project = ProjectStore::open_directory(&root).unwrap();
        project.set_ai_enabled(true).unwrap();
        project.flush_checkpoint("AI broker test").unwrap();
        root
    });
    let settings = SETTINGS.get_or_init(|| {
        let directory =
            std::env::temp_dir().join(format!("daena-ai-broker-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let store = SettingsStore::new(&directory);
        let mut configured = AppSettings::default();
        configured.ai.project_bindings.insert(
            root.to_string_lossy().into_owned(),
            crate::settings::AiProviderSettings {
                id: "test".into(),
                name: "Test".into(),
                adapter: "openai-compatible".into(),
                endpoint: "http://127.0.0.1:1234/v1".into(),
                model: "test".into(),
                embedding_model: String::new(),
                capabilities: vec!["text.generate".into()],
            },
        );
        store.save(&configured).unwrap();
        Arc::new(Mutex::new(store))
    });
    AiBrokerContext {
        app: None,
        core: None,
        settings: Some(settings.clone()),
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

struct HoldUntilCancelled;

impl ai::AiProvider for HoldUntilCancelled {
    fn generate(
        &self,
        request: ai::ProviderRequest,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
    ) -> Vec<ai::AiStreamEvent> {
        for _ in 0..3_000 {
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
            std::thread::sleep(Duration::from_millis(2));
        }
        vec![ai::AiStreamEvent {
            sequence: 0,
            request_id: request.request_id,
            phase: "completed".into(),
            delta: None,
            output: Some("still-running".into()),
            error: None,
        }]
    }
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
fn closing_a_webview_session_cancels_only_that_sessions_ai_request() {
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        HoldUntilCancelled,
    ))));
    let plugins = ai_test_host();
    let payload = serde_json::json!({
        "operation": "generate_text",
        "taskId": "text",
        "userInstruction": "rewrite",
        "immediateContext": {"selection": "hello"}
    });
    let mut webview = ai_test_context(runtime.clone());
    webview.session_id = "webview-session".into();
    let mut other = ai_test_context(runtime.clone());
    other.session_id = "bundled:daena.writing".into();
    let started = dispatch_host_rpc(
        &plugins,
        "daena.writing",
        "project",
        "ai.request.start",
        payload.clone(),
        webview,
    )
    .unwrap();
    let other_started = dispatch_host_rpc(
        &plugins,
        "daena.writing",
        "project",
        "ai.request.start",
        payload,
        other,
    )
    .unwrap();
    let request_id = started["requestId"].as_str().unwrap();
    let other_id = other_started["requestId"].as_str().unwrap();
    cancel_ai_requests_for_sessions(&plugins, &runtime, &["webview-session"]).unwrap();
    assert_eq!(
        wait_ai_terminal(&runtime, request_id).last().unwrap().phase,
        "cancelled"
    );
    let other_events = ai::poll_ai_events(&runtime, other_id).unwrap();
    assert!(
        other_events.iter().all(|event| event.phase != "cancelled"),
        "{other_events:?}"
    );
    assert!(
        ai::ai_request_result(&runtime, other_id)
            .unwrap_err()
            .contains("still running"),
        "the other session's request must keep running"
    );
    assert!(plugins
        .lock()
        .unwrap()
        .ai_request_ids_for_session("webview-session")
        .is_empty());
    assert_eq!(
        plugins
            .lock()
            .unwrap()
            .ai_request_ids_for_session("bundled:daena.writing"),
        vec![other_id.to_string()]
    );
    cancel_ai_requests_for_sessions(&plugins, &runtime, &["bundled:daena.writing"]).unwrap();
    assert_eq!(
        wait_ai_terminal(&runtime, other_id).last().unwrap().phase,
        "cancelled"
    );
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
fn broker_writing_rewrite_policy_passes_the_capability_checks_and_cites_related_text() {
    // The policy the Writing webview sends: the shell's `direct_retrieval_context` request for an entity.
    let core = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_memory(trusted_shell())
        .unwrap();
    let entity_id = {
        let session = current_session(&core).unwrap();
        let service = session.core.lock().unwrap();
        let project = service.project(trusted_shell()).unwrap();
        let entity = project
            .create_entity(CreateEntity {
                name: "Chapter one".into(),
                entity_type: Some("daena.writing:manuscript".into()),
            })
            .unwrap();
        let neighbour = project
            .create_entity(CreateEntity {
                name: "Harbour".into(),
                entity_type: Some("place".into()),
            })
            .unwrap();
        project
            .save_document(SaveDocument {
                entity_id: neighbour.id.clone(),
                body: "The harbour freezes every winter.".into(),
                format: Some("markdown".into()),
            })
            .unwrap();
        project
            .create_relationship(RelationshipInput {
                source_id: entity.id.clone(),
                target_id: neighbour.id.clone(),
                relationship_type: "set_in".into(),
                metadata: None,
            })
            .unwrap();
        entity.id
    };
    let runtime = Arc::new(Mutex::new(ai::AiRuntime::with_provider(Arc::new(
        ai::FakeLoopbackProvider,
    ))));
    let plugins = ai_test_host();
    let mut context = ai_test_context(runtime.clone());
    context.core = Some(core);
    // The Writing manifest's read grants, which the AI caller carries.
    context.caller.capabilities = [
        "document.read",
        "relationship.read",
        "field.read:self",
        "search.query",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let started = dispatch_host_rpc(
        &plugins,
        "daena.lore",
        "project",
        "ai.request.start",
        serde_json::json!({
            "operation": "generate_text",
            "taskId": "writing-rewrite",
            "userInstruction": "Rewrite this to be more vivid.",
            "immediateContext": {"selection": "The sea was cold."},
            "retrievalPolicy": {
                "mode": "related",
                "query": "Chapter one Rewrite this harbour",
                "seedIds": [entity_id],
                "allowedSourceKinds": [],
                "relationshipDepth": 2,
                "passageCount": 8,
                "includeSharedFields": true
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
    let output = ai::ai_request_result(&runtime, request_id).unwrap();
    assert!(output.contains("The sea was cold."), "{output}");
    assert!(output.contains("[RETRIEVED_CONTEXT]"), "{output}");
    assert!(
        output.contains("The harbour freezes every winter."),
        "{output}"
    );
    let citations = ai::ai_request_citations(&runtime, request_id).unwrap();
    assert!(citations
        .iter()
        .any(|source| source.source_kind == "document"));
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
    assert_eq!(sanitize_mutation_request_id("maps-request-1"), None);
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
    let houses = host.catalog.get("daena.houses").unwrap();
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
    assert_eq!(
        core_migration(&houses.manifest).unwrap().unwrap().id,
        "houses-v1"
    );
}

#[test]
fn bundled_workspace_manifests_do_not_declare_duplicate_sidebar_views() {
    let host = bundled_plugin_host(new_shared_core()).unwrap();
    for plugin_id in ["daena.lore", "daena.timeline", "daena.houses"] {
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
    project
        .set_module_enabled("daena.maps".into(), true)
        .unwrap();
    sync_project_usage(&project, &mut host).unwrap();

    let project_id = root.to_string_lossy().to_string();
    assert!(!host
        .declarations
        .views(&project_id, "daena.maps")
        .is_empty());
    assert!(project.is_module_enabled("daena.maps").unwrap());

    let map = project
        .create_entity(CreateEntity {
            name: "Materialization map".into(),
            entity_type: Some(daena_core::maps::MAP_ENTITY_TYPE.into()),
        })
        .unwrap();
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
fn plugin_webview_bounds_shift_below_the_title_bar_inset() {
    // Measured on macOS in windowed mode: the content view is 869pt tall but
    // WebKit starts the page 32pt down, below the title bar.
    let bounds = PluginWebviewBounds {
        x: 359.0,
        y: 214.0,
        width: 892.0,
        height: 583.0,
        viewport_width: 1291.0,
        viewport_height: 837.0,
    };
    let native = scale_plugin_bounds(bounds, 1291.0, 869.0);
    assert_eq!((native.x, native.y), (359.0, 246.0));
    assert_eq!((native.width, native.height), (892.0, 583.0));
    // Fullscreen: no inset, so the bounds pass through unchanged.
    let fullscreen = PluginWebviewBounds {
        viewport_height: 869.0,
        ..bounds
    };
    let native = scale_plugin_bounds(fullscreen, 1291.0, 869.0);
    assert_eq!((native.y, native.height), (214.0, 583.0));
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
    assert!(query.contains("hostSurface=daena.maps%2Feditor"));
    assert!(!query.contains("daena=1"));

    let map_url = plugin_webview_url(
        &policy,
        "project",
        Some("map-workspace"),
        Some(("daena.maps/editor", 1)),
        Some("018f89df-b93e-7ad0-a07f-08b1441d1550"),
        Some("f4c4f6b9-7c1e-4b8a-9d2e-0a3b5c7d9e11"),
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
fn language_webview_url_carries_entity_id_and_module_state() {
    let manifest: PluginManifest = serde_json::from_str(include_str!(
        "../../packages/modules/language/manifest.json"
    ))
    .unwrap();
    let policy = webview_policy(&manifest).unwrap();
    let bounds = PluginWebviewBounds {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
        viewport_width: 800.0,
        viewport_height: 600.0,
    };
    let url = plugin_webview_url(
        &policy,
        "project",
        Some("language-workspace"),
        Some(("daena.language/workspace", 1)),
        None,
        None,
        Some("018f89df-b93e-7ad0-a07f-08b1441d1550"),
        Some(r#"{"pane":"lexemes","q":"a b"}"#),
        bounds,
    )
    .unwrap();
    let tauri::WebviewUrl::External(url) = url else {
        panic!("plugin webview must use an external custom-protocol URL");
    };
    let query = url.query().unwrap();
    assert!(query.contains("&entityId=018f89df-b93e-7ad0-a07f-08b1441d1550"));
    assert!(query.contains("&state=%7B%22pane%22%3A%22lexemes%22%2C%22q%22%3A%22a%20b%22%7D"));
    assert!(!query.contains("mapEntityId="));

    let rejected = plugin_webview_url(
        &policy,
        "project",
        None,
        None,
        None,
        None,
        None,
        Some("{not json"),
        bounds,
    );
    assert!(rejected
        .err()
        .is_some_and(|error| error.contains("not valid JSON")));
}

#[test]
fn module_state_must_be_bounded_json() {
    assert!(validate_module_state(r#"{"pane":"lexemes"}"#).is_ok());
    assert!(validate_module_state("{not json")
        .unwrap_err()
        .contains("not valid JSON"));
    // Exactly at the limit is accepted; one byte over is rejected.
    let at_limit = format!("\"{}\"", "x".repeat(MAX_MODULE_STATE_BYTES - 2));
    assert_eq!(at_limit.len(), MAX_MODULE_STATE_BYTES);
    assert!(validate_module_state(&at_limit).is_ok());
    let over_limit = format!("\"{}\"", "x".repeat(MAX_MODULE_STATE_BYTES - 1));
    assert!(validate_module_state(&over_limit)
        .unwrap_err()
        .contains("exceeds"));
}

#[test]
fn host_message_script_serialises_message_and_bounds_size() {
    let message = serde_json::json!({ "kind": "probe", "text": "a\"); alert(1); (\"" });
    let json = serde_json::to_string(&message).unwrap();
    let script = host_message_script(&message).unwrap();
    assert!(script.starts_with(
        "try { window.dispatchEvent(new CustomEvent(\"daena:host-message\", { detail: "
    ));
    assert!(script.contains(&format!("detail: {json} }}")));
    assert!(script.contains("} catch (error) {"));

    let oversized = serde_json::json!({ "text": "x".repeat(MAX_HOST_MESSAGE_BYTES) });
    assert!(host_message_script(&oversized)
        .unwrap_err()
        .contains("64 KiB"));
}

#[test]
fn embedded_mount_key_changes_with_every_mount_param() {
    let key = |view: Option<&str>,
               map: Option<&str>,
               link: Option<&str>,
               entity: Option<&str>,
               state: Option<&str>| {
        embedded_mount_key("daena.writing", view, map, link, entity, state)
    };
    let base = key(Some("editor"), None, None, Some("e1"), Some("{}"));
    assert_eq!(
        base,
        key(Some("editor"), None, None, Some("e1"), Some("{}"))
    );
    // A different entity (or any other param) must recreate.
    assert_ne!(
        base,
        key(Some("editor"), None, None, Some("e2"), Some("{}"))
    );
    assert_ne!(base, key(Some("other"), None, None, Some("e1"), Some("{}")));
    assert_ne!(
        base,
        key(Some("editor"), Some("m"), None, Some("e1"), Some("{}"))
    );
    assert_ne!(
        base,
        key(Some("editor"), None, Some("l"), Some("e1"), Some("{}"))
    );
    assert_ne!(
        base,
        key(Some("editor"), None, None, Some("e1"), Some("{\"pane\":1}"))
    );
    assert_ne!(
        base,
        embedded_mount_key(
            "daena.language",
            Some("editor"),
            None,
            None,
            Some("e1"),
            Some("{}")
        )
    );
    // Unambiguous: None is not an empty string, and values cannot run together.
    assert_ne!(
        key(None, None, None, None, None),
        key(Some(""), None, None, None, None)
    );
    assert_ne!(
        key(Some("a"), Some("b"), None, None, None),
        key(Some("ab"), None, None, None, None)
    );
    assert_ne!(
        key(None, Some("a"), None, Some("b"), None),
        key(None, Some("a\",\"b"), None, None, None)
    );
}

#[test]
fn mount_reuses_only_a_live_label_with_the_same_mount_key() {
    let key = embedded_mount_key(
        "daena.writing",
        Some("editor"),
        None,
        None,
        Some("e1"),
        None,
    );
    let other = embedded_mount_key(
        "daena.writing",
        Some("editor"),
        None,
        None,
        Some("e2"),
        None,
    );
    assert!(mount_should_reuse(true, Some(&key), &key));
    // Different entity: recreate.
    assert!(!mount_should_reuse(true, Some(&key), &other));
    // No stored state for the label (not mounted, or state unknown): recreate.
    assert!(!mount_should_reuse(true, None, &key));
    assert!(!mount_should_reuse(true, Some(""), &key));
    // The caller did not ask for reuse: always recreate.
    assert!(!mount_should_reuse(false, Some(&key), &key));
}

#[test]
fn mount_retain_list_becomes_labels_and_rejects_bad_instances() {
    let a = "4f2a9c1b-1111-4222-8333-444444444444";
    let b = "4f2a9c1b-1111-4222-8333-555555555555";
    let refs = vec![
        WebviewInstanceRef {
            plugin_id: "daena.writing".into(),
            instance_id: a.into(),
        },
        WebviewInstanceRef {
            plugin_id: "daena.writing".into(),
            instance_id: b.into(),
        },
    ];
    let labels = retain_labels(&refs).unwrap();
    assert_eq!(
        labels,
        BTreeSet::from([
            plugin_instance_label("daena.writing", a).unwrap(),
            plugin_instance_label("daena.writing", b).unwrap(),
        ])
    );
    assert!(retain_labels(&[]).unwrap().is_empty());

    // A bad instance id fails the whole list; it is never skipped.
    for bad in ["", "NOT-HEX", "a:b", "../x", &"a".repeat(65)] {
        let mut with_bad = refs.clone();
        with_bad.push(WebviewInstanceRef {
            plugin_id: "daena.writing".into(),
            instance_id: bad.into(),
        });
        assert!(retain_labels(&with_bad).is_err(), "instance {bad:?}");
    }
    let bad_plugin = vec![WebviewInstanceRef {
        plugin_id: "Daena Writing".into(),
        instance_id: a.into(),
    }];
    assert!(retain_labels(&bad_plugin).is_err());
}

#[test]
fn mount_closes_every_embedded_label_that_is_not_retained() {
    let a = plugin_instance_label("daena.writing", "4f2a9c1b-1111-4222-8333-444444444444").unwrap();
    let b = plugin_instance_label("daena.writing", "4f2a9c1b-1111-4222-8333-555555555555").unwrap();
    let c = plugin_instance_label("daena.writing", "4f2a9c1b-1111-4222-8333-666666666666").unwrap();
    let lore = plugin_instance_label("daena.lore", "4f2a9c1b-1111-4222-8333-444444444444").unwrap();
    let external = plugin_window_label("daena.maps");
    let existing = vec![
        ("main".to_string(), false),
        (a.clone(), false),
        (b.clone(), false),
        (c.clone(), false),
        (lore.clone(), false),
        (external.clone(), true),
    ];

    // Nothing retained: all embedded labels but the mounting one close.
    let mut closed = embedded_labels_to_close(&existing, &a, &BTreeSet::new());
    closed.sort();
    let mut expected = vec![b.clone(), c.clone(), lore.clone()];
    expected.sort();
    assert_eq!(closed, expected);

    // Retained labels survive; the rest close.
    let retained = BTreeSet::from([b.clone(), c.clone()]);
    assert_eq!(
        embedded_labels_to_close(&existing, &a, &retained),
        vec![lore.clone()]
    );

    // The mounting label is never closed, even when it is not retained, and
    // `main` and an external bare window are never closed.
    let closed = embedded_labels_to_close(&existing, &lore, &retained);
    assert_eq!(closed, vec![a.clone()]);
    assert!(!closed.contains(&"main".to_string()));
    assert!(!closed.contains(&external));
    assert!(!closed.contains(&lore));
}

#[test]
fn plugin_state_events_forward_only_for_their_owner() {
    // The owning publisher forwards its own state to the matching shell event.
    assert_eq!(
        shell_event_for_plugin_event("daena.maps", "daena.maps/state@1"),
        Some("maps-state")
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.maps", "daena.maps/selection@1"),
        Some("maps-selection")
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.language", "daena.language/state@1"),
        Some("language-state")
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.writing", "daena.writing/state@1"),
        Some("writing-state")
    );
    // A different publisher cannot make the shell receive another plugin's state.
    assert_eq!(
        shell_event_for_plugin_event("com.example.x", "daena.language/state@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.timeline", "daena.maps/state@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.language", "daena.writing/state@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("com.example.x", "daena.writing/state@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena", "daena.maps/state@1"),
        None
    );
    // Unknown or unversioned event names are never forwarded.
    assert_eq!(
        shell_event_for_plugin_event("daena.language", "daena.language/selection@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.language", "daena.language/state"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.writing", "daena.writing/selection@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.writing", "daena.writing/state"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.lore", "daena.lore/state@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.lore", "daena.lore/shortcut@1"),
        Some("shell-shortcut")
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.timeline", "daena.timeline/shortcut@1"),
        Some("shell-shortcut")
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.lore", "daena.timeline/shortcut@1"),
        None
    );
    assert_eq!(
        shell_event_for_plugin_event("daena.timeline", "daena.lore/shortcut@1"),
        None
    );
}

#[test]
fn shell_shortcut_payload_keeps_only_known_tab_ids() {
    assert_eq!(
        shell_shortcut_id(&serde_json::json!({"shortcut": "tab-next"})),
        Some("tab-next")
    );
    assert_eq!(
        shell_shortcut_id(&serde_json::json!({"shortcut": "tab-last"})),
        Some("tab-last")
    );
    assert_eq!(
        shell_shortcut_id(&serde_json::json!({"shortcut": "tab-9"})),
        None
    );
    assert_eq!(shell_shortcut_id(&serde_json::json!({"shortcut": 1})), None);
    assert_eq!(shell_shortcut_id(&serde_json::json!({})), None);
    assert_eq!(shell_shortcut_id(&serde_json::json!(null)), None);
}

#[test]
fn forwarded_instance_id_comes_from_the_label_not_the_payload() {
    let instance = "4f2a9c1b-1111-4222-8333-444444444444";
    let origin = plugin_instance_label("daena.writing", instance).unwrap();
    let (event, body) = forwarded_shell_event(
        "daena.writing",
        &serde_json::json!({
            "type": "daena.writing/state@1",
            "payload": { "kind": "ready", "instanceId": "from-the-plugin" }
        }),
        &origin,
    )
    .unwrap();
    assert_eq!(event, "writing-state");
    assert_eq!(body["kind"], "ready");
    assert_eq!(body["instanceId"], instance);

    let (_, bare_body) = forwarded_shell_event(
        "daena.writing",
        &serde_json::json!({
            "type": "daena.writing/state@1",
            "payload": { "kind": "ready", "instanceId": "from-the-plugin" }
        }),
        &plugin_window_label("daena.writing"),
    )
    .unwrap();
    assert!(bare_body["instanceId"].is_null());

    let lore_instance = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let (shortcut_event, shortcut_body) = forwarded_shell_event(
        "daena.lore",
        &serde_json::json!({
            "type": "daena.lore/shortcut@1",
            "payload": { "shortcut": "tab-next", "instanceId": "from-the-plugin" }
        }),
        &plugin_instance_label("daena.lore", lore_instance).unwrap(),
    )
    .unwrap();
    assert_eq!(shortcut_event, "shell-shortcut");
    assert_eq!(shortcut_body, "tab-next");

    let (language_event, language_body) = forwarded_shell_event(
        "daena.language",
        &serde_json::json!({
            "type": "daena.language/state@1",
            "payload": { "kind": "ready", "instanceId": "from-the-plugin" }
        }),
        &plugin_instance_label("daena.language", instance).unwrap(),
    )
    .unwrap();
    assert_eq!(language_event, "language-state");
    assert_eq!(language_body["instanceId"], instance);

    let maps_instance = "abc123";
    let (_, maps_body) = forwarded_shell_event(
        "daena.maps",
        &serde_json::json!({
            "type": "daena.maps/state@1",
            "payload": {
                "mapEntityId": "map-1",
                "status": "saved",
                "detail": null,
                "instanceId": "from-the-plugin"
            }
        }),
        &plugin_instance_label("daena.maps", maps_instance).unwrap(),
    )
    .unwrap();
    assert_eq!(maps_body["instanceId"], maps_instance);
    assert_eq!(maps_body["mapEntityId"], "map-1");
    assert!(maps_body.get("from-the-plugin").is_none());

    let (_, selection) = forwarded_shell_event(
        "daena.maps",
        &serde_json::json!({
            "type": "daena.maps/selection@1",
            "payload": { "mapEntityId": "map-1", "anchor": {"id": "pin"}, "instanceId": "from-the-plugin" }
        }),
        &plugin_instance_label("daena.maps", maps_instance).unwrap(),
    )
    .unwrap();
    assert_eq!(selection["instanceId"], maps_instance);
    assert_eq!(selection["anchor"]["id"], "pin");
}

#[test]
fn foreign_instance_label_cannot_read_another_plugins_protocol() {
    let core: SharedCore = new_shared_core();
    let plugins: SharedPluginHost = Arc::new(Mutex::new(PluginHost::new()));
    let transfers: SharedBinaryTransfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let ai_runtime: ai::SharedAiRuntime = Arc::new(Mutex::new(ai::AiRuntime::default()));
    let request = tauri::http::Request::builder()
        .method("GET")
        .uri("plugin://daena.writing/dist/ui/writing.js")
        .body(Vec::new())
        .unwrap();
    let respond = |label: &str| {
        plugin_protocol_response(label, &request, &core, &plugins, &transfers, &ai_runtime)
    };
    let lore = plugin_instance_label("daena.lore", "4f2a9c1b-1111-4222-8333-444444444444").unwrap();
    assert_eq!(respond(&lore).status(), 403);
    assert_eq!(respond("plugin:daena/writing:GGGG").status(), 403);
    assert_eq!(respond("plugin:daena.writing:abcd").status(), 403);
    let writing =
        plugin_instance_label("daena.writing", "4f2a9c1b-1111-4222-8333-444444444444").unwrap();
    assert_ne!(
        respond(&writing).status(),
        403,
        "a writing instance passes the label gate for plugin://daena.writing"
    );
    assert_ne!(
        respond(&plugin_window_label("daena.writing")).status(),
        403,
        "the bare external-window label still passes the label gate"
    );
}

#[test]
fn two_instance_labels_bootstrap_as_separate_protocol_sessions() {
    let root = std::env::temp_dir().join(format!("daena-instance-label-{}", uuid::Uuid::new_v4()));
    let core: SharedCore = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
    let project_id = current_info(&core).unwrap().unwrap().root;
    let mut host = bundled_plugin_host(core.clone()).unwrap();
    {
        let session = current_session(&core).unwrap();
        let guard = session.core.lock().unwrap();
        sync_project_usage(guard.project(trusted_shell()).unwrap(), &mut host).unwrap();
    }
    let plugins: SharedPluginHost = Arc::new(Mutex::new(host));
    let transfers: SharedBinaryTransfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let ai_runtime: ai::SharedAiRuntime = Arc::new(Mutex::new(ai::AiRuntime::default()));
    let instance_a = "4f2a9c1b-1111-4222-8333-444444444444";
    let instance_b = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
    let label_a = plugin_instance_label("daena.writing", instance_a).unwrap();
    let label_b = plugin_instance_label("daena.writing", instance_b).unwrap();
    let post = |label: &str, body: serde_json::Value| {
        let request = tauri::http::Request::builder()
            .method("POST")
            .uri("plugin://daena.writing/__rpc")
            .body(serde_json::to_vec(&body).unwrap())
            .unwrap();
        let response =
            plugin_protocol_response(label, &request, &core, &plugins, &transfers, &ai_runtime);
        assert_eq!(response.status(), 200, "{response:?}");
        serde_json::from_slice::<serde_json::Value>(response.body()).unwrap()
    };
    let bootstrap = |label: &str| {
        let reply = post(
            label,
            serde_json::json!({
                "op": "bootstrap",
                "pluginId": "daena.writing",
                "projectId": project_id,
            }),
        );
        reply["sessionId"].as_str().expect("session id").to_string()
    };
    let version = |label: &str, session_id: &str, request_id: &str| {
        post(
            label,
            serde_json::json!({
                "op": "rpc",
                "request": {
                    "rpcVersion": 1,
                    "sessionId": session_id,
                    "requestId": request_id,
                    "method": "app.version",
                    "payload": {},
                },
            }),
        )
    };
    let session_a = bootstrap(&label_a);
    let session_b = bootstrap(&label_b);
    assert_ne!(session_a, session_b);
    assert_eq!(version(&label_a, &session_a, "a-live")["ok"], true);
    assert_eq!(version(&label_b, &session_b, "b-live")["ok"], true);
    let crossed = version(&label_b, &session_a, "crossed");
    assert_eq!(crossed["ok"], false, "{crossed}");
    assert_eq!(crossed["error"]["code"], "session.origin");
    // Re-bootstrapping A revokes only A's previous session.
    let session_a2 = bootstrap(&label_a);
    assert_ne!(session_a2, session_a);
    let revoked = version(&label_a, &session_a, "a-revoked");
    assert_eq!(revoked["error"]["code"], "session.revoked");
    assert_eq!(version(&label_b, &session_b, "b-still")["ok"], true);
    assert_eq!(version(&label_a, &session_a2, "a2")["ok"], true);
    // A lore instance still cannot bootstrap as writing.
    let lore = plugin_instance_label("daena.lore", instance_a).unwrap();
    let request = tauri::http::Request::builder()
        .method("POST")
        .uri("plugin://daena.writing/__rpc")
        .body(
            serde_json::to_vec(&serde_json::json!({
                "op": "bootstrap",
                "pluginId": "daena.writing",
                "projectId": project_id,
            }))
            .unwrap(),
        )
        .unwrap();
    let forbidden =
        plugin_protocol_response(&lore, &request, &core, &plugins, &transfers, &ai_runtime);
    assert_eq!(forbidden.status(), 403);
    let _ = std::fs::remove_dir_all(&root);
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
    let page = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.query",
        serde_json::json!({"query": "Broker", "entityTypes": ["person"], "limit": 25}),
        None,
    )
    .unwrap();
    assert_eq!(page["total"], 1);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["typeCounts"][0]["entityType"], "person");
    assert_eq!(page["hasMore"], false);
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
    assert!(shared_keys.contains("occupation"));

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
    assert_eq!(
        listed,
        ["birth", "death", "occupation"].into_iter().collect()
    );
}

#[test]
fn schema_list_returns_enabled_effective_manifests() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    core.project_mut(AuthorityContext::trusted_shell())
        .unwrap()
        .set_module_schema_overlay(
            "daena.lore".into(),
            Some(serde_json::json!({
                "version": daena_plugin_api::schema_overlay::SCHEMA_OVERLAY_VERSION,
                "customFields": [{
                    "key": "overlayDate",
                    "label": "Overlay date",
                    "type": "date",
                    "entityTypes": ["daena.lore:person"],
                    "shared": true,
                    "timeline": { "role": "point", "label": "Overlay", "layer": "dates" }
                }]
            })),
        )
        .unwrap();
    core.project_mut(AuthorityContext::trusted_shell())
        .unwrap()
        .set_module_enabled("daena.writing".into(), false)
        .unwrap();
    let host = bundled_plugin_host(new_shared_core()).unwrap();
    let listed = enabled_schema_manifests(
        core.project(AuthorityContext::trusted_shell()).unwrap(),
        &host,
    )
    .unwrap();
    let lore = listed
        .iter()
        .find(|manifest| manifest["id"] == "daena.lore")
        .expect("enabled lore manifest");
    let overlay_date = lore["schemas"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|schema| schema["fields"].as_array().into_iter().flatten())
        .find(|field| field["key"] == "overlayDate")
        .expect("overlay date field");
    assert_eq!(overlay_date["shared"], true);
    assert_eq!(overlay_date["timeline"]["role"], "point");
    assert_eq!(overlay_date["timeline"]["label"], "Overlay");
    assert_eq!(overlay_date["timeline"]["layer"], "dates");
    assert!(listed
        .iter()
        .all(|manifest| manifest["id"] != "daena.writing"));
    assert!(listed
        .iter()
        .all(|manifest| manifest.get("enabled").is_none()));

    let root = std::env::temp_dir().join(format!("daena-schema-list-{}", uuid::Uuid::new_v4()));
    let project = ProjectStore::open_directory(&root).unwrap();
    let project_id = root.to_string_lossy().to_string();
    let mut manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../examples/plugins/timeline-consumer/manifest.json"
    ))
    .unwrap();
    manifest["publisher"] = serde_json::Value::String("daena-archive".into());
    manifest["capabilities"]
        .as_array_mut()
        .unwrap()
        .retain(|capability| capability != "entity.read");
    let mut denied_host = PluginHost::new();
    denied_host
        .register_bundled_json(&serde_json::to_string(&manifest).unwrap())
        .unwrap();
    sync_project_usage(&project, &mut denied_host).unwrap();
    let denied = denied_host
        .authorize_bundled(
            "com.example.timeline-consumer",
            &project_id,
            "schema.list",
            serde_json::json!({}),
        )
        .unwrap_err();
    assert!(denied.to_string().contains("capability.denied"), "{denied}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn schema_list_is_served_over_the_plugin_protocol_bridge() {
    // Plugin webviews reach the broker through `/__rpc`, not the `plugin_rpc`
    // Tauri command, so Chronology's first call must succeed on this path.
    let root = std::env::temp_dir().join(format!("daena-schema-bridge-{}", uuid::Uuid::new_v4()));
    let core: SharedCore = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
    let project_id = current_info(&core).unwrap().unwrap().root;
    let mut host = bundled_plugin_host(core.clone()).unwrap();
    {
        let session = current_session(&core).unwrap();
        let core = session.core.lock().unwrap();
        sync_project_usage(core.project(trusted_shell()).unwrap(), &mut host).unwrap();
    }
    let plugins: SharedPluginHost = Arc::new(Mutex::new(host));
    let transfers: SharedBinaryTransfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let ai_runtime: ai::SharedAiRuntime = Arc::new(Mutex::new(ai::AiRuntime::default()));
    let label = plugin_window_label("daena.timeline");
    let post = |body: serde_json::Value| {
        let request = tauri::http::Request::builder()
            .method("POST")
            .uri(format!(
                "{}://daena.timeline/__rpc",
                daena_plugin_host::runtime::plugin_protocol("daena.timeline")
            ))
            .body(serde_json::to_vec(&body).unwrap())
            .unwrap();
        let response =
            plugin_protocol_response(&label, &request, &core, &plugins, &transfers, &ai_runtime);
        assert_eq!(response.status(), 200);
        serde_json::from_slice::<serde_json::Value>(response.body()).unwrap()
    };
    let bootstrap = post(serde_json::json!({
        "op": "bootstrap",
        "pluginId": "daena.timeline",
        "projectId": project_id,
    }));
    let session_id = bootstrap["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();
    let listed = post(serde_json::json!({
        "op": "rpc",
        "request": {
            "rpcVersion": 1,
            "sessionId": session_id,
            "requestId": "schema-1",
            "method": "schema.list",
            "payload": {},
        },
    }));
    assert_eq!(listed["ok"], true, "{listed}");
    let manifests = listed["result"].as_array().expect("manifest array");
    assert!(manifests
        .iter()
        .any(|manifest| manifest["id"] == "daena.timeline"));
    assert!(manifests
        .iter()
        .all(|manifest| manifest.get("enabled").is_none()));
    // The Chronology source pages entities by `items` / `hasMore` / `entityType`.
    let page = post(serde_json::json!({
        "op": "rpc",
        "request": {
            "rpcVersion": 1,
            "sessionId": session_id,
            "requestId": "entities-1",
            "method": "entity.query",
            "payload": { "offset": 0, "limit": 200 },
        },
    }));
    assert_eq!(page["ok"], true, "{page}");
    assert!(page["result"]["items"].is_array());
    assert!(page["result"]["hasMore"].is_boolean());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn plugin_protocol_bridge_preserves_session_error_codes() {
    let root = std::env::temp_dir().join(format!("daena-session-code-{}", uuid::Uuid::new_v4()));
    let core: SharedCore = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
    let project_id = current_info(&core).unwrap().unwrap().root;
    let mut host = bundled_plugin_host(core.clone()).unwrap();
    {
        let session = current_session(&core).unwrap();
        let core = session.core.lock().unwrap();
        sync_project_usage(core.project(trusted_shell()).unwrap(), &mut host).unwrap();
    }
    host.session_ttl = Duration::ZERO;
    let plugins: SharedPluginHost = Arc::new(Mutex::new(host));
    let transfers: SharedBinaryTransfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let ai_runtime: ai::SharedAiRuntime = Arc::new(Mutex::new(ai::AiRuntime::default()));
    let label = plugin_window_label("daena.timeline");
    let post = |body: serde_json::Value| {
        let request = tauri::http::Request::builder()
            .method("POST")
            .uri(format!(
                "{}://daena.timeline/__rpc",
                daena_plugin_host::runtime::plugin_protocol("daena.timeline")
            ))
            .body(serde_json::to_vec(&body).unwrap())
            .unwrap();
        let response =
            plugin_protocol_response(&label, &request, &core, &plugins, &transfers, &ai_runtime);
        assert_eq!(response.status(), 200, "{response:?}");
        serde_json::from_slice::<serde_json::Value>(response.body()).unwrap()
    };
    let rpc = |session_id: &str, request_id: &str| {
        post(serde_json::json!({
            "op": "rpc",
            "request": {
                "rpcVersion": 1,
                "sessionId": session_id,
                "requestId": request_id,
                "method": "entity.list",
                "payload": {},
            },
        }))
    };
    let expired_bootstrap = post(serde_json::json!({
        "op": "bootstrap",
        "pluginId": "daena.timeline",
        "projectId": project_id,
    }));
    let expired_id = expired_bootstrap["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();
    let expired = rpc(&expired_id, "expired-1");
    assert_eq!(expired["ok"], false, "{expired}");
    assert_eq!(expired["requestId"], "expired-1");
    assert_eq!(expired["error"]["code"], "session.expired");
    assert_eq!(expired["error"]["message"], "session has expired");
    assert_eq!(expired["error"]["retryable"], false);

    plugins.lock().unwrap().session_ttl = Duration::from_secs(15 * 60);
    let live_bootstrap = post(serde_json::json!({
        "op": "bootstrap",
        "pluginId": "daena.timeline",
        "projectId": project_id,
    }));
    let live_id = live_bootstrap["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();
    plugins.lock().unwrap().sessions.revoke_session(&live_id);
    let revoked = rpc(&live_id, "revoked-1");
    assert_eq!(revoked["ok"], false, "{revoked}");
    assert_eq!(revoked["requestId"], "revoked-1");
    assert_eq!(revoked["error"]["code"], "session.revoked");
    assert_eq!(revoked["error"]["message"], "session has been revoked");
    assert_eq!(revoked["error"]["retryable"], false);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn shared_asset_reads_are_served_over_the_plugin_protocol_bridge() {
    // Language reads project-scoped assets by portable path over `/__rpc`.
    // Entity-scoped assets, missing assets and malformed paths are refused
    // with messages that do not reveal which assets exist.
    let root = std::env::temp_dir().join(format!("daena-shared-read-{}", uuid::Uuid::new_v4()));
    let source =
        std::env::temp_dir().join(format!("daena-shared-source-{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&source, b"shared bytes").unwrap();
    let core: SharedCore = new_shared_core();
    current_session(&core)
        .unwrap()
        .core
        .lock()
        .unwrap()
        .open_directory(trusted_shell(), &root)
        .unwrap();
    let project_id = current_info(&core).unwrap().unwrap().root;
    let (shared_path, private_path) = {
        let session = current_session(&core).unwrap();
        let guard = session.core.lock().unwrap();
        let project = guard.project(trusted_shell()).unwrap();
        let entity = project
            .create_entity(CreateEntity {
                name: "Shared owner".into(),
                entity_type: None,
            })
            .unwrap();
        let register = |filename: &str| {
            project
                .register_asset_file(AssetFileInput {
                    entity_id: entity.id.clone(),
                    namespace: "language".into(),
                    source_path: source.to_string_lossy().into_owned(),
                    filename: filename.into(),
                    mime_type: "text/plain".into(),
                    provenance: None,
                })
                .unwrap()
        };
        let shared = register("shared.txt");
        let shared = project
            .update_asset_metadata_with_request(
                AssetMetadataUpdate {
                    asset_id: shared.id.clone(),
                    filename: None,
                    role: None,
                    reference_scope: Some("project".into()),
                },
                &shared.revision,
                Some(&uuid::Uuid::new_v4().to_string()),
            )
            .unwrap();
        assert_eq!(shared.reference_scope, "project");
        let private = register("private.txt");
        assert_eq!(private.reference_scope, "entity");
        (shared.path, private.path)
    };
    let mut host = bundled_plugin_host(core.clone()).unwrap();
    {
        let session = current_session(&core).unwrap();
        let core = session.core.lock().unwrap();
        sync_project_usage(core.project(trusted_shell()).unwrap(), &mut host).unwrap();
    }
    let plugins: SharedPluginHost = Arc::new(Mutex::new(host));
    let transfers: SharedBinaryTransfers = Arc::new(Mutex::new(BinaryTransferManager::default()));
    let ai_runtime: ai::SharedAiRuntime = Arc::new(Mutex::new(ai::AiRuntime::default()));
    let label = plugin_window_label("daena.language");
    let post = |body: serde_json::Value| {
        let request = tauri::http::Request::builder()
            .method("POST")
            .uri(format!(
                "{}://daena.language/__rpc",
                daena_plugin_host::runtime::plugin_protocol("daena.language")
            ))
            .body(serde_json::to_vec(&body).unwrap())
            .unwrap();
        let response =
            plugin_protocol_response(&label, &request, &core, &plugins, &transfers, &ai_runtime);
        assert_eq!(response.status(), 200);
        serde_json::from_slice::<serde_json::Value>(response.body()).unwrap()
    };
    let bootstrap = post(serde_json::json!({
        "op": "bootstrap",
        "pluginId": "daena.language",
        "projectId": project_id,
    }));
    let session_id = bootstrap["sessionId"]
        .as_str()
        .expect("session id")
        .to_string();
    let shared_read = |request_id: &str, path: &str| {
        post(serde_json::json!({
            "op": "rpc",
            "request": {
                "rpcVersion": 1,
                "sessionId": session_id,
                "requestId": request_id,
                "method": "asset.shared.read.begin",
                "payload": { "path": path },
            },
        }))
    };

    let shared = shared_read("shared-1", &shared_path);
    assert_eq!(shared["ok"], true, "{shared}");
    let result = &shared["result"];
    assert_eq!(result["size"], 12);
    assert_eq!(result["mimeType"], "text/plain");
    assert!(result["handle"].is_string(), "{shared}");
    assert!(result["revision"].is_string(), "{shared}");
    assert!(result["contentHash"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    let url = result["url"].as_str().expect("read url").to_string();
    // Plugin webviews fetch the bytes with a GET on the returned URL.
    let fetch = tauri::http::Request::builder()
        .method("GET")
        .uri(format!(
            "{}://daena.language{url}",
            daena_plugin_host::runtime::plugin_protocol("daena.language")
        ))
        .body(Vec::new())
        .unwrap();
    let response =
        plugin_protocol_response(&label, &fetch, &core, &plugins, &transfers, &ai_runtime);
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("Content-Type").unwrap(),
        "text/plain"
    );
    assert_eq!(response.body(), b"shared bytes");
    // The read token is one use.
    let replay = plugin_protocol_response(&label, &fetch, &core, &plugins, &transfers, &ai_runtime);
    assert_eq!(replay.status(), 404);

    let private = shared_read("private-1", &private_path);
    assert_eq!(private["ok"], false, "{private}");
    assert_eq!(private["error"]["message"], "shared asset not found");
    let missing = shared_read("missing-1", "assets/files/does-not-exist.txt");
    assert_eq!(missing["ok"], false, "{missing}");
    assert_eq!(missing["error"]["message"], "shared asset not found");
    for (request_id, path) in [
        ("traversal-1", "assets/../x"),
        ("outside-1", "private/x.txt"),
        ("nul-1", "assets/x\u{0}.txt"),
    ] {
        let refused = shared_read(request_id, path);
        assert_eq!(refused["ok"], false, "{refused}");
        assert_eq!(
            refused["error"]["message"], "invalid shared asset path",
            "{path}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_file(&source);
}

#[test]
fn shared_asset_read_requires_the_asset_read_shared_capability() {
    let root =
        std::env::temp_dir().join(format!("daena-shared-capability-{}", uuid::Uuid::new_v4()));
    let project = ProjectStore::open_directory(&root).unwrap();
    let project_id = root.to_string_lossy().to_string();
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../packages/modules/language/manifest.json"
    ))
    .unwrap();
    let mut granted_host = PluginHost::new();
    granted_host
        .register_bundled_json(&manifest.to_string())
        .unwrap();
    sync_project_usage(&project, &mut granted_host).unwrap();
    granted_host
        .authorize_bundled(
            "daena.language",
            &project_id,
            "asset.shared.read.begin",
            serde_json::json!({ "path": "assets/files/shared.txt" }),
        )
        .unwrap();

    let mut denied_manifest = manifest.clone();
    denied_manifest["capabilities"]
        .as_array_mut()
        .unwrap()
        .retain(|capability| capability != "asset.read:shared");
    let mut denied_host = PluginHost::new();
    denied_host
        .register_bundled_json(&denied_manifest.to_string())
        .unwrap();
    sync_project_usage(&project, &mut denied_host).unwrap();
    let denied = denied_host
        .authorize_bundled(
            "daena.language",
            &project_id,
            "asset.shared.read.begin",
            serde_json::json!({ "path": "assets/files/shared.txt" }),
        )
        .unwrap_err();
    assert!(denied.to_string().contains("capability.denied"), "{denied}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A Language webview bridge over `/__rpc` for one open project: the bundled
/// host, the binary transfer table, and the plugin protocol handler.
struct LanguageBridge {
    core: SharedCore,
    plugins: SharedPluginHost,
    transfers: SharedBinaryTransfers,
    ai_runtime: ai::SharedAiRuntime,
    plugin_id: &'static str,
    label: String,
}

fn create_upload_target(core: &SharedCore, name: &str) -> String {
    let session = current_session(core).unwrap();
    let guard = session.core.lock().unwrap();
    let project = guard.project(trusted_shell()).unwrap();
    project
        .create_entity(CreateEntity {
            name: name.into(),
            entity_type: None,
        })
        .unwrap()
        .id
}

fn entity_revision(core: &SharedCore, entity_id: &str) -> String {
    let session = current_session(core).unwrap();
    let guard = session.core.lock().unwrap();
    let project = guard.project(trusted_shell()).unwrap();
    project
        .list_entities()
        .unwrap()
        .into_iter()
        .find(|entity| entity.id == entity_id)
        .expect("upload target entity")
        .revision
}

impl LanguageBridge {
    fn open(root: &std::path::Path) -> Self {
        Self::open_as(root, "daena.language")
    }

    /// The same bridge speaking as another bundled sandboxed plugin.
    fn open_as(root: &std::path::Path, plugin_id: &'static str) -> Self {
        let core: SharedCore = new_shared_core();
        current_session(&core)
            .unwrap()
            .core
            .lock()
            .unwrap()
            .open_directory(trusted_shell(), root)
            .unwrap();
        let mut host = bundled_plugin_host(core.clone()).unwrap();
        {
            let session = current_session(&core).unwrap();
            let guard = session.core.lock().unwrap();
            sync_project_usage(guard.project(trusted_shell()).unwrap(), &mut host).unwrap();
        }
        Self {
            core,
            plugins: Arc::new(Mutex::new(host)),
            transfers: Arc::new(Mutex::new(BinaryTransferManager::default())),
            ai_runtime: Arc::new(Mutex::new(ai::AiRuntime::default())),
            plugin_id,
            label: plugin_window_label(plugin_id),
        }
    }

    fn protocol_uri(&self, path: &str) -> String {
        format!(
            "{}://{}{path}",
            daena_plugin_host::runtime::plugin_protocol(self.plugin_id),
            self.plugin_id
        )
    }

    fn respond(&self, request: &tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
        plugin_protocol_response(
            &self.label,
            request,
            &self.core,
            &self.plugins,
            &self.transfers,
            &self.ai_runtime,
        )
    }

    fn post(&self, body: serde_json::Value) -> serde_json::Value {
        let request = tauri::http::Request::builder()
            .method("POST")
            .uri(self.protocol_uri("/__rpc"))
            .body(serde_json::to_vec(&body).unwrap())
            .unwrap();
        let response = self.respond(&request);
        assert_eq!(response.status(), 200);
        serde_json::from_slice(response.body()).unwrap()
    }

    fn bootstrap(&self, project_id: &str) -> String {
        let reply = self.post(serde_json::json!({
            "op": "bootstrap",
            "pluginId": self.plugin_id,
            "projectId": project_id,
        }));
        reply["sessionId"].as_str().expect("session id").to_string()
    }

    fn rpc(
        &self,
        session_id: &str,
        request_id: &str,
        method: &str,
        payload: serde_json::Value,
    ) -> serde_json::Value {
        self.post(serde_json::json!({
            "op": "rpc",
            "request": {
                "rpcVersion": 1,
                "sessionId": session_id,
                "requestId": request_id,
                "method": method,
                "payload": payload,
            },
        }))
    }

    fn put_chunk(&self, url: &str, chunk: u64, body: &[u8]) -> tauri::http::Response<Vec<u8>> {
        let path = url.replacen("/0?", &format!("/{chunk}?"), 1);
        let request = tauri::http::Request::builder()
            .method("PUT")
            .uri(self.protocol_uri(&path))
            .body(body.to_vec())
            .unwrap();
        self.respond(&request)
    }

    fn send_chunks(&self, transfer: &serde_json::Value, bytes: &[u8]) {
        let max_chunk = transfer["maxChunkBytes"].as_u64().unwrap() as usize;
        let url = transfer["url"].as_str().unwrap();
        for (index, chunk) in bytes.chunks(max_chunk).enumerate() {
            let response = self.put_chunk(url, index as u64, chunk);
            assert_eq!(response.status(), 200, "chunk {index}");
        }
    }

    /// Begins an upload into `entity_id`, sends every chunk, and commits with
    /// the matching digest. Returns the commit reply.
    fn upload_file(
        &self,
        session_id: &str,
        entity_id: &str,
        filename: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> serde_json::Value {
        let begin = self.rpc(
            session_id,
            &format!("begin-{filename}"),
            "asset.upload.begin",
            serde_json::json!({
                "entityId": entity_id,
                "namespace": "language",
                "filename": filename,
                "mimeType": mime_type,
                "size": bytes.len(),
                "expectedRevision": entity_revision(&self.core, entity_id),
            }),
        );
        assert_eq!(begin["ok"], true, "{begin}");
        self.send_chunks(&begin["result"], bytes);
        self.rpc(
            session_id,
            &format!("commit-{filename}"),
            "asset.upload.commit",
            serde_json::json!({
                "handle": begin["result"]["handle"],
                "contentHash": format!("sha256:{:x}", Sha256::digest(bytes)),
            }),
        )
    }

    fn listed_count(&self, session_id: &str, entity_id: &str) -> usize {
        let listed = self.rpc(
            session_id,
            "list-count",
            "asset.list",
            serde_json::json!({"entityId": entity_id, "namespace": "language"}),
        );
        assert_eq!(listed["ok"], true, "{listed}");
        listed["result"].as_array().unwrap().len()
    }
}

#[test]
fn asset_uploads_round_trip_over_the_plugin_protocol_bridge() {
    let root = std::env::temp_dir().join(format!("daena-upload-rpc-{}", uuid::Uuid::new_v4()));
    let bridge = LanguageBridge::open(&root);
    let project_id = current_info(&bridge.core).unwrap().unwrap().root;
    let entity_id = create_upload_target(&bridge.core, "Upload target");
    let session_id = bridge.bootstrap(&project_id);

    // A file larger than one RPC frame goes up in several PUTs.
    let bytes: Vec<u8> = (0..300_000).map(|index| (index % 251) as u8).collect();
    let commit = bridge.upload_file(
        &session_id,
        &entity_id,
        "figure.bin",
        "application/octet-stream",
        &bytes,
    );
    assert_eq!(commit["ok"], true, "{commit}");
    let asset = commit["result"].clone();
    assert_eq!(asset["namespace"], "language");
    assert_eq!(asset["entity_id"], entity_id);
    assert_eq!(asset["size"].as_u64(), Some(bytes.len() as u64));
    assert_eq!(
        asset["content_hash"],
        format!("sha256:{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(asset["reference_scope"], "entity");
    let asset_id = asset["id"].as_str().unwrap().to_string();
    // The commit flushes the checkpoint, so the portable copy holds the bytes.
    let portable = asset["path"].as_str().unwrap();
    assert_eq!(std::fs::read(root.join(portable)).unwrap(), bytes);

    let listed = bridge.rpc(
        &session_id,
        "list-1",
        "asset.list",
        serde_json::json!({"entityId": entity_id, "namespace": "language"}),
    );
    assert_eq!(listed["result"][0]["id"], asset_id, "{listed}");
    let read = bridge.rpc(
        &session_id,
        "read-1",
        "asset.read.begin",
        serde_json::json!({"assetId": asset_id, "namespace": "language"}),
    );
    assert_eq!(read["ok"], true, "{read}");
    let fetch = tauri::http::Request::builder()
        .method("GET")
        .uri(bridge.protocol_uri(read["result"]["url"].as_str().unwrap()))
        .body(Vec::new())
        .unwrap();
    let response = bridge.respond(&fetch);
    assert_eq!(response.status(), 200);
    assert_eq!(response.body(), &bytes);

    // Entity-scoped uploads stay out of the shared list until promoted.
    let shared = bridge.rpc(
        &session_id,
        "shared-1",
        "asset.shared.list",
        serde_json::json!({}),
    );
    assert_eq!(shared["ok"], true, "{shared}");
    assert_eq!(shared["result"], serde_json::json!([]));
    let promoted = bridge.rpc(
        &session_id,
        "promote-1",
        "asset.update",
        serde_json::json!({
            "assetId": asset_id,
            "namespace": "language",
            "referenceScope": "project",
            "expectedRevision": asset["revision"],
        }),
    );
    assert_eq!(promoted["ok"], true, "{promoted}");
    let private = bridge.upload_file(
        &session_id,
        &entity_id,
        "note.txt",
        "text/plain",
        b"private",
    );
    assert_eq!(private["ok"], true, "{private}");
    let shared = bridge.rpc(
        &session_id,
        "shared-2",
        "asset.shared.list",
        serde_json::json!({}),
    );
    assert_eq!(shared["result"].as_array().unwrap().len(), 1, "{shared}");
    assert_eq!(shared["result"][0]["id"], asset_id);
    assert_eq!(bridge.listed_count(&session_id, &entity_id), 2);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_upload_refusals_leave_nothing_registered() {
    let root = std::env::temp_dir().join(format!("daena-upload-refuse-{}", uuid::Uuid::new_v4()));
    let bridge = LanguageBridge::open(&root);
    let project_id = current_info(&bridge.core).unwrap().unwrap().root;
    let entity_id = create_upload_target(&bridge.core, "Refusals");
    let session_id = bridge.bootstrap(&project_id);
    let revision = entity_revision(&bridge.core, &entity_id);
    let begin_payload = |namespace: &str, filename: &str, mime: &str, size: u64, expected: &str| {
        serde_json::json!({
            "entityId": entity_id,
            "namespace": namespace,
            "filename": filename,
            "mimeType": mime,
            "size": size,
            "expectedRevision": expected,
        })
    };

    // Refused at begin, before any bytes are accepted.
    for (request_id, payload, expected) in [
        (
            "foreign-1",
            begin_payload("lore", "a.txt", "text/plain", 3, &revision),
            "plugin does not own namespace",
        ),
        (
            "stale-1",
            begin_payload("language", "a.txt", "text/plain", 3, "stale"),
            "revision conflict",
        ),
        (
            "zero-1",
            begin_payload("language", "a.txt", "text/plain", 0, &revision),
            "asset upload size must be a positive integer",
        ),
        (
            "mime-1",
            begin_payload("language", "a.txt", " ", 3, &revision),
            "asset MIME type is required",
        ),
        (
            "name-1",
            begin_payload("language", "nested/a.txt", "text/plain", 3, &revision),
            "asset filename is invalid",
        ),
    ] {
        let reply = bridge.rpc(&session_id, request_id, "asset.upload.begin", payload);
        assert_eq!(reply["ok"], false, "{request_id}: {reply}");
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(message.contains(expected), "{request_id}: {message}");
    }
    let missing = bridge.rpc(
        &session_id,
        "missing-1",
        "asset.upload.begin",
        serde_json::json!({
            "entityId": "no-such-entity",
            "namespace": "language",
            "filename": "a.txt",
            "mimeType": "text/plain",
            "size": 3,
            "expectedRevision": revision,
        }),
    );
    assert_eq!(missing["error"]["message"], "entity not found", "{missing}");

    // A commit before the last chunk is refused and registers nothing.
    let bytes = vec![7_u8; 300_000];
    let begin = bridge.rpc(
        &session_id,
        "partial-begin",
        "asset.upload.begin",
        begin_payload(
            "language",
            "partial.bin",
            "application/octet-stream",
            bytes.len() as u64,
            &revision,
        ),
    );
    assert_eq!(begin["ok"], true, "{begin}");
    let max_chunk = begin["result"]["maxChunkBytes"].as_u64().unwrap() as usize;
    let first = bridge.put_chunk(
        begin["result"]["url"].as_str().unwrap(),
        0,
        &bytes[..max_chunk],
    );
    assert_eq!(first.status(), 200);
    let partial = bridge.rpc(
        &session_id,
        "partial-commit",
        "asset.upload.commit",
        serde_json::json!({
            "handle": begin["result"]["handle"],
            "contentHash": format!("sha256:{:x}", Sha256::digest(&bytes)),
        }),
    );
    assert_eq!(
        partial["error"]["message"], "asset upload is incomplete",
        "{partial}"
    );
    assert_eq!(bridge.listed_count(&session_id, &entity_id), 0);

    // A wrong digest is refused, and the handle still commits with the right one.
    let begin = bridge.rpc(
        &session_id,
        "digest-begin",
        "asset.upload.begin",
        begin_payload("language", "digest.txt", "text/plain", 19, &revision),
    );
    assert_eq!(begin["ok"], true, "{begin}");
    bridge.send_chunks(&begin["result"], b"digest checked text");
    let wrong = bridge.rpc(
        &session_id,
        "digest-wrong",
        "asset.upload.commit",
        serde_json::json!({
            "handle": begin["result"]["handle"],
            "contentHash": format!("sha256:{:x}", Sha256::digest(b"other text")),
        }),
    );
    assert_eq!(
        wrong["error"]["message"], "asset upload content hash does not match bytes",
        "{wrong}"
    );
    assert_eq!(bridge.listed_count(&session_id, &entity_id), 0);
    let right = bridge.rpc(
        &session_id,
        "digest-right",
        "asset.upload.commit",
        serde_json::json!({
            "handle": begin["result"]["handle"],
            "contentHash": format!("sha256:{:x}", Sha256::digest(b"digest checked text")),
        }),
    );
    assert_eq!(right["ok"], true, "{right}");
    assert_eq!(bridge.listed_count(&session_id, &entity_id), 1);

    // A handle belongs to the session that began it. Bootstrapping again
    // revokes the first session, so the new session must not commit its handle.
    let begin = bridge.rpc(
        &session_id,
        "session-begin",
        "asset.upload.begin",
        begin_payload(
            "language",
            "owned.txt",
            "text/plain",
            5,
            &entity_revision(&bridge.core, &entity_id),
        ),
    );
    assert_eq!(begin["ok"], true, "{begin}");
    bridge.send_chunks(&begin["result"], b"owned");
    let active = bridge.bootstrap(&project_id);
    let stolen = bridge.rpc(
        &active,
        "session-steal",
        "asset.upload.commit",
        serde_json::json!({
            "handle": begin["result"]["handle"],
            "contentHash": format!("sha256:{:x}", Sha256::digest(b"owned")),
        }),
    );
    assert_eq!(
        stolen["error"]["message"], "asset upload handle is not valid for this session or project",
        "{stolen}"
    );
    assert_eq!(bridge.listed_count(&active, &entity_id), 1);

    // A cancelled handle cannot commit.
    let cancelled = bridge.rpc(
        &active,
        "cancel-begin",
        "asset.upload.begin",
        begin_payload(
            "language",
            "cancelled.txt",
            "text/plain",
            3,
            &entity_revision(&bridge.core, &entity_id),
        ),
    );
    assert_eq!(cancelled["ok"], true, "{cancelled}");
    let handle = cancelled["result"]["handle"].clone();
    let cancel = bridge.rpc(
        &active,
        "cancel-1",
        "asset.transfer.cancel",
        serde_json::json!({"handle": handle}),
    );
    assert_eq!(cancel["ok"], true, "{cancel}");
    let after_cancel = bridge.rpc(
        &active,
        "cancel-commit",
        "asset.upload.commit",
        serde_json::json!({
            "handle": handle,
            "contentHash": format!("sha256:{:x}", Sha256::digest(b"nop")),
        }),
    );
    assert_eq!(
        after_cancel["error"]["message"], "asset upload handle is invalid or expired",
        "{after_cancel}"
    );
    assert_eq!(bridge.listed_count(&active, &entity_id), 1);

    let _ = std::fs::remove_dir_all(&root);
}

/// Writing's manifest has no `document.write`. The isolated webview therefore cannot create an entry that
/// carries a document, and cannot call `document.save`. Creating the entry itself stays allowed; the shell
/// saves an opening note afterwards through `project_save_entry`.
#[test]
fn writing_plugin_cannot_write_documents() {
    let root = std::env::temp_dir().join(format!("daena-writing-caps-{}", uuid::Uuid::new_v4()));
    let project = ProjectStore::open_directory(&root).unwrap();
    let project_id = root.to_string_lossy().to_string();
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../packages/modules/writing/manifest.json")).unwrap();
    assert!(manifest["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .all(|capability| capability != "document.write"));
    assert!(manifest["templates"]
        .as_array()
        .unwrap()
        .iter()
        .all(|template| template["document"].is_string()));
    let with_note = serde_json::json!({
        "name": "Draft",
        "type": "daena.writing:manuscript",
        "document": {"body": "First lines", "format": "markdown"},
    });
    let without_note = serde_json::json!({"name": "Draft", "type": "daena.writing:manuscript"});
    let save = serde_json::json!({
        "entityId": "entity",
        "body": "First lines",
        "format": "markdown",
        "expectedRevision": "rev",
    });

    let mut host = bundled_plugin_host(new_shared_core()).unwrap();
    sync_project_usage(&project, &mut host).unwrap();
    let denied_create = host
        .authorize_bundled("daena.writing", &project_id, "entity.create", with_note)
        .unwrap_err();
    assert!(
        denied_create.to_string().contains("capability.denied"),
        "{denied_create}"
    );
    host.authorize_bundled("daena.writing", &project_id, "entity.create", without_note)
        .unwrap_or_else(|error| {
            panic!("entity.create without a document should be granted: {error}")
        });
    let denied_save = host
        .authorize_bundled("daena.writing", &project_id, "document.save", save)
        .unwrap_err();
    assert!(
        denied_save.to_string().contains("capability.denied"),
        "{denied_save}"
    );
    drop(project);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn asset_upload_and_shared_list_require_their_capabilities() {
    let root = std::env::temp_dir().join(format!("daena-upload-caps-{}", uuid::Uuid::new_v4()));
    let project = ProjectStore::open_directory(&root).unwrap();
    let project_id = root.to_string_lossy().to_string();
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../packages/modules/language/manifest.json"
    ))
    .unwrap();
    let begin = serde_json::json!({
        "entityId": "entity",
        "namespace": "language",
        "filename": "a.txt",
        "mimeType": "text/plain",
        "size": 1,
        "expectedRevision": "rev",
    });
    let commit = serde_json::json!({"handle": "h", "contentHash": "sha256:x"});

    let mut granted_host = PluginHost::new();
    granted_host
        .register_bundled_json(&manifest.to_string())
        .unwrap();
    sync_project_usage(&project, &mut granted_host).unwrap();
    for (method, payload) in [
        ("asset.upload.begin", &begin),
        ("asset.upload.commit", &commit),
        ("asset.shared.list", &serde_json::json!({})),
    ] {
        granted_host
            .authorize_bundled("daena.language", &project_id, method, payload.clone())
            .unwrap_or_else(|error| panic!("{method} should be granted: {error}"));
    }

    let mut denied_manifest = manifest.clone();
    denied_manifest["capabilities"]
        .as_array_mut()
        .unwrap()
        .retain(|capability| capability != "asset.register" && capability != "asset.read:shared");
    let mut denied_host = PluginHost::new();
    denied_host
        .register_bundled_json(&denied_manifest.to_string())
        .unwrap();
    sync_project_usage(&project, &mut denied_host).unwrap();
    for (method, payload) in [
        ("asset.upload.begin", &begin),
        ("asset.upload.commit", &commit),
        ("asset.shared.list", &serde_json::json!({})),
    ] {
        let denied = denied_host
            .authorize_bundled("daena.language", &project_id, method, payload.clone())
            .unwrap_err();
        assert!(
            denied.to_string().contains("capability.denied"),
            "{method}: {denied}"
        );
    }
    drop(project);
    let _ = std::fs::remove_dir_all(&root);
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
        Some(RecordOwnerConstraint::Package(vec!["language".into()])),
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
        Some(RecordOwnerConstraint::Package(vec!["language".into()])),
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
fn broker_dispatch_allows_effective_schema_record_owners() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let person = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "Person", "type": "daena.lore:person"}),
        None,
    )
    .unwrap();
    let faction = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "Guild", "type": "daena.lore:faction"}),
        None,
    )
    .unwrap();
    let species = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "Drake", "type": "daena.lore:species"}),
        None,
    )
    .unwrap();
    let live = RecordOwnerConstraint::EffectiveSchema {
        live_types: vec![
            "daena.lore:person".into(),
            "daena.lore:faction".into(),
            "daena.lore:species".into(),
        ],
        unique_per_owner: true,
    };
    let profile = serde_json::json!({"schemaVersion": 1, "components": []});
    let created = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(live.clone()),
        "record.create",
        serde_json::json!({
            "collection": "profile",
            "ownerEntityId": person["id"],
            "value": profile
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert_eq!(created["value"]["schemaVersion"], 1);
    dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(live.clone()),
        "record.create",
        serde_json::json!({
            "collection": "profile",
            "ownerEntityId": faction["id"],
            "value": profile
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    let species_profile = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(live),
        "record.create",
        serde_json::json!({
            "collection": "profile",
            "ownerEntityId": species["id"],
            "value": profile
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    let disabled = RecordOwnerConstraint::EffectiveSchema {
        live_types: vec!["daena.lore:person".into(), "daena.lore:faction".into()],
        unique_per_owner: true,
    };
    let denied = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(disabled.clone()),
        "record.create",
        serde_json::json!({
            "collection": "profile",
            "ownerEntityId": species["id"],
            "value": profile
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(denied, Err(CoreError::Unauthorized { .. })));
    let updated = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(disabled),
        "record.update",
        serde_json::json!({
            "collection": "profile",
            "id": species_profile["id"],
            "ownerEntityId": species["id"],
            "value": {"schemaVersion": 1, "components": [{"id": "c1"}]},
            "expectedRevision": species_profile["revision"]
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert_eq!(updated["value"]["components"][0]["id"], "c1");
    let duplicate = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(RecordOwnerConstraint::EffectiveSchema {
            live_types: vec!["daena.lore:person".into()],
            unique_per_owner: true,
        }),
        "record.create",
        serde_json::json!({
            "collection": "profile",
            "ownerEntityId": person["id"],
            "value": profile
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(duplicate, Err(CoreError::Validation(_))));
    let map = dispatch_module_rpc(
        &mut core,
        None,
        None,
        None,
        "entity.create",
        serde_json::json!({"name": "World", "type": "daena.maps:world-map"}),
        None,
    )
    .unwrap();
    let maps_denied = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(RecordOwnerConstraint::EffectiveSchema {
            live_types: vec!["daena.lore:person".into(), "daena.lore:faction".into()],
            unique_per_owner: true,
        }),
        "record.create",
        serde_json::json!({
            "collection": "profile",
            "ownerEntityId": map["id"],
            "value": profile
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(maps_denied, Err(CoreError::Unauthorized { .. })));
    let change = serde_json::json!({
        "schemaVersion": 1,
        "date": { "calendar": "gregorian", "year": 1, "era": "CE", "precision": "year" },
        "patches": [{ "componentId": "str", "value": { "type": "number", "value": 11 } }]
    });
    let live_changes = RecordOwnerConstraint::EffectiveSchema {
        live_types: vec![
            "daena.lore:person".into(),
            "daena.lore:faction".into(),
            "daena.lore:species".into(),
        ],
        unique_per_owner: false,
    };
    let species_change = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(live_changes),
        "record.create",
        serde_json::json!({
            "collection": "profile-change",
            "ownerEntityId": species["id"],
            "value": change
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    let disabled_changes = RecordOwnerConstraint::EffectiveSchema {
        live_types: vec!["daena.lore:person".into(), "daena.lore:faction".into()],
        unique_per_owner: false,
    };
    let change_denied = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(disabled_changes.clone()),
        "record.create",
        serde_json::json!({
            "collection": "profile-change",
            "ownerEntityId": species["id"],
            "value": change
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(change_denied, Err(CoreError::Unauthorized { .. })));
    let change_updated = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(disabled_changes),
        "record.update",
        serde_json::json!({
            "collection": "profile-change",
            "id": species_change["id"],
            "ownerEntityId": species["id"],
            "value": {
                "schemaVersion": 1,
                "date": { "calendar": "gregorian", "year": 2, "era": "CE", "precision": "year" },
                "patches": [{ "componentId": "str", "value": { "type": "number", "value": 12 } }]
            },
            "expectedRevision": species_change["revision"]
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert_eq!(change_updated["value"]["date"]["year"], 2);
}

#[test]
fn broker_dispatch_project_scoped_preset_crud_rejects_entity_owner_gaps() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let project = || RecordOwnerConstraint::Project;
    let entity = || RecordOwnerConstraint::EffectiveSchema {
        live_types: vec!["daena.lore:person".into()],
        unique_per_owner: true,
    };
    let changes = || RecordOwnerConstraint::EffectiveSchema {
        live_types: vec!["daena.lore:person".into()],
        unique_per_owner: false,
    };
    let value = serde_json::json!({
        "schemaVersion": 1,
        "name": "Custom",
        "builtin": true,
        "components": []
    });
    let preset_id = "ef2bce76-ca39-59f5-ab9d-28a0b665eb15";
    let created = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(project()),
        "record.create",
        serde_json::json!({
            "collection": "profile-preset",
            "id": preset_id,
            "value": value
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert!(created.get("owner_entity_id").is_none());
    assert_eq!(created["id"], preset_id);
    let owned = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(project()),
        "record.create",
        serde_json::json!({
            "collection": "profile-preset",
            "ownerEntityId": "daena.lore:person",
            "value": value
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(owned, Err(CoreError::Validation(_))));
    let empty_owner = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(project()),
        "record.create",
        serde_json::json!({
            "collection": "profile-preset",
            "ownerEntityId": " ",
            "value": value
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(empty_owner, Err(CoreError::Validation(_))));
    let listed = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(project()),
        "record.list",
        serde_json::json!({ "collection": "profile-preset" }),
        None,
    )
    .unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    let updated = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(project()),
        "record.update",
        serde_json::json!({
            "collection": "profile-preset",
            "id": preset_id,
            "value": {
                "schemaVersion": 1,
                "name": "Custom sheet",
                "builtin": true,
                "components": []
            },
            "expectedRevision": created["revision"]
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert_eq!(updated["value"]["name"], "Custom sheet");
    assert!(updated.get("owner_entity_id").is_none());
    let missing_profile = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(entity()),
        "record.create",
        serde_json::json!({
            "collection": "profile",
            "value": { "schemaVersion": 1, "components": [] }
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(missing_profile, Err(CoreError::Validation(_))));
    let missing_change = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(changes()),
        "record.create",
        serde_json::json!({
            "collection": "profile-change",
            "value": { "schemaVersion": 1, "patches": [] }
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    );
    assert!(matches!(missing_change, Err(CoreError::Validation(_))));
    dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(project()),
        "record.delete",
        serde_json::json!({
            "collection": "profile-preset",
            "id": preset_id,
            "expectedRevision": updated["revision"]
        }),
        Some(&uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    let after_delete = dispatch_module_rpc(
        &mut core,
        Some("daena.lore"),
        None,
        Some(project()),
        "record.list",
        serde_json::json!({ "collection": "profile-preset" }),
        None,
    )
    .unwrap();
    assert!(after_delete.as_array().unwrap().is_empty());
}

#[test]
fn effective_schema_record_owners_use_merged_overlay_types() {
    let mut core = CoreService::new();
    core.open_memory(AuthorityContext::trusted_shell()).unwrap();
    let package =
        daena_plugin_api::parse_manifest(include_str!("../../packages/modules/lore/manifest.json"))
            .unwrap();
    let overlay = serde_json::json!({
        "version": daena_plugin_api::schema_overlay::SCHEMA_OVERLAY_VERSION,
        "customEntityTypes": [{
            "id": "daena.lore:species",
            "name": "Species",
            "icon": { "kind": "catalog", "id": "animal" },
            "iconColor": { "kind": "preset", "id": "moss" }
        }]
    });
    core.project_mut(AuthorityContext::trusted_shell())
        .unwrap()
        .set_module_schema_overlay("daena.lore".into(), Some(overlay))
        .unwrap();
    let collection = package
        .records
        .iter()
        .find(|collection| collection.id == "profile")
        .expect("lore profile collection");
    let constraint = record_owner_constraint_from_declaration(
        core.project(AuthorityContext::trusted_shell()).unwrap(),
        &package,
        collection,
    )
    .unwrap();
    match constraint {
        RecordOwnerConstraint::EffectiveSchema {
            live_types,
            unique_per_owner,
        } => {
            assert!(unique_per_owner);
            assert!(live_types.iter().any(|id| id == "daena.lore:person"));
            assert!(live_types.iter().any(|id| id == "daena.lore:species"));
            assert!(!live_types.iter().any(|id| id == "daena.maps:world-map"));
        }
        RecordOwnerConstraint::Package(_) | RecordOwnerConstraint::Project => {
            panic!("profile owners use effective schema")
        }
    }
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

fn text_transfer(declared_size: usize, expires_at: Instant) -> BinaryTransfer {
    BinaryTransfer::TextTransfer {
        plugin_id: "daena.writing".into(),
        session_id: "session".into(),
        project_id: "project".into(),
        declared_size,
        next_chunk: 0,
        bytes: Vec::new(),
        committed: false,
        expires_at,
    }
}

#[test]
fn text_transfers_round_trip_and_are_claimed_once_by_their_plugin() {
    let mut manager = BinaryTransferManager::default();
    let text = "Chapter one\nCafé, 日本語, 🙂";
    let bytes = text.as_bytes();
    let token = manager.token(text_transfer(
        bytes.len(),
        Instant::now() + ASSET_TRANSFER_TTL,
    ));
    // A chunk boundary may fall inside a multi-byte character.
    let (first, second) = bytes.split_at(14);
    assert!(manager
        .append_upload(&token, "daena.language", "session", 0, first)
        .is_err());
    assert!(manager
        .append_upload(&token, "daena.writing", "other-session", 0, first)
        .is_err());
    assert_eq!(
        manager
            .append_upload(&token, "daena.writing", "session", 0, first)
            .unwrap(),
        first.len()
    );
    // Nothing is claimable before the commit, and an incomplete commit fails.
    assert!(manager
        .take_text(&token, "daena.writing", "project")
        .is_err());
    assert!(manager
        .commit_text(&token, "daena.writing", "session", "project")
        .unwrap_err()
        .contains("incomplete"));
    assert_eq!(
        manager
            .append_upload(&token, "daena.writing", "session", 1, second)
            .unwrap(),
        bytes.len()
    );
    assert!(manager
        .commit_text(&token, "daena.language", "session", "project")
        .is_err());
    assert!(manager
        .commit_text(&token, "daena.writing", "other-session", "project")
        .is_err());
    assert!(manager
        .commit_text(&token, "daena.writing", "session", "other-project")
        .is_err());
    assert_eq!(
        manager
            .commit_text(&token, "daena.writing", "session", "project")
            .unwrap(),
        bytes.len()
    );
    // A committed transfer is frozen and cannot be committed again.
    assert!(manager
        .append_upload(&token, "daena.writing", "session", 2, b"x")
        .unwrap_err()
        .contains("already committed"));
    assert!(manager
        .commit_text(&token, "daena.writing", "session", "project")
        .is_err());

    // Another plugin or project cannot claim it, and failed claims leave it.
    assert!(manager
        .take_text(&token, "daena.language", "project")
        .is_err());
    assert!(manager
        .take_text(&token, "daena.writing", "other-project")
        .is_err());
    assert_eq!(
        manager
            .take_text(&token, "daena.writing", "project")
            .unwrap(),
        text
    );
    assert!(manager
        .take_text(&token, "daena.writing", "project")
        .is_err());
    assert!(manager.transfers.is_empty());
}

#[test]
fn text_transfers_reject_invalid_utf8_and_oversized_chunks() {
    let mut manager = BinaryTransferManager::default();
    let token = manager.token(text_transfer(3, Instant::now() + ASSET_TRANSFER_TTL));
    manager
        .append_upload(&token, "daena.writing", "session", 0, &[0x61, 0xff, 0xfe])
        .unwrap();
    assert!(manager
        .commit_text(&token, "daena.writing", "session", "project")
        .unwrap_err()
        .contains("UTF-8"));
    assert!(manager
        .take_text(&token, "daena.writing", "project")
        .is_err());
    // The plugin can still cancel what it started; nobody else can.
    assert!(manager.cancel(&token, "daena.language").is_err());
    manager.cancel(&token, "daena.writing").unwrap();
    assert!(manager.transfers.is_empty());

    // More bytes than declared are refused.
    let token = manager.token(text_transfer(2, Instant::now() + ASSET_TRANSFER_TTL));
    assert!(manager
        .append_upload(&token, "daena.writing", "session", 0, b"abc")
        .is_err());
}

#[test]
fn text_transfers_expire_and_chunks_keep_them_alive() {
    let mut manager = BinaryTransferManager::default();
    let expired = manager.token(text_transfer(1, Instant::now() - Duration::from_secs(1)));
    assert!(manager
        .append_upload(&expired, "daena.writing", "session", 0, b"a")
        .is_err());
    let committed_then_expired = manager.token(BinaryTransfer::TextTransfer {
        plugin_id: "daena.writing".into(),
        session_id: "session".into(),
        project_id: "project".into(),
        declared_size: 1,
        next_chunk: 1,
        bytes: b"a".to_vec(),
        committed: true,
        expires_at: Instant::now() - Duration::from_secs(1),
    });
    assert!(manager
        .take_text(&committed_then_expired, "daena.writing", "project")
        .is_err());

    // An accepted chunk pushes the deadline out, which a failed one does not.
    let soon = Instant::now() + Duration::from_millis(500);
    let token = manager.token(text_transfer(2, soon));
    assert!(manager
        .append_upload(&token, "daena.writing", "session", 3, b"a")
        .is_err());
    assert_eq!(
        manager.transfers.get_mut(&token).unwrap().expires_at_mut(),
        &soon
    );
    manager
        .append_upload(&token, "daena.writing", "session", 0, b"a")
        .unwrap();
    assert!(
        *manager.transfers.get_mut(&token).unwrap().expires_at_mut()
            > soon + Duration::from_secs(30)
    );
    // Commit gives the shell a fresh window to claim the text.
    manager
        .append_upload(&token, "daena.writing", "session", 1, b"b")
        .unwrap();
    *manager.transfers.get_mut(&token).unwrap().expires_at_mut() = soon;
    manager
        .commit_text(&token, "daena.writing", "session", "project")
        .unwrap();
    assert!(
        *manager.transfers.get_mut(&token).unwrap().expires_at_mut()
            > soon + Duration::from_secs(30)
    );
    assert_eq!(manager.open_text_transfers("daena.writing", "session"), 1);
    assert_eq!(manager.open_text_transfers("daena.writing", "other"), 0);
}

#[test]
fn text_transfers_round_trip_over_the_plugin_protocol_bridge() {
    let root = std::env::temp_dir().join(format!("daena-text-transfer-{}", uuid::Uuid::new_v4()));
    let bridge = LanguageBridge::open_as(&root, "daena.writing");
    let project_id = current_info(&bridge.core).unwrap().unwrap().root;
    let session_id = bridge.bootstrap(&project_id);

    // Larger than one RPC frame, with multi-byte characters across chunk edges.
    let text = "Prologue — café 日本語 🙂\n".repeat(20_000);
    let begin = bridge.rpc(
        &session_id,
        "text-begin",
        "text.transfer.begin",
        serde_json::json!({ "size": text.len() }),
    );
    assert_eq!(begin["ok"], true, "{begin}");
    assert!(text.len() > begin["result"]["maxChunkBytes"].as_u64().unwrap() as usize);
    let transfer_id = begin["result"]["transferId"].as_str().unwrap().to_string();
    bridge.send_chunks(&begin["result"], text.as_bytes());

    // The shell cannot claim text the plugin has not committed.
    assert!(bridge
        .transfers
        .lock()
        .unwrap()
        .take_text(&transfer_id, "daena.writing", &project_id)
        .is_err());
    let commit = bridge.rpc(
        &session_id,
        "text-commit",
        "text.transfer.commit",
        serde_json::json!({ "transferId": transfer_id }),
    );
    assert_eq!(commit["ok"], true, "{commit}");
    assert_eq!(commit["result"]["size"].as_u64(), Some(text.len() as u64));
    // The webview cannot read the text back through the asset GET route.
    let fetch = tauri::http::Request::builder()
        .method("GET")
        .uri(bridge.protocol_uri(&format!("/__asset/{transfer_id}?sessionId={session_id}")))
        .body(Vec::new())
        .unwrap();
    assert_eq!(bridge.respond(&fetch).status(), 404);
    assert_eq!(
        bridge
            .transfers
            .lock()
            .unwrap()
            .take_text(&transfer_id, "daena.writing", &project_id)
            .unwrap(),
        text
    );
    assert!(bridge
        .transfers
        .lock()
        .unwrap()
        .take_text(&transfer_id, "daena.writing", &project_id)
        .is_err());

    // Refusals at begin: zero, over the cap, malformed and unknown fields.
    for (request_id, payload, expected) in [
        (
            "zero",
            serde_json::json!({ "size": 0 }),
            "text transfer size must be a positive integer",
        ),
        (
            "over",
            serde_json::json!({ "size": MAX_TEXT_TRANSFER_BYTES + 1 }),
            "text transfer exceeds the host limit",
        ),
        (
            "negative",
            serde_json::json!({ "size": -1 }),
            "text transfer size must be a positive integer",
        ),
        ("missing", serde_json::json!({}), "requires size"),
        (
            "extra",
            serde_json::json!({ "size": 1, "mimeType": "text/plain" }),
            "unknown key",
        ),
    ] {
        let reply = bridge.rpc(&session_id, request_id, "text.transfer.begin", payload);
        assert_eq!(reply["ok"], false, "{request_id}: {reply}");
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(message.contains(expected), "{request_id}: {message}");
    }
    // The cap is exactly the documented 32 MiB.
    let at_cap = bridge.rpc(
        &session_id,
        "at-cap",
        "text.transfer.begin",
        serde_json::json!({ "size": MAX_TEXT_TRANSFER_BYTES }),
    );
    assert_eq!(at_cap["ok"], true, "{at_cap}");
    assert_eq!(MAX_TEXT_TRANSFER_BYTES, 32 * 1024 * 1024);

    // A session holds a bounded number of open transfers; cancelling frees one.
    let mut open = vec![at_cap["result"]["transferId"].as_str().unwrap().to_string()];
    loop {
        let reply = bridge.rpc(
            &session_id,
            &format!("open-{}", open.len()),
            "text.transfer.begin",
            serde_json::json!({ "size": 8 }),
        );
        if reply["ok"] != true {
            assert!(reply["error"]["message"]
                .as_str()
                .unwrap()
                .contains("too many text transfers"));
            break;
        }
        open.push(reply["result"]["transferId"].as_str().unwrap().to_string());
        assert!(open.len() <= 8, "the open-transfer cap never applied");
    }
    let cancel = bridge.rpc(
        &session_id,
        "cancel-1",
        "asset.transfer.cancel",
        serde_json::json!({ "handle": open[0] }),
    );
    assert_eq!(cancel["ok"], true, "{cancel}");
    let again = bridge.rpc(
        &session_id,
        "again-1",
        "text.transfer.begin",
        serde_json::json!({ "size": 8 }),
    );
    assert_eq!(again["ok"], true, "{again}");

    // Committing something that is not a text transfer, or a stranger's, fails.
    let bogus = bridge.rpc(
        &session_id,
        "bogus",
        "text.transfer.commit",
        serde_json::json!({ "transferId": "not-a-transfer" }),
    );
    assert_eq!(bogus["ok"], false, "{bogus}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn text_transfer_methods_require_the_text_transfer_capability() {
    let root = std::env::temp_dir().join(format!("daena-text-denied-{}", uuid::Uuid::new_v4()));
    // Language does not declare `text.transfer`, so the broker refuses it.
    let bridge = LanguageBridge::open(&root);
    let project_id = current_info(&bridge.core).unwrap().unwrap().root;
    let session_id = bridge.bootstrap(&project_id);
    for (request_id, method, payload) in [
        (
            "begin",
            "text.transfer.begin",
            serde_json::json!({ "size": 8 }),
        ),
        (
            "commit",
            "text.transfer.commit",
            serde_json::json!({ "transferId": "t" }),
        ),
    ] {
        let reply = bridge.rpc(&session_id, request_id, method, payload);
        assert_eq!(reply["ok"], false, "{method}: {reply}");
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("operation is not granted: text.transfer"),
            "{method}: {message}"
        );
    }
    assert!(bridge.transfers.lock().unwrap().transfers.is_empty());
    let _ = std::fs::remove_dir_all(&root);
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
    let page = String::from_utf8_lossy(response.body());
    assert!(page.contains("./lore.js"));
    assert!(page.contains("id=\"lore-graph-root\""));
    assert!(!page.contains("plugin.js"));

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
fn language_webview_routes_are_served_with_document_content_types() {
    for (path, content_type) in [
        ("/dist/ui/index.html", "text/html"),
        ("/dist/ui/language.js", "text/javascript"),
        ("/dist/ui/language.css", "text/css"),
    ] {
        let request = tauri::http::Request::builder()
            .uri(format!("plugin://daena.language{path}"))
            .body(Vec::new())
            .unwrap();
        let response = plugin_asset_response("daena.language", &request, None, None);
        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            content_type,
            "{path}"
        );
    }
    let unknown = tauri::http::Request::builder()
        .uri("plugin://daena.language/dist/ui/missing.js")
        .body(Vec::new())
        .unwrap();
    assert_eq!(
        plugin_asset_response("daena.language", &unknown, None, None).status(),
        404
    );
}

#[test]
fn writing_webview_routes_are_served_with_document_content_types() {
    for (path, content_type) in [
        ("/dist/ui/index.html", "text/html"),
        ("/dist/ui/writing.js", "text/javascript"),
        ("/dist/ui/writing.css", "text/css"),
    ] {
        let request = tauri::http::Request::builder()
            .uri(format!("plugin://daena.writing{path}"))
            .body(Vec::new())
            .unwrap();
        let response = plugin_asset_response("daena.writing", &request, None, None);
        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            content_type,
            "{path}"
        );
        // The shared rich-text editor writes inline styles, so Writing shares the relaxed style policy.
        let csp = response
            .headers()
            .get("Content-Security-Policy")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            csp.contains("style-src-elem 'self' 'unsafe-inline'"),
            "{path}"
        );
        assert!(csp.contains("script-src 'self';"), "{path}");
        assert!(!csp.contains("script-src 'self' 'unsafe-inline'"), "{path}");
    }
    let page = plugin_asset_response(
        "daena.writing",
        &tauri::http::Request::builder()
            .uri("plugin://daena.writing/dist/ui/index.html")
            .body(Vec::new())
            .unwrap(),
        None,
        None,
    );
    let page = String::from_utf8_lossy(page.body());
    assert!(page.contains("./writing.js"));
    assert!(!page.contains("plugin.js"));
    let unknown = tauri::http::Request::builder()
        .uri("plugin://daena.writing/dist/ui/missing.js")
        .body(Vec::new())
        .unwrap();
    assert_eq!(
        plugin_asset_response("daena.writing", &unknown, None, None).status(),
        404
    );
}

#[test]
fn lore_webview_routes_are_served_with_document_content_types() {
    for (path, content_type) in [
        ("/dist/ui/index.html", "text/html"),
        ("/dist/ui/lore.js", "text/javascript"),
        ("/dist/ui/lore.css", "text/css"),
    ] {
        let request = tauri::http::Request::builder()
            .uri(format!("plugin://daena.lore{path}"))
            .body(Vec::new())
            .unwrap();
        let response = plugin_asset_response("daena.lore", &request, None, None);
        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            content_type,
            "{path}"
        );
        // The graph injects a <style> element, so the lore webview must allow
        // inline styles while keeping scripts and network access locked down.
        let csp = response
            .headers()
            .get("Content-Security-Policy")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            csp.contains("style-src-elem 'self' 'unsafe-inline'"),
            "{path}"
        );
        assert!(csp.contains("script-src 'self';"), "{path}");
        assert!(!csp.contains("script-src 'self' 'unsafe-inline'"), "{path}");
    }
    let unknown = tauri::http::Request::builder()
        .uri("plugin://daena.lore/dist/ui/missing.js")
        .body(Vec::new())
        .unwrap();
    assert_eq!(
        plugin_asset_response("daena.lore", &unknown, None, None).status(),
        404
    );
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
        optional_features: vec![daena_plugin_api::APPEARANCE_FEATURE.to_string()],
        appearance: daena_plugin_api::builtin_plugin_appearance(),
        package_digest: "digest".into(),
        manifest: serde_json::from_str(include_str!("../../packages/modules/lore/manifest.json"))
            .unwrap(),
    })
    .unwrap();
    assert_eq!(value["sessionId"], "session");
    assert!(value.get("session_id").is_none());
    assert_eq!(value["appearance"]["preference"], "system");
    assert_eq!(value["appearance"]["pack"], serde_json::Value::Null);
    assert!(value["appearance"]["tokens"].is_object());
}

#[test]
fn plugin_appearance_apply_js_uses_catalog_ids() {
    let script = plugin_appearance_apply_js();
    assert!(script.contains("\"ink\""));
    assert!(script.contains("\"rail-bg\""));
    assert!(!script.contains("__THEME_TOKEN_IDS__"));
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
        let map = project
            .accept_vector_map(
                "Test Map".into(),
                daena_core::maps::empty_canonical_bytes(),
                serde_json::json!({
                    "id": "daena-landmass",
                    "version": 1,
                    "seed": 1,
                    "settings": {
                        "landPercent": 40,
                        "continentCount": 3,
                        "coastlineRoughness": "medium",
                        "islandFrequency": "medium"
                    }
                }),
                None,
            )
            .unwrap()
            .entity;
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

#[test]
fn close_handshake_confirm_starts_the_close_once() {
    let mut close = CloseHandshake::new();
    assert!(close.request().is_some());
    assert!(close.confirm());
    assert!(
        !close.confirm(),
        "a second confirmation does not start another close"
    );
    assert!(close.is_in_progress());
    assert!(
        close.request().is_none(),
        "a request during the close is held open without arming a timer"
    );
}

#[test]
fn close_handshake_ignores_repeated_requests_while_pending() {
    let mut close = CloseHandshake::new();
    let generation = close.request().expect("the first request is pending");
    assert!(
        close.request().is_none(),
        "a second request while pending arms nothing"
    );
    assert!(close.is_pending());
    assert!(
        close.timed_out(generation),
        "the one armed timer still closes the window"
    );
}

#[test]
fn close_handshake_cancel_keeps_the_window_and_disarms_the_fallback() {
    let mut close = CloseHandshake::new();
    let generation = close.request().expect("the request is pending");
    close.cancel();
    assert!(!close.is_pending());
    assert!(
        !close.timed_out(generation),
        "the fallback does nothing after a cancel"
    );
    assert!(!close.confirm(), "nothing is left to confirm");
    assert!(!close.is_in_progress());
}

#[test]
fn close_handshake_stale_timer_cannot_close_a_later_request() {
    let mut close = CloseHandshake::new();
    let cancelled = close.request().expect("first request");
    close.cancel();
    let later = close.request().expect("second request after the cancel");
    assert_ne!(cancelled, later);
    assert!(
        !close.timed_out(cancelled),
        "the timer of the cancelled request is ignored"
    );
    assert!(
        close.is_pending(),
        "the later request still waits for the shell"
    );
    assert!(
        close.confirm(),
        "the later request is confirmed by the shell"
    );
}

#[test]
fn close_handshake_fallback_closes_when_the_shell_never_answers() {
    let mut close = CloseHandshake::new();
    let generation = close.request().expect("the request is pending");
    assert!(close.timed_out(generation));
    assert!(close.is_in_progress());
    assert!(
        !close.confirm(),
        "a late confirmation does not start a second close"
    );
    close.cancel();
    assert!(
        close.is_in_progress(),
        "a late cancel cannot hold a timed-out close open"
    );
}

#[test]
fn close_handshake_abandon_after_failed_destroy_accepts_requests_again() {
    let mut close = CloseHandshake::new();
    assert!(close.request().is_some());
    assert!(close.confirm());
    close.abandon();
    assert!(!close.is_in_progress());
    assert!(
        close.request().is_some(),
        "a later close request is held and armed again"
    );
}

#[test]
fn close_handshake_acknowledgement_disarms_the_fallback() {
    let mut close = CloseHandshake::new();
    let generation = close.request().expect("the request is pending");
    assert!(close.acknowledge(generation));
    assert!(close.is_acknowledged());
    assert!(
        !close.timed_out(generation),
        "an acknowledged request has no fallback"
    );
    assert!(
        close.is_pending(),
        "the request waits for confirm or cancel"
    );
    assert!(close.confirm(), "the shell can still confirm the close");
}

#[test]
fn close_handshake_stale_acknowledgement_is_ignored() {
    let mut close = CloseHandshake::new();
    let cancelled = close.request().expect("first request");
    close.cancel();
    let later = close.request().expect("second request after the cancel");
    assert!(
        !close.acknowledge(cancelled),
        "an acknowledgement for the cancelled request is ignored"
    );
    assert!(!close.is_acknowledged());
    assert!(
        close.timed_out(later),
        "the later request still falls back to the timer"
    );
}

#[test]
fn close_handshake_without_acknowledgement_the_fallback_still_closes() {
    let mut close = CloseHandshake::new();
    let generation = close.request().expect("the request is pending");
    assert!(close.timed_out(generation));
    assert!(close.is_in_progress());
}

#[test]
fn close_handshake_acknowledgement_needs_a_pending_request() {
    let mut close = CloseHandshake::new();
    assert!(
        !close.acknowledge(0),
        "nothing is pending before the first request"
    );
    let generation = close.request().expect("the request is pending");
    assert!(close.confirm());
    assert!(
        !close.acknowledge(generation),
        "a closing request cannot be acknowledged"
    );
}

#[test]
fn close_handshake_acknowledged_request_ends_in_cancel_and_the_next_one_falls_back() {
    let mut close = CloseHandshake::new();
    let generation = close.request().expect("the request is pending");
    assert!(close.acknowledge(generation));
    close.cancel();
    assert!(!close.is_pending());
    assert!(!close.timed_out(generation));
    let next = close.request().expect("a new request after the cancel");
    assert!(
        !close.is_acknowledged(),
        "a new request starts unacknowledged"
    );
    assert!(close.timed_out(next), "and falls back to the timer");
}

#[test]
fn exit_is_routed_through_the_close_while_the_main_window_is_open() {
    assert!(should_intercept_exit(false, true));
}

#[test]
fn exit_is_allowed_after_our_own_close_or_without_a_main_window() {
    assert!(
        !should_intercept_exit(true, true),
        "our confirmed destroy exits"
    );
    assert!(
        !should_intercept_exit(false, false),
        "the last window went away"
    );
    assert!(!should_intercept_exit(true, false));
}

#[test]
fn quit_closes_the_main_window_when_it_exists_and_exits_otherwise() {
    assert_eq!(quit_action(true), QuitAction::CloseMainWindow);
    assert_eq!(quit_action(false), QuitAction::ExitApp);
}

#[test]
fn app_menu_shortcut_maps_tab_items_and_ignores_quit_and_close() {
    assert_eq!(menu_shortcut("tab-new"), Some("tab-new"));
    assert_eq!(menu_shortcut("tab-reopen"), Some("tab-reopen"));
    assert_eq!(menu_shortcut("tab-close"), Some("tab-close"));
    assert_eq!(menu_shortcut("tab-next"), Some("tab-next"));
    assert_eq!(menu_shortcut("tab-previous"), Some("tab-previous"));
    assert_eq!(menu_shortcut("app-quit"), None);
    assert_eq!(menu_shortcut("window-close"), None);
    assert_eq!(menu_shortcut(""), None);
}

#[test]
fn app_menu_window_close_closes_the_main_window_when_it_exists_and_does_nothing_otherwise() {
    assert_eq!(
        window_close_action(true),
        WindowCloseAction::CloseMainWindow
    );
    assert_eq!(window_close_action(false), WindowCloseAction::Ignore);
}

#[test]
fn project_saves_run_while_a_close_is_pending_and_are_refused_once_confirmed() {
    // The handshake is process-wide, so this is the only test that drives it.
    cancel_close();
    abandon_close();
    assert!(request_close().is_some());
    assert!(
        acknowledge_close(),
        "the shell acknowledges the pending request"
    );
    assert!(
        begin_project_command().is_ok(),
        "saves still run while the shell flushes its draft"
    );
    assert!(confirm_close());
    assert!(
        begin_project_command().is_err(),
        "saves are refused once the close is confirmed"
    );
    abandon_close();
    assert!(begin_project_command().is_ok());
}

#[test]
fn historical_cache_key_changes_with_epoch_view_and_planetary_versions() {
    let identity = format!("sha256:{}", "a".repeat(64));
    let forcing = daena_physical::history::HistoricalForcingParameters::default_for_seed(7);
    let earth = daena_physical::planetary::PlanetaryConfiguration::earth_like();
    let first = historical_cache_key(
        &identity,
        forcing,
        0,
        earth,
        daena_physical::engine::PRODUCT_VIEW_WIDTH,
        daena_physical::engine::PRODUCT_VIEW_HEIGHT,
    );
    assert!(first.contains(&format!(
        "engine:{}",
        daena_physical::engine::WORLD_ENGINE_VERSION
    )));
    assert!(first.contains(&format!("fmt:{}", daena_physical::engine::SOURCE_VERSION)));
    assert!(first.contains("history-v"));
    assert!(first.contains("hazards-v"));
    assert!(first.contains("climate-v"));
    assert!(first.contains("hydro-v"));
    assert_ne!(
        first,
        historical_cache_key(
            &identity,
            forcing,
            1,
            earth,
            daena_physical::engine::PRODUCT_VIEW_WIDTH,
            daena_physical::engine::PRODUCT_VIEW_HEIGHT,
        )
    );
    assert_ne!(
        first,
        historical_cache_key(
            &identity,
            forcing,
            0,
            earth,
            daena_physical::engine::PRODUCT_VIEW_WIDTH * 2,
            daena_physical::engine::PRODUCT_VIEW_HEIGHT * 2,
        )
    );
    let mut close = earth;
    close.retained_heat_centi_c += 1;
    assert_ne!(
        first,
        historical_cache_key(
            &identity,
            forcing,
            0,
            close,
            daena_physical::engine::PRODUCT_VIEW_WIDTH,
            daena_physical::engine::PRODUCT_VIEW_HEIGHT,
        )
    );
    let other = daena_physical::history::HistoricalForcingParameters::default_for_seed(8);
    assert_ne!(
        first,
        historical_cache_key(
            &identity,
            other,
            0,
            earth,
            daena_physical::engine::PRODUCT_VIEW_WIDTH,
            daena_physical::engine::PRODUCT_VIEW_HEIGHT,
        )
    );
}

#[test]
fn volcanic_source_ids_serialize_as_decimal_strings_beyond_two_to_the_53() {
    let products = engine_products(32);
    let mut hazards = products.hazards().clone();
    let id = u64::MAX - 1;
    hazards
        .volcanic_sources
        .push(daena_physical::products::VolcanicSource {
            stable_id: id,
            cell: 0,
            origin: daena_physical::products::VolcanicOrigin::Hotspot,
            activity_class: daena_physical::products::VolcanicActivity::Dormant,
            eruptions_per_model_year_nano: 1,
        });
    let json = engine_hazard_products(&hazards, products.resolution(), 6_371_000.0);
    let value = &json["volcanicSources"].as_array().unwrap().last().unwrap()["stableId"];
    assert_eq!(value, &serde_json::json!(id.to_string()));
    assert_eq!(value.as_str().unwrap().parse::<u64>().unwrap(), id);
}

#[test]
fn physical_hazard_products_place_canonical_sources_and_match_the_generation_grid() {
    let products = engine_products(32);
    let generation = products.generation().unwrap().clone();
    let (resolution, radius_metres) = generation_source_grid(&generation).unwrap();
    assert_eq!(resolution, 32);
    let snapshot = daena_physical::engine::decode_source(products.source())
        .unwrap()
        .settings_snapshot();
    assert_eq!(resolution, snapshot.resolution);
    assert_eq!(radius_metres, snapshot.radius_m);

    let mut hazards = products.hazards().clone();
    hazards
        .volcanic_sources
        .push(daena_physical::products::VolcanicSource {
            stable_id: u64::MAX - 1,
            cell: 0,
            origin: daena_physical::products::VolcanicOrigin::SubductionArc,
            activity_class: daena_physical::products::VolcanicActivity::Vigorous,
            eruptions_per_model_year_nano: 2,
        });
    let json = engine_hazard_products(&hazards, resolution, radius_metres);
    let view_cells = daena_physical::engine::PRODUCT_VIEW_WIDTH as usize
        * daena_physical::engine::PRODUCT_VIEW_HEIGHT as usize;
    for key in [
        "earthquakeHazardPpm",
        "volcanicHazardPpm",
        "earthquakeAnnualRateNano",
        "volcanicAnnualRateNano",
        "earthquakeRatePerMillionYearsPpm",
        "eruptionRatePerMillionYearsPpm",
    ] {
        assert_eq!(json[key].as_array().unwrap().len(), view_cells);
    }
    let source = json["volcanicSources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["stableId"] == serde_json::json!((u64::MAX - 1).to_string()))
        .unwrap();
    let [longitude, latitude] =
        daena_physical::engine::canonical_cell_center_microdegrees(resolution, radius_metres, 0)
            .unwrap();
    assert_eq!(source["longitudeMicro"], serde_json::json!(longitude));
    assert_eq!(source["latitudeMicro"], serde_json::json!(latitude));
    assert_eq!(source["origin"], "subduction-arc");
    assert_eq!(source["activityClass"], "vigorous");
}

#[test]
fn engine_cache_rebuilds_after_the_derived_directory_is_deleted() {
    let products = engine_products(32);
    let generation = products.generation().unwrap().clone();
    let source = products.source().to_vec();
    let validated = daena_core::maps::physical::validate_source(&source, &generation).unwrap();
    let root = std::env::temp_dir().join(format!("daena-engine-cache-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let planetary = daena_physical::planetary::PlanetaryConfiguration::earth_like();
    let first = load_or_fill_engine_static(
        root.to_str().unwrap(),
        &validated.identity,
        &validated.source,
        planetary,
    )
    .unwrap();
    let cache_root = root.join(daena_core::maps::physical::PHYSICAL_DERIVED_CACHE_RELATIVE);
    assert!(cache_root.exists());
    std::fs::remove_dir_all(&cache_root).unwrap();
    let rebuilt = load_or_fill_engine_static(
        root.to_str().unwrap(),
        &validated.identity,
        &validated.source,
        planetary,
    )
    .unwrap();
    assert_eq!(first, rebuilt);
    std::fs::remove_dir_all(&root).unwrap();
}

fn largest_landmass_pair(surface: &daena_physical::engine::ToolSurface) -> (u32, u32) {
    let mut counts = BTreeMap::<u32, Vec<u32>>::new();
    for cell in 0..surface.cell_count() as u32 {
        let id = surface.landmass_id(cell);
        if id != daena_physical::engine::TOOL_SURFACE_NONE {
            counts.entry(id).or_default().push(cell);
        }
    }
    let cells = counts
        .into_iter()
        .max_by_key(|(_, cells)| cells.len())
        .expect("a generated world has land")
        .1;
    let latitude_key =
        |cell: &&u32| (surface.cell_lat_lon_degrees(**cell).0 * 1_000.0).round() as i64;
    let north = *cells
        .iter()
        .max_by_key(latitude_key)
        .expect("a landmass has cells");
    let south = *cells
        .iter()
        .min_by_key(latitude_key)
        .expect("a landmass has cells");
    (south, north)
}

#[test]
#[ignore = "writes E8 identity artifacts when DAENA_E8_ARTIFACTS is set"]
fn e8_identity_artifacts() {
    let Some(dir) = std::env::var_os("DAENA_E8_ARTIFACTS") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let specs = [
        (
            831_429u64,
            512u32,
            daena_physical::engine::WorldPreset::Earthlike,
        ),
        (
            831_429,
            1024,
            daena_physical::engine::WorldPreset::Earthlike,
        ),
        (4, 64, daena_physical::engine::WorldPreset::Earthlike),
        (4, 64, daena_physical::engine::WorldPreset::Pangea),
    ];
    for (seed, resolution, preset) in specs {
        let settings = daena_physical::engine::Settings::from_parts(
            seed,
            preset,
            resolution,
            daena_physical::DEFAULT_RADIUS_METRES,
            0,
            daena_physical::planetary::PlanetaryConfiguration::earth_like(),
        );
        let products =
            daena_physical::engine::generate(settings, &mut daena_physical::NoopProgress).unwrap();
        let label = format!("{}-{}-{}", seed, resolution, preset.as_str());
        let write = |name: &str, value: &serde_json::Value| {
            std::fs::write(
                dir.join(format!("{label}.{name}.json")),
                serde_json::to_vec(value).unwrap(),
            )
            .unwrap();
        };
        let generation = products.generation().unwrap().clone();
        write("generation", &generation);
        let validated =
            daena_core::maps::physical::validate_source(products.source(), &generation).unwrap();
        let snapshot = validated.source.settings_snapshot();
        write("climate", &engine_climate_products(products.climate()));
        write(
            "hydrology",
            &engine_hydrology_products(products.hydrology()),
        );
        write(
            "hazards",
            &engine_hazard_products(products.hazards(), snapshot.resolution, snapshot.radius_m),
        );
        std::fs::write(dir.join(format!("{label}.source.bin")), products.source()).unwrap();
        if resolution != 64 {
            continue;
        }
        let surface =
            tool_surface_for_source(&validated.identity, &validated.source, &generation, 0)
                .unwrap();
        let find = physical_find_place_response(
            &validated.identity,
            &validated.source,
            &generation,
            0,
            &daena_physical::find_place::FindPlaceQuery {
                temperature_centi_c_min: Some(-10_000),
                precipitation_mm_min: Some(0),
                ..daena_physical::find_place::FindPlaceQuery::default()
            },
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{label}.find.json")),
            serde_json::to_vec(&find).unwrap(),
        )
        .unwrap();
        let epoch_find = physical_find_place_response(
            &validated.identity,
            &validated.source,
            &generation,
            -21_000,
            &daena_physical::find_place::FindPlaceQuery {
                temperature_centi_c_min: Some(-10_000),
                precipitation_mm_min: Some(0),
                ..daena_physical::find_place::FindPlaceQuery::default()
            },
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{label}.epoch-find.json")),
            serde_json::to_vec(&epoch_find).unwrap(),
        )
        .unwrap();
        let land = (0..surface.cell_count() as u32)
            .filter(|cell| !surface.ocean(*cell))
            .collect::<Vec<_>>();
        let start = land.first().copied().unwrap();
        let end = surface
            .neighbors(start)
            .into_iter()
            .find(|cell| {
                *cell != daena_physical::engine::TOOL_SURFACE_NONE && !surface.ocean(*cell)
            })
            .unwrap_or(start);
        let start_point = surface.center_microdegrees(start);
        let end_point = surface.center_microdegrees(end);
        let routes = physical_routes_response(
            &validated.identity,
            &validated.source,
            &generation,
            0,
            &daena_physical::routing::RouteSuggestQuery {
                start_longitude_microdegrees: start_point[0],
                start_latitude_microdegrees: start_point[1],
                end_longitude_microdegrees: end_point[0],
                end_latitude_microdegrees: end_point[1],
                existing_routes: Vec::new(),
            },
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{label}.routes.json")),
            serde_json::to_vec(&routes).unwrap(),
        )
        .unwrap();
        let region = physical_region_response(
            &validated.identity,
            &validated.source,
            &generation,
            0,
            start_point[0],
            start_point[1],
            daena_physical::region::DETECTOR_LANDMASS,
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{label}.region.json")),
            serde_json::to_vec(&region).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn physical_tool_commands_use_canonical_cells_and_the_surface_cache() {
    let products = engine_products(64);
    let generation = products.generation().unwrap().clone();
    let validated =
        daena_core::maps::physical::validate_source(products.source(), &generation).unwrap();
    let surface =
        tool_surface_for_source(&validated.identity, &validated.source, &generation, 0).unwrap();
    let cached =
        tool_surface_for_source(&validated.identity, &validated.source, &generation, 0).unwrap();
    assert_eq!(cached.cell_count(), surface.cell_count());
    let cache_bytes = tool_surface_cache()
        .lock()
        .unwrap()
        .iter()
        .map(|(_, entry)| entry.heap_bytes())
        .sum::<usize>();
    assert!(cache_bytes <= TOOL_SURFACE_CACHE_BYTES);
    let canonical_cells = 6 * validated.source.settings_snapshot().resolution.pow(2);

    let find = physical_find_place_response(
        &validated.identity,
        &validated.source,
        &generation,
        0,
        &daena_physical::find_place::FindPlaceQuery {
            temperature_centi_c_min: Some(-10_000),
            precipitation_mm_min: Some(0),
            ..daena_physical::find_place::FindPlaceQuery::default()
        },
    )
    .unwrap();
    assert!(find.candidate_count >= 1);
    assert!(find.candidate_count <= daena_physical::find_place::FIND_PLACE_MAX_CANDIDATES);
    for candidate in &find.candidates {
        assert!(!candidate.cells.is_empty());
        assert!(candidate.cells.iter().all(|cell| *cell < canonical_cells));
        assert!(
            candidate.means.island_id < canonical_cells
                || candidate.means.island_id == daena_physical::engine::TOOL_SURFACE_NONE
        );
    }

    let (start_cell, end_cell) = largest_landmass_pair(&surface);
    let start = surface.center_microdegrees(start_cell);
    let end = surface.center_microdegrees(end_cell);
    let routes = physical_routes_response(
        &validated.identity,
        &validated.source,
        &generation,
        0,
        &daena_physical::routing::RouteSuggestQuery {
            start_longitude_microdegrees: start[0],
            start_latitude_microdegrees: start[1],
            end_longitude_microdegrees: end[0],
            end_latitude_microdegrees: end[1],
            existing_routes: Vec::new(),
        },
    )
    .unwrap();
    assert!(routes.suggestion_count >= 1);
    for suggestion in &routes.suggestions {
        assert_eq!(suggestion.cells.first().copied(), Some(start_cell));
        assert_eq!(suggestion.cells.last().copied(), Some(end_cell));
        assert!(suggestion
            .cells
            .iter()
            .all(|cell| *cell < canonical_cells && !surface.ocean(*cell)));
    }

    let click = surface
        .center_microdegrees(cells_for_landmass(&surface, surface.landmass_id(start_cell))[0]);
    let region = physical_region_response(
        &validated.identity,
        &validated.source,
        &generation,
        0,
        click[0],
        click[1],
        daena_physical::region::DETECTOR_LANDMASS,
    )
    .unwrap();
    assert_eq!(region.region_id, surface.landmass_id(start_cell));
    assert!(region.cell_count > 0);
    assert!(region.epoch_dependent);
    let geometry_type = region.geometry["type"].as_str().unwrap_or_default();
    assert!(geometry_type == "Polygon" || geometry_type == "MultiPolygon");
}

fn cells_for_landmass(surface: &daena_physical::engine::ToolSurface, id: u32) -> Vec<u32> {
    (0..surface.cell_count() as u32)
        .filter(|cell| surface.landmass_id(*cell) == id)
        .collect()
}

#[test]
fn physical_tool_commands_honor_a_non_zero_epoch() {
    let products = engine_products(64);
    let generation = products.generation().unwrap().clone();
    let validated =
        daena_core::maps::physical::validate_source(products.source(), &generation).unwrap();
    let forcing = historical_forcing_from_generation(&generation).expect("generation forcing");
    let epoch = forcing.period_years() / 4;
    let reference =
        tool_surface_for_source(&validated.identity, &validated.source, &generation, 0).unwrap();
    let epoch_surface =
        tool_surface_for_source(&validated.identity, &validated.source, &generation, epoch)
            .unwrap();
    assert!(!Arc::ptr_eq(&reference, &epoch_surface));
    assert_eq!(epoch_surface.sea_level_mm, {
        let solved = daena_physical::history::solve_epoch(
            &validated.source,
            forcing,
            epoch,
            &daena_physical::NoopProgress,
        )
        .unwrap();
        solved.sea_level_mm
    });
    let land = (0..epoch_surface.cell_count() as u32)
        .find(|cell| !epoch_surface.ocean(*cell))
        .expect("epoch world has land");
    let point = epoch_surface.center_microdegrees(land);
    let find = physical_find_place_response(
        &validated.identity,
        &validated.source,
        &generation,
        epoch,
        &daena_physical::find_place::FindPlaceQuery {
            temperature_centi_c_min: Some(-20_000),
            ..daena_physical::find_place::FindPlaceQuery::default()
        },
    )
    .unwrap();
    assert!(find.matched_cells > 0);
    let region = physical_region_response(
        &validated.identity,
        &validated.source,
        &generation,
        epoch,
        point[0],
        point[1],
        daena_physical::region::DETECTOR_LANDMASS,
    )
    .unwrap();
    assert!(region.cell_count > 0);
    assert_eq!(region.region_id, epoch_surface.landmass_id(land));
}

fn record_e5a_measurement(line: &str) {
    use std::io::Write;
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/e5a-measurements.txt");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("measurement file");
    writeln!(file, "{line}").expect("measurement line");
}

#[test]
#[ignore = "wall-clock: release-only E5a timing and surface size table"]
fn e5a_release_timing_table() {
    for resolution in [512, 1024] {
        clear_tool_surface_cache().unwrap();
        let products = engine_products(resolution);
        let generation = products.generation().unwrap().clone();
        let validated =
            daena_core::maps::physical::validate_source(products.source(), &generation).unwrap();
        let identity = format!("{}|e5a-{resolution}", validated.identity);
        let forcing = historical_forcing_from_generation(&generation).unwrap();
        let epoch = forcing.period_years() / 4;

        let started = Instant::now();
        let present =
            tool_surface_for_source(&identity, &validated.source, &generation, 0).unwrap();
        let present_time = started.elapsed();
        let started = Instant::now();
        let epoch_surface =
            tool_surface_for_source(&identity, &validated.source, &generation, epoch).unwrap();
        let epoch_time = started.elapsed();
        record_e5a_measurement(&format!(
            "n={resolution}: present surface {} bytes ({:.2} B/cell) cold {present_time:?}; epoch surface {} bytes ({:.2} B/cell) cold {epoch_time:?}; epoch sea level {} mm",
            present.heap_bytes(),
            present.heap_bytes() as f64 / present.cell_count() as f64,
            epoch_surface.heap_bytes(),
            epoch_surface.heap_bytes() as f64 / epoch_surface.cell_count() as f64,
            epoch_surface.sea_level_mm
        ));

        let started = Instant::now();
        let find = physical_find_place_response(
            &identity,
            &validated.source,
            &generation,
            0,
            &daena_physical::find_place::FindPlaceQuery {
                temperature_centi_c_min: Some(0),
                precipitation_mm_min: Some(400),
                coast_distance_km_max: Some(500),
                ..daena_physical::find_place::FindPlaceQuery::default()
            },
        )
        .unwrap();
        let find_time = started.elapsed();

        let (start_cell, end_cell) = largest_landmass_pair(&present);
        let start = present.center_microdegrees(start_cell);
        let end = present.center_microdegrees(end_cell);
        let route_query = daena_physical::routing::RouteSuggestQuery {
            start_longitude_microdegrees: start[0],
            start_latitude_microdegrees: start[1],
            end_longitude_microdegrees: end[0],
            end_latitude_microdegrees: end[1],
            existing_routes: Vec::new(),
        };
        let started = Instant::now();
        let routes =
            physical_routes_response(&identity, &validated.source, &generation, 0, &route_query)
                .unwrap();
        let route_time = started.elapsed();

        let (land, _) = largest_landmass_pair(&epoch_surface);
        let point = epoch_surface.center_microdegrees(land);
        let started = Instant::now();
        let region = physical_region_response(
            &identity,
            &validated.source,
            &generation,
            epoch,
            point[0],
            point[1],
            daena_physical::region::DETECTOR_LANDMASS,
        )
        .unwrap();
        let region_time = started.elapsed();
        record_e5a_measurement(&format!(
            "n={resolution}: warm find place {find_time:?} ({} candidates, {} matched); warm route {route_time:?} ({} suggestions, {} cells); warm region {region_time:?} ({} cells, {} bytes geometry)",
            find.candidate_count,
            find.matched_cells,
            routes.suggestion_count,
            routes.suggestions.first().map_or(0, |item| item.cell_count),
            region.cell_count,
            region.geometry.to_string().len()
        ));
        drop(present);
        drop(epoch_surface);
        clear_tool_surface_cache().unwrap();
    }
}

#[test]
fn physical_event_materialization_uses_engine_events_and_survives_rebuild() {
    let root = std::env::temp_dir().join(format!("daena-engine-events-{}", uuid::Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let products = engine_products(64);
    let generation = products.generation().unwrap().clone();
    let accepted = store
        .accept_physical_map(
            "Engine events".into(),
            products.source().to_vec(),
            generation.clone(),
            Some("00000000-0000-4000-8000-0000000000e6"),
        )
        .unwrap();
    let map_id = accepted.entity.id.clone();
    let validated =
        daena_core::maps::physical::validate_source(products.source(), &generation).unwrap();
    let flood_request = daena_physical::events::EventMaterializationRequest {
        event_kind: daena_physical::events::NaturalEventKind::Flood,
        interval_start_years: -100,
        interval_end_years: 100,
        max_events: 16,
        hazard_seed: 7,
    };
    let floods = physical_materialize_events_response(
        &store,
        &map_id,
        &flood_request,
        Some(uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert!(!floods.events.is_empty());
    assert_eq!(floods.materialization_version, 3);
    assert_eq!(floods.model_label, "engine-lake-spill-v3");
    let surface = tool_surface_for_source(
        &accepted.physical_identity,
        &validated.source,
        &generation,
        0,
    )
    .unwrap();
    let provenance = |entity_id: &str| {
        store
            .list_fields(entity_id.to_string())
            .unwrap()
            .into_iter()
            .find(|field| {
                field.namespace == daena_core::maps::PHYSICAL_EVENT_NAMESPACE
                    && field.key == "provenance"
            })
            .unwrap()
            .value
    };
    for event in &floods.events {
        let value = serde_json::to_value(event).unwrap();
        let minimum_cell = event.event.sampled_center_id.unwrap();
        let lake_id =
            daena_physical::hydro_claim::canonical_lake_id(u32::try_from(minimum_cell).unwrap());
        assert_eq!(
            value["sampledCenterId"],
            serde_json::json!(minimum_cell.to_string())
        );
        let locations = store.map_locations(event.entity_id.clone()).unwrap();
        assert_eq!(locations.len(), 1);
        match &locations[0].anchor {
            daena_core::maps::Anchor::ProviderFeature { feature_id, .. } => {
                assert_eq!(feature_id, &lake_id);
                assert!(daena_physical::hydro_claim::resolve_canonical(
                    &surface,
                    daena_physical::hydro_claim::KIND_LAKE,
                    feature_id
                )
                .is_some());
            }
            other => panic!("flood anchor is {other:?}"),
        }
        let provenance = provenance(&event.entity_id);
        assert_eq!(provenance["cellKind"], "canonical-cube");
        assert_eq!(provenance["sourceResolution"], 64);
        assert_eq!(
            provenance["sampledCenterId"],
            serde_json::json!(minimum_cell.to_string())
        );
        assert_eq!(provenance["eventModel"], "engine-lake-spill-v3");
        assert_eq!(provenance["generatorId"], daena_physical::GENERATOR_ID);
        assert_eq!(
            provenance["generatorVersion"],
            u64::from(daena_physical::engine::WORLD_ENGINE_VERSION)
        );
        assert!(provenance.get("sourceRetryIndex").is_none());
        assert_eq!(provenance["fillPpm"], 1_000_000);
    }

    let earthquake_request = daena_physical::events::EventMaterializationRequest {
        event_kind: daena_physical::events::NaturalEventKind::Earthquake,
        interval_start_years: 0,
        interval_end_years: 1_000,
        max_events: 8,
        hazard_seed: 11,
    };
    let earthquakes = physical_materialize_events_response(
        &store,
        &map_id,
        &earthquake_request,
        Some(uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert!(!earthquakes.events.is_empty());
    assert_eq!(earthquakes.model_label, "poisson-gutenberg-richter-v2");
    let earthquake_provenance = provenance(&earthquakes.events[0].entity_id);
    assert_eq!(earthquake_provenance["cellKind"], "canonical-cube");
    assert_eq!(
        earthquake_provenance["sampledCenterId"],
        serde_json::Value::Null
    );
    assert_eq!(
        earthquake_provenance["eventModel"],
        "poisson-gutenberg-richter-v2"
    );
    let years = (earthquake_request.interval_end_years - earthquake_request.interval_start_years
        + 1) as f64;
    let sampled_mass = validated
        .source
        .sample_events(None, &earthquake_request, &mut daena_physical::NoopProgress)
        .unwrap()
        .expected_lambda;
    assert!(
        (sampled_mass - products.hazards().metrics.earthquake_rate_mass_nano as f64 / 1e9 * years)
            .abs()
            < 1e-9
    );
    let eruption_request = daena_physical::events::EventMaterializationRequest {
        event_kind: daena_physical::events::NaturalEventKind::Eruption,
        interval_start_years: 0,
        interval_end_years: 1_000,
        max_events: 8,
        hazard_seed: 11,
    };
    let eruption_mass = validated
        .source
        .sample_events(None, &eruption_request, &mut daena_physical::NoopProgress)
        .unwrap()
        .expected_lambda;
    assert!(
        (eruption_mass - products.hazards().metrics.volcanic_rate_mass_nano as f64 / 1e9 * years)
            .abs()
            < 1e-9
    );

    let entity_count = store.list_entities().unwrap().len();
    let first_ids = floods
        .events
        .iter()
        .map(|event| event.entity_id.clone())
        .collect::<Vec<_>>();
    let rerun = physical_materialize_events_response(
        &store,
        &map_id,
        &flood_request,
        Some(uuid::Uuid::new_v4().to_string()),
    )
    .unwrap();
    assert_eq!(
        rerun
            .events
            .iter()
            .map(|event| event.entity_id.clone())
            .collect::<Vec<_>>(),
        first_ids
    );
    assert_eq!(store.list_entities().unwrap().len(), entity_count);

    let event_ids = floods
        .events
        .iter()
        .chain(earthquakes.events.iter())
        .map(|event| event.entity_id.clone())
        .collect::<Vec<_>>();
    store
        .flush_checkpoint("physical event materialization test")
        .unwrap();
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let reopened = ProjectStore::open_directory(&root).unwrap();
    let entities = reopened.list_entities().unwrap();
    for id in &event_ids {
        assert!(entities.iter().any(|entity| &entity.id == id));
        assert!(reopened
            .list_fields(id.clone())
            .unwrap()
            .iter()
            .any(|field| {
                field.namespace == daena_core::maps::PHYSICAL_EVENT_NAMESPACE
                    && field.key == "provenance"
            }));
    }
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

fn atlas_studio_get(uri: &str) -> tauri::http::Request<Vec<u8>> {
    tauri::http::Request::builder()
        .method("GET")
        .uri(uri)
        .body(Vec::new())
        .unwrap()
}

#[test]
fn atlas_studio_protocol_accepts_zoom_12_and_rejects_13() {
    let token = "00000000-0000-4000-8000-0000000000ab";
    let parsed = crate::atlas_studio::parse_studio_tile_request(&atlas_studio_get(&format!(
        "atlas-studio://localhost/{token}/12/15/20.png?scale=1"
    )))
    .unwrap();
    assert_eq!(parsed.z, 12);
    assert_eq!(parsed.x, 15);
    assert_eq!(parsed.y, 20);
    let rejected = crate::atlas_studio::parse_studio_tile_request(&atlas_studio_get(&format!(
        "atlas-studio://localhost/{token}/13/0/0.png"
    )))
    .unwrap_err();
    assert_eq!(rejected.code, daena_atlas::studio::CODE_STUDIO_TILE_INVALID);
}

#[test]
fn atlas_studio_legacy_zoom_stops_at_eight_and_terrain_styles_select_terrain() {
    assert_eq!(
        crate::atlas_studio::studio_render_kind("daena-atlas-imagery"),
        "terrain"
    );
    assert_eq!(
        crate::atlas_studio::studio_render_kind("daena-atlas-relief"),
        "terrain"
    );
    assert_eq!(
        crate::atlas_studio::studio_render_kind("daena-atlas-biome"),
        "legacy"
    );
    assert_eq!(
        crate::atlas_studio::studio_render_kind("daena-atlas-antique"),
        "legacy"
    );
    assert!(crate::atlas_studio::studio_zoom_allowed("legacy", 8));
    assert!(!crate::atlas_studio::studio_zoom_allowed("legacy", 9));
    assert!(crate::atlas_studio::studio_zoom_allowed("terrain", 12));
    assert!(!crate::atlas_studio::studio_zoom_allowed("terrain", 13));
}

#[test]
fn atlas_studio_parses_vector_tiles_and_legacy_rejects_them() {
    let token = "00000000-0000-4000-8000-0000000000ab";
    let rivers = crate::atlas_studio::parse_studio_tile_request(&atlas_studio_get(&format!(
        "atlas-studio://localhost/{token}/4/1/2.rivers.json"
    )))
    .unwrap();
    assert_eq!(rivers.z, 4);
    assert_eq!(rivers.x, 1);
    assert_eq!(rivers.y, 2);
    assert_eq!(
        rivers.layer,
        Some(daena_atlas::terrain::TerrainVectorLayer::Rivers)
    );
    assert_eq!(rivers.device_scale, 1);
    assert!(!rivers.prefetch);
    let contours = crate::atlas_studio::parse_studio_tile_request(&atlas_studio_get(&format!(
        "atlas-studio://localhost/{token}/12/3/4.contours.json?priority=prefetch"
    )))
    .unwrap();
    assert_eq!(contours.z, 12);
    assert_eq!(contours.x, 3);
    assert_eq!(contours.y, 4);
    assert_eq!(
        contours.layer,
        Some(daena_atlas::terrain::TerrainVectorLayer::Contours)
    );
    let streams = crate::atlas_studio::parse_studio_tile_request(&atlas_studio_get(&format!(
        "atlas-studio://localhost/{token}/8/5/6.streams.json"
    )))
    .unwrap();
    assert_eq!(streams.z, 8);
    assert_eq!(streams.x, 5);
    assert_eq!(streams.y, 6);
    assert_eq!(
        streams.layer,
        Some(daena_atlas::terrain::TerrainVectorLayer::Streams)
    );
    assert!(contours.prefetch);
    let foo = crate::atlas_studio::parse_studio_tile_request(&atlas_studio_get(&format!(
        "atlas-studio://localhost/{token}/1/0/0.foo.json"
    )))
    .unwrap_err();
    assert_eq!(foo.code, daena_atlas::studio::CODE_STUDIO_TILE_INVALID);
    let rejected = crate::atlas_studio::reject_studio_tile_for_kind("legacy", &rivers).unwrap_err();
    assert_eq!(rejected.code, daena_atlas::studio::CODE_STUDIO_TILE_INVALID);
}
