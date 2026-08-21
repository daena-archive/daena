use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap();
    let archive = manifest_dir.join("plugin-assets/maps/fmg-v1.119.zip");
    let script = workspace_root.join("scripts/ensure-fmg-archive.mjs");

    println!("cargo:rerun-if-changed={}", script.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace_root
            .join("docs/maps/fmg-v1.119-vendor.json")
            .display()
    );
    println!("cargo:rerun-if-changed={}", archive.display());
    println!("cargo:rerun-if-env-changed=DAENA_FMG_SOURCE");

    let status = Command::new("node")
        .arg(&script)
        .arg(&archive)
        .status()
        .expect("failed to run Node.js for FMG archive verification");
    assert!(status.success(), "FMG archive is missing or stale; set DAENA_FMG_SOURCE to the pinned FMG checkout and retry");

    tauri_build::build();
}
