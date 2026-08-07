use super::*;

#[test]
fn canonical_bundled_manifests_validate() {
    let lore = include_str!("../../../packages/modules/lore/manifest.json");
    let timeline = include_str!("../../../packages/modules/timeline/manifest.json");
    assert_eq!(parse_manifest(lore).unwrap().id, "daena.lore");
    assert_eq!(parse_manifest(timeline).unwrap().id, "daena.timeline");
}

#[test]
fn unknown_manifest_fields_are_rejected() {
    let json = include_str!("../../../packages/modules/lore/manifest.json").replace(
        "\"name\": \"Lore\"",
        "\"name\": \"Lore\", \"unexpected\": true",
    );
    assert!(parse_manifest(&json).is_err());
}

#[test]
fn template_references_must_match_declared_schema() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.templates[0].entity_type = "unknown".into();
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.templates[0].fields = serde_json::json!({"unknown": "value"});
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn field_entity_types_must_match_declared_types() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    assert!(validate_manifest(&manifest).is_ok());

    manifest.schemas[0].fields[2].entity_types = Some(vec!["unknown".into()]);
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.templates[0].fields = serde_json::json!({"occupation": "Archivist"});
    assert!(validate_manifest(&manifest).is_ok());
    manifest.templates[1].fields = serde_json::json!({"occupation": "Archivist"});
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn template_required_fields_must_match_template_schema() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.templates[0].required_fields = Some(vec!["occupation".into()]);
    assert!(validate_manifest(&manifest).is_ok());

    manifest.templates[0].required_fields = Some(vec!["region".into()]);
    assert!(validate_manifest(&manifest).is_err());

    manifest.templates[0].required_fields = Some(vec!["unknown".into()]);
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn relationship_fields_require_valid_target_metadata() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    assert!(validate_manifest(&manifest).is_ok());

    manifest.schemas[0].fields[3].relationship_type = None;
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].fields[3].target_entity_types = Some(vec!["unknown".into()]);
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn host_components_are_schema_and_capability_bound() {
    let json = include_str!("../../../examples/plugins/declarative/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    assert_eq!(manifest.views[0].components.len(), 6);

    let list_index = manifest.views[0]
        .components
        .iter()
        .position(|component| matches!(component, ViewComponent::EntityList { .. }))
        .unwrap();
    manifest.views[0].components[list_index] = ViewComponent::EntityList {
        id: "notes".into(),
        title: "Notes".into(),
        entity_type: "unknown".into(),
        limit: 10,
    };
    assert!(validate_manifest(&manifest).is_err());

    manifest.views[0].components[list_index] = ViewComponent::EntityList {
        id: "notes".into(),
        title: "Notes".into(),
        entity_type: "note".into(),
        limit: 10,
    };
    manifest
        .capabilities
        .retain(|capability| capability != "entity.read");
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    let form = manifest.views[0]
        .components
        .iter_mut()
        .find_map(|component| match component {
            ViewComponent::FieldForm { fields, .. } => Some(fields),
            _ => None,
        })
        .unwrap();
    form[0] = "undeclared".into();
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.commands[0].action = None;
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn lifecycle_is_fail_closed() {
    assert!(lifecycle_transition(
        LifecycleState::Resolved,
        LifecycleState::Activating
    ));
    assert!(!lifecycle_transition(
        LifecycleState::Active,
        LifecycleState::Installed
    ));
}

#[test]
fn semver_validates_pre_release_and_build_suffixes() {
    for valid in ["1.0.0", "0.1.0", "10.20.30", "1.2.3-alpha", "1.2.3-alpha.1", "1.2.3-alpha-1", "1.2.3+build.5", "1.2.3-alpha+20260101"] {
        assert!(is_semver(valid), "{valid} should be valid");
    }
    for invalid in ["1.0", "1.0.0.0", "01.0.0", "1.2.3-alpha beta", "1.2.3+", "1.2.3-", "1.2.3+foo!", "v1.0.0"] {
        assert!(!is_semver(invalid), "{invalid} should be invalid");
    }
}

#[test]
fn host_api_range_matches_ts_semantics() {
    for valid in [">=1.0.0 <2.0.0", "^1.0.0", "~0.1.0", ">=1.2.3 <=2.0.0", "=1.0.0", ">1.0.0", "1.0.0", ">=1.0.0-alpha"] {
        assert!(is_host_api_range(valid), "{valid} should be a valid range");
    }
    for invalid in ["", "   ", "banana", ">=banana", ">=1.0.0 <", ">=1.0.0 <2"] {
        assert!(!is_host_api_range(invalid), "{invalid} should be an invalid range");
    }
}

#[test]
fn package_path_rejects_dot_segments() {
    assert!(is_package_path("dist/ui/index.html"));
    assert!(is_package_path("wasm/service.wasm"));
    assert!(!is_package_path("dist/./index.html"));
    assert!(!is_package_path("../escape"));
    assert!(!is_package_path("/abs/path"));
    assert!(!is_package_path("dist\\index.html"));
    assert!(!is_package_path(""));
}
