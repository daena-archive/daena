//! Runtime boundaries for untrusted plugin UI and background components.
//!
//! This module deliberately contains policy, not application UI code.  The
//! policy is shared by the Tauri adapter and the conformance tests so a plugin
//! cannot get a weaker development path than a packaged plugin.

use std::time::Duration;

use wasmtime::{Config, Engine, Instance, Module, Store};
use worldbuilder_plugin_api::{PluginKind, PluginManifest, RpcRequest};

pub const HOST_ORIGIN: &str = "https://worldbuilder.local";
pub const PLUGIN_ORIGIN_PREFIX: &str = "https://plugin.worldbuilder.local/";
pub const MAX_RPC_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasmLimits {
    pub max_memory_bytes: usize,
    pub fuel: u64,
    pub timeout: Duration,
    pub max_failures: u8,
}

impl Default for WasmLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 16 * 1024 * 1024,
            fuel: 5_000_000,
            timeout: Duration::from_millis(2_000),
            max_failures: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginWebviewPolicy {
    pub label: String,
    pub origin: String,
    pub csp: String,
    pub entrypoint: String,
}

pub fn webview_policy(manifest: &PluginManifest) -> Option<PluginWebviewPolicy> {
    if manifest.kind != PluginKind::Sandboxed {
        return None;
    }
    let entrypoint = manifest.entrypoints.ui.clone()?;
    Some(PluginWebviewPolicy {
        label: format!("plugin:{}", manifest.id),
        origin: format!("{PLUGIN_ORIGIN_PREFIX}{}", manifest.id),
        // No inline/eval code, no network, no frames, no form submission, and
        // no parent/opener access. Static assets are served by the host only.
        csp: "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'".into(),
        entrypoint,
    })
}

pub fn validate_bridge_request(request: &RpcRequest) -> Result<(), &'static str> {
    if request.session_id.trim().is_empty() || request.request_id.trim().is_empty() {
        return Err("bridge request requires session and request IDs");
    }
    if request.method.is_empty() || request.method.len() > 128 {
        return Err("bridge method is invalid");
    }
    let bytes = serde_json::to_vec(request).map_err(|_| "bridge request is not serializable")?;
    if bytes.len() > MAX_RPC_BYTES {
        return Err("bridge request exceeds payload limit");
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmFailure {
    InvalidModule(String),
    AmbientImport(String),
    MemoryLimit,
    FuelExhausted,
    TimedOut,
    MissingEntryPoint,
    Trap(String),
}

/// Executes a plugin component without linking WASI or any host imports.
///
/// This is intentionally a deny-by-default WASI boundary: a module that asks
/// for WASI or another import cannot start. Capabilities will be added as
/// narrowly-scoped host functions in later phases, never by inheriting the
/// process environment or opening a preopened directory.
pub struct WasmRuntime {
    engine: Engine,
    limits: WasmLimits,
}

impl WasmRuntime {
    pub fn new(limits: WasmLimits) -> Result<Self, WasmFailure> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|e| WasmFailure::InvalidModule(e.to_string()))?;
        Ok(Self { engine, limits })
    }

    pub fn run(&self, bytes: &[u8]) -> Result<i32, WasmFailure> {
        let module = Module::new(&self.engine, bytes)
            .map_err(|e| WasmFailure::InvalidModule(e.to_string()))?;
        if let Some(import) = module.imports().next() {
            return Err(WasmFailure::AmbientImport(import.name().to_string()));
        }
        for export in module.exports() {
            if let wasmtime::ExternType::Memory(memory) = export.ty() {
                let maximum = memory.maximum().unwrap_or(u64::MAX);
                let max_pages = (self.limits.max_memory_bytes / 65_536) as u64;
                if memory.minimum() as u64 > max_pages || maximum > max_pages {
                    return Err(WasmFailure::MemoryLimit);
                }
            }
        }
        let mut store = Store::new(&self.engine, ());
        store
            .set_fuel(self.limits.fuel)
            .map_err(|e| WasmFailure::Trap(e.to_string()))?;
        store.set_epoch_deadline(1);
        let engine = self.engine.clone();
        let timeout = self.limits.timeout;
        let _timer = std::thread::spawn(move || {
            std::thread::sleep(timeout);
            engine.increment_epoch();
        });
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| WasmFailure::Trap(e.to_string()))?;
        let entry = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .map_err(|_| WasmFailure::MissingEntryPoint)?;
        let result = entry.call(&mut store, ()).map_err(|e| {
            let message = e.to_string();
            if message.contains("all fuel consumed") {
                WasmFailure::FuelExhausted
            } else if message.contains("epoch deadline") {
                WasmFailure::TimedOut
            } else {
                WasmFailure::Trap(message)
            }
        });
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldbuilder_plugin_api::{Entrypoints, PluginKind};

    fn manifest(kind: PluginKind) -> PluginManifest {
        PluginManifest {
            manifest_version: 1,
            id: "com.example.plugin".into(),
            name: "Plugin".into(),
            version: "1.0.0".into(),
            publisher: "com.example".into(),
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
            views: vec![],
            commands: vec![],
            services: worldbuilder_plugin_api::Services {
                provides: vec![],
                consumes: vec![],
            },
            events: worldbuilder_plugin_api::Events {
                publishes: vec![],
                subscribes: vec![],
            },
            migrations: vec![],
        }
    }

    #[test]
    fn policy_uses_an_app_controlled_origin_and_denies_ambient_network() {
        let policy = webview_policy(&manifest(PluginKind::Sandboxed)).unwrap();
        assert_eq!(
            policy.origin,
            "https://plugin.worldbuilder.local/com.example.plugin"
        );
        assert!(policy.csp.contains("connect-src 'none'"));
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
        let pure =
            wat::parse_str("(module (func (export \"run\") (result i32) i32.const 7))").unwrap();
        assert_eq!(runtime.run(&pure).unwrap(), 7);
        let imported = wat::parse_str(
            "(module (import \"wasi_snapshot_preview1\" \"clock_time_get\" (func)))",
        )
        .unwrap();
        assert!(matches!(
            runtime.run(&imported),
            Err(WasmFailure::AmbientImport(_))
        ));
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
        let memory_module = wat::parse_str("(module (memory (export \"memory\") 2) (func (export \"run\") (result i32) i32.const 0))").unwrap();
        assert_eq!(
            small_memory.run(&memory_module),
            Err(WasmFailure::MemoryLimit)
        );
    }
}
