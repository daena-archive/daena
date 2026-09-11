#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    if let Ok(json) = std::str::from_utf8(data) {
        let _ = daena_plugin_api::parse_manifest(json);
    }
});
