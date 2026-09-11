#![no_main]

use daena_plugin_api::{
    rpc_method, validate_rpc_payload, NamespaceView, RpcAuthorizationContext, RpcRequest,
};
use libfuzzer_sys::fuzz_target;

struct FuzzNamespaces;

impl NamespaceView for FuzzNamespaces {
    fn owner(&self, _namespace: &str) -> Option<&str> {
        Some("fuzz.plugin")
    }

    fn field_is_shared(&self, _namespace: &str, _key: &str) -> bool {
        true
    }

    fn namespace_has_shared_fields(&self, _namespace: &str) -> bool {
        true
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let Ok(json) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(request) = serde_json::from_str::<RpcRequest>(json) else {
        return;
    };
    let _ = validate_rpc_payload(&request.method, &request.payload);
    if let Some(method) = rpc_method(&request.method) {
        let context = RpcAuthorizationContext {
            plugin_id: "fuzz.plugin",
            namespaces: &FuzzNamespaces,
        };
        let _ = method.capability.resolve(&request.payload, &context);
    }
});
