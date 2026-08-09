use daena_plugin_api::{parse_manifest, validate_manifest};
use serde_json::Value;
use std::path::PathBuf;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join("fixtures")
        .join("manifest")
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_root().join(name)).unwrap()
}

#[test]
fn fixture_battery_matches_expected_outcomes() {
    let index: Value = serde_json::from_str(&fixture("index.json")).unwrap();
    let cases = index["fixtures"].as_array().unwrap();
    assert!(
        cases.len() >= 18,
        "expected at least 18 fixtures, found {}",
        cases.len()
    );

    let mut checked = 0;
    for case in cases {
        let rule = case["rule"].as_str().unwrap();
        let expected = case["expected"].as_str().unwrap();
        let file = case["file"].as_str().unwrap();
        let json = fixture(file);

        let parsed = parse_manifest(&json);
        let outcome = match parsed {
            Ok(manifest) => match validate_manifest(&manifest) {
                Ok(()) => "accepted",
                Err(_) => "rejected",
            },
            Err(_) => "rejected",
        };

        assert_eq!(
            outcome, expected,
            "fixture {rule} ({file}): expected {expected}, got {outcome}"
        );
        checked += 1;
    }

    assert_eq!(
        checked,
        cases.len(),
        "battery must exercise every indexed fixture"
    );
}

#[test]
fn bundled_manifests_are_positive_controls() {
    for path in [
        "packages/modules/lore/manifest.json",
        "packages/modules/timeline/manifest.json",
        "packages/modules/writing/manifest.json",
        "examples/plugins/declarative/manifest.json",
        "examples/plugins/ui/manifest.json",
        "examples/plugins/wasm-service/manifest.json",
    ] {
        let json = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join(path),
        )
        .unwrap_or_else(|_| panic!("missing bundled manifest {path}"));
        let manifest =
            parse_manifest(&json).unwrap_or_else(|e| panic!("{path} failed to parse: {e}"));
        validate_manifest(&manifest).unwrap_or_else(|e| panic!("{path} failed to validate: {e}"));
    }
}
