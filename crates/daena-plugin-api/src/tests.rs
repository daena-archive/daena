use super::*;

#[test]
fn canonical_bundled_manifests_validate() {
    let lore = include_str!("../../../packages/modules/lore/manifest.json");
    let timeline = include_str!("../../../packages/modules/timeline/manifest.json");
    let maps = include_str!("../../../packages/modules/maps/manifest.json");
    let houses = include_str!("../../../packages/modules/houses/manifest.json");
    assert_eq!(parse_manifest(lore).unwrap().id, "daena.lore");
    assert_eq!(parse_manifest(timeline).unwrap().id, "daena.timeline");
    assert_eq!(parse_manifest(houses).unwrap().id, "daena.houses");
    let maps = parse_manifest(maps).unwrap();
    assert_eq!(
        maps.views[0].renderer,
        ViewRenderer::HostSurface {
            id: "daena.maps/editor".into(),
            major: 1,
        }
    );
}

#[test]
fn host_surface_renderer_requires_a_valid_versioned_id() {
    let json = include_str!("../../../packages/modules/maps/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.views[0].renderer = ViewRenderer::HostSurface {
        id: "not-a-surface".into(),
        major: 1,
    };
    assert!(validate_manifest(&manifest).is_err());

    manifest.views[0].renderer = ViewRenderer::HostSurface {
        id: "daena.maps/editor".into(),
        major: 0,
    };
    assert!(validate_manifest(&manifest).is_err());
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
fn entity_type_icons_use_the_closed_catalog_or_safe_package_svg_paths() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].entity_types[0].icon = IconRef::Catalog {
        id: "not-in-the-catalog".into(),
    };
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].entity_types[0].icon = IconRef::PluginSvg {
        path: "../outside.svg".into(),
    };
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].entity_types[0].icon = IconRef::PluginSvg {
        path: "icons/person.svg".into(),
    };
    assert!(validate_manifest(&manifest).is_ok());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].entity_types[0].icon = IconRef::UserSvg {
        svg: r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 4h16v16H4z"/></svg>"#.into(),
    };
    assert!(validate_manifest(&manifest).is_err());
    assert!(
        validate_passive_svg(match &manifest.schemas[0].entity_types[0].icon {
            IconRef::UserSvg { svg } => svg.as_bytes(),
            _ => unreachable!(),
        })
        .is_ok()
    );
}

#[test]
fn timeline_contributions_require_shared_date_fields_and_grouped_boundaries() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    assert!(validate_manifest(&manifest).is_ok());

    let birth = manifest.schemas[0]
        .fields
        .iter_mut()
        .find(|field| field.key == "birth")
        .unwrap();
    birth.shared = false;
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    let birth = manifest.schemas[0]
        .fields
        .iter_mut()
        .find(|field| field.key == "birth")
        .unwrap();
    birth.timeline.as_mut().unwrap().group = Some(String::new());
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    let birth = manifest.schemas[0]
        .fields
        .iter_mut()
        .find(|field| field.key == "birth")
        .unwrap();
    birth.field_type = "text".into();
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
    manifest.schemas[0].fields[3].target_entity_types =
        Some(vec!["daena.lore:place".into(), "daena.lore:place".into()]);
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn relationship_metadata_fields_require_valid_enum_options() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].fields[3].metadata_fields = Some(vec![MetadataFieldDefinition {
        key: "status".into(),
        label: "Status".into(),
        field_type: "enum".into(),
        required: None,
        options: Some(Vec::new()),
        one_of: None,
    }]);
    assert!(validate_manifest(&manifest).is_err());

    manifest.schemas[0].fields[3]
        .metadata_fields
        .as_mut()
        .unwrap()[0]
        .options = Some(vec!["active".into(), "inactive".into()]);
    assert!(validate_manifest(&manifest).is_ok());

    manifest.schemas[0].fields[3]
        .metadata_fields
        .as_mut()
        .unwrap()[0]
        .field_type = "unsupported".into();
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn non_relationship_field_rejects_metadata_fields() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].fields[0].metadata_fields = Some(vec![MetadataFieldDefinition {
        key: "note".into(),
        label: "Note".into(),
        field_type: "text".into(),
        required: None,
        options: None,
        one_of: None,
    }]);
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn relationship_metadata_field_keys_are_unique() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].fields[3].metadata_fields = Some(vec![
        MetadataFieldDefinition {
            key: "validFrom".into(),
            label: "Valid from".into(),
            field_type: "date".into(),
            required: None,
            options: None,
            one_of: None,
        },
        MetadataFieldDefinition {
            key: "validFrom".into(),
            label: "Also valid from".into(),
            field_type: "date".into(),
            required: None,
            options: None,
            one_of: None,
        },
    ]);
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
    for valid in [
        "1.0.0",
        "0.1.0",
        "10.20.30",
        "1.2.3-alpha",
        "1.2.3-alpha.1",
        "1.2.3-alpha-1",
        "1.2.3+build.5",
        "1.2.3-alpha+20260101",
    ] {
        assert!(is_semver(valid), "{valid} should be valid");
    }
    for invalid in [
        "1.0",
        "1.0.0.0",
        "01.0.0",
        "1.2.3-alpha beta",
        "1.2.3+",
        "1.2.3-",
        "1.2.3+foo!",
        "v1.0.0",
    ] {
        assert!(!is_semver(invalid), "{invalid} should be invalid");
    }
}

