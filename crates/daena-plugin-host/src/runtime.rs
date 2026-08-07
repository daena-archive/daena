//! Runtime boundaries for untrusted plugin UI and background components.
//!
//! This module deliberately contains policy, not application UI code.  The
//! policy is shared by the Tauri adapter and the conformance tests so a plugin
//! cannot get a weaker development path than a packaged plugin.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use wasmtime::{
    Config, Engine, Instance, Module, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
};
use daena_plugin_api::{PluginKind, PluginManifest, RpcRequest};

pub const HOST_ORIGIN: &str = "https://daena.local";
pub const MAX_RPC_BYTES: usize = 256 * 1024;
pub const PLUGIN_PROTOCOL: &str = "plugin";
/// Synchronous service ABI for packaged WASM providers. The module exports
/// `alloc(i32) -> i32`, `handle_json(i32, i32) -> i64`, and `memory`; the input
/// and output are UTF-8 JSON. The returned i64 packs `(len << 32) | ptr`.
/// Background event loops are intentionally unsupported in this phase.
pub const WASM_SERVICE_ABI: &str = "wb.service.sync.v1";
pub const WASM_SERVICE_MAX_BYTES: usize = MAX_RPC_BYTES;
pub const BUNDLED_TIMELINE_SERVICE_WASM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/timeline-service.wasm"));

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
    pub protocol: String,
    pub origin: String,
    pub url: String,
    pub csp: String,
    pub entrypoint: String,
}

pub fn plugin_protocol(plugin_id: &str) -> String {
    let _ = plugin_id;
    PLUGIN_PROTOCOL.into()
}

pub fn plugin_window_label(plugin_id: &str) -> String {
    let sanitized = plugin_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '/' | '_' | ':') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("plugin:{sanitized}")
}

