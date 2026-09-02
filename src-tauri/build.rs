fn normalize_override(raw: &str) -> Option<&str> {
    let trimmed = raw.trim().trim_start_matches(['v', 'V']);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn resolve_app_version() -> String {
    std::env::var("DAENA_VERSION")
        .ok()
        .as_deref()
        .and_then(normalize_override)
        .map(str::to_string)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

fn main() {
    println!("cargo:rerun-if-env-changed=DAENA_VERSION");
    println!(
        "cargo:rustc-env=DAENA_APP_VERSION={}",
        resolve_app_version()
    );
    tauri_build::build();
}
