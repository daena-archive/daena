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
