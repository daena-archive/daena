use super::*;
use rusqlite::params;

fn maps_tables() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE entities (id TEXT PRIMARY KEY, entity_type TEXT, deleted INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE assets (
                 id TEXT PRIMARY KEY,
                 entity_id TEXT NOT NULL,
                 namespace TEXT NOT NULL,
                 mime_type TEXT NOT NULL DEFAULT 'application/octet-stream'
             );
             CREATE TABLE entity_fields (
                 entity_id TEXT NOT NULL,
                 namespace TEXT NOT NULL,
                 key TEXT NOT NULL,
                 value TEXT NOT NULL,
                 PRIMARY KEY(entity_id, namespace, key)
             );",
        )
        .unwrap();
    connection
}

fn insert_entity(connection: &Connection, id: &str, entity_type: &str) {
    connection
        .execute(
            "INSERT INTO entities (id, entity_type) VALUES (?1, ?2)",
            params![id, entity_type],
        )
        .unwrap();
}

fn insert_asset(connection: &Connection, id: &str, entity_id: &str, mime_type: &str) {
    connection
        .execute(
            "INSERT INTO assets (id, entity_id, namespace, mime_type) VALUES (?1, ?2, ?3, ?4)",
            params![id, entity_id, MAP_NAMESPACE, mime_type],
        )
        .unwrap();
}

fn fmg_descriptor(source_asset_id: Option<String>) -> Value {
    serde_json::json!({
        "schemaVersion": 1,
        "provider": {"id": FMG_PROVIDER, "adapterVersion": 1, "sourceFormat": "fmg-map"},
        "sourceAssetId": source_asset_id,
        "previewAssetId": null,
        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
    })
}

fn vector_descriptor(source_asset_id: &str, preview_asset_id: Option<&str>) -> Value {
    serde_json::json!({
        "schemaVersion": 1,
        "provider": {"id": VECTOR_PROVIDER, "adapterVersion": 1, "sourceFormat": VECTOR_SOURCE_FORMAT},
        "sourceAssetId": source_asset_id,
        "previewAssetId": preview_asset_id,
        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
    })
}

#[test]
fn provider_registry_dispatches_all_descriptor_variants() {
    let cases = [
        (
            ProviderDescriptor {
                id: FMG_PROVIDER.into(),
                adapter_version: 1,
                source_format: "fmg-map".into(),
            },
            ProviderKind::Fmg,
        ),
        (
            ProviderDescriptor {
                id: VECTOR_PROVIDER.into(),
                adapter_version: 1,
                source_format: VECTOR_SOURCE_FORMAT.into(),
            },
            ProviderKind::Vector,
        ),
        (
            ProviderDescriptor {
                id: PHYSICAL_PROVIDER.into(),
                adapter_version: PHYSICAL_ADAPTER_VERSION,
                source_format: PHYSICAL_SOURCE_FORMAT.into(),
            },
            ProviderKind::Physical,
        ),
    ];

    for (descriptor, expected_kind) in cases {
        assert_eq!(provider_spec(&descriptor).unwrap().kind, expected_kind);
    }
    assert!(provider_spec(&ProviderDescriptor {
        id: "unknown-provider".into(),
        adapter_version: 1,
        source_format: "unknown".into(),
    })
    .is_err());
}

#[test]
fn rejects_out_of_range_and_open_geometry() {
    assert!(point(&Point(1.1, 0.2)).is_err());
    assert!(anchor(
        &serde_json::json!({"kind":"area","rings":[[[0.1,0.1],[0.2,0.1],[0.2,0.2],[0.1,0.2]]]})
    )
    .is_err());
    assert!(anchor(&serde_json::json!({"kind":"path","points":[[0.1,0.1],[0.2,0.2]]})).is_ok());
    let too_long = (0..=crate::maps::IMAGE_MAX_PATH_POINTS)
        .map(|index| serde_json::json!([0.0, index as f64 / 1000.0]))
        .collect::<Vec<_>>();
    assert!(
        anchor(&serde_json::json!({"kind":"path","points": too_long}))
            .unwrap_err()
            .to_string()
            .contains("budget of")
    );
    assert!(point(&Point(1.1, 0.2))
        .unwrap_err()
        .to_string()
        .contains("maps: invalid geometry:"));
}

