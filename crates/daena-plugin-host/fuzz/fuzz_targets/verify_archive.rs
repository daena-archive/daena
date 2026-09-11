#![no_main]

use daena_plugin_host::{verify_archive_bytes, ArchiveLimits, VerificationPolicy};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let limits = ArchiveLimits {
        max_compressed_bytes: 64 * 1024,
        max_uncompressed_bytes: 256 * 1024,
        max_file_count: 64,
        max_path_length: 256,
        max_file_bytes: 64 * 1024,
    };
    let _ = verify_archive_bytes(data, limits, &VerificationPolicy::with_unsigned_consent());
});