#[test]
fn host_api_range_matches_ts_semantics() {
    for valid in [
        ">=1.0.0 <2.0.0",
        "^1.0.0",
        "~0.1.0",
        ">=1.2.3 <=2.0.0",
        "=1.0.0",
        ">1.0.0",
        "1.0.0",
        ">=1.0.0-alpha",
    ] {
        assert!(is_host_api_range(valid), "{valid} should be a valid range");
    }
    for invalid in ["", "   ", "banana", ">=banana", ">=1.0.0 <", ">=1.0.0 <2"] {
        assert!(
            !is_host_api_range(invalid),
            "{invalid} should be an invalid range"
        );
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

fn host_surface_only_manifest() -> PluginManifest {
    let json = include_str!("../../../packages/modules/maps/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.kind = PluginKind::Declarative;
    manifest.entrypoints = Entrypoints {
        ui: None,
        wasm: None,
    };
    manifest.services.provides.clear();
    manifest
}

#[test]
fn host_surface_only_declarative_plugins_may_omit_entrypoints() {
    let manifest = host_surface_only_manifest();
    assert!(validate_manifest(&manifest).is_ok());
}

#[test]
fn houses_manifest_is_a_valid_declarative_workspace_plugin() {
    let manifest = parse_manifest(include_str!(
        "../../../packages/modules/houses/manifest.json"
    ))
    .unwrap();
    validate_manifest(&manifest).expect("houses manifest must validate");
    assert!(manifest.entrypoints.ui.is_none());
    assert!(manifest.entrypoints.wasm.is_none());
    assert!(manifest.views.is_empty());
    let parents = manifest.schemas[0]
        .fields
        .iter()
        .find(|field| field.key == "parents")
        .expect("parents");
    assert_eq!(parents.relationship_direction.as_deref(), Some("incoming"));
    assert!(manifest.schemas[0]
        .fields
        .iter()
        .any(|field| field.key == "children"
            && field.relationship_type.as_deref() == Some("family_parent_of")));
    assert!(manifest
        .capabilities
        .iter()
        .any(|capability| capability == "schema.overlay"));
    assert!(manifest
        .capabilities
        .iter()
        .any(|capability| capability == "entity.delete"));
    assert!(manifest.schemas[0]
        .entity_types
        .iter()
        .any(|entity_type| entity_type.id == "daena.houses:house"));
    assert!(manifest.schemas[0].fields.iter().any(|field| {
        field.key == "houses" && field.relationship_type.as_deref() == Some("family_member_of")
    }));
}

#[test]
fn empty_entrypoints_reject_invalid_variants() {
    let mut manifest = host_surface_only_manifest();
    manifest.kind = PluginKind::Sandboxed;
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = host_surface_only_manifest();
    manifest.views[0].renderer = ViewRenderer::Declarative;
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = host_surface_only_manifest();
    manifest.views.clear();
    assert!(validate_manifest(&manifest).is_ok());

    let mut manifest = host_surface_only_manifest();
    manifest.services.provides.push(Service {
        name: "daena.example/service".into(),
        major: 1,
    });
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn field_entity_types_may_reference_declared_dependencies() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    assert!(validate_manifest(&manifest).is_ok());
    manifest.dependencies.remove("daena.language");
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].fields.push(FieldDefinition {
        key: "ghostNote".into(),
        label: "Ghost note".into(),
        field_type: "text".into(),
        required: None,
        options: None,
        entity_types: Some(vec!["daena.ghost:ghost".into()]),
        relationship_type: None,
        target_entity_types: None,
        shared: false,
        multiple: false,
        cardinality: None,
        one_of: None,
        metadata_fields: None,
        timeline: None,
        relationship_constraints: None,
        relationship_direction: None,
    });
    assert!(validate_manifest(&manifest).is_err());
}

#[test]
fn relationship_constraints_are_relationship_only_and_must_agree() {
    let json = include_str!("../../../packages/modules/lore/manifest.json");
    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].fields[0].relationship_constraints = Some(RelationshipConstraints {
        allow_self: false,
        acyclic: true,
        unique: RelationshipUniqueness::Directed,
    });
    assert!(validate_manifest(&manifest).is_err());

    let mut manifest = parse_manifest(json).unwrap();
    manifest.schemas[0].fields[3].relationship_constraints = Some(RelationshipConstraints {
        allow_self: false,
        acyclic: true,
        unique: RelationshipUniqueness::Directed,
    });
    assert!(validate_manifest(&manifest).is_ok());

    manifest.schemas[0].fields.push(FieldDefinition {
        key: "alsoSpeaks".into(),
        label: "Also speaks".into(),
        field_type: "relationship".into(),
        required: None,
        options: None,
        entity_types: None,
        relationship_type: Some("speaks".into()),
        target_entity_types: Some(vec!["daena.language:language".into()]),
        shared: false,
        multiple: false,
        cardinality: None,
        one_of: None,
        metadata_fields: None,
        timeline: None,
        relationship_constraints: Some(RelationshipConstraints {
            allow_self: true,
            acyclic: false,
            unique: RelationshipUniqueness::None,
        }),
        relationship_direction: None,
    });
    assert!(validate_manifest(&manifest).is_err());
}