#[test]
fn validates_layers_only_on_map_entities() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    let place_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    insert_entity(&connection, &place_id, "place");
    let layers = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": Uuid::new_v4(),
            "name": "Political",
            "order": 0,
            "defaultVisible": true,
            "style": {},
            "selector": {}
        }]
    });
    assert!(validate_field(&connection, &map_id, "layers", &layers).is_ok());
    assert!(validate_field(&connection, &place_id, "layers", &layers)
        .unwrap_err()
        .to_string()
        .contains("layers belong only on a map entity"));
}

#[test]
fn validates_descriptor_with_null_source_until_first_save() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    let other_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    insert_entity(&connection, &other_id, "place");
    assert!(validate_field(&connection, &map_id, "map", &fmg_descriptor(None)).is_ok());

    let foreign_asset = Uuid::new_v4().to_string();
    insert_asset(
        &connection,
        &foreign_asset,
        &other_id,
        "application/x-fmg-map",
    );
    assert!(
        validate_field(
            &connection,
            &map_id,
            "map",
            &fmg_descriptor(Some(foreign_asset.clone()))
        )
        .is_err(),
        "an asset owned by another entity must be rejected"
    );

    let owned_asset = Uuid::new_v4().to_string();
    insert_asset(&connection, &owned_asset, &map_id, "application/x-fmg-map");
    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &fmg_descriptor(Some(owned_asset))
    )
    .is_ok());
}

#[test]
fn validates_dates_without_inventing_precision() {
    let year = serde_json::json!({"calendar":"gregorian","era":"CE","year":42,"precision":"year"});
    assert!(date(&year, "date").is_ok());
    let incomplete_month =
        serde_json::json!({"calendar":"gregorian","era":"CE","year":42,"precision":"month"});
    assert!(date(&incomplete_month, "date").is_err());
}

#[test]
fn rejects_inverted_validity_date_bounds() {
    let from_late =
        serde_json::json!({"calendar":"gregorian","era":"CE","year":2025,"precision":"year"});
    let to_early =
        serde_json::json!({"calendar":"gregorian","era":"CE","year":2020,"precision":"year"});
    let inverted = serde_json::json!({"from": from_late, "to": to_early});
    assert!(validity(&inverted, "validity").is_err());

    let valid = serde_json::json!({"from": to_early, "to": from_late});
    assert!(validity(&valid, "validity").is_ok());
}

#[test]
fn image_descriptors_and_raster_layers_round_trip() {
    let png = serde_json::json!({
        "schemaVersion": 1,
        "provider": {"id": VECTOR_PROVIDER, "adapterVersion": 1, "sourceFormat": VECTOR_SOURCE_FORMAT},
        "sourceAssetId": "018f89ec-25fc-7816-8b47-6f80905f2868",
        "previewAssetId": "018f89ec-25fc-7816-8b47-6f80905f2869",
        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
    });
    let descriptor: MapDescriptor = serde_json::from_value(png.clone()).unwrap();
    assert_eq!(
        serde_json::from_value::<MapDescriptor>(serde_json::to_value(&descriptor).unwrap())
            .unwrap(),
        descriptor
    );

    let layers = serde_json::json!({
        "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
        "name": "Countries",
        "order": 0,
        "defaultVisible": true,
        "style": {},
        "selector": {},
        "kind": "raster",
        "rasterAssetId": "018f8a03-bc44-70e2-a910-f4d2ef8d93df",
        "opacity": 1.0,
        "locked": false
    });
    let layer: LayerDefinition = serde_json::from_value(layers.clone()).unwrap();
    assert!(matches!(layer, LayerDefinition::Raster(_)));
    assert_eq!(
        serde_json::from_value::<LayerDefinition>(serde_json::to_value(&layer).unwrap()).unwrap(),
        layer
    );

    let semantic = serde_json::json!({
        "id": "018f8a01-9c20-ae05-b442-46dd3de2446c",
        "name": "Settlements",
        "order": 0,
        "defaultVisible": true,
        "style": {"color": "#334155"},
        "selector": {"roles": ["birthplace"]}
    });
    let layer: LayerDefinition = serde_json::from_value(semantic.clone()).unwrap();
    assert!(matches!(layer, LayerDefinition::Semantic(_)));
    assert_eq!(
        serde_json::from_value::<LayerDefinition>(serde_json::to_value(&layer).unwrap()).unwrap(),
        layer
    );

    let tagged = serde_json::json!({
        "id": "018f8a01-9c20-ae05-b442-46dd3de2446c",
        "name": "Routes",
        "order": 1,
        "defaultVisible": true,
        "style": {"stroke": "#d5ab6c", "strokeWidth": 2},
        "selector": {"anchorKind": "path", "roles": ["route"]},
        "kind": "semantic"
    });
    let layer: LayerDefinition = serde_json::from_value(tagged).unwrap();
    assert!(matches!(layer, LayerDefinition::Semantic(_)));
}

