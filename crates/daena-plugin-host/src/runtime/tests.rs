use super::*;
use daena_plugin_api::{Entrypoints, PluginKind};

fn manifest(kind: PluginKind) -> PluginManifest {
    PluginManifest {
        manifest_version: 1,
        id: "com.example.plugin".into(),
        name: "Plugin".into(),
        version: "1.0.0".into(),
        publisher: "com.example".into(),
        enabled_by_default: None,
        stability: None,
        host_api: ">=1.0.0 <2.0.0".into(),
        kind,
        entrypoints: Entrypoints {
            ui: Some("dist/index.html".into()),
            wasm: None,
        },
        capabilities: vec![],
        dependencies: Default::default(),
        namespaces: vec![],
        schemas: vec![],
        templates: vec![],
        records: vec![],
        views: vec![],
        commands: vec![],
        services: daena_plugin_api::Services {
            provides: vec![],
            consumes: vec![],
        },
        events: daena_plugin_api::Events {
            publishes: vec![],
            subscribes: vec![],
        },
        migrations: vec![],
    }
}

#[test]
fn policy_uses_an_app_controlled_origin_and_denies_ambient_network() {
    let policy = webview_policy(&manifest(PluginKind::Sandboxed)).unwrap();
    assert_eq!(policy.protocol, "plugin");
    assert_eq!(policy.label, "plugin:com-example-plugin");
    assert_eq!(policy.origin, "plugin://com.example.plugin");
    assert_eq!(policy.url, "plugin://com.example.plugin/dist/index.html");
    assert!(policy.csp.contains("connect-src 'self'"));
    assert!(!policy.csp.contains("http://") && !policy.csp.contains("https://"));
    assert!(webview_policy(&manifest(PluginKind::Declarative)).is_none());
}

#[test]
fn bridge_rejects_oversized_payloads() {
    let request = RpcRequest {
        rpc_version: 1,
        session_id: "s".into(),
        request_id: "r".into(),
        method: "entity.read".into(),
        payload: serde_json::json!({"data": "x".repeat(MAX_RPC_BYTES)}),
    };
    assert_eq!(
        validate_bridge_request(&request),
        Err("bridge request exceeds payload limit")
    );
}

#[test]
fn wasm_runtime_rejects_ambient_imports_and_runs_a_pure_module() {
    let runtime = WasmRuntime::new(WasmLimits {
        timeout: Duration::from_millis(50),
        ..Default::default()
    })
    .unwrap();
    let pure = wat::parse_str("(module (func (export \"run\") (result i32) i32.const 7))").unwrap();
    assert_eq!(runtime.run(&pure).unwrap(), 7);
    let imported =
        wat::parse_str("(module (import \"wasi_snapshot_preview1\" \"clock_time_get\" (func)))")
            .unwrap();
    assert!(matches!(
        runtime.run(&imported),
        Err(WasmFailure::AmbientImport(_))
    ));
}

#[test]
fn wasm_json_service_abi_round_trips_a_bounded_response() {
    let runtime = WasmRuntime::new(WasmLimits::default()).unwrap();
    let module = wat::parse_str(
        r#"(module
                (memory (export "memory") 1 1)
                (data (i32.const 16) "{\"ok\":true}")
                (func (export "alloc") (param i32) (result i32) i32.const 0)
                (func (export "handle_json") (param i32 i32) (result i64) i64.const 47244640272)
            )"#,
    )
    .unwrap();
    let mut loaded = runtime.clone();
    loaded.load(&module).unwrap();
    assert_eq!(
        loaded
            .invoke_service(&serde_json::json!({"request": true}))
            .unwrap(),
        serde_json::json!({"ok": true})
    );
}

#[test]
fn wasm_runtime_enforces_fuel_and_memory_limits() {
    let fuel_runtime = WasmRuntime::new(WasmLimits {
        fuel: 10_000,
        timeout: Duration::from_secs(1),
        ..Default::default()
    })
    .unwrap();
    let loop_module = wat::parse_str("(module (func (export \"run\") (loop br 0)))").unwrap();
    let loop_result = fuel_runtime.run(&loop_module);
    assert!(
        loop_result.is_err(),
        "runaway module completed unexpectedly: {loop_result:?}"
    );
    let small_memory = WasmRuntime::new(WasmLimits {
        max_memory_bytes: 64 * 1024,
        ..Default::default()
    })
    .unwrap();
    let memory_module = wat::parse_str(
        "(module (memory (export \"memory\") 2) (func (export \"run\") (result i32) i32.const 0))",
    )
    .unwrap();
    assert_eq!(
        small_memory.run(&memory_module),
        Err(WasmFailure::MemoryLimit)
    );
}

#[test]
fn completed_run_cannot_timeout_a_later_run() {
    let runtime = WasmRuntime::new(WasmLimits {
        timeout: Duration::from_millis(25),
        ..Default::default()
    })
    .unwrap();
    let pure = wat::parse_str("(module (func (export \"run\") (result i32) i32.const 7))").unwrap();
    assert_eq!(runtime.run(&pure).unwrap(), 7);
    std::thread::sleep(Duration::from_millis(35));
    assert_eq!(runtime.run(&pure).unwrap(), 7);
}

#[test]
fn runtime_limits_unexported_memory() {
    let runtime = WasmRuntime::new(WasmLimits {
        max_memory_bytes: 65_536,
        ..Default::default()
    })
    .unwrap();
    let module = wat::parse_str(
        "(module (memory 1) (func (export \"run\") (result i32) i32.const 2 memory.grow))",
    )
    .unwrap();
    assert_eq!(runtime.run(&module).unwrap(), -1);
}

#[test]
fn registry_starts_and_stops_packaged_wasm() {
    let root = std::env::temp_dir().join(format!("daena-wasm-{}", std::process::id()));
    let entrypoint = root.join("dist");
    std::fs::create_dir_all(&entrypoint).unwrap();
    let module =
        wat::parse_str("(module (func (export \"run\") (result i32) i32.const 1))").unwrap();
    std::fs::write(entrypoint.join("service.wasm"), module).unwrap();
    let mut registry = WasmRuntimeRegistry::default();
    registry
        .start(
            "project",
            "com.example.plugin",
            &root,
            Some("dist/service.wasm"),
            WasmLimits::default(),
        )
        .unwrap();
    assert!(registry.is_running("project", "com.example.plugin"));
    registry.stop("project", "com.example.plugin");
    assert!(!registry.is_running("project", "com.example.plugin"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn loaded_wasm_service_keeps_successful_instance_state_between_calls() {
    let module = wat::parse_str(
            "(module (global $count (mut i32) (i32.const 0)) (func (export \"run\") (result i32) global.get $count i32.const 1 i32.add global.set $count global.get $count))",
        )
        .unwrap();
    let mut runtime = WasmRuntime::new(WasmLimits::default()).unwrap();
    runtime.load(&module).unwrap();
    assert_eq!(runtime.invoke().unwrap(), 1);
    assert_eq!(runtime.invoke().unwrap(), 2);
}
