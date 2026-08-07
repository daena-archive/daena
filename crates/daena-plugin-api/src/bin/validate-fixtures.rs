use daena_plugin_api::{parse_manifest, validate_manifest};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join("fixtures")
        .join("manifest")
}

fn outcome_for(json: &str) -> &'static str {
    match parse_manifest(json) {
        Ok(manifest) => {
            if validate_manifest(&manifest).is_ok() {
                "accepted"
            } else {
                "rejected"
            }
        }
        Err(_) => "rejected",
    }
}

fn main() {
    let dir = fixture_root();
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("index.json")).unwrap()).unwrap();
    let cases = index["fixtures"].as_array().unwrap();
    let rows: Vec<Value> = cases
        .iter()
        .map(|case| {
            let file = case["file"].as_str().unwrap();
            let json = std::fs::read_to_string(Path::new(&dir).join(file)).unwrap();
            json!({ "file": file, "outcome": outcome_for(&json) })
        })
        .collect();
    println!("{}", serde_json::to_string(&rows).unwrap());
}