#[test]
fn vector_descriptors_layers_and_feature_anchors_round_trip() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    let asset_id = Uuid::new_v4().to_string();
    insert_asset(&connection, &asset_id, &map_id, VECTOR_MIME);

    let descriptor = serde_json::json!({
        "schemaVersion": 1,
        "provider": {"id": VECTOR_PROVIDER, "adapterVersion": 1, "sourceFormat": VECTOR_SOURCE_FORMAT},
        "sourceAssetId": asset_id,
        "previewAssetId": null,
        "defaultView": {"center": [0.5, 0.5], "zoom": 1},
        "generation": {
            "id": "daena-landmass",
            "version": 1,
            "seed": 831429,
            "settings": {
                "landPercent": 40,
                "continentCount": 3,
                "coastlineRoughness": "medium",
                "islandFrequency": "medium"
            }
        }
    });
    validate_field(&connection, &map_id, "map", &descriptor).unwrap();
    let parsed: MapDescriptor = serde_json::from_value(descriptor.clone()).unwrap();
    assert_eq!(
        serde_json::from_value::<MapDescriptor>(serde_json::to_value(&parsed).unwrap()).unwrap(),
        parsed
    );

    let polar = serde_json::json!({
        "schemaVersion": 1,
        "provider": {"id": VECTOR_PROVIDER, "adapterVersion": 1, "sourceFormat": VECTOR_SOURCE_FORMAT},
        "sourceAssetId": asset_id,
        "previewAssetId": null,
        "defaultView": {"center": [0.5, 0.01], "zoom": 1}
    });
    assert!(validate_field(&connection, &map_id, "map", &polar)
        .unwrap_err()
        .to_string()
        .contains("center"));

    let layer = serde_json::json!({
        "id": "018f89ec-25fc-7816-8b47-6f80905f2868",
        "name": "Countries",
        "order": 10,
        "defaultVisible": true,
        "locked": false,
        "selector": {},
        "style": {
            "fill": "#8f6fd1",
            "fillOpacity": 0.35,
            "stroke": "#5e4893",
            "strokeWidth": 1.5,
            "pointRadius": 5
        },
        "kind": "vector"
    });
    let parsed_layer: LayerDefinition = serde_json::from_value(layer.clone()).unwrap();
    assert!(matches!(parsed_layer, LayerDefinition::Vector(_)));
    validate_field(
        &connection,
        &map_id,
        "layers",
        &serde_json::json!({"schemaVersion": 1, "layers": [layer]}),
    )
    .unwrap();

    let feature_id = "018f89ec-25fc-7816-8b47-6f80905f2801";
    assert!(anchor(&serde_json::json!({
        "kind": "provider-feature",
        "provider": VECTOR_PROVIDER,
        "featureKind": "geojson-feature",
        "featureId": feature_id,
        "fallbackPoint": [0.5, 0.5]
    }))
    .is_ok());
    assert!(anchor(&serde_json::json!({
        "kind": "provider-feature",
        "provider": VECTOR_PROVIDER,
        "featureKind": "burg",
        "featureId": feature_id,
        "fallbackPoint": [0.5, 0.5]
    }))
    .is_err());
}