pub fn webview_policy(manifest: &PluginManifest) -> Option<PluginWebviewPolicy> {
    if manifest.kind != PluginKind::Sandboxed {
        return None;
    }
    let entrypoint = manifest.entrypoints.ui.clone()?;
    let protocol = plugin_protocol(&manifest.id);
    let origin = format!("{protocol}://{}", manifest.id);
    Some(PluginWebviewPolicy {
        label: plugin_window_label(&manifest.id),
        protocol,
        url: format!("{origin}/{entrypoint}"),
        origin,
        // FMG uses inline style attributes and data-backed icon fonts. Permit
        // those local resources while keeping scripts, network, frames,
        // objects, forms, and parent access denied. All source I/O remains
        // brokered.
        csp: if manifest.id == "daena.maps" {
            "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; style-src-elem 'self' 'unsafe-inline'; style-src-attr 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; manifest-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'; frame-ancestors 'none'"
        } else {
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; manifest-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
        }.into(),
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

impl std::fmt::Display for WasmFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

/// Executes a plugin component without linking WASI or any host imports.
///
/// This is intentionally a deny-by-default WASI boundary: a module that asks
/// for WASI or another import cannot start. Capabilities will be added as
/// narrowly-scoped host functions in later phases, never by inheriting the
/// process environment or opening a preopened directory.
#[derive(Clone)]
pub struct WasmRuntime {
    engine: Engine,
    limits: WasmLimits,
    module: Option<Module>,
    instance: Arc<Mutex<Option<PersistentInstance>>>,
}

struct PersistentInstance {
    store: Store<StoreLimits>,
    entry: Option<TypedFunc<(), i32>>,
    alloc: Option<TypedFunc<i32, i32>>,
    handle_json: Option<TypedFunc<(i32, i32), i64>>,
    memory: Option<wasmtime::Memory>,
}

#[derive(Clone, Default)]
pub struct WasmRuntimeRegistry {
    runtimes: BTreeMap<(String, String), WasmRuntime>,
    failures: BTreeMap<(String, String), u8>,
}

impl WasmRuntimeRegistry {
    pub fn start(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        package_root: &Path,
        entrypoint: Option<&str>,
        limits: WasmLimits,
    ) -> Result<(), WasmFailure> {
        self.start_with_bytes(
            project_id,
            plugin_id,
            package_root,
            entrypoint,
            None,
            limits,
        )
    }

    pub fn start_with_bytes(
        &mut self,
        project_id: &str,
        plugin_id: &str,
        package_root: &Path,
        entrypoint: Option<&str>,
        embedded_bytes: Option<&[u8]>,
        limits: WasmLimits,
    ) -> Result<(), WasmFailure> {
        let Some(entrypoint) = entrypoint else {
            return Ok(());
        };
        let bytes = if let Some(embedded_bytes) = embedded_bytes {
            embedded_bytes.to_vec()
        } else if package_root.as_os_str().is_empty() {
            return Ok(());
        } else {
            std::fs::read(package_root.join(entrypoint))
                .map_err(|error| WasmFailure::InvalidModule(error.to_string()))?
        };
        let mut runtime = WasmRuntime::new(limits)?;
        runtime.load(&bytes)?;
        if runtime.has_run_entrypoint()? {
            if let Err(error) = runtime.invoke() {
                let key = (project_id.into(), plugin_id.into());
                let failures = self.failures.entry(key.clone()).or_default();
                *failures = failures.saturating_add(1);
                return Err(error);
            }
        }
        self.runtimes
            .insert((project_id.into(), plugin_id.into()), runtime);
        Ok(())
    }

    pub fn stop(&mut self, project_id: &str, plugin_id: &str) {
        self.runtimes
            .remove(&(project_id.to_owned(), plugin_id.to_owned()));
    }

    pub fn is_running(&self, project_id: &str, plugin_id: &str) -> bool {
        self.runtimes
            .contains_key(&(project_id.to_owned(), plugin_id.to_owned()))
    }

    pub fn runtime(&self, project_id: &str, plugin_id: &str) -> Option<WasmRuntime> {
        self.runtimes
            .get(&(project_id.to_owned(), plugin_id.to_owned()))
            .cloned()
    }
}

impl WasmRuntime {
    pub fn new(limits: WasmLimits) -> Result<Self, WasmFailure> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|e| WasmFailure::InvalidModule(e.to_string()))?;
        Ok(Self {
            engine,
            limits,
            module: None,
            instance: Arc::new(Mutex::new(None)),
        })
    }

    pub fn run(&self, bytes: &[u8]) -> Result<i32, WasmFailure> {
        let module = self.compile(bytes)?;
        let mut instance = self.instantiate(&module)?;
        self.run_instance(&mut instance)
    }

    pub fn load(&mut self, bytes: &[u8]) -> Result<(), WasmFailure> {
        let module = self.compile(bytes)?;
        let instance = self.instantiate(&module)?;
        self.module = Some(module);
        *self
            .instance
            .lock()
            .map_err(|_| WasmFailure::Trap("WASM runtime state lock poisoned".into()))? =
            Some(instance);
        Ok(())
    }

    pub fn invoke(&self) -> Result<i32, WasmFailure> {
        if self.module.is_none() {
            return Err(WasmFailure::MissingEntryPoint);
        }
        let mut guard = self
            .instance
            .lock()
            .map_err(|_| WasmFailure::Trap("WASM runtime state lock poisoned".into()))?;
        let instance = guard.as_mut().ok_or(WasmFailure::MissingEntryPoint)?;
        let result = self.run_instance(instance);
        if result.is_err() {
            // A trapped or interrupted store is not reused. Successful calls
            // retain the instance and its linear memory for later requests.
            *guard = None;
        }
        result
    }

    pub fn has_run_entrypoint(&self) -> Result<bool, WasmFailure> {
        Ok(self
            .instance
            .lock()
            .map_err(|_| WasmFailure::Trap("WASM runtime state lock poisoned".into()))?
            .as_ref()
            .is_some_and(|instance| instance.entry.is_some()))
    }

    pub fn invoke_service(
        &self,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, WasmFailure> {
        let input = serde_json::to_vec(payload)
            .map_err(|error| WasmFailure::Trap(format!("service request is not JSON: {error}")))?;
        if input.len() > WASM_SERVICE_MAX_BYTES {
            return Err(WasmFailure::Trap(
                "service request exceeds payload limit".into(),
            ));
        }
        let supports_json = self
            .instance
            .lock()
            .map_err(|_| WasmFailure::Trap("WASM runtime state lock poisoned".into()))?
            .as_ref()
            .is_some_and(|instance| {
                instance.alloc.is_some()
                    && instance.handle_json.is_some()
                    && instance.memory.is_some()
            });
        if !supports_json {
            return self
                .invoke()
                .map(|value| serde_json::json!({ "value": value }));
        }
        let mut guard = self
            .instance
            .lock()
            .map_err(|_| WasmFailure::Trap("WASM runtime state lock poisoned".into()))?;
        let instance = guard.as_mut().ok_or(WasmFailure::MissingEntryPoint)?;
        let alloc = instance
            .alloc
            .as_ref()
            .ok_or(WasmFailure::MissingEntryPoint)?
            .clone();
        let handle = instance
            .handle_json
            .as_ref()
            .ok_or(WasmFailure::MissingEntryPoint)?
            .clone();
        let memory = instance.memory.ok_or(WasmFailure::MissingEntryPoint)?;
        instance
            .store
            .set_fuel(self.limits.fuel)
            .map_err(|e| WasmFailure::Trap(e.to_string()))?;
        instance.store.set_epoch_deadline(1);
        let ptr = alloc
            .call(&mut instance.store, input.len() as i32)
            .map_err(|e| WasmFailure::Trap(e.to_string()))?;
        if ptr < 0 {
            return Err(WasmFailure::Trap(
                "WASM allocator returned a negative pointer".into(),
            ));
        }
        memory
            .write(&mut instance.store, ptr as usize, &input)
            .map_err(|e| WasmFailure::Trap(e.to_string()))?;
        let engine = self.engine.clone();
        let timeout = self.limits.timeout;
        let (cancel_timer, timer_cancelled) = std::sync::mpsc::channel();
        let timer = std::thread::spawn(move || {
            if timer_cancelled.recv_timeout(timeout).is_err() {
                engine.increment_epoch();
            }
        });
        let result = handle
            .call(&mut instance.store, (ptr, input.len() as i32))
            .map_err(|e| {
                let message = e.to_string();
                if message.contains("all fuel consumed") {
                    WasmFailure::FuelExhausted
                } else if message.contains("epoch deadline") {
                    WasmFailure::TimedOut
                } else {
                    WasmFailure::Trap(message)
                }
            });
        let _ = cancel_timer.send(());
        let _ = timer.join();
        let packed = result? as u64;
        let output_ptr = (packed & u32::MAX as u64) as usize;
        let output_len = (packed >> 32) as usize;
        if output_len > WASM_SERVICE_MAX_BYTES {
            return Err(WasmFailure::Trap(
                "service response exceeds payload limit".into(),
            ));
        }
        let mut output = vec![0; output_len];
        memory
            .read(&mut instance.store, output_ptr, &mut output)
            .map_err(|e| WasmFailure::Trap(e.to_string()))?;
        serde_json::from_slice(&output)
            .map_err(|error| WasmFailure::Trap(format!("service response is not JSON: {error}")))
    }

    fn compile(&self, bytes: &[u8]) -> Result<Module, WasmFailure> {
        let module = Module::new(&self.engine, bytes)
            .map_err(|e| WasmFailure::InvalidModule(e.to_string()))?;
        if let Some(import) = module.imports().next() {
            return Err(WasmFailure::AmbientImport(import.name().to_string()));
        }
        for export in module.exports() {
            if let wasmtime::ExternType::Memory(memory) = export.ty() {
                let maximum = memory.maximum().unwrap_or(u64::MAX);
                let max_pages = (self.limits.max_memory_bytes / 65_536) as u64;
                if memory.minimum() > max_pages || maximum > max_pages {
                    return Err(WasmFailure::MemoryLimit);
                }
            }
        }
        Ok(module)
    }

    fn instantiate(&self, module: &Module) -> Result<PersistentInstance, WasmFailure> {
        let store_limits = StoreLimitsBuilder::new()
            .memory_size(self.limits.max_memory_bytes)
            .build();
        let mut store = Store::new(&self.engine, store_limits);
        store.limiter(|limits| limits);
        let instance =
            Instance::new(&mut store, module, &[]).map_err(|e| WasmFailure::Trap(e.to_string()))?;
        let entry = instance.get_typed_func::<(), i32>(&mut store, "run").ok();
        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .ok();
        let handle_json = instance
            .get_typed_func::<(i32, i32), i64>(&mut store, "handle_json")
            .ok();
        if entry.is_none() && (alloc.is_none() || handle_json.is_none()) {
            return Err(WasmFailure::MissingEntryPoint);
        }
        let memory = instance.get_memory(&mut store, "memory");
        Ok(PersistentInstance {
            store,
            entry,
            alloc,
            handle_json,
            memory,
        })
    }

    fn run_instance(&self, instance: &mut PersistentInstance) -> Result<i32, WasmFailure> {
        let entry = instance
            .entry
            .as_ref()
            .ok_or(WasmFailure::MissingEntryPoint)?;
        instance
            .store
            .set_fuel(self.limits.fuel)
            .map_err(|e| WasmFailure::Trap(e.to_string()))?;
        instance.store.set_epoch_deadline(1);
        let engine = self.engine.clone();
        let timeout = self.limits.timeout;
        let (cancel_timer, timer_cancelled) = std::sync::mpsc::channel();
        let timer = std::thread::spawn(move || {
            if timer_cancelled.recv_timeout(timeout).is_err() {
                engine.increment_epoch();
            }
        });
        let result = entry.call(&mut instance.store, ()).map_err(|e| {
            let message = e.to_string();
            if message.contains("all fuel consumed") {
                WasmFailure::FuelExhausted
            } else if message.contains("epoch deadline") {
                WasmFailure::TimedOut
            } else {
                WasmFailure::Trap(message)
            }
        });
        let _ = cancel_timer.send(());
        let _ = timer.join();
        result
    }
}

#[cfg(test)]
mod tests;