#[test]
fn rejects_invalid_semantic_style_and_selector() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    let invalid_style = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": Uuid::new_v4(),
            "name": "Routes",
            "order": 0,
            "defaultVisible": true,
            "style": {"unknown": true},
            "selector": {}
        }]
    });
    assert!(
        validate_field(&connection, &map_id, "layers", &invalid_style)
            .unwrap_err()
            .to_string()
            .contains("unsupported property")
    );
    let invalid_kind = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": Uuid::new_v4(),
            "name": "Routes",
            "order": 0,
            "defaultVisible": true,
            "style": {},
            "selector": {"anchorKind": "river"}
        }]
    });
    assert!(
        validate_field(&connection, &map_id, "layers", &invalid_kind)
            .unwrap_err()
            .to_string()
            .contains("anchorKind")
    );
}

#[test]
fn rejects_unknown_provider_tuples() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    let asset_id = Uuid::new_v4().to_string();
    insert_asset(&connection, &asset_id, &map_id, "image/png");

    let mixed = serde_json::json!({
        "schemaVersion": 1,
        "provider": {"id": FMG_PROVIDER, "adapterVersion": 1, "sourceFormat": "png"},
        "sourceAssetId": asset_id,
        "previewAssetId": null,
        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
    });
    assert!(validate_field(&connection, &map_id, "map", &mixed)
        .unwrap_err()
        .to_string()
        .contains("unsupported map provider"));

    let unknown = serde_json::json!({
        "schemaVersion": 1,
        "provider": {"id": "unknown-provider", "adapterVersion": 1, "sourceFormat": "png"},
        "sourceAssetId": asset_id,
        "previewAssetId": null,
        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
    });
    assert!(validate_field(&connection, &map_id, "map", &unknown).is_err());
}

#[test]
fn validates_image_source_ownership_and_mime() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    let other_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    insert_entity(&connection, &other_id, MAP_ENTITY_TYPE);
    let geojson = Uuid::new_v4().to_string();
    insert_asset(&connection, &geojson, &map_id, VECTOR_MIME);

    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &vector_descriptor(&geojson, Some(&Uuid::new_v4().to_string()))
    )
    .unwrap_err()
    .to_string()
    .contains("previewAssetId"));

    let foreign = Uuid::new_v4().to_string();
    insert_asset(&connection, &foreign, &other_id, "image/png");
    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &vector_descriptor(&geojson, Some(&foreign))
    )
    .is_err());

    let wrong_mime = Uuid::new_v4().to_string();
    insert_asset(
        &connection,
        &wrong_mime,
        &map_id,
        "application/octet-stream",
    );
    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &vector_descriptor(&geojson, Some(&wrong_mime))
    )
    .unwrap_err()
    .to_string()
    .contains("MIME"));

    let png = Uuid::new_v4().to_string();
    insert_asset(&connection, &png, &map_id, "image/png");
    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &vector_descriptor(&geojson, Some(&png))
    )
    .is_ok());

    let jpeg = Uuid::new_v4().to_string();
    insert_asset(&connection, &jpeg, &map_id, "image/jpeg");
    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &vector_descriptor(&geojson, Some(&jpeg))
    )
    .is_ok());

    let svg = Uuid::new_v4().to_string();
    insert_asset(&connection, &svg, &map_id, "image/svg+xml");
    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &vector_descriptor(&geojson, Some(&svg))
    )
    .is_ok());

    assert!(validate_field(
        &connection,
        &map_id,
        "map",
        &serde_json::json!({
            "schemaVersion": 1,
            "provider": {"id": "daena-image", "adapterVersion": 1, "sourceFormat": "png"},
            "sourceAssetId": png,
            "previewAssetId": null,
            "defaultView": {"center": [0.5, 0.5], "zoom": 1}
        })
    )
    .unwrap_err()
    .to_string()
    .contains("unsupported map provider"));
}

#[test]
fn validates_raster_layers_and_rejects_malformed_shapes() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    let other_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    insert_entity(&connection, &other_id, MAP_ENTITY_TYPE);
    let raster_id = Uuid::new_v4().to_string();
    let other_raster = Uuid::new_v4().to_string();
    let jpeg_id = Uuid::new_v4().to_string();
    insert_asset(&connection, &raster_id, &map_id, "image/png");
    insert_asset(&connection, &other_raster, &other_id, "image/png");
    insert_asset(&connection, &jpeg_id, &map_id, "image/jpeg");

    let valid = serde_json::json!({
        "schemaVersion": 1,
        "layers": [
            {
                "id": "018f8a01-9c20-ae05-b442-46dd3de2446c",
                "name": "Settlements",
                "order": 1,
                "defaultVisible": true,
                "style": {"color": "#334155"},
                "selector": {}
            },
            {
                "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
                "name": "Countries",
                "order": 0,
                "defaultVisible": true,
                "style": {},
                "selector": {},
                "kind": "raster",
                "rasterAssetId": raster_id,
                "opacity": 0.5,
                "locked": true
            }
        ]
    });
    assert!(validate_field(&connection, &map_id, "layers", &valid).is_ok());

    let dangling = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
            "name": "Countries",
            "order": 0,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster",
            "rasterAssetId": Uuid::new_v4().to_string(),
            "opacity": 1,
            "locked": false
        }]
    });
    assert!(validate_field(&connection, &map_id, "layers", &dangling)
        .unwrap_err()
        .to_string()
        .contains("rasterAssetId"));

    let cross_entity = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
            "name": "Countries",
            "order": 0,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster",
            "rasterAssetId": other_raster,
            "opacity": 1,
            "locked": false
        }]
    });
    assert!(validate_field(&connection, &map_id, "layers", &cross_entity).is_err());

    let bad_mime = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
            "name": "Countries",
            "order": 0,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster",
            "rasterAssetId": jpeg_id,
            "opacity": 1,
            "locked": false
        }]
    });
    assert!(validate_field(&connection, &map_id, "layers", &bad_mime)
        .unwrap_err()
        .to_string()
        .contains("PNG"));

    let bad_opacity = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
            "name": "Countries",
            "order": 0,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster",
            "rasterAssetId": raster_id,
            "opacity": 1.5,
            "locked": false
        }]
    });
    assert!(validate_field(&connection, &map_id, "layers", &bad_opacity)
        .unwrap_err()
        .to_string()
        .contains("opacity"));

    let malformed = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": "018f89f7-69fd-7fa2-811f-13aa0abf1139",
            "name": "Countries",
            "order": 0,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster"
        }]
    });
    assert!(validate_field(&connection, &map_id, "layers", &malformed)
        .unwrap_err()
        .to_string()
        .contains("layer definition is invalid"));

    let mut too_many = Vec::new();
    for index in 0..=IMAGE_MAX_RASTER_LAYERS {
        let raster_id = Uuid::new_v4().to_string();
        insert_asset(&connection, &raster_id, &map_id, "image/png");
        too_many.push(serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "name": format!("Layer {index}"),
            "order": index as i64,
            "defaultVisible": true,
            "style": {},
            "selector": {},
            "kind": "raster",
            "rasterAssetId": raster_id,
            "opacity": 1,
            "locked": false
        }));
    }
    let overflow = serde_json::json!({"schemaVersion": 1, "layers": too_many});
    assert!(validate_field(&connection, &map_id, "layers", &overflow)
        .unwrap_err()
        .to_string()
        .contains("raster layer count"));
}

#[test]
fn checkpoint_image_bytes_must_decode_and_stay_safe() {
    let connection = maps_tables();
    let map_id = Uuid::new_v4().to_string();
    let source_id = Uuid::new_v4().to_string();
    insert_entity(&connection, &map_id, MAP_ENTITY_TYPE);
    insert_asset(&connection, &source_id, &map_id, "image/svg+xml");
    let geojson = Uuid::new_v4().to_string();
    insert_asset(&connection, &geojson, &map_id, VECTOR_MIME);
    let descriptor = vector_descriptor(&geojson, Some(&source_id));
    connection
        .execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
            params![
                map_id,
                MAP_NAMESPACE,
                "map",
                serde_json::to_string(&descriptor).unwrap()
            ],
        )
        .unwrap();
    let unsafe_svg = b"<svg\nonload=\"alert(1)\" viewBox=\"0 0 10 10\"></svg>";
    let empty = crate::maps::empty_canonical_bytes();
    let error = validate_image_map_content(&connection, |asset_id| {
        if asset_id == geojson {
            Ok(empty.clone())
        } else {
            Ok(unsafe_svg.to_vec())
        }
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("unsupported active"), "{error}");
}
