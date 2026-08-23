use super::*;
use crate::{
    ImportDiagnostic, ImportDiagnosticSeverity, ImportSource, ImportSourceKind,
    ImportValidationIssue, ImportValidationSeverity, ImporterIdentity, StagedDocument, StagedLink,
    UnsupportedSourceData, ValidatedImportAsset, ValidatedImportField, ValidatedImportObject,
    ValidatedImportRelationship, ValidatedImportSourceContext,
};
use daena_plugin_api::MetadataFieldDefinition;
use std::collections::BTreeMap;

fn canonical_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, current: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) == Some(".daena") {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if relative == "project.json"
                    || relative.starts_with("entities/")
                    || relative.starts_with("plugins/")
                    || relative.starts_with("assets/")
                {
                    files.insert(relative, std::fs::read(path).unwrap());
                }
            }
        }
    }

    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn directory_session_lock_rejects_second_writer_and_reclaims_dead_owner() {
    let root = std::env::temp_dir().join(format!("daena-lock-{}", Uuid::new_v4()));
    let first = ProjectStore::open_directory(&root).unwrap();
    assert!(matches!(
        ProjectStore::open_directory(&root),
        Err(CoreError::Conflict(message)) if message.contains("already open")
    ));
    drop(first);
    std::fs::write(root.join(".daena/project.lock"), b"").unwrap();
    assert!(matches!(
        ProjectStore::open_directory(&root),
        Err(CoreError::Conflict(message)) if message.contains("already open")
    ));
    std::fs::remove_file(root.join(".daena/project.lock")).unwrap();
    std::fs::write(
        root.join(".daena/project.lock"),
        format!("{}\ndead-owner\n", i32::MAX),
    )
    .unwrap();
    let reclaimed = ProjectStore::open_directory(&root).unwrap();
    drop(reclaimed);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn update_relationship_metadata_validates_and_roundtrips() {
    let root = std::env::temp_dir().join(format!("daena-relationship-metadata-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    let source = store
        .create_entity(CreateEntity {
            name: "Artifact".into(),
            entity_type: Some("artifact".into()),
        })
        .unwrap();
    let target = store
        .create_entity(CreateEntity {
            name: "Owner".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    store
        .set_relationship_metadata_schemas(BTreeMap::from([(
            "owned_by".into(),
            vec![
                MetadataFieldDefinition {
                    key: "validFrom".into(),
                    label: "Valid from".into(),
                    field_type: "date".into(),
                    required: Some(true),
                    options: None,
                    one_of: None,
                },
                MetadataFieldDefinition {
                    key: "status".into(),
                    label: "Status".into(),
                    field_type: "enum".into(),
                    required: None,
                    options: Some(vec!["active".into(), "ended".into()]),
                    one_of: None,
                },
            ],
        )]))
        .unwrap();

    let relationship = store
        .create_relationship(RelationshipInput {
            source_id: source.id.clone(),
            target_id: target.id,
            relationship_type: "owned_by".into(),
            metadata: Some(
                serde_json::json!({
                    "validFrom": "2024-01-01",
                    "unknown": "preserved"
                })
                .to_string(),
            ),
        })
        .unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&relationship.metadata).unwrap(),
        serde_json::json!({"validFrom": "2024-01-01", "unknown": "preserved"})
    );

    let replacement_target = store
        .create_entity(CreateEntity {
            name: "Replacement owner".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let original_revision = relationship.revision.clone();
    let update = RelationshipUpdate {
        id: relationship.id.clone(),
        metadata: Some(
            serde_json::json!({
                    "validFrom": "2025-01-01",
                    "status": "active",
                    "unknown": "preserved"
            })
            .to_string(),
        ),
        target_id: Some(replacement_target.id.clone()),
    };
    let request_id = Uuid::new_v4().to_string();
    let updated = store
        .update_relationship_with_options(
            update.clone(),
            Some(&original_revision),
            Some(&request_id),
        )
        .unwrap();
    assert_eq!(updated.id, relationship.id);
    assert_eq!(updated.target_id, replacement_target.id);
    assert_ne!(updated.revision, original_revision);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&updated.metadata).unwrap(),
        serde_json::json!({"validFrom": "2025-01-01", "status": "active", "unknown": "preserved"})
    );

    let repeated = store
        .update_relationship_with_options(update, Some(&original_revision), Some(&request_id))
        .unwrap();
    assert_eq!(repeated.id, updated.id);
    assert_eq!(repeated.revision, updated.revision);

    let stale = store.update_relationship_with_options(
        RelationshipUpdate {
            id: relationship.id.clone(),
            metadata: Some("{}".into()),
            target_id: None,
        },
        Some(&original_revision),
        None,
    );
    assert!(
        matches!(stale, Err(CoreError::Conflict(message)) if message.contains("relationship revision conflict"))
    );

    let invalid_update = store.update_relationship_with_options(
        RelationshipUpdate {
            id: relationship.id.clone(),
            metadata: Some(serde_json::json!({"validFrom": "not-a-date"}).to_string()),
            target_id: None,
        },
        Some(&updated.revision),
        None,
    );
    assert!(
        matches!(invalid_update, Err(CoreError::Validation(message)) if message.contains("validFrom"))
    );

    let missing_required = store.create_relationship(RelationshipInput {
        source_id: source.id.clone(),
        target_id: relationship.target_id.clone(),
        relationship_type: "owned_by".into(),
        metadata: Some("{}".into()),
    });
    assert!(
        matches!(missing_required, Err(CoreError::Validation(message)) if message.contains("validFrom"))
    );

    let invalid_date = store.create_relationship(RelationshipInput {
        source_id: source.id.clone(),
        target_id: relationship.target_id.clone(),
        relationship_type: "owned_by".into(),
        metadata: Some(serde_json::json!({"validFrom": "not-a-date"}).to_string()),
    });
    assert!(
        matches!(invalid_date, Err(CoreError::Validation(message)) if message.contains("validFrom"))
    );

    store
        .flush_checkpoint("relationship metadata test")
        .unwrap();
    let before_recovery = canonical_files(&root);
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    rebuilt
        .flush_checkpoint("relationship metadata recovery test")
        .unwrap();
    assert_eq!(before_recovery, canonical_files(&root));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn physical_map_acceptance_is_atomic_and_request_idempotent() {
    let settings = daena_physical::GenerationSettings {
        width: 16,
        height: 8,
        radius_metres: daena_physical::DEFAULT_RADIUS_METRES,
        target_land_fraction_ppm: 300_000,
    };
    let mut progress = daena_physical::NoopProgress;
    let world = daena_physical::generate_world(settings, 831_429, 0, &mut progress).unwrap();
    let generation = serde_json::json!({
        "id": crate::maps::PHYSICAL_GENERATOR_ID,
        "version": crate::maps::PHYSICAL_GENERATOR_VERSION,
        "seed": 831429,
        "retryIndex": 0,
        "settings": {
            "width": settings.width,
            "height": settings.height,
            "radiusMetres": settings.radius_metres,
            "targetLandFractionPpm": settings.target_land_fraction_ppm,
            "referenceWaterInventoryM3": world.report.reference_water_inventory_m3,
            "plateCount": world.tectonics.settings.plate_count,
            "continentalPlateCount": world.tectonics.settings.continental_plate_count,
            "tectonicActivityPpm": world.tectonics.settings.tectonic_activity_ppm,
            "islandActivityPpm": world.tectonics.settings.island_activity_ppm,
            "evolutionPreset": "mature",
            "hazardDerivationVersion": daena_physical::hazards::HAZARD_DERIVATION_VERSION,
            "historicalForcing": {
                "version": 2,
                "components": [
                    { "amplitudeCentiC": 180, "periodYears": 12000, "phaseOffsetYears": 0 },
                    { "amplitudeCentiC": 90, "periodYears": 4100, "phaseOffsetYears": 200 },
                    { "amplitudeCentiC": 40, "periodYears": 2300, "phaseOffsetYears": 800 }
                ],
                "sensitivityPpm": 1000000,
                "landIceAmplitudePpm": 24000,
                "iceResponseYears": 800,
                "iceMidpointCentiC": 0,
                "iceTransitionWidthCentiC": 400,
                "thermalExpansionPpmPerDegreeC": 210
            }
        }
    });
    let root = std::env::temp_dir().join(format!("daena-physical-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let accepted = store
        .accept_physical_map(
            "Physical test".into(),
            world.source.clone(),
            generation.clone(),
            Some("00000000-0000-4000-8000-000000000001"),
        )
        .unwrap();
    let expected_identity = crate::maps::physical::validate_source(&world.source, &generation)
        .unwrap()
        .identity;
    assert_eq!(accepted.physical_identity, expected_identity);
    let replayed = store
        .accept_physical_map(
            "Physical test".into(),
            world.source.clone(),
            generation.clone(),
            Some("00000000-0000-4000-8000-000000000001"),
        )
        .unwrap();
    assert_eq!(accepted.entity.id, replayed.entity.id);
    assert_eq!(accepted.physical_identity, replayed.physical_identity);
    assert_eq!(store.list_entities().unwrap().len(), 1);
    assert_eq!(
        store.asset_bytes(accepted.source.id.clone()).unwrap(),
        world.source
    );
    let descriptor = store
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "map")
        .unwrap();
    assert_eq!(
        crate::maps::physical::validate_source(&world.source, &descriptor.value["generation"])
            .unwrap()
            .identity,
        accepted.physical_identity
    );
    store
        .update_entity(
            accepted.entity.id.clone(),
            Some("Renamed presentation".into()),
            None,
        )
        .unwrap();
    let mut presentation = descriptor.clone();
    presentation.value["defaultView"]["zoom"] = serde_json::json!(2);
    store.set_field(presentation).unwrap();
    let presentation_descriptor = store
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "map")
        .unwrap();
    assert_eq!(
        crate::maps::physical::validate_source(
            &world.source,
            &presentation_descriptor.value["generation"],
        )
        .unwrap()
        .identity,
        accepted.physical_identity
    );
    let mut identity_change = presentation_descriptor.clone();
    identity_change.value["generation"]["settings"]["evolutionPreset"] = serde_json::json!("young");
    assert!(matches!(
        store.set_field(identity_change),
        Err(CoreError::Validation(message))
            if message.contains("physical identity fields are immutable")
    ));
    assert_eq!(
        descriptor.value["provider"]["id"],
        crate::maps::PHYSICAL_PROVIDER
    );
    assert_eq!(descriptor.value["sourceAssetId"], accepted.source.id);
    let authored_source_id = descriptor.value["authoredSourceAssetId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(authored_source_id, accepted.source.id);
    assert_eq!(
        store.asset_bytes(authored_source_id.clone()).unwrap(),
        crate::maps::vector::empty_canonical_bytes()
    );
    let layers = store
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap();
    for layer_id in ["base", "land", "ocean", "lakes", "rivers", "islands", "ice"] {
        assert_eq!(
            layers.value["layers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|layer| layer["id"] == layer_id)
                .and_then(|layer| layer["locked"].as_bool()),
            Some(true)
        );
    }
    assert!(matches!(
        store.update_map_layer(
            accepted.entity.id.clone(),
            "land".into(),
            RasterLayerUpdate {
                name: Some("tampered".into()),
                order: None,
                default_visible: None,
                opacity: None,
                locked: None,
                style: None,
                selector: None,
            },
            &layers.revision,
            None,
        ),
        Err(CoreError::Validation(message)) if message.contains("physical layers are immutable")
    ));
    let mut tampered_layers = layers.clone();
    tampered_layers.value["layers"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|layer| layer["id"] == "land")
        .unwrap()["name"] = serde_json::json!("tampered");
    assert!(matches!(
        store.set_field(tampered_layers),
        Err(CoreError::Validation(message)) if message.contains("physical layer definitions are immutable")
    ));
    let authored_base_feature = serde_json::to_vec(&serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "id": "00000000-0000-4000-8000-000000000002",
            "properties": {"daenaLayerId": "base", "kind": "land", "name": null},
            "geometry": {"type": "Polygon", "coordinates": [[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]]}
        }]
    }))
    .unwrap();
    let authored_revision = store.asset(authored_source_id.clone()).unwrap().revision;
    let authored_hash = format!("sha256:{:x}", Sha256::digest(&authored_base_feature));
    assert!(matches!(
        store.replace_vector_source(
            authored_source_id.clone(),
            authored_base_feature,
            authored_hash,
            &authored_revision,
            None,
        ),
        Err(CoreError::Validation(message)) if message.contains("physical layers are immutable")
    ));
    assert!(matches!(
        store.accept_physical_map(
            "Different name".into(),
            world.source.clone(),
            generation,
            Some("00000000-0000-4000-8000-000000000001"),
        ),
        Err(CoreError::Conflict(_))
    ));
    store.flush_checkpoint("physical acceptance test").unwrap();
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(rebuilt.list_entities().unwrap().len(), 1);
    assert_eq!(
        rebuilt.asset_bytes(accepted.source.id.clone()).unwrap(),
        world.source
    );
    let rebuilt_descriptor = rebuilt
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "map")
        .unwrap();
    assert_eq!(
        crate::maps::physical::validate_source(
            &rebuilt.asset_bytes(accepted.source.id.clone()).unwrap(),
            &rebuilt_descriptor.value["generation"],
        )
        .unwrap()
        .identity,
        accepted.physical_identity
    );
    assert_eq!(
        rebuilt.asset_bytes(authored_source_id).unwrap(),
        crate::maps::vector::empty_canonical_bytes()
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_import_commit_is_idempotent_and_survives_clean_rebuild() {
    let root = std::env::temp_dir().join(format!("daena-external-commit-{}", Uuid::new_v4()));
    let source_root =
        std::env::temp_dir().join(format!("daena-external-source-{}", Uuid::new_v4()));
    std::fs::create_dir_all(source_root.join("assets")).unwrap();
    let asset_bytes = b"\x89PNG\r\n\x1a\nfixture";
    std::fs::write(source_root.join("assets/map.png"), asset_bytes).unwrap();
    let store = ProjectStore::open_directory(&root).unwrap();
    let existing = store
        .create_entity(CreateEntity {
            name: "Existing note".into(),
            entity_type: Some("note".into()),
        })
        .unwrap();
    let generation = store.content_generation().unwrap();
    let importer = ImporterIdentity {
        id: "test.importer".into(),
        version: "1".into(),
        name: "Test importer".into(),
    };
    let source = ImportSource {
        id: "source-root".into(),
        kind: ImportSourceKind::Folder,
        display_name: "Fixture".into(),
    };
    let plan = ValidatedImportPlan {
        schema_version: VALIDATED_IMPORT_PLAN_SCHEMA_VERSION,
        plan_id: "validated-plan".into(),
        candidate_plan_id: "candidate-plan".into(),
        session_id: "session".into(),
        importer,
        source,
        content_generation: generation,
        manifest_fingerprint: "manifest".into(),
        objects: vec![
            ValidatedImportObject {
                staged_object_id: "create-object".into(),
                source_id: "create-source".into(),
                source_path: "Created.md".into(),
                content_hash: "create-hash".into(),
                title: "Created note".into(),
                entity_type: Some("note".into()),
                document: Some(StagedDocument {
                    format: "markdown".into(),
                    body: "# Created note".into(),
                }),
                fields: vec![ValidatedImportField {
                    source_key: "summary".into(),
                    namespace: "test.importer".into(),
                    key: "summary".into(),
                    value: serde_json::Value::String("Imported summary".into()),
                }],
                source_context: ValidatedImportSourceContext {
                    source_kind: "obsidian_markdown".into(),
                    tags: vec!["lore".into()],
                    aliases: vec!["Created alias".into()],
                    links: vec![StagedLink {
                        kind: StagedLinkKind::Internal,
                        target: "Missing note".into(),
                        label: None,
                        resolution: StagedLinkResolution::Missing,
                        resolved_object_id: None,
                        candidate_object_ids: Vec::new(),
                        raw: None,
                    }],
                    ..ValidatedImportSourceContext::default()
                },
                decision: ImportObjectDecision::Create,
            },
            ValidatedImportObject {
                staged_object_id: "mapped-object".into(),
                source_id: "mapped-source".into(),
                source_path: "Mapped.md".into(),
                content_hash: "mapped-hash".into(),
                title: "Mapped note".into(),
                entity_type: existing.entity_type.clone(),
                document: None,
                fields: Vec::new(),
                source_context: ValidatedImportSourceContext::default(),
                decision: ImportObjectDecision::MapToExisting {
                    entity_id: existing.id.clone(),
                    expected_revision: existing.revision.clone(),
                },
            },
            ValidatedImportObject {
                staged_object_id: "skipped-object".into(),
                source_id: "skipped-source".into(),
                source_path: "Skipped.md".into(),
                content_hash: "skipped-hash".into(),
                title: "Skipped note".into(),
                entity_type: Some("note".into()),
                document: None,
                fields: Vec::new(),
                source_context: ValidatedImportSourceContext::default(),
                decision: ImportObjectDecision::Skip,
            },
        ],
        relationships: vec![ValidatedImportRelationship {
            source_staged_object_id: "create-object".into(),
            target_staged_object_id: "mapped-object".into(),
            relationship_type: "references".into(),
            source_kind: "internal".into(),
            source_target: "Mapped".into(),
        }],
        assets: vec![ValidatedImportAsset {
            staged_asset_id: "asset-object".into(),
            owner_staged_object_id: "create-object".into(),
            source_path: "assets/map.png".into(),
            filename: "map.png".into(),
            content_hash: format!("sha256:{}", digest_bytes(asset_bytes)),
            size: asset_bytes.len() as u64,
            mime_type: "image/png".into(),
        }],
        unsupported: vec![UnsupportedSourceData {
            source_path: "Unsupported.bin".into(),
            source_kind: "file".into(),
            reason: "unsupported fixture".into(),
            raw_metadata: BTreeMap::new(),
        }],
        diagnostics: vec![ImportDiagnostic {
            severity: ImportDiagnosticSeverity::Warning,
            code: "fixture_diagnostic".into(),
            message: "Fixture diagnostic".into(),
            source_path: Some("Created.md".into()),
            object_id: Some("create-object".into()),
        }],
        warnings: vec![ImportValidationIssue {
            severity: ImportValidationSeverity::Warning,
            code: "fixture_warning".into(),
            message: "Fixture warning".into(),
            source_path: None,
            object_id: None,
            existing_entity_id: None,
        }],
    };

    let request_id = "00000000-0000-4000-8000-000000000001";
    assert!(store
        .commit_external_import(&plan, None, false, request_id)
        .is_err());
    std::fs::write(source_root.join("assets/map.png"), b"changed").unwrap();
    assert!(store
        .commit_external_import(&plan, Some(&source_root), true, request_id)
        .is_err());
    assert_eq!(store.list_entities().unwrap().len(), 1);
    std::fs::write(source_root.join("assets/map.png"), asset_bytes).unwrap();
    let runtime_asset = runtime_asset_path(&root, &plan.assets[0].content_hash).unwrap();
    assert!(store
        .commit_external_import(&plan, Some(&source_root), true, "invalid-request-id")
        .is_err());
    assert!(!runtime_asset.exists());
    let first = store
        .commit_external_import(&plan, Some(&source_root), true, request_id)
        .unwrap();
    let retry = store
        .commit_external_import(&plan, Some(&source_root), true, request_id)
        .unwrap();
    assert_eq!(retry, first);
    assert_eq!(first.created.len(), 1);
    assert_eq!(first.mapped.len(), 1);
    assert_eq!(first.assets.len(), 1);
    assert_eq!(first.relationships.len(), 1);
    assert_eq!(first.decisions.len(), 3);
    assert_eq!(first.fields.len(), 1);
    assert_eq!(first.unsupported.len(), 1);
    assert_eq!(first.missing_references.len(), 1);
    assert_eq!(first.diagnostics.len(), 1);
    assert_eq!(
        first.relationships[0].source_entity_id,
        first.created[0].entity_id
    );
    assert_eq!(first.relationships[0].target_entity_id, existing.id);
    assert_eq!(
        store.asset_bytes(first.assets[0].asset_id.clone()).unwrap(),
        asset_bytes
    );
    assert_eq!(first.skipped_source_paths, vec!["Skipped.md"]);
    assert_eq!(store.list_entities().unwrap().len(), 2);
    let duplicates = store
        .external_import_duplicate_targets(
            "test.importer",
            &[
                ("create-object".into(), "create-source".into()),
                ("mapped-object".into(), "mapped-source".into()),
            ],
        )
        .unwrap();
    assert_eq!(
        duplicates["create-object"],
        vec![first.created[0].entity_id.clone()]
    );
    assert_eq!(duplicates["mapped-object"], vec![existing.id]);

    store
        .flush_checkpoint("external import commit test")
        .unwrap();
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(rebuilt.list_entities().unwrap().len(), 2);
    assert_eq!(
        rebuilt
            .list_relationships(first.created[0].entity_id.clone())
            .unwrap()
            .len(),
        1
    );
    let rebuilt_duplicates = rebuilt
        .external_import_duplicate_targets(
            "test.importer",
            &[("create-object".into(), "create-source".into())],
        )
        .unwrap();
    assert_eq!(
        rebuilt_duplicates["create-object"],
        vec![first.created[0].entity_id.clone()]
    );
    let rebuilt_assets = rebuilt
        .list_assets(first.created[0].entity_id.clone())
        .unwrap();
    assert_eq!(rebuilt_assets.len(), 1);
    assert_eq!(
        rebuilt.asset_bytes(rebuilt_assets[0].id.clone()).unwrap(),
        asset_bytes
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(source_root).unwrap();
}

#[test]
fn read_only_project_store_reads_while_writer_session_is_open() {
    let root = std::env::temp_dir().join(format!("daena-read-only-{}", Uuid::new_v4()));
    let writer = ProjectStore::open_directory(&root).unwrap();
    let entity = writer
        .create_entity(CreateEntity {
            name: "Independent read".into(),
            entity_type: Some("note".into()),
        })
        .unwrap();

    let reader = ProjectStore::open_read_only(&root).unwrap();
    assert_eq!(reader.list_entities().unwrap()[0].id, entity.id);
    drop(reader);
    drop(writer);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fresh_runtime_starts_with_checkpoint_generation_metadata() {
    let root = std::env::temp_dir().join(format!("daena-runtime-meta-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let metadata = store
        .connection
        .query_row(
            "SELECT schema_version, portable_format_version, content_generation, exported_generation, checkpoint_digest, export_error FROM runtime_meta WHERE key='runtime'",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(metadata.0, RUNTIME_SCHEMA_VERSION);
    assert_eq!(metadata.1, 3);
    assert_eq!(metadata.2, 0);
    assert_eq!(metadata.3, 0);
    assert!(metadata.4.is_some());
    assert!(metadata.5.is_none());
    for obsolete in ["sync_state", "dirty_count", "clean_shutdown"] {
        assert!(!store
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('runtime_meta') WHERE name=?1)",
                [obsolete],
                |row| row.get::<_, bool>(0),
            )
            .unwrap());
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn portable_content_mutations_advance_generation_in_the_same_database() {
    let root = std::env::temp_dir().join(format!("daena-runtime-generation-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let generation = |store: &ProjectStore| {
        store
            .connection
            .query_row(
                "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
    };
    assert_eq!(generation(&store), 0);
    let entity = store
        .create_entity(CreateEntity {
            name: "Generation owner".into(),
            entity_type: None,
        })
        .unwrap();
    assert_eq!(generation(&store), 1);
    store
        .update_entity(entity.id, Some("Updated generation owner".into()), None)
        .unwrap();
    assert_eq!(generation(&store), 2);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn completed_export_installs_checkpoint_manifest_before_advancing_state() {
    let root = std::env::temp_dir().join(format!("daena-checkpoint-export-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Checkpoint owner".into(),
            entity_type: None,
        })
        .unwrap();
    let flushed_generation = store.flush_checkpoint("checkpoint test").unwrap();
    let checkpoint_path = root.join(crate::storage::CHECKPOINT_MANIFEST_FILE);
    let checkpoint =
        crate::storage::read_json::<crate::storage::CheckpointManifest>(&checkpoint_path).unwrap();
    crate::storage::validate_checkpoint(&root, &checkpoint).unwrap();
    let state = store
        .connection
        .query_row(
            "SELECT content_generation, exported_generation, checkpoint_digest FROM runtime_meta WHERE key='runtime'",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(state.0, checkpoint.content_generation);
    assert_eq!(state.1, checkpoint.content_generation);
    assert_eq!(flushed_generation, checkpoint.content_generation);
    assert!(state.2.is_some());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn checkpoint_handle_flushes_without_borrowing_the_live_store() {
    let root = std::env::temp_dir().join(format!("daena-checkpoint-handle-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Detached barrier".into(),
            entity_type: None,
        })
        .unwrap();

    let handle = store.checkpoint_handle().unwrap();
    let generation = handle.flush_checkpoint("detached checkpoint test").unwrap();
    assert_eq!(generation, 1);
    assert_eq!(store.sync_summary().unwrap().state, "clean");
    let checkpoint: crate::storage::CheckpointManifest =
        crate::storage::read_json(&root.join(crate::storage::CHECKPOINT_MANIFEST_FILE)).unwrap();
    assert_eq!(checkpoint.content_generation, generation);

    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_checkpoint_handles_share_the_export_worker() {
    let root = std::env::temp_dir().join(format!("daena-checkpoint-race-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Concurrent barrier".into(),
            entity_type: None,
        })
        .unwrap();

    let first = store.checkpoint_handle().unwrap();
    let second = store.checkpoint_handle().unwrap();
    let first = std::thread::spawn(move || first.flush_checkpoint("first concurrent barrier"));
    let second = std::thread::spawn(move || second.flush_checkpoint("second concurrent barrier"));
    assert!(first.join().unwrap().is_ok());
    assert!(second.join().unwrap().is_ok());
    assert_eq!(store.sync_summary().unwrap().state, "clean");

    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn runtime_asset_bytes_survive_an_interrupted_export() {
    let root = std::env::temp_dir().join(format!("daena-runtime-asset-{}", Uuid::new_v4()));
    let source = std::env::temp_dir().join(format!("daena-asset-source-{}", Uuid::new_v4()));
    std::fs::write(&source, b"durable runtime asset").unwrap();

    let mut store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Asset owner".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .export_worker
        .take()
        .unwrap()
        .stop_without_drain()
        .unwrap();
    store.suppress_sync.set(true);
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: entity.id,
            namespace: "lore".into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "durable.bin".into(),
            mime_type: "application/octet-stream".into(),
        })
        .unwrap();
    assert!(!root.join(&asset.path).exists());
    drop(store);
    std::fs::remove_file(&source).unwrap();

    let reopened = ProjectStore::open_directory(&root).unwrap();
    reopened
        .flush_checkpoint("recover interrupted asset export")
        .unwrap();
    assert_eq!(
        std::fs::read(root.join(&asset.path)).unwrap(),
        b"durable runtime asset"
    );
    assert_eq!(reopened.sync_summary().unwrap().state, "clean");

    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn previous_runtime_schema_is_reset_required() {
    let root = std::env::temp_dir().join(format!("daena-runtime-reset-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .connection
        .execute(
            "UPDATE runtime_meta SET schema_version=1 WHERE key='runtime'",
            [],
        )
        .unwrap();
    drop(store);
    assert!(matches!(
        ProjectStore::open_directory(&root),
        Err(CoreError::ResetRequired(_))
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn portable_backup_flushes_pending_runtime_changes_before_serializing() {
    let root = std::env::temp_dir().join(format!("daena-portable-backup-{}", Uuid::new_v4()));
    let backup_dir = std::env::temp_dir().join(format!("daena-backup-output-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Backup owner".into(),
            entity_type: None,
        })
        .unwrap();

    let backup = store.portable_backup_to(&backup_dir).unwrap();
    assert!(Path::new(&backup).join("project.json").is_file());
    let entity_file = std::fs::read_dir(Path::new(&backup).join("entities"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("entity.json");
    assert!(std::fs::read_to_string(entity_file)
        .unwrap()
        .contains("Backup owner"));

    drop(store);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(backup_dir).unwrap();
}

#[test]
fn portable_backup_restores_from_files_without_runtime_database() {
    let source_root =
        std::env::temp_dir().join(format!("daena-portable-source-{}", Uuid::new_v4()));
    let target_root =
        std::env::temp_dir().join(format!("daena-portable-target-{}", Uuid::new_v4()));
    let output =
        std::env::temp_dir().join(format!("daena-portable-restore-output-{}", Uuid::new_v4()));
    let source = ProjectStore::open_directory(&source_root).unwrap();
    source
        .create_entity(CreateEntity {
            name: "Portable source".into(),
            entity_type: None,
        })
        .unwrap();
    let backup = source.portable_backup_to(&output).unwrap();
    drop(source);

    let mut target = ProjectStore::open_directory(&target_root).unwrap();
    target.restore(backup).unwrap();
    assert_eq!(target.list_entities().unwrap()[0].name, "Portable source");
    drop(target);
    std::fs::remove_dir_all(source_root).unwrap();
    std::fs::remove_dir_all(target_root).unwrap();
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn portable_backup_rejects_invalid_canonical_files() {
    let root = std::env::temp_dir().join(format!("daena-invalid-backup-{}", Uuid::new_v4()));
    let output =
        std::env::temp_dir().join(format!("daena-invalid-backup-output-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store.flush_checkpoint("test export").unwrap();
    std::fs::write(root.join("project.json"), b"{ invalid json").unwrap();

    assert!(matches!(
        store.backup_to(&output),
        Err(CoreError::NotFound(_))
            | Err(CoreError::Serialization(_))
            | Err(CoreError::Validation(_))
            | Err(CoreError::Conflict(_))
    ));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
    let _ = std::fs::remove_dir_all(output);
}

#[test]
fn rebuilding_disposable_index_invalidates_revisions_by_epoch() {
    let root = std::env::temp_dir().join(format!("daena-epoch-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Epoch owner".into(),
            entity_type: None,
        })
        .unwrap();
    let revision = entity.revision;
    drop(store);
    std::fs::remove_file(root.join(".daena/index.sqlite")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    let rebuilt_revision = rebuilt.list_entities().unwrap()[0].revision.clone();
    assert_ne!(revision, rebuilt_revision);
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn request_id_reuse_with_different_entity_input_fails_closed() {
    let root = std::env::temp_dir().join(format!("daena-request-fingerprint-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let request_id = Uuid::new_v4().to_string();
    store
        .create_entity_with_request(
            CreateEntity {
                name: "First request".into(),
                entity_type: None,
            },
            Some(&request_id),
        )
        .unwrap();
    let retry = store.create_entity_with_request(
        CreateEntity {
            name: "Incompatible retry".into(),
            entity_type: None,
        },
        Some(&request_id),
    );
    assert!(
        matches!(retry, Err(CoreError::Conflict(message)) if message.contains("different inputs"))
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn creates_entities_and_rejects_empty_names() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Eldermere".into(),
            entity_type: None,
        })
        .unwrap();
    assert_eq!(store.list_entities().unwrap()[0].id, entity.id);
    assert!(store
        .create_entity(CreateEntity {
            name: "  ".into(),
            entity_type: None
        })
        .is_err());
}

#[test]
fn recovery_copy_is_markdown_and_stays_outside_canonical_sources() {
    let root = std::env::temp_dir().join(format!("daena-recovery-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Recovery record".into(),
            entity_type: None,
        })
        .unwrap();
    let path = store
        .save_recovery_copy(&entity.id, "Draft\r\nwithout final newline")
        .unwrap();
    assert!(path.starts_with(".daena/conflicts/") && path.ends_with(".md"));
    assert_eq!(
        std::fs::read_to_string(root.join(&path)).unwrap(),
        "Draft\nwithout final newline\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_preflight_lists_only_canonical_paths_and_rejects_staged_unrelated_files() {
    let root = std::env::temp_dir().join(format!("daena-git-preview-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    assert!(run_git(&["init", "-q"]).status.success());
    assert!(run_git(&["config", "user.email", "tests@daena.local"])
        .status
        .success());
    assert!(run_git(&["config", "user.name", "Daena tests"])
        .status
        .success());
    assert!(run_git(&["config", "commit.gpgsign", "false"])
        .status
        .success());
    assert!(run_git(&["add", "--all"]).status.success());
    assert!(run_git(&["commit", "-qm", "base"]).status.success());

    store
        .create_entity(CreateEntity {
            name: "Preview entity".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store.flush_checkpoint("test export").unwrap();
    let preview = store.git_staging_preview().unwrap();
    assert!(preview.ready);
    assert!(preview
        .staging_paths
        .iter()
        .any(|path| path.starts_with("entities/") && path.ends_with("/entity.json")));
    assert!(preview
        .staging_paths
        .iter()
        .all(|path| ProjectStore::is_canonical_git_path(path)));

    std::fs::write(root.join("README.md"), "unrelated\n").unwrap();
    assert!(run_git(&["add", "README.md"]).status.success());
    let rejected = store.git_preflight().unwrap();
    assert!(!rejected.ready);
    assert!(rejected
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.starts_with("git.noncanonical-staged:")));
    assert!(rejected
        .staging_paths
        .iter()
        .all(|path| { ProjectStore::is_canonical_git_path(path) }));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_tool_info_reports_system_git() {
    let info = ProjectStore::git_tool_info();
    assert!(info.available, "{:?}", info.error);
    assert!(info
        .version
        .as_deref()
        .is_some_and(|version| version.to_ascii_lowercase().contains("git")));
}

#[test]
fn git_integration_does_not_attach_to_a_parent_repository() {
    let parent = std::env::temp_dir().join(format!("daena-git-parent-{}", Uuid::new_v4()));
    let root = parent.join("project");
    std::fs::create_dir_all(&parent).unwrap();
    let run_git = |directory: &std::path::Path, args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(directory)
            .output()
            .unwrap()
    };
    assert!(run_git(&parent, &["init", "-q"]).status.success());

    let store = ProjectStore::open_directory(&root).unwrap();
    assert!(!store.git_status().unwrap().repository);

    let initialized = store.git_init().unwrap();
    assert!(initialized.repository);
    let top_level =
        String::from_utf8(run_git(&root, &["rev-parse", "--show-toplevel"]).stdout).unwrap();
    assert_eq!(
        std::fs::canonicalize(top_level.trim()).unwrap(),
        std::fs::canonicalize(&root).unwrap()
    );
    std::fs::remove_dir_all(parent).unwrap();
}

#[test]
fn git_rename_status_and_snapshot_changes_use_the_destination_path() {
    let root = std::env::temp_dir().join(format!("daena-git-rename-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    assert!(run_git(&["init", "-q"]).status.success());
    assert!(run_git(&["config", "user.email", "tests@daena.local"])
        .status
        .success());
    assert!(run_git(&["config", "user.name", "Daena tests"])
        .status
        .success());
    assert!(run_git(&["config", "commit.gpgsign", "false"])
        .status
        .success());
    std::fs::create_dir_all(root.join("assets/files")).unwrap();
    std::fs::write(root.join("assets/files/old.txt"), "rename me\n").unwrap();
    assert!(run_git(&["add", "--all"]).status.success());
    assert!(run_git(&["commit", "-qm", "base"]).status.success());

    assert!(
        run_git(&["mv", "assets/files/old.txt", "assets/files/new.txt"])
            .status
            .success()
    );
    let status = store.git_status().unwrap();
    assert!(status
        .canonical_changes
        .iter()
        .any(|path| path == "assets/files/new.txt"));
    assert!(!status
        .canonical_changes
        .iter()
        .any(|path| path == "assets/files/old.txt"));

    assert!(run_git(&["commit", "-qm", "rename asset"]).status.success());
    let head = store.git_rev_parse("HEAD").unwrap().unwrap();
    let changes = store.git_show_changes(&head).unwrap();
    assert!(changes
        .iter()
        .any(|change| change.status.starts_with('R') && change.path == "assets/files/new.txt"));
    assert!(changes.iter().all(|change| !change.path.contains('\t')));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_commit_rejects_paths_outside_preflight_and_accepts_subset() {
    let root = std::env::temp_dir().join(format!("daena-git-select-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    assert!(run_git(&["init", "-q"]).status.success());
    assert!(run_git(&["config", "user.email", "tests@daena.local"])
        .status
        .success());
    assert!(run_git(&["config", "user.name", "Daena tests"])
        .status
        .success());
    assert!(run_git(&["config", "commit.gpgsign", "false"])
        .status
        .success());
    assert!(run_git(&["add", "--all"]).status.success());
    assert!(run_git(&["commit", "-qm", "base"]).status.success());

    store
        .create_entity(CreateEntity {
            name: "Select entity".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let preview = store.git_staging_preview().unwrap();
    assert!(preview.ready);
    let entity_json = preview
        .staging_paths
        .iter()
        .find(|path| path.ends_with("/entity.json"))
        .cloned()
        .expect("entity.json in staging preview");

    let rejected = store.git_commit("should fail".into(), Some(vec!["README.md".into()]));
    assert!(matches!(rejected, Err(CoreError::Git(_))));

    store
        .git_commit("select entity".into(), Some(vec![entity_json.clone()]))
        .unwrap();
    let after = store.git_staging_preview().unwrap();
    assert!(!after.staging_paths.contains(&entity_json));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_commit_subset_does_not_include_previously_staged_canonical_paths() {
    let root = std::env::temp_dir().join(format!("daena-git-select-staged-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    assert!(run_git(&["init", "-q"]).status.success());
    assert!(run_git(&["config", "user.email", "tests@daena.local"])
        .status
        .success());
    assert!(run_git(&["config", "user.name", "Daena tests"])
        .status
        .success());
    assert!(run_git(&["config", "commit.gpgsign", "false"])
        .status
        .success());
    assert!(run_git(&["add", "--all"]).status.success());
    assert!(run_git(&["commit", "-qm", "base"]).status.success());

    let entity = store
        .create_entity(CreateEntity {
            name: "Select staged entity".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Document body\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let preview = store.git_staging_preview().unwrap();
    let entity_json = format!("entities/{}/entity.json", entity.id);
    let document = format!("entities/{}/document.md", entity.id);
    assert!(preview.staging_paths.contains(&entity_json));
    assert!(preview.staging_paths.contains(&document));

    assert!(run_git(&["add", "--", &document]).status.success());
    store
        .git_commit(
            "select only identity".into(),
            Some(vec![entity_json.clone()]),
        )
        .unwrap();

    let staged = String::from_utf8(run_git(&["diff", "--cached", "--name-only"]).stdout).unwrap();
    assert!(!staged.lines().any(|path| path == document));
    let working_tree = String::from_utf8(run_git(&["status", "--porcelain"]).stdout).unwrap();
    assert!(working_tree.lines().any(|path| path.ends_with(&document)));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_show_tree_filters_to_canonical_paths_and_reset_moves_head() {
    let root = std::env::temp_dir().join(format!("daena-git-reset-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    assert!(run_git(&["init", "-q"]).status.success());
    assert!(run_git(&["config", "user.email", "tests@daena.local"])
        .status
        .success());
    assert!(run_git(&["config", "user.name", "Daena tests"])
        .status
        .success());
    assert!(run_git(&["config", "commit.gpgsign", "false"])
        .status
        .success());
    assert!(run_git(&["add", "--all"]).status.success());
    assert!(run_git(&["commit", "-qm", "base"]).status.success());
    let base = store.git_log().unwrap()[0].hash.clone();

    store
        .create_entity(CreateEntity {
            name: "Later entity".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let preview = store.git_staging_preview().unwrap();
    store
        .git_commit("later".into(), Some(preview.staging_paths.clone()))
        .unwrap();
    let later = store.git_log().unwrap()[0].hash.clone();
    assert_ne!(base, later);

    let tree = store.git_show_tree(&later).unwrap();
    assert!(tree.iter().any(|path| path == "project.json"));
    assert!(tree
        .iter()
        .all(|path| ProjectStore::is_canonical_git_path(path)));
    assert!(!tree.iter().any(|path| path.starts_with(".daena/")));

    let body = store.git_show_file(&later, "project.json").unwrap();
    assert!(
        body.contains("formatVersion") || body.contains("format_version") || body.contains("name")
    );

    let reset = store.git_reset_hard(&base).unwrap();
    assert_eq!(
        reset.current_head.as_deref(),
        store.git_rev_parse("HEAD").unwrap().as_deref()
    );
    assert!(!reset.diverged_from_upstream);
    let entities = store.list_entities().unwrap();
    assert!(entities.iter().all(|entity| entity.name != "Later entity"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_remote_recovery_restores_upstream_and_force_pushes_with_lease() {
    let root = std::env::temp_dir().join(format!("daena-git-recovery-{}", Uuid::new_v4()));
    let remote = std::env::temp_dir().join(format!("daena-git-recovery-remote-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |directory: &std::path::Path, args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(directory)
            .output()
            .unwrap()
    };
    assert!(run_git(&root, &["init", "-q"]).status.success());
    assert!(
        run_git(&root, &["config", "user.email", "tests@daena.local"])
            .status
            .success()
    );
    assert!(run_git(&root, &["config", "user.name", "Daena tests"])
        .status
        .success());
    assert!(run_git(&root, &["config", "commit.gpgsign", "false"])
        .status
        .success());
    assert!(run_git(&root, &["add", "--all"]).status.success());
    assert!(run_git(&root, &["commit", "-qm", "base"]).status.success());
    let branch = store.git_status().unwrap().branch.unwrap();
    let base = store.git_rev_parse("HEAD").unwrap().unwrap();

    std::fs::create_dir_all(&remote).unwrap();
    assert!(run_git(&remote, &["init", "--bare", "-q"]).status.success());
    store
        .git_remote_add("origin", &remote.to_string_lossy())
        .unwrap();
    store.git_push("origin", Some(&branch), false).unwrap();
    let configured_upstream = store.git_upstream().unwrap().unwrap();
    assert_eq!(configured_upstream.remote, "origin");
    assert_eq!(configured_upstream.branch, branch);

    store
        .create_entity(CreateEntity {
            name: "Remote recovery entity".into(),
            entity_type: None,
        })
        .unwrap();
    let preview = store.git_staging_preview().unwrap();
    store
        .git_commit("remote recovery entity".into(), Some(preview.staging_paths))
        .unwrap();
    let later = store.git_rev_parse("HEAD").unwrap().unwrap();
    assert!(run_git(&root, &["push", "-q", "origin", &branch])
        .status
        .success());

    let squashed = store
        .git_super_squash_after_checkpoint("consolidated history")
        .unwrap();
    assert!(squashed.diverged_from_upstream);
    assert_ne!(squashed.current_head.as_deref(), Some(later.as_str()));
    let restored = store.git_restore_from_upstream().unwrap();
    assert_eq!(restored.current_head.as_deref(), Some(later.as_str()));
    assert!(store
        .list_entities()
        .unwrap()
        .iter()
        .any(|entity| entity.name == "Remote recovery entity"));

    let reset = store.git_reset_hard(&base).unwrap();
    assert!(reset.diverged_from_upstream);
    let pushed = store.git_push("origin", Some(&branch), true).unwrap();
    assert_eq!(pushed.branch.as_deref(), Some(branch.as_str()));
    let remote_head = String::from_utf8(run_git(&remote, &["rev-parse", "HEAD"]).stdout)
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(remote_head, base);

    drop(store);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(remote).unwrap();
}

#[test]
fn git_remote_add_list_and_remove_round_trip() {
    let root = std::env::temp_dir().join(format!("daena-git-remote-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap()
    };
    assert!(run_git(&["init", "-q"]).status.success());
    let remotes = store
        .git_remote_add("origin", "https://example.com/daena.git")
        .unwrap();
    assert_eq!(remotes.len(), 1);
    assert_eq!(remotes[0].name, "origin");
    assert_eq!(remotes[0].fetch_url, "https://example.com/daena.git");
    let remotes = store
        .git_remote_set_url("origin", "https://example.com/daena-archive.git")
        .unwrap();
    assert_eq!(
        remotes[0].fetch_url,
        "https://example.com/daena-archive.git"
    );
    let remotes = store.git_remote_remove("origin").unwrap();
    assert!(remotes.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn directory_mutations_return_revisions_and_replay_requests() {
    let root = std::env::temp_dir().join(format!("daena-revision-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let request_id = Uuid::new_v4().to_string();
    let first = store
        .create_entity_with_request(
            CreateEntity {
                name: "Revisioned entity".into(),
                entity_type: Some("place".into()),
            },
            Some(&request_id),
        )
        .unwrap();
    let replay = store
        .create_entity_with_request(
            CreateEntity {
                name: "Revisioned entity".into(),
                entity_type: Some("place".into()),
            },
            Some(&request_id),
        )
        .unwrap();
    assert_eq!(first.id, replay.id);
    assert!(!first.revision.is_empty());
    assert_eq!(store.list_entities().unwrap().len(), 1);

    let conflict = store.update_entity_with_options(
        first.id.clone(),
        Some("Changed concurrently".into()),
        None,
        Some("sha256:stale"),
        Some(&Uuid::new_v4().to_string()),
    );
    assert!(matches!(conflict, Err(CoreError::Conflict(_))));
    let updated = store
        .update_entity_with_options(
            first.id.clone(),
            Some("Changed safely".into()),
            None,
            Some(&first.revision),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    assert_ne!(first.revision, updated.revision);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn asset_file_import_is_committed_with_canonical_metadata() {
    let root = std::env::temp_dir().join(format!("daena-asset-{}", Uuid::new_v4()));
    let source = root.with_extension("source.bin");
    std::fs::write(&source, b"asset bytes").unwrap();
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Asset owner".into(),
            entity_type: None,
        })
        .unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: entity.id.clone(),
            namespace: "core".into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "sample.bin".into(),
            mime_type: "application/octet-stream".into(),
        })
        .unwrap();
    store.flush_checkpoint("test export").unwrap();
    assert_eq!(
        std::fs::read(root.join(&asset.path)).unwrap(),
        b"asset bytes"
    );
    assert_eq!(
        store.list_assets(entity.id).unwrap()[0].revision,
        asset.revision
    );
    drop(store);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn asset_metadata_survives_checkpoint_and_runtime_rebuild() {
    let root = std::env::temp_dir().join(format!("daena-asset-metadata-{}", Uuid::new_v4()));
    let source = root.with_extension("source.png");
    let replacement = root.with_extension("replacement.webp");
    std::fs::write(&source, b"profile bytes").unwrap();
    std::fs::write(&replacement, b"replacement profile bytes").unwrap();
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Profile owner".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: entity.id.clone(),
            namespace: "lore".into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "source.png".into(),
            mime_type: "image/png".into(),
        })
        .unwrap();
    let original_path = asset.path.clone();
    store.flush_checkpoint("initial asset export").unwrap();
    assert!(root.join(&original_path).is_file());
    let updated = store
        .update_asset_metadata_with_request(
            AssetMetadataUpdate {
                asset_id: asset.id.clone(),
                filename: Some("portrait.png".into()),
                role: Some(ASSET_ROLE_PROFILE.into()),
                reference_scope: Some(ASSET_REFERENCE_SCOPE_PROJECT.into()),
            },
            &asset.revision,
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let replace_request_id = Uuid::new_v4().to_string();
    let replace_input = AssetFileReplaceInput {
        asset_id: updated.id.clone(),
        source_path: replacement.to_string_lossy().into_owned(),
        mime_type: "image/webp".into(),
    };
    let replace_expected_revision = updated.revision.clone();
    let updated = store
        .replace_asset_file_with_request(
            replace_input.clone(),
            &replace_expected_revision,
            Some(&replace_request_id),
        )
        .unwrap();
    assert_eq!(updated.filename, "portrait.png");
    assert_eq!(updated.role, ASSET_ROLE_PROFILE);
    assert_eq!(updated.reference_scope, ASSET_REFERENCE_SCOPE_PROJECT);
    std::fs::remove_file(&replacement).unwrap();
    let replay = store
        .replace_asset_file_with_request(
            replace_input.clone(),
            &replace_expected_revision,
            Some(&replace_request_id),
        )
        .unwrap();
    assert_eq!(replay.revision, updated.revision);
    assert!(matches!(
        store.replace_asset_file_with_request(
            replace_input,
            "different-input",
            Some(&replace_request_id),
        ),
        Err(CoreError::Conflict(_))
    ));
    store.flush_checkpoint("asset metadata export").unwrap();

    let canonical: crate::storage::AssetsFile =
        crate::storage::read_json(&root.join("entities").join(&entity.id).join("assets.json"))
            .unwrap();
    assert_eq!(canonical.assets[0].filename, "portrait.png");
    assert_eq!(canonical.assets[0].role, ASSET_ROLE_PROFILE);
    assert_eq!(
        canonical.assets[0].reference_scope,
        ASSET_REFERENCE_SCOPE_PROJECT
    );
    assert_eq!(
        std::fs::read(root.join(&updated.path)).unwrap(),
        b"replacement profile bytes"
    );
    assert!(!root.join(original_path).exists());

    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let reopened = ProjectStore::open_directory(&root).unwrap();
    let rebuilt = reopened.asset(updated.id).unwrap();
    assert_eq!(rebuilt.filename, "portrait.png");
    assert_eq!(rebuilt.role, ASSET_ROLE_PROFILE);
    assert_eq!(rebuilt.reference_scope, ASSET_REFERENCE_SCOPE_PROJECT);
    assert_eq!(rebuilt.path, updated.path);

    drop(reopened);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn checkpoint_export_stages_existing_assets_from_the_transaction_tree() {
    let root = std::env::temp_dir().join(format!("daena-asset-export-{}", Uuid::new_v4()));
    let source = root.with_extension("source.bin");
    std::fs::write(&source, b"asset bytes").unwrap();
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Asset export owner".into(),
            entity_type: None,
        })
        .unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: entity.id,
            namespace: "maps".into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "map.map".into(),
            mime_type: "application/octet-stream".into(),
        })
        .unwrap();

    store.flush_checkpoint("asset export regression").unwrap();
    assert_eq!(
        std::fs::read(root.join(&asset.path)).unwrap(),
        b"asset bytes"
    );
    assert!(store.sync_summary().unwrap().export_error.is_none());

    drop(store);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn disabled_module_survives_directory_reopen() {
    let root = std::env::temp_dir().join(format!("daena-disabled-module-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .set_module_enabled("daena.lore".into(), false)
        .unwrap();
    assert!(!store.is_module_enabled("daena.lore").unwrap());
    drop(store);

    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert!(!reopened.is_module_enabled("daena.lore").unwrap());
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn module_records_are_scoped_revisioned_and_rebuild_from_checkpoint() {
    let root = std::env::temp_dir().join(format!("daena-module-records-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let language = store
        .create_entity(CreateEntity {
            name: "Asteri".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    let other = store
        .create_entity(CreateEntity {
            name: "Other".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    let request_id = Uuid::new_v4().to_string();
    let first = store
        .create_module_record(
            "daena.language",
            "lexemes",
            &language.id,
            serde_json::json!({"lemma": "sol", "meanings": ["sun"]}),
            Some(&request_id),
        )
        .unwrap();
    let retried = store
        .create_module_record(
            "daena.language",
            "lexemes",
            &language.id,
            serde_json::json!({"lemma": "sol", "meanings": ["sun"]}),
            Some(&request_id),
        )
        .unwrap();
    assert_eq!(first.id, retried.id);
    assert!(store
        .create_module_record(
            "daena.language",
            "lexemes",
            &language.id,
            serde_json::json!({"lemma": "sol", "meanings": ["star"]}),
            Some(&request_id),
        )
        .is_err());
    assert!(store
        .list_module_records("daena.language", "lexemes", &other.id, None, 50, 0,)
        .unwrap()
        .is_empty());
    assert!(store
        .update_module_record(
            "daena.language",
            "lexemes",
            &first.id,
            &other.id,
            serde_json::json!({"lemma": "sol", "meanings": ["sun"]}),
            &first.revision,
            Some(&Uuid::new_v4().to_string()),
        )
        .is_err());
    let updated = store
        .update_module_record(
            "daena.language",
            "lexemes",
            &first.id,
            &language.id,
            serde_json::json!({"lemma": "sol", "meanings": ["sun", "day"]}),
            &first.revision,
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    assert_ne!(updated.revision, first.revision);
    assert!(store
        .update_module_record(
            "daena.language",
            "lexemes",
            &first.id,
            &language.id,
            serde_json::json!({"lemma": "sol", "meanings": ["sun"]}),
            &first.revision,
            Some(&Uuid::new_v4().to_string()),
        )
        .is_err());
    store
        .create_module_record(
            "daena.language",
            "lexemes",
            &language.id,
            serde_json::json!({
                "lemma": "sol",
                "meanings": ["soil"],
                "status": "archaic",
                "tags": ["nature"],
                "senses": [{ "id": "s1", "gloss": "soil", "definition": "earth" }]
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let all = store
        .list_module_records("daena.language", "lexemes", &language.id, None, 50, 0)
        .unwrap();
    assert_eq!(
        all.len(),
        2,
        "all records: {:?}",
        all.iter().map(|record| &record.value).collect::<Vec<_>>()
    );
    let by_status = store
        .list_module_records_with(
            "daena.language",
            "lexemes",
            &language.id,
            crate::ModuleRecordListParams {
                status: Some("archaic"),
                limit: 50,
                ..crate::ModuleRecordListParams::default()
            },
        )
        .unwrap();
    assert_eq!(
        by_status.len(),
        1,
        "status filter: {:?}",
        by_status
            .iter()
            .map(|record| &record.value)
            .collect::<Vec<_>>()
    );
    let filtered = store
        .list_module_records_with(
            "daena.language",
            "lexemes",
            &language.id,
            crate::ModuleRecordListParams {
                status: Some("archaic"),
                tag: Some("nature"),
                sort: Some("status"),
                limit: 50,
                ..crate::ModuleRecordListParams::default()
            },
        )
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].value["lemma"], "sol");
    assert!(store
        .list_module_records_with(
            "daena.language",
            "lexemes",
            &language.id,
            crate::ModuleRecordListParams {
                sort: Some("createdAt"),
                limit: 50,
                ..crate::ModuleRecordListParams::default()
            },
        )
        .is_err());
    assert_eq!(
        store
            .list_module_records_with(
                "daena.language",
                "lexemes",
                &language.id,
                crate::ModuleRecordListParams {
                    homonyms_only: true,
                    limit: 50,
                    ..crate::ModuleRecordListParams::default()
                },
            )
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        store
            .list_module_records(
                "daena.language",
                "lexemes",
                &language.id,
                Some("earth"),
                50,
                0,
            )
            .unwrap()
            .len(),
        1
    );
    let disposable = store
        .create_module_record(
            "daena.language",
            "lexemes",
            &language.id,
            serde_json::json!({"lemma": "luna", "meanings": ["moon"]}),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let delete_request_id = Uuid::new_v4().to_string();
    store
        .delete_module_record(
            "daena.language",
            "lexemes",
            &disposable.id,
            &language.id,
            &disposable.revision,
            Some(&delete_request_id),
        )
        .unwrap();
    store
        .delete_module_record(
            "daena.language",
            "lexemes",
            &disposable.id,
            &language.id,
            &disposable.revision,
            Some(&delete_request_id),
        )
        .unwrap();
    assert_eq!(
        store
            .list_module_records(
                "daena.language",
                "lexemes",
                &language.id,
                Some("sol"),
                50,
                0,
            )
            .unwrap()
            .len(),
        2
    );
    store.flush_checkpoint("module-record-test").unwrap();
    drop(store);

    let plugin_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("plugins/daena.language.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(plugin_json["records"].as_array().unwrap().len(), 2);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(
        rebuilt
            .list_module_records("daena.language", "lexemes", &language.id, None, 50, 0,)
            .unwrap()
            .len(),
        2
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn language_phonology_and_orthography_records_round_trip() {
    let root = std::env::temp_dir().join(format!("daena-language-phonology-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let language = store
        .create_entity(CreateEntity {
            name: "Asteri".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "phonemes",
            &language.id,
            serde_json::json!({
                "symbol": "ʒ",
                "kind": "consonant",
                "place": "postalveolar",
                "manner": "fricative"
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "phonemes",
            &language.id,
            serde_json::json!({
                "symbol": "a",
                "kind": "vowel",
                "height": "open",
                "backness": "front"
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "phonology",
            &language.id,
            serde_json::json!({ "syllableStructure": "(C)V(C)", "tone": "none" }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "orthographies",
            &language.id,
            serde_json::json!({
                "name": "High script",
                "mappings": [{ "id": "m1", "grapheme": "zh", "sounds": ["ʒ"] }]
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let phonemes = store
        .list_module_records_with(
            "daena.language",
            "phonemes",
            &language.id,
            crate::ModuleRecordListParams {
                sort: Some("symbol"),
                limit: 50,
                ..crate::ModuleRecordListParams::default()
            },
        )
        .unwrap();
    assert_eq!(
        phonemes
            .iter()
            .map(|record| record.value["symbol"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["a", "ʒ"]
    );
    assert_eq!(
        store
            .list_module_records(
                "daena.language",
                "phonemes",
                &language.id,
                Some("fricative"),
                50,
                0,
            )
            .unwrap()
            .len(),
        1
    );
    store.flush_checkpoint("phonology-test").unwrap();
    drop(store);
    let plugin_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("plugins/daena.language.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(plugin_json["records"].as_array().unwrap().len(), 4);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(
        rebuilt
            .list_module_records("daena.language", "phonemes", &language.id, None, 50, 0)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        rebuilt
            .list_module_records("daena.language", "orthographies", &language.id, None, 50, 0)
            .unwrap()
            .len(),
        1
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn language_grammar_records_round_trip_and_rebuild_from_checkpoint() {
    let root = std::env::temp_dir().join(format!("daena-language-grammar-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let language = store
        .create_entity(CreateEntity {
            name: "Asteri".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    let lexeme = store
        .create_module_record(
            "daena.language",
            "lexemes",
            &language.id,
            serde_json::json!({"lemma": "sol", "meanings": ["sun"]}),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "grammar",
            &language.id,
            serde_json::json!({
                "recordKind": "custom-rule",
                "schemaVersion": 1,
                "title": "Verb stems",
                "tags": ["morphology"],
                "body": "See linked lexeme.",
                "examples": [],
                "links": [{ "id": "l1", "kind": "lexeme", "targetId": lexeme.id, "label": "sol" }]
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "grammar",
            &language.id,
            serde_json::json!({
                "recordKind": "system",
                "schemaVersion": 1,
                "systemId": "syntax.basic-word-order",
                "status": "configured",
                "config": {
                    "order": "svo",
                    "strength": "strict",
                    "influences": []
                },
                "notes": "",
                "examples": [],
                "links": []
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "grammar",
            &language.id,
            serde_json::json!({
                "recordKind": "agreement",
                "schemaVersion": 1,
                "title": "Subject verb",
                "controller": { "kind": "subject" },
                "target": { "kind": "verb" },
                "features": [{ "sourceSystemId": "nouns.number", "categoryId": "plural", "label": "Number" }],
                "behavior": "full",
                "notes": "",
                "examples": [],
                "links": []
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let records = store
        .list_module_records_with(
            "daena.language",
            "grammar",
            &language.id,
            crate::ModuleRecordListParams {
                sort: Some("title"),
                limit: 50,
                ..crate::ModuleRecordListParams::default()
            },
        )
        .unwrap();
    let titles: Vec<&str> = records
        .iter()
        .filter_map(|record| record.value["title"].as_str())
        .collect();
    assert_eq!(titles, vec!["Subject verb", "Verb stems"]);
    assert_eq!(
        store
            .list_module_records(
                "daena.language",
                "grammar",
                &language.id,
                Some("svo"),
                50,
                0,
            )
            .unwrap()
            .len(),
        1
    );
    let other = store
        .create_entity(CreateEntity {
            name: "Other".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    assert!(store
        .list_module_records("daena.language", "grammar", &other.id, None, 50, 0)
        .unwrap()
        .is_empty());
    store.flush_checkpoint("grammar-test").unwrap();
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    let rebuilt_records = rebuilt
        .list_module_records("daena.language", "grammar", &language.id, None, 50, 0)
        .unwrap();
    assert_eq!(rebuilt_records.len(), 3);
    assert!(rebuilt_records.iter().any(|record| {
        record.value["recordKind"] == "system"
            && record.value["systemId"] == "syntax.basic-word-order"
            && record.value["config"]["order"] == "svo"
    }));
    assert!(rebuilt_records
        .iter()
        .any(|record| record.value["recordKind"] == "agreement"));
    assert!(rebuilt_records
        .iter()
        .any(|record| record.value["recordKind"] == "custom-rule"));
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn language_paradigms_round_trip_and_sort_by_name() {
    let root = std::env::temp_dir().join(format!("daena-language-paradigms-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let language = store
        .create_entity(CreateEntity {
            name: "Asteri".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "paradigms",
            &language.id,
            serde_json::json!({
                "name": "Weak verb",
                "kind": "inflection",
                "slots": [{ "id": "s1", "label": "1sg" }],
                "rules": [{
                    "id": "r1",
                    "name": "default",
                    "kind": "inflection",
                    "operations": [{ "id": "o1", "slotId": "s1", "op": "suffix", "value": "o" }]
                }]
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "paradigms",
            &language.id,
            serde_json::json!({
                "name": "Agent noun",
                "kind": "derivation",
                "slots": [{ "id": "s1", "label": "agent" }],
                "rules": [{
                    "id": "r1",
                    "name": "agent",
                    "kind": "derivation",
                    "operations": [{ "id": "o1", "slotId": "s1", "op": "suffix", "value": "er" }]
                }]
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let tables = store
        .list_module_records_with(
            "daena.language",
            "paradigms",
            &language.id,
            crate::ModuleRecordListParams {
                sort: Some("name"),
                limit: 50,
                ..crate::ModuleRecordListParams::default()
            },
        )
        .unwrap();
    assert_eq!(
        tables
            .iter()
            .map(|record| record.value["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["Agent noun", "Weak verb"]
    );
    assert_eq!(
        store
            .list_module_records(
                "daena.language",
                "paradigms",
                &language.id,
                Some("Agent"),
                50,
                0,
            )
            .unwrap()
            .len(),
        1
    );
    let other = store
        .create_entity(CreateEntity {
            name: "Other".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    assert!(store
        .list_module_records("daena.language", "paradigms", &other.id, None, 50, 0)
        .unwrap()
        .is_empty());
    store.flush_checkpoint("paradigm-test").unwrap();
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(
        rebuilt
            .list_module_records("daena.language", "paradigms", &language.id, None, 50, 0)
            .unwrap()
            .len(),
        2
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn language_samples_round_trip_and_sort_by_title() {
    let root = std::env::temp_dir().join(format!("daena-language-samples-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let language = store
        .create_entity(CreateEntity {
            name: "Asteri".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    let lexeme = store
        .create_module_record(
            "daena.language",
            "lexemes",
            &language.id,
            serde_json::json!({"lemma": "sol", "meanings": ["sun"]}),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "samples",
            &language.id,
            serde_json::json!({
                "title": "Sunrise",
                "kind": "sentence",
                "text": "sol oritur",
                "translation": "the sun rises",
                "tokens": [{
                    "id": "t1",
                    "text": "sol",
                    "gloss": "sun",
                    "grammar": "NOM",
                    "lexemeId": lexeme.id
                }]
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "samples",
            &language.id,
            serde_json::json!({
                "title": "Evening",
                "kind": "paragraph",
                "text": "luna lucet.",
                "tokens": []
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let items = store
        .list_module_records_with(
            "daena.language",
            "samples",
            &language.id,
            crate::ModuleRecordListParams {
                sort: Some("title"),
                limit: 50,
                ..crate::ModuleRecordListParams::default()
            },
        )
        .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|record| record.value["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["Evening", "Sunrise"]
    );
    assert_eq!(
        store
            .list_module_records(
                "daena.language",
                "samples",
                &language.id,
                Some("oritur"),
                50,
                0,
            )
            .unwrap()
            .len(),
        1
    );
    let other = store
        .create_entity(CreateEntity {
            name: "Other".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    assert!(store
        .list_module_records("daena.language", "samples", &other.id, None, 50, 0)
        .unwrap()
        .is_empty());
    store.flush_checkpoint("sample-test").unwrap();
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(
        rebuilt
            .list_module_records("daena.language", "samples", &language.id, None, 50, 0)
            .unwrap()
            .len(),
        2
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn lore_schema_overlay_survives_directory_reopen_and_checkpoint() {
    let root = std::env::temp_dir().join(format!("daena-lore-overlay-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let overlay = serde_json::json!({
        "version": 1,
        "disabledTemplates": ["concept"],
        "fieldScopeOverrides": [{ "fieldKey": "aliases", "entityTypes": ["person", "faction"] }],
        "templateOverrides": [{
            "templateId": "person",
            "fields": { "summary": "", "aliases": "", "occupation": "" },
            "requiredFields": ["occupation"]
        }],
        "customEntityTypes": ["species"],
        "customFields": [{
            "key": "lifespan",
            "label": "Lifespan",
            "type": "text",
            "entityTypes": ["species"]
        }],
        "customTemplates": [{
            "id": "species",
            "name": "Species",
            "entityType": "species",
            "fields": { "summary": "", "lifespan": "" }
        }]
    });
    store
        .set_module_schema_overlay("daena.lore".into(), Some(overlay.clone()))
        .unwrap();
    assert_eq!(
        store.module_schema_overlay("daena.lore").unwrap(),
        Some(overlay.clone())
    );
    store.flush_checkpoint("lore-overlay-test").unwrap();
    // Give the export worker a moment to write portable plugin state.
    std::thread::sleep(std::time::Duration::from_millis(200));
    drop(store);

    let plugin_path = root.join("plugins/daena.lore.json");
    let plugin_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&plugin_path).unwrap()).unwrap();
    assert_eq!(plugin_json["schemaOverlay"], overlay);

    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(
        rebuilt.module_schema_overlay("daena.lore").unwrap(),
        Some(overlay)
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn timeline_schema_overlay_survives_directory_reopen_and_checkpoint() {
    let root = std::env::temp_dir().join(format!("daena-timeline-overlay-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let overlay = serde_json::json!({
        "version": 1,
        "disabledFields": ["endsAt"],
        "customFields": [{
            "key": "importance",
            "label": "Importance",
            "type": "number",
            "entityTypes": ["event"]
        }]
    });
    store
        .set_module_schema_overlay("daena.timeline".into(), Some(overlay.clone()))
        .unwrap();
    store.flush_checkpoint("timeline-overlay-test").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    drop(store);

    let plugin_path = root.join("plugins/daena.timeline.json");
    let plugin_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&plugin_path).unwrap()).unwrap();
    assert_eq!(plugin_json["schemaOverlay"], overlay);

    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(
        rebuilt.module_schema_overlay("daena.timeline").unwrap(),
        Some(overlay)
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn file_backed_project_paths_are_rejected() {
    let path = std::env::temp_dir().join(format!("daena-legacy-{}.sqlite", Uuid::new_v4()));
    std::fs::write(&path, b"legacy database placeholder").unwrap();
    let error = match ProjectStore::open(&path) {
        Ok(_) => panic!("file-backed project paths must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("opened from a directory"));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn pre_cut_runtime_database_requires_reset() {
    let root = std::env::temp_dir().join(format!("daena-pre-cut-db-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    drop(store);

    let database = root.join(".daena/index.sqlite");
    std::fs::remove_file(&database).unwrap();
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE project_meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO project_meta(key, value) VALUES ('schema_version', '1');",
        )
        .unwrap();
    drop(connection);

    let error = match ProjectStore::open_directory(&root) {
        Ok(_) => panic!("pre-cut runtime database must be rejected"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("remove .daena"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn search_matches_prefixes() {
    let store = ProjectStore::in_memory().unwrap();
    store
        .create_entity(CreateEntity {
            name: "Amulet".into(),
            entity_type: Some("artifact".into()),
        })
        .unwrap();

    let matches = store.search("Am".into()).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].name, "Amulet");
}

#[test]
fn entity_query_filters_sorts_counts_and_paginates_in_storage() {
    let store = ProjectStore::in_memory().unwrap();
    let alpha = store
        .create_entity(CreateEntity {
            name: "Alpha".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: alpha.id.clone(),
            body: "Keeper of the hidden valley".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store
        .create_entity(CreateEntity {
            name: "Beta".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store
        .create_entity(CreateEntity {
            name: "Gamma".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();

    let first = store
        .query_entities(EntityListQuery {
            entity_types: vec!["person".into()],
            sort_field: Some(EntitySortField::Name),
            limit: Some(1),
            ..EntityListQuery::default()
        })
        .unwrap();
    assert_eq!(first.total, 2);
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.items[0].name, "Alpha");
    assert!(first.has_more);
    assert_eq!(first.type_counts[0].entity_type.as_deref(), Some("person"));
    assert_eq!(first.type_counts[0].count, 2);

    let second = store
        .query_entities(EntityListQuery {
            entity_types: vec!["person".into()],
            sort_field: Some(EntitySortField::Name),
            offset: Some(1),
            limit: Some(1),
            ..EntityListQuery::default()
        })
        .unwrap();
    assert_eq!(second.items[0].name, "Gamma");
    assert!(!second.has_more);

    let searched = store
        .query_entities(EntityListQuery {
            query: Some("hidden val".into()),
            entity_types: vec!["person".into()],
            ..EntityListQuery::default()
        })
        .unwrap();
    assert_eq!(searched.total, 1);
    assert_eq!(searched.items[0].id, alpha.id);

    let excluded = store
        .query_entities(EntityListQuery {
            excluded_entity_types: vec!["person".into()],
            limit: Some(u32::MAX),
            ..EntityListQuery::default()
        })
        .unwrap();
    assert_eq!(excluded.limit, MAX_ENTITY_QUERY_LIMIT);
    assert_eq!(excluded.items.len(), 1);
    assert_eq!(excluded.items[0].name, "Beta");

    assert_eq!(store.get_entity(&alpha.id).unwrap().unwrap().id, alpha.id);
    assert!(store.get_entity("missing").unwrap().is_none());
    assert!(matches!(
        store.query_entities(EntityListQuery {
            query: Some("x".repeat(513)),
            ..EntityListQuery::default()
        }),
        Err(CoreError::Validation(message)) if message.contains("512")
    ));
}

#[test]
fn create_entry_writes_template_content_atomically() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entry(CreateEntry {
            name: "The Ash Court".into(),
            entity_type: Some("faction".into()),
            document: Some(CreateEntryDocument {
                body: "A quiet power.".into(),
                format: Some("plain-text".into()),
            }),
            fields: vec![CreateEntryField {
                namespace: "lore".into(),
                key: "summary".into(),
                value: serde_json::json!("A quiet power."),
            }],
            relationships: vec![],
        })
        .unwrap();
    assert_eq!(
        store.list_documents(entity.id.clone()).unwrap()[0].body,
        "A quiet power."
    );
    assert_eq!(store.list_fields(entity.id).unwrap()[0].key, "summary");

    let result = store.create_entry(CreateEntry {
        name: "Should roll back".into(),
        entity_type: Some("place".into()),
        document: Some(CreateEntryDocument {
            body: "Not persisted".into(),
            format: None,
        }),
        fields: vec![
            CreateEntryField {
                namespace: "lore".into(),
                key: "summary".into(),
                value: serde_json::json!("first"),
            },
            CreateEntryField {
                namespace: "lore".into(),
                key: "summary".into(),
                value: serde_json::json!("duplicate"),
            },
        ],
        relationships: vec![],
    });
    assert!(result.is_err());
    assert_eq!(store.list_entities().unwrap().len(), 1);
}

#[test]
fn create_entry_writes_multiple_relationships_atomically() {
    let store = ProjectStore::in_memory().unwrap();
    let first_leader = store
        .create_entity(CreateEntity {
            name: "First leader".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let second_leader = store
        .create_entity(CreateEntity {
            name: "Second leader".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let faction = store
        .create_entry(CreateEntry {
            name: "The Twin Council".into(),
            entity_type: Some("faction".into()),
            document: None,
            fields: vec![],
            relationships: vec![CreateEntryRelationship {
                relationship_type: "led_by".into(),
                target_ids: vec![first_leader.id.clone(), second_leader.id.clone()],
            }],
        })
        .unwrap();
    assert_eq!(
        store.list_relationships(faction.id.clone()).unwrap().len(),
        2
    );

    let relationship = store.list_relationships(faction.id).unwrap().remove(0);
    store.delete_relationship(relationship.id).unwrap();
    let remaining_relationships = store.list_relationships(first_leader.id).unwrap().len()
        + store.list_relationships(second_leader.id).unwrap().len();
    assert_eq!(remaining_relationships, 1);

    let result = store.create_entry(CreateEntry {
        name: "Should roll back".into(),
        entity_type: Some("faction".into()),
        document: None,
        fields: vec![],
        relationships: vec![CreateEntryRelationship {
            relationship_type: "led_by".into(),
            target_ids: vec!["missing".into()],
        }],
    });
    assert!(result.is_err());
    assert_eq!(store.list_entities().unwrap().len(), 3);
}

#[test]
fn export_round_trip_preserves_entities_and_documents() {
    let source = ProjectStore::in_memory().unwrap();
    let entity = source
        .create_entity(CreateEntity {
            name: "Ash Court".into(),
            entity_type: Some("faction".into()),
        })
        .unwrap();
    source
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "A quiet power.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let target = ProjectStore::in_memory().unwrap();
    let imported = target
        .import_json_with_mode_and_sync_with_request(
            &source.export_json().unwrap(),
            false,
            true,
            None,
        )
        .unwrap();
    assert_eq!(imported, 1);
    assert_eq!(target.list_entities().unwrap()[0].name, "Ash Court");
    assert_eq!(
        target.list_documents(entity.id).unwrap()[0].body,
        "A quiet power."
    );
}

#[test]
fn importing_the_same_snapshot_twice_preserves_children() {
    let source = ProjectStore::in_memory().unwrap();
    let entity = source
        .create_entity(CreateEntity {
            name: "Repeated import".into(),
            entity_type: None,
        })
        .unwrap();
    source
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Content".into(),
            format: Some("plain-text".into()),
        })
        .unwrap();
    let payload = source.export_json().unwrap();
    let target = ProjectStore::in_memory().unwrap();
    target
        .import_json_with_mode_and_sync_with_request(&payload, false, true, None)
        .unwrap();
    target
        .import_json_with_mode_and_sync_with_request(&payload, false, true, None)
        .unwrap();
    assert_eq!(target.list_entities().unwrap().len(), 1);
    assert_eq!(target.list_documents(entity.id).unwrap().len(), 1);
}

#[test]
fn updates_canonical_document_and_preserves_namespaced_fields() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Harbor".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "First".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Second".into(),
            format: Some("plain-text".into()),
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: "lore".into(),
            key: "summary".into(),
            value: serde_json::json!("A port"),
            revision: String::new(),
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: "timeline".into(),
            key: "startsAt".into(),
            value: serde_json::json!("0010-01-01"),
            revision: String::new(),
        })
        .unwrap();
    assert_eq!(store.list_documents(entity.id.clone()).unwrap().len(), 1);
    assert_eq!(
        store.list_documents(entity.id.clone()).unwrap()[0].body,
        "Second"
    );
    assert_eq!(store.list_fields(entity.id).unwrap().len(), 2);
}

#[test]
fn saving_identical_document_content_preserves_revision() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Stable revision".into(),
            entity_type: Some("manuscript".into()),
        })
        .unwrap();
    let document = SaveDocument {
        entity_id: entity.id.clone(),
        body: "The same content.".into(),
        format: Some("markdown".into()),
    };

    store.save_document(document.clone()).unwrap();
    let first_revision = store.list_documents(entity.id.clone()).unwrap()[0]
        .revision
        .clone();
    store.save_document(document).unwrap();
    let second_revision = store.list_documents(entity.id).unwrap()[0].revision.clone();

    assert_eq!(first_revision, second_revision);
}

#[test]
fn empty_document_revision_allows_first_document_save() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "First document".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();

    store
        .save_document_with_options(
            SaveDocument {
                entity_id: entity.id.clone(),
                body: "Initial notes".into(),
                format: Some("markdown".into()),
            },
            Some(""),
            None,
        )
        .unwrap();

    assert_eq!(
        store.list_documents(entity.id).unwrap()[0].body,
        "Initial notes"
    );
}

#[test]
fn opening_and_updating_rebuilds_search_for_documents_and_fields() {
    let path = std::env::temp_dir().join(format!("daena-search-test-{}", Uuid::new_v4()));
    {
        let store = ProjectStore::open_directory(&path).unwrap();
        let entity = store
            .create_entity(CreateEntity {
                name: "Search target".into(),
                entity_type: Some("place".into()),
            })
            .unwrap();
        store
            .save_document(SaveDocument {
                entity_id: entity.id.clone(),
                body: "old prose".into(),
                format: Some("markdown".into()),
            })
            .unwrap();
        store
            .set_field(FieldValue {
                entity_id: entity.id,
                namespace: "lore".into(),
                key: "summary".into(),
                value: serde_json::json!("old field"),
                revision: String::new(),
            })
            .unwrap();
        store
            .connection
            .execute("DELETE FROM world_search", [])
            .unwrap();
    }

    let store = ProjectStore::open_directory(&path).unwrap();
    let entity = store.search("old prose".into()).unwrap();
    assert_eq!(entity.len(), 1);
    let field_match = store.search("old field".into()).unwrap();
    assert_eq!(field_match.len(), 1);

    let entity_id = entity[0].id.clone();
    store
        .save_document(SaveDocument {
            entity_id: entity_id.clone(),
            body: "new prose".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    assert!(store.search("old prose".into()).unwrap().is_empty());
    assert_eq!(store.search("new prose".into()).unwrap().len(), 1);

    store
        .set_field(FieldValue {
            entity_id,
            namespace: "lore".into(),
            key: "summary".into(),
            value: serde_json::json!("new field"),
            revision: String::new(),
        })
        .unwrap();
    assert!(store.search("old field".into()).unwrap().is_empty());
    assert_eq!(store.search("new field".into()).unwrap().len(), 1);

    drop(store);
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn valid_runtime_open_skips_full_portable_scan() {
    let root = std::env::temp_dir().join(format!("daena-fast-open-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Fast open owner".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_checkpoint("test export").unwrap();
    drop(store);

    std::fs::write(
        root.join("entities").join(entity.id).join("entity.json"),
        "{not valid json",
    )
    .unwrap();
    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(reopened.list_entities().unwrap().len(), 1);
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn passage_search_preserves_ranked_source_identity() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Passage target".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "The silver harbor keeps the oldest bell.".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let passages = store.search_passages("silver harbor".into(), 8).unwrap();
    assert_eq!(passages.len(), 1);
    assert_eq!(passages[0].entity_id, entity.id);
    assert!(passages[0].source_path.ends_with("/document.md"));
    assert!(passages[0].content.contains("oldest bell"));
    assert!(passages[0].lexical_rank.is_finite());
}

#[test]
fn typed_fields_round_trip_as_json_values() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Typed fields".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: "test".into(),
            key: "count".into(),
            value: serde_json::json!(42),
            revision: String::new(),
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: "test".into(),
            key: "published".into(),
            value: serde_json::json!(true),
            revision: String::new(),
        })
        .unwrap();
    let fields = store.list_fields(entity.id).unwrap();
    assert!(fields
        .iter()
        .any(|field| field.value == serde_json::json!(42)));
    assert!(fields
        .iter()
        .any(|field| field.value == serde_json::json!(true)));
}

#[test]
fn save_entry_rejects_invalid_fields_before_writing_document() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Atomic entry".into(),
            entity_type: None,
        })
        .unwrap();
    let result = store.save_entry(SaveEntry {
        document: SaveDocument {
            entity_id: entity.id.clone(),
            body: "Should not persist".into(),
            format: Some("plain-text".into()),
        },
        fields: vec![FieldValue {
            entity_id: "different-entity".into(),
            namespace: "test".into(),
            key: "value".into(),
            value: serde_json::json!("invalid"),
            revision: String::new(),
        }],
    });
    assert!(result.is_err());
    assert!(store.list_documents(entity.id).unwrap().is_empty());
}

#[test]
fn rename_updates_search_and_relationships_require_live_entities() {
    let store = ProjectStore::in_memory().unwrap();
    let source = store
        .create_entity(CreateEntity {
            name: "Old Name".into(),
            entity_type: None,
        })
        .unwrap();
    let target = store
        .create_entity(CreateEntity {
            name: "Target".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .update_entity(source.id.clone(), Some("New Name".into()), None)
        .unwrap();
    assert!(store.search("Old Name".into()).unwrap().is_empty());
    assert_eq!(store.search("New Name".into()).unwrap()[0].id, source.id);
    store.delete_entity(target.id.clone()).unwrap();
    assert!(store
        .create_relationship(RelationshipInput {
            source_id: source.id,
            target_id: target.id,
            relationship_type: "points_to".into(),
            metadata: None
        })
        .is_err());
}

#[test]
fn assets_and_module_state_survive_export_import() {
    let source = ProjectStore::in_memory().unwrap();
    let entity = source
        .create_entity(CreateEntity {
            name: "Map Room".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let asset = source
        .register_asset(AssetInput {
            entity_id: entity.id.clone(),
            namespace: "lore".into(),
            filename: "map.png".into(),
            content_hash: "abc123".into(),
            size: 42,
            mime_type: "image/png".into(),
            path: "map.png".into(),
        })
        .unwrap();
    source
        .set_module_enabled("daena.lore".into(), false)
        .unwrap();
    source
        .set_module_package_version("daena.lore", Some("1.2.0"))
        .unwrap();
    let target = ProjectStore::in_memory().unwrap();
    target
        .import_json_with_mode_and_sync_with_request(
            &source.export_json().unwrap(),
            false,
            true,
            None,
        )
        .unwrap();
    assert_eq!(target.list_assets(entity.id).unwrap()[0].id, asset.id);
    assert!(!target.is_module_enabled("daena.lore").unwrap());
    assert_eq!(
        target
            .module_package_version("daena.lore")
            .unwrap()
            .as_deref(),
        Some("1.2.0")
    );
}

#[test]
fn seed_example_is_repeatable_after_modules_are_initialized() {
    let mut store = ProjectStore::in_memory().unwrap();
    store.set_module_enabled("daena.lore".into(), true).unwrap();
    store
        .set_module_enabled("daena.timeline".into(), true)
        .unwrap();
    let record_owner = store
        .create_entity(CreateEntity {
            name: "Record owner before seed".into(),
            entity_type: Some("language".into()),
        })
        .unwrap();
    store
        .create_module_record(
            "daena.language",
            "lexemes",
            &record_owner.id,
            serde_json::json!({"lemma": "seed"}),
            None,
        )
        .unwrap();

    assert_eq!(store.seed_example().unwrap(), 25);
    assert_eq!(store.seed_example().unwrap(), 25);
    let entities = store.list_entities().unwrap();
    assert_eq!(entities.len(), 25);
    assert_eq!(
        entities
            .iter()
            .map(|entity| store.list_relationships(entity.id.clone()).unwrap().len())
            .sum::<usize>(),
        38
    );
    assert_eq!(store.search("Highland Culture".into()).unwrap().len(), 1);
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.name == "Frostgate Pass")
            .count(),
        1
    );
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.entity_type.as_deref() == Some(crate::maps::MAP_ENTITY_TYPE))
            .count(),
        0
    );
}

#[test]
fn seed_example_survives_reopen() {
    let root = std::env::temp_dir().join(format!("daena-seed-example-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();

    assert_eq!(store.seed_example().unwrap(), 25);
    let entities = store.list_entities().unwrap();
    assert_eq!(entities.len(), 25);
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.entity_type.as_deref() == Some(crate::maps::MAP_ENTITY_TYPE))
            .count(),
        0
    );

    drop(store);
    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(reopened.list_entities().unwrap().len(), 25);
    assert_eq!(
        reopened
            .list_entities()
            .unwrap()
            .iter()
            .filter(|entity| entity.entity_type.as_deref() == Some(crate::maps::MAP_ENTITY_TYPE))
            .count(),
        0
    );
    drop(reopened);

    let mut again = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(again.seed_example().unwrap(), 25);
    assert_eq!(again.list_entities().unwrap().len(), 25);
    drop(again);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn markdown_export_uses_flat_named_files_and_relative_relationship_links() {
    let root = std::env::temp_dir().join(format!("daena-markdown-export-{}", Uuid::new_v4()));
    let destination =
        std::env::temp_dir().join(format!("daena-markdown-destination-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    store.seed_example().unwrap();

    let eldermere = store
        .list_entities()
        .unwrap()
        .into_iter()
        .find(|entity| entity.name == "Eldermere")
        .unwrap();
    let lord_ashford = store
        .list_entities()
        .unwrap()
        .into_iter()
        .find(|entity| entity.name == "Lord Ashford")
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: eldermere.id.clone(),
            body: format!(
                "The court follows [[Lord Ashford]]({}). The record also names [Lord Ashford](daena://entity/{}).",
                lord_ashford.id, lord_ashford.id
            ),
            format: Some("markdown".into()),
        })
        .unwrap();

    let export = store.export_markdown_to(&destination).unwrap();
    let export = Path::new(&export);
    let eldermere_markdown = std::fs::read_to_string(export.join("Eldermere.md")).unwrap();
    assert!(eldermere_markdown.contains("[Lord Ashford](Lord%20Ashford.md)"));
    assert!(eldermere_markdown.contains("## Relationships"));
    assert!(!eldermere_markdown.contains("[[Lord Ashford]]"));
    assert!(export.join("Lord Ashford.md").is_file());
    assert!(!export.join("entities").exists());

    drop(store);
    std::fs::remove_dir_all(&root).unwrap();
    std::fs::remove_dir_all(&destination).unwrap();
}

#[test]
fn markdown_export_prefixes_colliding_entity_names() {
    let destination =
        std::env::temp_dir().join(format!("daena-markdown-collision-{}", Uuid::new_v4()));
    let store = ProjectStore::in_memory().unwrap();
    let first = store
        .create_entity(CreateEntity {
            name: "Twin".into(),
            entity_type: None,
        })
        .unwrap();
    let second = store
        .create_entity(CreateEntity {
            name: "Twin".into(),
            entity_type: None,
        })
        .unwrap();

    let export = store.export_markdown_to(&destination).unwrap();
    let export = Path::new(&export);
    assert!(export.join(format!("Twin-{}.md", &first.id[..8])).is_file());
    assert!(export
        .join(format!("Twin-{}.md", &second.id[..8]))
        .is_file());
    std::fs::remove_dir_all(&destination).unwrap();
}

#[test]
fn wiki_page_export_is_manifest_aware_standalone_and_safe() {
    let destination =
        std::env::temp_dir().join(format!("daena-wiki-page-export-{}", Uuid::new_v4()));
    let store = ProjectStore::in_memory().unwrap();
    let person = store
        .create_entity(CreateEntity {
            name: "Ada / Archivist".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "The Glass Library".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: person.id.clone(),
            body: format!(
                "## Early life\n\nVisited [[The Glass Library]]({}).\n\n<script>alert('no')</script>",
                place.id
            ),
            format: Some("markdown".into()),
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: person.id.clone(),
            namespace: "lore".into(),
            key: "occupation".into(),
            value: serde_json::json!("Royal archivist"),
            revision: String::new(),
        })
        .unwrap();
    store
        .create_relationship(RelationshipInput {
            source_id: person.id.clone(),
            target_id: place.id.clone(),
            relationship_type: "originates_from".into(),
            metadata: None,
        })
        .unwrap();
    let manifest: daena_plugin_api::PluginManifest = serde_json::from_str(include_str!(
        "../../../../packages/modules/lore/manifest.json"
    ))
    .unwrap();

    let markdown_path = store
        .export_wiki_page_to(
            &person.id,
            &destination,
            WikiPageExportFormat::Markdown,
            &manifest,
        )
        .unwrap();
    let markdown = std::fs::read_to_string(markdown_path).unwrap();
    assert!(markdown.contains("# Ada / Archivist"));
    assert!(markdown.contains("| Occupation | Royal archivist |"));
    assert!(markdown.contains("Visited The Glass Library."));
    assert!(markdown.contains("**Origin:** The Glass Library"));
    assert!(!markdown.contains("[[The Glass Library]]"));

    let html_path = store
        .export_wiki_page_to(
            &person.id,
            &destination,
            WikiPageExportFormat::Html,
            &manifest,
        )
        .unwrap();
    let html = std::fs::read_to_string(html_path).unwrap();
    assert!(html.contains("<!doctype html>"));
    assert!(html.contains("<h2>Early life</h2>"));
    assert!(html.contains("&lt;script&gt;alert(&#39;no&#39;)&lt;/script&gt;"));
    assert!(!html.contains("<script>"));
    assert!(!html.contains("https://"));

    std::fs::remove_dir_all(&destination).unwrap();
}

#[test]
fn restore_replaces_records_missing_from_the_backup() {
    let source = ProjectStore::in_memory().unwrap();
    source
        .create_entity(CreateEntity {
            name: "From backup".into(),
            entity_type: None,
        })
        .unwrap();
    let path = std::env::temp_dir().join(format!("daena-restore-test-{}.json", Uuid::new_v4()));
    std::fs::write(&path, source.export_json().unwrap()).unwrap();

    let mut target = ProjectStore::in_memory().unwrap();
    target
        .create_entity(CreateEntity {
            name: "Stale record".into(),
            entity_type: None,
        })
        .unwrap();
    target.restore(path.to_string_lossy().into_owned()).unwrap();

    assert_eq!(target.list_entities().unwrap().len(), 1);
    assert_eq!(target.list_entities().unwrap()[0].name, "From backup");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn applying_migration_creates_backup_and_records_version() {
    let mut store = ProjectStore::in_memory().unwrap();
    let migration = crate::migrations::Migration {
        id: "timeline-v1".into(),
        module_id: "daena.timeline".into(),
        from: 0,
        to: 1,
        operations: vec![crate::migrations::Operation::CreateNamespace {
            namespace: "timeline".into(),
        }],
        recovery: "backup".into(),
        package_digest: "sha256:test-package".into(),
    };
    store.apply_migration(&migration).unwrap();
    assert_eq!(store.get_module_version("daena.timeline").unwrap(), 1);
    let snapshot: serde_json::Value = serde_json::from_str(&store.export_json().unwrap()).unwrap();
    let history = &snapshot["migration_history"][0];
    assert_eq!(history["package_digest"], "sha256:test-package");
    assert!(history["applied_at"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
}

#[test]
fn plugin_backup_restores_schema_and_migration_history() {
    let directory = std::env::temp_dir().join(format!("daena-plugin-backup-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&directory).unwrap();
    // Runtime/plugin recovery must not promote a full portable scan to its
    // normal backup path. An unrelated malformed external file is therefore
    // allowed to remain diagnostic-only while the DB snapshot is backed up.
    std::fs::create_dir_all(directory.join("entities/external-draft")).unwrap();
    std::fs::write(
        directory.join("entities/external-draft/entity.json"),
        b"{ malformed",
    )
    .unwrap();
    let backup = store
        .create_plugin_backup("daena.timeline", Some("0.1.0"), Some("0.2.0"), 0)
        .unwrap();
    assert_eq!(
        store
            .latest_plugin_backup("daena.timeline", Some("0.1.0"), Some("0.2.0"),)
            .unwrap()
            .unwrap()
            .id,
        backup.id
    );
    let migration = crate::migrations::Migration {
        id: "timeline-v1".into(),
        module_id: "daena.timeline".into(),
        from: 0,
        to: 1,
        operations: vec![crate::migrations::Operation::CreateNamespace {
            namespace: "timeline".into(),
        }],
        recovery: "backup".into(),
        package_digest: String::new(),
    };
    store.apply_migration(&migration).unwrap();
    std::fs::remove_dir_all(directory.join("entities/external-draft")).unwrap();
    store.restore_plugin_backup(&backup).unwrap();
    assert_eq!(store.get_module_version("daena.timeline").unwrap(), 0);
    store.apply_migration(&migration).unwrap();
    assert_eq!(store.get_module_version("daena.timeline").unwrap(), 1);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn plugin_data_deletion_requires_confirmation_and_keeps_backup() {
    let mut store = ProjectStore::in_memory().unwrap();
    let migration = crate::migrations::Migration {
        id: "lore-v1".into(),
        module_id: "daena.lore".into(),
        from: 0,
        to: 1,
        operations: vec![crate::migrations::Operation::CreateNamespace {
            namespace: "lore".into(),
        }],
        recovery: "backup".into(),
        package_digest: String::new(),
    };
    store.apply_migration(&migration).unwrap();
    assert!(store.delete_plugin_data("daena.lore", "no").is_err());
    let backup = store
        .delete_plugin_data("daena.lore", "daena.lore")
        .unwrap();
    assert!(std::path::Path::new(&backup).is_file());
    assert_eq!(store.get_module_version("daena.lore").unwrap(), 0);
    std::fs::remove_file(backup).unwrap();
}

#[test]
fn migration_chain_failure_restores_the_pre_chain_state() {
    let mut store = ProjectStore::in_memory().unwrap();
    let first = crate::migrations::Migration {
        id: "lore-v1".into(),
        module_id: "daena.lore".into(),
        from: 0,
        to: 1,
        operations: vec![crate::migrations::Operation::CreateNamespace {
            namespace: "lore".into(),
        }],
        recovery: "backup".into(),
        package_digest: String::new(),
    };
    let second = crate::migrations::Migration {
        id: "lore-v2".into(),
        module_id: "daena.lore".into(),
        from: 1,
        to: 2,
        operations: vec![crate::migrations::Operation::AddField {
            namespace: "missing".into(),
            field: crate::migrations::FieldDefinition {
                key: "summary".into(),
                field_type: "text".into(),
                required: false,
            },
        }],
        recovery: "backup".into(),
        package_digest: String::new(),
    };
    assert!(store.apply_migrations(&[first, second]).is_err());
    assert_eq!(store.get_module_version("daena.lore").unwrap(), 0);
}

#[test]
fn directory_projects_create_portable_layout() {
    let root = std::env::temp_dir().join(format!("daena-project-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(store.info().unwrap().root, root.to_string_lossy());
    assert!(root.join("project.json").is_file());
    assert!(root.join(".daena/index.sqlite").is_file());
    assert!(root.join("entities").is_dir());
    assert!(root.join("plugins").is_dir());
    let manifest =
        crate::storage::read_json::<crate::storage::ProjectManifest>(&root.join("project.json"))
            .unwrap();
    assert_eq!(manifest.format_version, 3);
    assert_eq!(manifest.name, root.file_name().unwrap().to_string_lossy());
    assert_eq!(
        std::fs::read_to_string(root.join(".gitignore")).unwrap(),
        ".daena/\ncheckpoint.json\n"
    );
    assert!(root.join("assets/images").is_dir());
    assert!(root.join("assets/videos").is_dir());
    assert!(root.join("assets/maps").is_dir());
    assert!(root.join("assets/files").is_dir());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ai_enabled_defaults_to_false_and_round_trips() {
    let root = std::env::temp_dir().join(format!("daena-ai-flag-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    assert!(!store.info().unwrap().ai_enabled);
    let manifest =
        crate::storage::read_json::<crate::storage::ProjectManifest>(&root.join("project.json"))
            .unwrap();
    assert!(!manifest.ai_enabled);
    drop(store);

    let store = ProjectStore::open_directory(&root).unwrap();
    let info = store.set_ai_enabled(true).unwrap();
    assert!(info.ai_enabled);
    // Canonical file carries the flag and survives a reopen (fresh .daena state).
    let manifest =
        crate::storage::read_json::<crate::storage::ProjectManifest>(&root.join("project.json"))
            .unwrap();
    assert!(manifest.ai_enabled);
    drop(store);

    let store = ProjectStore::open_directory(&root).unwrap();
    assert!(store.info().unwrap().ai_enabled);
    let info = store.set_ai_enabled(false).unwrap();
    assert!(!info.ai_enabled);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn manifest_without_ai_flag_reads_as_disabled() {
    let root = std::env::temp_dir().join(format!("daena-ai-legacy-{}", Uuid::new_v4()));
    ProjectStore::open_directory(&root).unwrap();
    // Simulate a canonical file written before the field existed.
    let path = root.join("project.json");
    let manifest = crate::storage::read_json::<crate::storage::ProjectManifest>(&path).unwrap();
    let without_flag = serde_json::json!({
        "formatVersion": manifest.format_version,
        "id": manifest.id,
        "name": manifest.name,
        "createdAt": manifest.created_at,
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&without_flag).unwrap()).unwrap();
    let parsed = crate::storage::read_json::<crate::storage::ProjectManifest>(&path).unwrap();
    parsed.validate(&path).unwrap();
    assert!(!parsed.ai_enabled);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn directory_assets_are_copied_and_hashed() {
    let root = std::env::temp_dir().join(format!("daena-project-{}", Uuid::new_v4()));
    let source = std::env::temp_dir().join(format!("daena-asset-{}.txt", Uuid::new_v4()));
    std::fs::write(&source, b"asset contents").unwrap();
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Asset owner".into(),
            entity_type: None,
        })
        .unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: entity.id,
            namespace: "lore".into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "notes.txt".into(),
            mime_type: "text/plain".into(),
        })
        .unwrap();
    store.flush_checkpoint("test export").unwrap();
    assert_eq!(
        asset.content_hash,
        "sha256:f64ec9687efc98edc9ed69b2024bb23bcee2ba0a4e52b64ac3ab204f818716d4"
    );
    assert!(asset.path.starts_with("assets/files/"));
    assert!(root.join(&asset.path).is_file());
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(reopened.list_assets(asset.entity_id).unwrap().len(), 1);
    drop(reopened);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn canonical_files_survive_disposable_index_deletion() {
    let root = std::env::temp_dir().join(format!("daena-canonical-{}", Uuid::new_v4()));
    let mut first = ProjectStore::open_directory(&root).unwrap();
    let source = first
        .create_entity(CreateEntity {
            name: "Source".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let target = first
        .create_entity(CreateEntity {
            name: "Target".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    first
        .apply_migration(&crate::migrations::Migration {
            id: "notes-v1".into(),
            module_id: "com.example.notes".into(),
            from: 0,
            to: 1,
            operations: vec![crate::migrations::Operation::CreateNamespace {
                namespace: "notes".into(),
            }],
            recovery: "backup".into(),
            package_digest: "sha256:test".into(),
        })
        .unwrap();
    first
        .save_document(SaveDocument {
            entity_id: source.id.clone(),
            body: "# Canonical prose".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    first
        .set_field(FieldValue {
            entity_id: source.id.clone(),
            namespace: "notes".into(),
            key: "summary".into(),
            value: serde_json::json!("stored in files"),
            revision: String::new(),
        })
        .unwrap();
    first
        .create_relationship(RelationshipInput {
            source_id: source.id.clone(),
            target_id: target.id,
            relationship_type: "located-in".into(),
            metadata: None,
        })
        .unwrap();
    first.flush_checkpoint("test export").unwrap();
    assert!(root
        .join("entities")
        .join(&source.id)
        .join("entity.json")
        .is_file());
    assert!(root
        .join("entities")
        .join(&source.id)
        .join("document.md")
        .is_file());
    assert!(root.join("plugins/com.example.notes.json").is_file());
    first.flush_checkpoint("test export").unwrap();
    let canonical_before = canonical_files(&root);
    let search_before = first
        .search("Canonical prose".into())
        .unwrap()
        .into_iter()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    let checkpoint: crate::storage::CheckpointManifest =
        crate::storage::read_json(&root.join(crate::storage::CHECKPOINT_MANIFEST_FILE)).unwrap();
    assert!(checkpoint
        .files
        .iter()
        .any(|file| { file.path == format!("entities/{}/document.md", source.id) }));
    drop(first);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();

    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(canonical_files(&root), canonical_before);
    let entities = reopened.list_entities().unwrap();
    assert_eq!(entities.len(), 2);
    assert_eq!(
        reopened.list_documents(source.id.clone()).unwrap()[0].body,
        "# Canonical prose\n"
    );
    assert_eq!(reopened.list_fields(source.id.clone()).unwrap().len(), 1);
    assert_eq!(
        reopened
            .list_relationships(source.id.clone())
            .unwrap()
            .len(),
        1
    );
    let search_after = reopened
        .search("Canonical prose".into())
        .unwrap()
        .into_iter()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    assert_eq!(search_after, search_before);
    assert!(reopened
        .connection
        .query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='source_files')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .unwrap());
    assert!(!root.join(".daena/index.sqlite.next").exists());
    let document_path = root.join("entities").join(&source.id).join("document.md");
    std::fs::write(&document_path, b"# External change\n").unwrap();
    assert!(reopened.search("Canonical prose".into()).is_ok());
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rebuild_initializes_clean_checkpoint_metadata() {
    let root = std::env::temp_dir().join(format!("daena-rebuild-clean-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let sync = store.sync_summary().unwrap();
    assert_eq!(sync.state, "clean");
    assert_eq!(sync.dirty_count, 0);
    assert!(root
        .join(crate::storage::CHECKPOINT_MANIFEST_FILE)
        .is_file());
    let generations: (i64, i64) = store
        .connection
        .query_row(
            "SELECT content_generation,exported_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(generations, (0, 0));
    let journal_mode: String = store
        .connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode, "wal");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_failure_is_persisted_until_successful_barrier() {
    let root = std::env::temp_dir().join(format!("daena-export-error-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Failure probe".into(),
            entity_type: None,
        })
        .unwrap();
    let project_json = std::fs::read(root.join("project.json")).unwrap();
    std::fs::remove_file(root.join("project.json")).unwrap();
    assert!(store.flush_checkpoint("forced export failure").is_err());
    assert!(store.sync_summary().unwrap().export_error.is_some());
    std::fs::write(root.join("project.json"), project_json).unwrap();
    store.flush_checkpoint("recovery export").unwrap();
    assert!(store.sync_summary().unwrap().export_error.is_none());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reopening_a_dirty_runtime_wakes_checkpoint_export() {
    let root = std::env::temp_dir().join(format!("daena-export-reopen-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .connection
        .execute(
            "UPDATE runtime_meta SET content_generation=content_generation+1, exported_generation=0 WHERE key='runtime'",
            [],
        )
        .unwrap();
    drop(store);

    let reopened = ProjectStore::open_directory(&root).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let state: (i64, i64) = reopened
            .connection
            .query_row(
                "SELECT content_generation,exported_generation FROM runtime_meta WHERE key='runtime'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        if state.0 == state.1 {
            let checkpoint: crate::storage::CheckpointManifest =
                crate::storage::read_json(&root.join(crate::storage::CHECKPOINT_MANIFEST_FILE))
                    .unwrap();
            assert_eq!(checkpoint.content_generation, state.0);
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "reopened exporter did not converge"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn checkpoint_import_uses_new_epoch_and_rejects_dirty_runtime() {
    let root = std::env::temp_dir().join(format!("daena-import-epoch-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    let epoch: String = store
        .connection
        .query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    store
        .create_entity(CreateEntity {
            name: "Dirty import probe".into(),
            entity_type: None,
        })
        .unwrap();
    assert!(matches!(
        store.import_checkpoint(),
        Err(CoreError::Conflict(_))
    ));
    store.flush_checkpoint("prepare import").unwrap();
    store.import_checkpoint().unwrap();
    let new_epoch: String = store
        .connection
        .query_row(
            "SELECT database_epoch FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_ne!(epoch, new_epoch);
    let journal_mode: String = store
        .connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(journal_mode, "wal");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn create_map_creates_descriptor_with_null_source_until_first_save() {
    let root = std::env::temp_dir().join(format!("daena-create-map-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let map = store.create_map("New map".into()).unwrap();
    assert_eq!(
        map.entity_type.as_deref(),
        Some(crate::maps::MAP_ENTITY_TYPE)
    );
    assert!(
        store.list_assets(map.id.clone()).unwrap().is_empty(),
        "a fresh map has no source asset until its first save"
    );
    let field = store
        .list_fields(map.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "map")
        .unwrap();
    assert_eq!(field.value["sourceAssetId"], serde_json::Value::Null);
    let locations = serde_json::json!({
        "schemaVersion": 1,
        "locations": []
    });
    assert!(
        store
            .set_field(FieldValue {
                entity_id: map.id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: locations,
                revision: String::new(),
            })
            .is_ok(),
        "map metadata must be writable before the first save"
    );

    let source_path = std::env::temp_dir().join(format!("daena-map-{}.map", Uuid::new_v4()));
    std::fs::write(&source_path, b"fresh map source").unwrap();
    let asset = store
        .register_asset_file_with_request(
            AssetFileInput {
                entity_id: map.id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                source_path: source_path.to_string_lossy().into_owned(),
                filename: "map.map".into(),
                mime_type: "application/x-fmg-map".into(),
            },
            None,
        )
        .unwrap();
    assert!(asset.size > 0);
    store
        .set_field(FieldValue {
            entity_id: map.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "map".into(),
            value: serde_json::json!({
                "schemaVersion": 1,
                "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                "sourceAssetId": asset.id,
                "previewAssetId": null,
                "defaultView": {"center": [0.5, 0.5], "zoom": 1}
            }),
            revision: String::new(),
        })
        .unwrap();
    drop(store);
    std::fs::remove_file(&source_path).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn map_entities_and_locations_survive_disposable_index_rebuild() {
    let root = std::env::temp_dir().join(format!("daena-maps-canonical-{}", Uuid::new_v4()));
    let source_a = std::env::temp_dir().join(format!("daena-map-a-{}.map", Uuid::new_v4()));
    let source_b = std::env::temp_dir().join(format!("daena-map-b-{}.map", Uuid::new_v4()));
    std::fs::write(&source_a, b"map-a-source").unwrap();
    std::fs::write(&source_b, b"map-b-source").unwrap();

    let store = ProjectStore::open_directory(&root).unwrap();
    let map_a = store
        .create_entity(CreateEntity {
            name: "World map".into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
        })
        .unwrap();
    let map_b = store
        .create_entity(CreateEntity {
            name: "Regional map".into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
        })
        .unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Old Harbor".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let asset_a = store
        .register_asset_file(AssetFileInput {
            entity_id: map_a.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            source_path: source_a.to_string_lossy().into_owned(),
            filename: "world.map".into(),
            mime_type: "application/x-fmg-map".into(),
        })
        .unwrap();
    let asset_b = store
        .register_asset_file(AssetFileInput {
            entity_id: map_b.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            source_path: source_b.to_string_lossy().into_owned(),
            filename: "regional.map".into(),
            mime_type: "application/x-fmg-map".into(),
        })
        .unwrap();

    for (map, asset) in [(&map_a, &asset_a), (&map_b, &asset_b)] {
        store
                .set_field(FieldValue {
                    entity_id: map.id.clone(),
                    namespace: crate::maps::MAP_NAMESPACE.into(),
                    key: "map".into(),
                    value: serde_json::json!({
                        "schemaVersion": 1,
                        "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                        "sourceAssetId": asset.id,
                        "previewAssetId": null,
                        "defaultView": {"center": [0.5, 0.5], "zoom": 1}
                    }),
                    revision: String::new(),
                })
                .unwrap();
    }

    let layer_id = Uuid::new_v4().to_string();
    store
        .set_field(FieldValue {
            entity_id: map_a.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "layers".into(),
            value: serde_json::json!({
                "schemaVersion": 1,
                "layers": [{
                    "id": layer_id,
                    "name": "Settlements",
                    "order": 0,
                    "defaultVisible": true,
                    "style": {"color": "#334155"},
                    "selector": {"roles": ["birthplace"]}
                }]
            }),
            revision: String::new(),
        })
        .unwrap();

    store
        .create_relationship(RelationshipInput {
            source_id: place.id.clone(),
            target_id: map_b.id.clone(),
            relationship_type: crate::maps::DETAIL_MAP_RELATIONSHIP.into(),
            metadata: None,
        })
        .unwrap();

    let location_a = Uuid::new_v4().to_string();
    let location_b = Uuid::new_v4().to_string();
    let location_c = Uuid::new_v4().to_string();
    store
            .set_field(FieldValue {
                entity_id: place.id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({
                    "schemaVersion": 1,
                    "locations": [
                        {
                            "id": location_a,
                            "mapEntityId": map_a.id,
                            "role": "birthplace",
                            "label": "Old Harbor",
                            "anchor": {"kind": "provider-feature", "provider": "azgaar-fmg", "featureKind": "burg", "featureId": "42", "fallbackPoint": [0.613, 0.428]},
                            "validity": {"from": null, "to": null}
                        },
                        {
                            "id": location_b,
                            "mapEntityId": map_b.id,
                            "role": "trade-port",
                            "label": "Regional harbor",
                            "anchor": {"kind": "point", "point": [0.2, 0.8]},
                            "validity": {"from": null, "to": null}
                        },
                        {
                            "id": location_c,
                            "mapEntityId": map_a.id,
                            "role": "route",
                            "label": "Coast road",
                            "anchor": {"kind": "path", "points": [[0.1, 0.2], [0.3, 0.4]]},
                            "validity": {"from": null, "to": null}
                        }
                    ]
                }),
                revision: String::new(),
            })
            .unwrap();

    store.flush_checkpoint("test export").unwrap();
    let canonical_before = canonical_files(&root);
    let projection_before = store.map_locations_for_entity(place.id.clone()).unwrap();
    assert_eq!(projection_before.len(), 3);
    assert!(projection_before
        .iter()
        .all(|location| location["resolution"] == "resolved"));
    let anchor_kinds = projection_before
        .iter()
        .map(|location| location["anchorKind"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        anchor_kinds,
        BTreeSet::from(["path", "point", "provider-feature"])
    );
    let search_before = store
        .search("World map".into())
        .unwrap()
        .into_iter()
        .map(|entity| entity.id)
        .collect::<BTreeSet<_>>();
    assert!(search_before.contains(&map_a.id));
    let relationships_before = store
        .list_relationships(place.id.clone())
        .unwrap()
        .into_iter()
        .filter(|relationship| {
            relationship.relationship_type == crate::maps::DETAIL_MAP_RELATIONSHIP
        })
        .map(|relationship| {
            (
                relationship.source_id,
                relationship.target_id,
                relationship.relationship_type,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(relationships_before.len(), 1);
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();

    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(canonical_files(&root), canonical_before);
    assert_eq!(
        rebuilt
            .list_entities()
            .unwrap()
            .into_iter()
            .filter(|entity| entity.entity_type.as_deref() == Some(crate::maps::MAP_ENTITY_TYPE))
            .count(),
        2
    );
    assert_eq!(
        rebuilt.map_locations_for_entity(place.id.clone()).unwrap(),
        projection_before
    );
    let layers = rebuilt
        .list_fields(map_a.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.namespace == crate::maps::MAP_NAMESPACE && field.key == "layers")
        .expect("layers field");
    assert_eq!(layers.value["layers"][0]["id"], layer_id);
    let search_after = rebuilt
        .search("World map".into())
        .unwrap()
        .into_iter()
        .map(|entity| entity.id)
        .collect::<BTreeSet<_>>();
    assert_eq!(search_after, search_before);
    let relationships_after = rebuilt
        .list_relationships(place.id)
        .unwrap()
        .into_iter()
        .filter(|relationship| {
            relationship.relationship_type == crate::maps::DETAIL_MAP_RELATIONSHIP
        })
        .map(|relationship| {
            (
                relationship.source_id,
                relationship.target_id,
                relationship.relationship_type,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(relationships_after, relationships_before);
    drop(rebuilt);
    std::fs::remove_file(source_a).unwrap();
    std::fs::remove_file(source_b).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn map_locations_reject_dangling_maps_and_invalid_geometry() {
    let store = ProjectStore::in_memory().unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Unbound place".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let dangling = store.set_field(FieldValue {
        entity_id: place.id.clone(),
        namespace: crate::maps::MAP_NAMESPACE.into(),
        key: "locations".into(),
        value: serde_json::json!({
            "schemaVersion": 1,
            "locations": [{
                "id": Uuid::new_v4(),
                "mapEntityId": Uuid::new_v4(),
                "role": "origin",
                "label": "Nowhere",
                "anchor": {"kind": "point", "point": [0.5, 0.5]},
                "validity": {"from": null, "to": null}
            }]
        }),
        revision: String::new(),
    });
    assert!(dangling
        .unwrap_err()
        .to_string()
        .contains("maps: dangling map reference"));

    let map = store.create_map("Bound map".into()).unwrap();
    let malformed = store.set_field(FieldValue {
        entity_id: place.id,
        namespace: crate::maps::MAP_NAMESPACE.into(),
        key: "locations".into(),
        value: serde_json::json!({
            "schemaVersion": 1,
            "locations": [{
                "id": Uuid::new_v4(),
                "mapEntityId": map.id,
                "role": "origin",
                "label": "Out of bounds",
                "anchor": {"kind": "point", "point": [1.5, 0.5]},
                "validity": {"from": null, "to": null}
            }]
        }),
        revision: String::new(),
    });
    assert!(malformed
        .unwrap_err()
        .to_string()
        .contains("maps: invalid geometry:"));
}

#[test]
fn map_layers_round_trip_and_reject_non_map_owners() {
    let store = ProjectStore::in_memory().unwrap();
    let map = store.create_map("Layered map".into()).unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Not a map".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let layers = serde_json::json!({
        "schemaVersion": 1,
        "layers": [{
            "id": Uuid::new_v4(),
            "name": "Culture",
            "order": 1,
            "defaultVisible": false,
            "style": {},
            "selector": {"roles": ["place"]}
        }]
    });
    store
        .set_field(FieldValue {
            entity_id: map.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "layers".into(),
            value: layers.clone(),
            revision: String::new(),
        })
        .unwrap();
    let stored = store
        .list_fields(map.id)
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap();
    assert_eq!(stored.value, layers);
    let rejected = store.set_field(FieldValue {
        entity_id: place.id,
        namespace: crate::maps::MAP_NAMESPACE.into(),
        key: "layers".into(),
        value: layers,
        revision: String::new(),
    });
    assert!(rejected
        .unwrap_err()
        .to_string()
        .contains("maps: layers belong only on a map entity"));
}

#[test]
fn map_projection_refresh_matches_full_rebuild_after_location_upsert() {
    let store = ProjectStore::in_memory().unwrap();
    let map = store.create_map("Incremental map".into()).unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Incremental place".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let location_id = Uuid::new_v4().to_string();
    store
        .upsert_map_location(
            place.id.clone(),
            crate::maps::LocationReference {
                id: location_id.clone(),
                map_entity_id: map.id.clone(),
                role: "landmark".into(),
                label: "Tower".into(),
                anchor: crate::maps::Anchor::Point {
                    point: crate::maps::Point(0.25, 0.75),
                },
                validity: crate::maps::Validity {
                    from: None,
                    to: None,
                },
            },
            None,
        )
        .unwrap();
    let incremental = store.map_locations_for_entity(place.id.clone()).unwrap();
    assert_eq!(incremental.len(), 1);
    assert_eq!(incremental[0]["id"], location_id);
    store.reconcile_map_links(map.id).unwrap();
    assert_eq!(
        store.map_locations_for_entity(place.id).unwrap(),
        incremental
    );
}

#[test]
fn transaction_request_ids_must_be_uuids_but_may_be_absent() {
    let root = std::env::temp_dir().join(format!("daena-map-rid-{}", Uuid::new_v4()));
    let source = std::env::temp_dir().join(format!("daena-map-rid-src-{}.map", Uuid::new_v4()));
    std::fs::write(&source, br#"{"features": []}"#).unwrap();

    let store = ProjectStore::open_directory(&root).unwrap();
    let map = store.create_map("Rid map".into()).unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Rid place".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: map.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "world.map".into(),
            mime_type: "application/x-fmg-map".into(),
        })
        .unwrap();
    let map_id = map.id.clone();
    let place_id = place.id.clone();
    let asset_id = asset.id.clone();
    let revision = asset.revision.clone();

    // Correlation tokens like the FMG bridge's 'maps-fmg-N' are not
    // UUIDs: the core transaction layer rejects them outright. The host
    // sanitizes such ids to None before reaching the core (see
    // sanitize_mutation_request_id in src-tauri), and None must be
    // accepted here with a generated UUID receipt.
    let bytes = br#"{"features": [{"kind": "burg", "id": "3", "x": 1, "y": 1}]}"#;
    let rejected = store.replace_asset_bytes_with_request(
        AssetReplaceInput {
            asset_id: asset_id.clone(),
            content_hash: format!("sha256:{}", digest_bytes(bytes)),
            size: bytes.len() as i64,
            mime_type: "application/x-fmg-map".into(),
        },
        bytes.to_vec(),
        &revision,
        Some("maps-fmg-1"),
    );
    assert!(rejected.is_err());
    assert!(rejected
        .unwrap_err()
        .to_string()
        .contains("transaction request ID must be a UUID"));
    let accepted = store.replace_asset_bytes_with_request(
        AssetReplaceInput {
            asset_id: asset_id.clone(),
            content_hash: format!("sha256:{}", digest_bytes(bytes)),
            size: bytes.len() as i64,
            mime_type: "application/x-fmg-map".into(),
        },
        bytes.to_vec(),
        &revision,
        None,
    );
    assert!(accepted.is_ok(), "{accepted:?}");
    store
        .set_field_with_request(
            FieldValue {
                entity_id: place_id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({
                    "schemaVersion": 1,
                    "locations": [{
                        "id": Uuid::new_v4(),
                        "mapEntityId": map_id,
                        "role": "origin",
                        "label": "Rid place",
                        "anchor": {"kind": "point", "point": [0.5, 0.5]},
                        "validity": {"from": null, "to": null}
                    }]
                }),
                revision: String::new(),
            },
            None,
        )
        .expect("absent request ids must be accepted");
    assert_eq!(store.map_locations(place_id).unwrap().len(), 1);

    drop(store);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn map_recovery_copies_are_canonical_listed_newest_first_and_restored() {
    let root = std::env::temp_dir().join(format!("daena-map-recovery-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let map = store.create_map("Recovered map".into()).unwrap();
    let source = std::env::temp_dir().join(format!("daena-map-source-{}.map", Uuid::new_v4()));
    std::fs::write(&source, b"original-source").unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: map.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "map.map".into(),
            mime_type: "application/x-fmg-map".into(),
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: map.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "map".into(),
            value: serde_json::json!({
                "schemaVersion": 1,
                "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                "sourceAssetId": asset.id,
                "previewAssetId": null,
                "defaultView": {"center": [0.5, 0.5], "zoom": 1}
            }),
            revision: String::new(),
        })
        .unwrap();
    store.flush_checkpoint("test export").unwrap();
    std::fs::remove_file(&source).unwrap();
    let before = store.list_map_recovery_copies(&map.id).unwrap();
    assert!(before.is_empty());
    let first_path = store.save_map_recovery_copy(&map.id, b"draft-v1").unwrap();
    let second_path = store.save_map_recovery_copy(&map.id, b"draft-v2").unwrap();
    assert!(first_path.starts_with(".daena/conflicts/maps/") && first_path.ends_with(".map"));
    assert!(second_path.starts_with(".daena/conflicts/maps/") && second_path.ends_with(".map"));
    assert_eq!(std::fs::read(root.join(&second_path)).unwrap(), b"draft-v2");

    let copies = store.list_map_recovery_copies(&map.id).unwrap();
    assert_eq!(copies.len(), 2);
    assert!(copies
        .iter()
        .any(|copy| copy.file_name == first_path.rsplit('/').next().unwrap()));
    assert!(copies
        .iter()
        .any(|copy| copy.file_name == second_path.rsplit('/').next().unwrap()));
    assert!(copies
        .iter()
        .all(|copy| copy.path.starts_with(".daena/conflicts/maps/")));
    assert!(copies
        .iter()
        .all(|copy| copy.created_at.chars().all(|c| c.is_ascii_digit())));
    assert!(copies[0].created_at >= copies[1].created_at);

    let expected_bytes = std::fs::read(root.join(&copies[0].path)).unwrap();
    let restored = store
        .restore_map_recovery_copy(&map.id, &copies[0].file_name, None)
        .unwrap();
    store.flush_checkpoint("test export").unwrap();
    assert_eq!(
        std::fs::read(root.join(&restored.path)).unwrap(),
        expected_bytes
    );
    let asset = store.list_assets(map.id).unwrap().pop().unwrap();
    assert_eq!(asset.size as usize, expected_bytes.len());
    assert_eq!(
        asset.content_hash,
        format!("sha256:{}", digest_bytes(&expected_bytes))
    );
    assert_eq!(asset.mime_type, "application/x-fmg-map");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn map_recovery_copies_require_map_entities_and_reject_traversal() {
    let store = ProjectStore::in_memory().unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Not a map".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    assert!(store.save_map_recovery_copy(&place.id, b"x").is_err());
    assert!(store.list_map_recovery_copies(&place.id).is_err());
    assert!(store
        .restore_map_recovery_copy(&place.id, "../escape.map", None)
        .is_err());
    let map = store.create_map("Traversal map".into()).unwrap();
    assert!(store
        .restore_map_recovery_copy(&map.id, "../escape.map", None)
        .is_err());
    assert!(store
        .restore_map_recovery_copy(
            &map.id,
            "other-entity-00000000-0000-0000-0000-000000000000.map",
            None
        )
        .is_err());
    assert!(store
        .restore_map_recovery_copy(&map.id, "missing.map", None)
        .is_err());
}

#[test]
fn fresh_git_clone_rebuilds_its_ignored_index() {
    let root = std::env::temp_dir().join(format!("daena-git-clone-source-{}", Uuid::new_v4()));
    let clone = std::env::temp_dir().join(format!("daena-git-clone-copy-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Cloned canonical entry".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    drop(store);

    let run_git = |cwd: &Path, args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap()
    };
    assert!(run_git(&root, &["init", "-q"]).status.success());
    assert!(
        run_git(&root, &["config", "user.email", "tests@daena.local"])
            .status
            .success()
    );
    assert!(run_git(&root, &["config", "user.name", "Daena tests"])
        .status
        .success());
    assert!(run_git(&root, &["config", "commit.gpgsign", "false"])
        .status
        .success());
    assert!(run_git(&root, &["add", "--all"]).status.success());
    assert!(run_git(&root, &["commit", "-qm", "canonical project"])
        .status
        .success());
    let clone_output = Command::new("git")
        .args([
            "clone",
            "--quiet",
            root.to_str().unwrap(),
            clone.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(clone_output.status.success());
    assert!(!clone.join(".daena").exists());

    let reopened = ProjectStore::open_directory(&clone).unwrap();
    assert_eq!(reopened.list_entities().unwrap()[0].id, entity.id);
    drop(reopened);
    std::fs::remove_dir_all(clone.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&clone).unwrap();
    assert_eq!(
        rebuilt.list_entities().unwrap()[0].name,
        "Cloned canonical entry"
    );
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(clone).unwrap();
}

#[test]
fn create_and_save_entry_enforce_map_field_validation() {
    let store = ProjectStore::in_memory().unwrap();
    let invalid_field = CreateEntryField {
        namespace: crate::maps::MAP_NAMESPACE.into(),
        key: "map".into(),
        value: serde_json::json!({"schemaVersion": 99}),
    };
    let err = store.create_entry_with_request(
        CreateEntry {
            name: "Bad map entity".into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
            fields: vec![invalid_field.clone()],
            document: None,
            relationships: vec![],
        },
        None,
    );
    assert!(err.is_err());

    let map = store.create_map("Valid Map".into()).unwrap();
    let save_err = store.save_entry_with_options(
        SaveEntry {
            document: SaveDocument {
                entity_id: map.id.clone(),
                format: None,
                body: String::new(),
            },
            fields: vec![FieldValue {
                entity_id: map.id,
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "map".into(),
                value: serde_json::json!({"schemaVersion": 99}),
                revision: String::new(),
            }],
        },
        None,
        None,
    );
    assert!(save_err.is_err());
}

#[test]
fn entry_batch_is_atomic_idempotent_and_rejects_request_input_reuse() {
    let root = std::env::temp_dir().join(format!("daena-entry-batch-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let map = store.create_map("Materialization map".into()).unwrap();
    let map_id = map.id.clone();
    let request_id = "00000000-0000-4000-8000-000000000042";
    let input = CreateEntry {
        name: "Earthquake · year 12 · M 5.100".into(),
        entity_type: Some(crate::maps::PHYSICAL_EVENT_ENTITY_TYPE.into()),
        document: Some(CreateEntryDocument {
            body: "# Earthquake".into(),
            format: Some("markdown".into()),
        }),
        fields: vec![
            CreateEntryField {
                namespace: crate::maps::PHYSICAL_EVENT_NAMESPACE.into(),
                key: "provenance".into(),
                value: serde_json::json!({"prediction": false}),
            },
            CreateEntryField {
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: crate::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY.into(),
                value: serde_json::json!({
                    "contractVersion": 1,
                    "kind": "physical-offset-years",
                    "reference": "accepted-source",
                    "startOffsetYears": 12,
                    "endOffsetYears": 12
                }),
            },
            CreateEntryField {
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({
                    "schemaVersion": 1,
                    "locations": [{
                        "id": "00000000-0000-4000-8000-000000000043",
                        "mapEntityId": map.id,
                        "role": "physical-event",
                        "label": "Earthquake",
                        "anchor": {"kind": "point", "point": [0.5, 0.5]},
                        "validity": {"from": null, "to": null}
                    }]
                }),
            },
        ],
        relationships: vec![CreateEntryRelationship {
            relationship_type: crate::maps::PHYSICAL_EVENT_ON_MAP_RELATIONSHIP.into(),
            target_ids: vec![map.id.clone()],
        }],
    };
    let first = store
        .create_entries_with_request(vec![input.clone()], Some(request_id))
        .unwrap();
    let replayed = store
        .create_entries_with_request(vec![input], Some(request_id))
        .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(replayed.len(), 1);
    assert_eq!(first[0].id, replayed[0].id);
    assert_eq!(first[0].revision, replayed[0].revision);
    assert_eq!(store.map_locations(first[0].id.clone()).unwrap().len(), 1);
    assert_eq!(
        store.map_location_projection(map_id.clone()).unwrap().len(),
        1
    );
    assert_eq!(store.list_entities().unwrap().len(), 2);

    let conflict = store.create_entries_with_request(
        vec![CreateEntry {
            name: "Different input".into(),
            entity_type: Some(crate::maps::PHYSICAL_EVENT_ENTITY_TYPE.into()),
            document: None,
            fields: vec![],
            relationships: vec![CreateEntryRelationship {
                relationship_type: crate::maps::PHYSICAL_EVENT_ON_MAP_RELATIONSHIP.into(),
                target_ids: vec![map.id.clone()],
            }],
        }],
        Some(request_id),
    );
    assert!(
        matches!(conflict, Err(CoreError::Conflict(message)) if message.contains("different inputs"))
    );
    assert_eq!(store.list_entities().unwrap().len(), 2);
    let invalid = store.create_entries_with_request(
        vec![CreateEntry {
            name: "Invalid chronology".into(),
            entity_type: Some(crate::maps::PHYSICAL_EVENT_ENTITY_TYPE.into()),
            document: None,
            fields: vec![CreateEntryField {
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: crate::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY.into(),
                value: serde_json::json!({
                    "contractVersion": 1,
                    "kind": "physical-offset-years",
                    "reference": "accepted-source",
                    "startOffsetYears": 2,
                    "endOffsetYears": 1
                }),
            }],
            relationships: vec![],
        }],
        Some("00000000-0000-4000-8000-000000000044"),
    );
    assert!(
        matches!(invalid, Err(CoreError::Validation(message)) if message.contains("cannot be after"))
    );
    assert_eq!(store.list_entities().unwrap().len(), 2);
    let out_of_bounds = store.create_entries_with_request(
        vec![CreateEntry {
            name: "Out of bounds chronology".into(),
            entity_type: Some(crate::maps::PHYSICAL_EVENT_ENTITY_TYPE.into()),
            document: None,
            fields: vec![CreateEntryField {
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: crate::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY.into(),
                value: serde_json::json!({
                    "contractVersion": 1,
                    "kind": "physical-offset-years",
                    "reference": "accepted-source",
                    "startOffsetYears": 100_001,
                    "endOffsetYears": 100_001
                }),
            }],
            relationships: vec![],
        }],
        Some("00000000-0000-4000-8000-000000000045"),
    );
    assert!(
        matches!(out_of_bounds, Err(CoreError::Validation(message)) if message.contains("within +/-100000"))
    );
    assert_eq!(store.list_entities().unwrap().len(), 2);
    store
        .flush_checkpoint("persist materialized event")
        .unwrap();
    drop(store);
    std::fs::remove_file(root.join(".daena/index.sqlite")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(rebuilt.list_entities().unwrap().len(), 2);
    assert_eq!(rebuilt.map_locations(first[0].id.clone()).unwrap().len(), 1);
    assert_eq!(rebuilt.map_location_projection(map_id).unwrap().len(), 1);
    assert_eq!(
        rebuilt
            .list_relationships(first[0].id.clone())
            .unwrap()
            .len(),
        1
    );
    let chronology = rebuilt
        .list_fields(first[0].id.clone())
        .unwrap()
        .into_iter()
        .find(|field| {
            field.namespace == crate::maps::MAP_NAMESPACE
                && field.key == crate::maps::PHYSICAL_EVENT_CHRONOLOGY_KEY
        })
        .unwrap();
    assert_eq!(chronology.value["startOffsetYears"], 12);
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn feature_resolution_returns_unresolved_when_json_asset_lacks_features_key() {
    let root = std::env::temp_dir().join(format!("daena-map-no-feat-{}", Uuid::new_v4()));
    let source = std::env::temp_dir().join(format!("daena-map-no-feat-src-{}.map", Uuid::new_v4()));
    std::fs::write(&source, br#"{"info": "no features key here"}"#).unwrap();

    let store = ProjectStore::open_directory(&root).unwrap();
    let map = store.create_map("Map without features".into()).unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Place".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: map.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "nofeat.map".into(),
            mime_type: "application/x-fmg-map".into(),
        })
        .unwrap();

    store
        .set_field(FieldValue {
            entity_id: map.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "map".into(),
            value: serde_json::json!({
                "schemaVersion": 1,
                "provider": {"id": "azgaar-fmg", "adapterVersion": 1, "sourceFormat": "fmg-map"},
                "sourceAssetId": asset.id,
                "previewAssetId": null,
                "defaultView": {"center": [0.5, 0.5], "zoom": 1}
            }),
            revision: String::new(),
        })
        .unwrap();

    let loc_id = Uuid::new_v4().to_string();
    store.set_field(FieldValue {
            entity_id: place.id,
            namespace: crate::maps::MAP_NAMESPACE.into(),
            key: "locations".into(),
            value: serde_json::json!({
                "schemaVersion": 1,
                "locations": [{
                    "id": loc_id,
                    "mapEntityId": map.id,
                    "role": "origin",
                    "label": "Test",
                    "anchor": {"kind": "provider-feature", "provider": "azgaar-fmg", "featureKind": "burg", "featureId": "1", "fallbackPoint": [0.5, 0.5]},
                    "validity": {"from": null, "to": null}
                }]
            }),
            revision: String::new(),
        }).unwrap();

    store.flush_checkpoint("test export").unwrap();
    let projection = store.map_location_projection(map.id).unwrap();
    assert_eq!(projection[0]["resolution"], "unresolved");

    drop(store);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn asset_replacement_rejects_wrong_hash_size_and_revision() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Map source".into(),
            entity_type: Some("daena.maps:map".into()),
        })
        .unwrap();
    let asset = store
        .register_asset(AssetInput {
            entity_id: entity.id.clone(),
            namespace: crate::maps::MAP_NAMESPACE.into(),
            filename: "world.map".into(),
            content_hash: "sha256:old".into(),
            size: 3,
            mime_type: "application/octet-stream".into(),
            path: "assets/maps/world.map".into(),
        })
        .unwrap();
    let correct_hash = format!("sha256:{:x}", Sha256::digest(b"new"));
    assert!(store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: asset.id.clone(),
                content_hash: "sha256:wrong".into(),
                size: 3,
                mime_type: "application/octet-stream".into(),
            },
            b"new".to_vec(),
            &asset.revision,
            None,
        )
        .is_err());
    assert!(store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: asset.id.clone(),
                content_hash: correct_hash.clone(),
                size: 4,
                mime_type: "application/octet-stream".into(),
            },
            b"new".to_vec(),
            &asset.revision,
            None,
        )
        .is_err());
    let replaced = store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: asset.id.clone(),
                content_hash: correct_hash,
                size: 3,
                mime_type: "application/octet-stream".into(),
            },
            b"new".to_vec(),
            &asset.revision,
            None,
        )
        .unwrap();
    assert_ne!(replaced.revision, asset.revision);
    assert!(store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: asset.id,
                content_hash: replaced.content_hash,
                size: 3,
                mime_type: replaced.mime_type,
            },
            b"new".to_vec(),
            "stale-revision",
            None,
        )
        .is_err());
}

#[test]
fn asset_metadata_rename_profile_scope_and_replacement_are_consistent() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Portrait subject".into(),
            entity_type: Some("person".into()),
        })
        .unwrap();
    let first = store
        .register_asset(AssetInput {
            entity_id: entity.id.clone(),
            namespace: "lore".into(),
            filename: "first.png".into(),
            content_hash: "sha256:first".into(),
            size: 1,
            mime_type: "image/png".into(),
            path: "assets/images/original-first.png".into(),
        })
        .unwrap();
    let second = store
        .register_asset(AssetInput {
            entity_id: entity.id.clone(),
            namespace: "lore".into(),
            filename: "second.webp".into(),
            content_hash: "sha256:second".into(),
            size: 1,
            mime_type: "image/webp".into(),
            path: "assets/images/original-second.webp".into(),
        })
        .unwrap();
    let other_namespace = store
        .register_asset(AssetInput {
            entity_id: entity.id.clone(),
            namespace: "writing".into(),
            filename: "manuscript-cover.png".into(),
            content_hash: "sha256:writing".into(),
            size: 1,
            mime_type: "image/png".into(),
            path: "assets/images/manuscript-cover.png".into(),
        })
        .unwrap();

    assert_eq!(first.role, ASSET_ROLE_ATTACHMENT);
    assert_eq!(first.reference_scope, ASSET_REFERENCE_SCOPE_ENTITY);
    let first_profile = store
        .update_asset_metadata_with_request(
            AssetMetadataUpdate {
                asset_id: first.id.clone(),
                filename: Some("portrait.png".into()),
                role: Some(ASSET_ROLE_PROFILE.into()),
                reference_scope: Some(ASSET_REFERENCE_SCOPE_PROJECT.into()),
            },
            &first.revision,
            None,
        )
        .unwrap();
    assert_eq!(first_profile.filename, "portrait.png");
    assert_eq!(
        first_profile.path,
        format!("assets/images/{}-portrait.png", first.id)
    );
    assert_eq!(first_profile.content_hash, first.content_hash);
    assert_eq!(first_profile.role, ASSET_ROLE_PROFILE);
    assert_eq!(first_profile.reference_scope, ASSET_REFERENCE_SCOPE_PROJECT);
    assert!(store
        .update_asset_metadata_with_request(
            AssetMetadataUpdate {
                asset_id: first.id.clone(),
                filename: None,
                role: Some(ASSET_ROLE_ATTACHMENT.into()),
                reference_scope: None,
            },
            &first.revision,
            None,
        )
        .is_err());

    store
        .update_asset_metadata_with_request(
            AssetMetadataUpdate {
                asset_id: other_namespace.id.clone(),
                filename: None,
                role: Some(ASSET_ROLE_PROFILE.into()),
                reference_scope: None,
            },
            &other_namespace.revision,
            None,
        )
        .unwrap();

    let second_profile = store
        .update_asset_metadata_with_request(
            AssetMetadataUpdate {
                asset_id: second.id.clone(),
                filename: None,
                role: Some(ASSET_ROLE_PROFILE.into()),
                reference_scope: None,
            },
            &second.revision,
            None,
        )
        .unwrap();
    let assets = store.list_assets(entity.id).unwrap();
    assert_eq!(
        assets
            .iter()
            .filter(|asset| asset.namespace == "lore" && asset.role == ASSET_ROLE_PROFILE)
            .count(),
        1
    );
    assert_eq!(
        assets
            .iter()
            .filter(|asset| asset.namespace == "writing" && asset.role == ASSET_ROLE_PROFILE)
            .count(),
        1
    );
    assert_eq!(
        assets
            .iter()
            .find(|asset| asset.id == first.id)
            .unwrap()
            .role,
        ASSET_ROLE_ATTACHMENT
    );

    let replacement = b"replacement".to_vec();
    let replacement_hash = format!("sha256:{:x}", Sha256::digest(&replacement));
    let replaced = store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: second_profile.id,
                content_hash: replacement_hash,
                size: replacement.len() as i64,
                mime_type: "image/jpeg".into(),
            },
            replacement,
            &second_profile.revision,
            None,
        )
        .unwrap();
    assert_eq!(replaced.role, ASSET_ROLE_PROFILE);
    assert_eq!(replaced.reference_scope, ASSET_REFERENCE_SCOPE_ENTITY);
    assert_eq!(replaced.filename, second.filename);
    assert_eq!(replaced.path, second.path);
}

#[test]
fn asset_metadata_rejects_invalid_values_and_non_image_profiles() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Attachment owner".into(),
            entity_type: None,
        })
        .unwrap();
    let asset = store
        .register_asset(AssetInput {
            entity_id: entity.id,
            namespace: "lore".into(),
            filename: "notes.txt".into(),
            content_hash: "sha256:notes".into(),
            size: 5,
            mime_type: "text/plain".into(),
            path: "assets/files/notes.txt".into(),
        })
        .unwrap();

    for update in [
        AssetMetadataUpdate {
            asset_id: asset.id.clone(),
            filename: Some("../notes.txt".into()),
            role: None,
            reference_scope: None,
        },
        AssetMetadataUpdate {
            asset_id: asset.id.clone(),
            filename: None,
            role: Some("cover".into()),
            reference_scope: None,
        },
        AssetMetadataUpdate {
            asset_id: asset.id.clone(),
            filename: None,
            role: None,
            reference_scope: Some("global".into()),
        },
        AssetMetadataUpdate {
            asset_id: asset.id.clone(),
            filename: None,
            role: Some(ASSET_ROLE_PROFILE.into()),
            reference_scope: None,
        },
    ] {
        assert!(store
            .update_asset_metadata_with_request(update, &asset.revision, None)
            .is_err());
    }
}

#[test]
fn repeated_checkpoint_skips_unchanged_portable_files() {
    let root = std::env::temp_dir().join(format!("daena-checkpoint-skip-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Stable checkpoint".into(),
            entity_type: Some("note".into()),
        })
        .unwrap();
    let generation = store.flush_checkpoint("initial checkpoint").unwrap();
    let snapshot = store.export_snapshot().unwrap();

    assert_eq!(
        store
            .export_complete_snapshot(&root, &snapshot, generation)
            .unwrap(),
        0
    );

    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn entity_revision_batch_matches_point_revision() {
    let store = ProjectStore::in_memory().unwrap();
    let source = store
        .create_entity(CreateEntity {
            name: "Revision source".into(),
            entity_type: Some("note".into()),
        })
        .unwrap();
    let target = store
        .create_entity(CreateEntity {
            name: "Revision target".into(),
            entity_type: Some("note".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: source.id.clone(),
            body: "Revision body".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: source.id.clone(),
            namespace: "test".into(),
            key: "value".into(),
            value: serde_json::json!("revision field"),
            revision: String::new(),
        })
        .unwrap();
    store
        .create_relationship(RelationshipInput {
            source_id: source.id.clone(),
            target_id: target.id,
            relationship_type: "references".into(),
            metadata: None,
        })
        .unwrap();

    let listed = store
        .list_entities()
        .unwrap()
        .into_iter()
        .find(|entity| entity.id == source.id)
        .unwrap();
    assert_eq!(
        listed.revision,
        store.revision_for_entity(&source.id).unwrap()
    );
}

#[test]
fn search_updates_only_the_changed_source_row() {
    let store = ProjectStore::in_memory().unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Granular search".into(),
            entity_type: Some("note".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "First document".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    for (key, value) in [("first", "alpha"), ("second", "beta")] {
        store
            .set_field(FieldValue {
                entity_id: entity.id.clone(),
                namespace: "test".into(),
                key: key.into(),
                value: serde_json::json!(value),
                revision: String::new(),
            })
            .unwrap();
    }
    let rowids = |store: &ProjectStore| {
        store
            .connection
            .prepare("SELECT source_key,rowid FROM world_search WHERE entity_id=?1")
            .unwrap()
            .query_map(params![entity.id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .unwrap()
            .collect::<Result<BTreeMap<_, _>, _>>()
            .unwrap()
    };
    let before_document = rowids(&store);
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Second document".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let after_document = rowids(&store);
    assert_eq!(before_document["entity"], after_document["entity"]);
    assert_eq!(
        before_document["field:test/first"],
        after_document["field:test/first"]
    );
    assert_eq!(
        before_document["field:test/second"],
        after_document["field:test/second"]
    );
    let document_key = before_document
        .keys()
        .find(|key| key.starts_with("document:"))
        .unwrap();
    assert_ne!(before_document[document_key], after_document[document_key]);

    store
        .set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: "test".into(),
            key: "first".into(),
            value: serde_json::json!("updated"),
            revision: String::new(),
        })
        .unwrap();
    let after_field = rowids(&store);
    assert_ne!(
        after_document["field:test/first"],
        after_field["field:test/first"]
    );
    assert_eq!(
        after_document["field:test/second"],
        after_field["field:test/second"]
    );

    store
        .update_entity(entity.id.clone(), Some("Renamed search".into()), None)
        .unwrap();
    let after_entity = rowids(&store);
    assert_ne!(after_field["entity"], after_entity["entity"]);
    assert_eq!(after_field[document_key], after_entity[document_key]);
    assert_eq!(
        after_field["field:test/first"],
        after_entity["field:test/first"]
    );
    assert_eq!(
        after_field["field:test/second"],
        after_entity["field:test/second"]
    );
}

#[test]
fn search_projection_repairs_missing_maintenance_triggers() {
    let store = ProjectStore::in_memory().unwrap();
    store
        .connection
        .execute_batch("DROP TRIGGER entities_search_deleted")
        .unwrap();

    store.ensure_search_projection().unwrap();

    let trigger_exists: bool = store
        .connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='trigger' AND name='entities_search_deleted')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(trigger_exists);
}

#[test]
fn entity_scoped_reads_use_covering_indexes() {
    let store = ProjectStore::in_memory().unwrap();
    let query_plan = |sql: &str| {
        store
            .connection
            .prepare(sql)
            .unwrap()
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("\n")
    };

    assert!(query_plan("EXPLAIN QUERY PLAN SELECT id,entity_id,format,body,updated_at FROM documents WHERE entity_id='entity' ORDER BY updated_at DESC").contains("documents_entity_updated_idx"));
    assert!(query_plan("EXPLAIN QUERY PLAN SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at FROM assets WHERE entity_id='entity' ORDER BY created_at").contains("assets_entity_created_idx"));
}

#[test]
fn image_map_import_layer_mutations_and_checkpoint_rebuild() {
    let root = std::env::temp_dir().join(format!("daena-image-map-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let png = crate::maps::encode_transparent_png(8, 6).unwrap();
    let imported = store
        .import_image_map(
            "Atlas".into(),
            png.clone(),
            "image/png".into(),
            "atlas.png".into(),
            None,
        )
        .unwrap();
    assert_eq!(
        imported.entity.entity_type.as_deref(),
        Some(crate::maps::MAP_ENTITY_TYPE)
    );
    let descriptor = store
        .list_fields(imported.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "map")
        .unwrap();
    assert_eq!(
        descriptor.value["provider"]["id"],
        crate::maps::VECTOR_PROVIDER
    );
    assert_eq!(descriptor.value["sourceAssetId"], imported.source.id);
    assert_eq!(descriptor.value["previewAssetId"], imported.preview.id);
    assert_eq!(imported.source.mime_type, crate::maps::VECTOR_MIME);
    assert_eq!(imported.preview.mime_type, "image/png");

    let jpeg_map = store
        .import_image_map(
            "Photo".into(),
            crate::maps::image::VALID_JPEG.to_vec(),
            "image/jpeg".into(),
            "photo.jpg".into(),
            None,
        )
        .unwrap();
    assert_eq!(jpeg_map.preview.mime_type, "image/jpeg");

    let unsafe_svg = b"<svg viewBox=\"0 0 10 10\"><script>alert(1)</script></svg>".to_vec();
    assert!(store
        .import_image_map(
            "Bad".into(),
            unsafe_svg,
            "image/svg+xml".into(),
            "bad.svg".into(),
            None
        )
        .unwrap_err()
        .to_string()
        .contains("unsupported active"));

    let request_id = Uuid::new_v4().to_string();
    let layers_revision = store
        .list_fields(imported.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap()
        .revision;
    let created = store
        .create_raster_layer(
            imported.entity.id.clone(),
            "Ink".into(),
            &layers_revision,
            Some(&request_id),
        )
        .unwrap();
    assert!(!created.layers.revision.is_empty());
    assert!(!created.asset.as_ref().unwrap().revision.is_empty());
    let retried = store
        .create_raster_layer(
            imported.entity.id.clone(),
            "Ink".into(),
            &layers_revision,
            Some(&request_id),
        )
        .unwrap();
    assert_eq!(created.layer_id, retried.layer_id);
    assert_eq!(
        created.asset.as_ref().unwrap().id,
        retried.asset.as_ref().unwrap().id
    );
    assert_eq!(created.layers.revision, retried.layers.revision);
    assert!(store
        .create_raster_layer(
            imported.entity.id.clone(),
            "Other".into(),
            &layers_revision,
            Some(&request_id),
        )
        .unwrap_err()
        .to_string()
        .contains("request ID"));

    assert!(store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: imported.preview.id.clone(),
                content_hash: crate::maps::image::content_hash(&png),
                size: png.len() as i64,
                mime_type: "image/png".into(),
            },
            png.clone(),
            &store.asset(imported.preview.id.clone()).unwrap().revision,
            None,
        )
        .unwrap_err()
        .to_string()
        .contains("cannot be replaced"));

    let stale = store.create_raster_layer(
        imported.entity.id.clone(),
        "Other".into(),
        "not-a-revision",
        None,
    );
    assert!(stale.unwrap_err().to_string().contains("revision"));

    let layers_revision = created.layers.revision.clone();
    let updated = store
        .update_map_layer(
            imported.entity.id.clone(),
            created.layer_id.clone(),
            RasterLayerUpdate {
                name: Some("Coast".into()),
                order: Some(3),
                default_visible: Some(false),
                opacity: Some(0.25),
                locked: Some(true),
                style: None,
                selector: None,
            },
            &layers_revision,
            None,
        )
        .unwrap();
    assert_eq!(updated.layers.value["layers"][0]["name"], "Coast");
    assert_eq!(updated.layers.value["layers"][0]["opacity"], 0.25);

    let painted = crate::maps::encode_transparent_png(8, 6).unwrap();
    let raster_id = created.asset.as_ref().unwrap().id.clone();
    store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: raster_id.clone(),
                content_hash: crate::maps::image::content_hash(&painted),
                size: painted.len() as i64,
                mime_type: "image/png".into(),
            },
            painted,
            &store.asset(raster_id.clone()).unwrap().revision,
            None,
        )
        .unwrap();
    let wrong_size = crate::maps::encode_transparent_png(2, 2).unwrap();
    assert!(store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: raster_id.clone(),
                content_hash: crate::maps::image::content_hash(&wrong_size),
                size: wrong_size.len() as i64,
                mime_type: "image/png".into(),
            },
            wrong_size,
            &store.asset(raster_id.clone()).unwrap().revision,
            None,
        )
        .unwrap_err()
        .to_string()
        .contains("dimensions"));

    store.flush_checkpoint("image map export").unwrap();
    let checkpoint = crate::storage::read_json::<crate::storage::CheckpointManifest>(
        &root.join(crate::storage::CHECKPOINT_MANIFEST_FILE),
    )
    .unwrap();
    crate::storage::validate_checkpoint(&root, &checkpoint).unwrap();
    let before = canonical_files(&root);
    let source_hash = store
        .asset(imported.source.id.clone())
        .unwrap()
        .content_hash;
    let layer_hash = store.asset(raster_id.clone()).unwrap().content_hash;
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(canonical_files(&root), before);
    assert_eq!(
        rebuilt
            .asset(imported.source.id.clone())
            .unwrap()
            .content_hash,
        source_hash
    );
    assert_eq!(
        rebuilt.asset(raster_id.clone()).unwrap().content_hash,
        layer_hash
    );
    let layers = rebuilt
        .list_fields(imported.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap();
    assert_eq!(layers.value["layers"].as_array().unwrap().len(), 1);
    assert_eq!(layers.value["layers"][0]["rasterAssetId"], raster_id);

    rebuilt
        .delete_raster_layer(
            imported.entity.id.clone(),
            created.layer_id.clone(),
            &layers.revision,
            None,
        )
        .unwrap();
    assert!(rebuilt.asset(raster_id).is_err());
    let leftover = rebuilt
        .list_fields(imported.entity.id)
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap();
    assert_eq!(leftover.value["layers"].as_array().unwrap().len(), 0);
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

fn vector_generation() -> serde_json::Value {
    serde_json::json!({
        "id": "daena-landmass",
        "version": 1,
        "seed": 831429,
        "settings": {
            "landPercent": 40,
            "continentCount": 3,
            "coastlineRoughness": "medium",
            "islandFrequency": "medium"
        }
    })
}

fn vector_candidate() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "Polygon",
                "coordinates": [[[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]
            }
        }]
    }))
    .unwrap()
}

#[test]
fn vector_map_accept_replace_layer_delete_and_checkpoint_rebuild() {
    let root = std::env::temp_dir().join(format!("daena-vector-map-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let candidate = vector_candidate();
    let request_id = Uuid::new_v4().to_string();
    let accepted = store
        .accept_vector_map(
            "World".into(),
            candidate.clone(),
            vector_generation(),
            Some(&request_id),
        )
        .unwrap();
    let retried = store
        .accept_vector_map(
            "World".into(),
            candidate.clone(),
            vector_generation(),
            Some(&request_id),
        )
        .unwrap();
    assert_eq!(accepted.entity.id, retried.entity.id);
    assert_eq!(accepted.source.id, retried.source.id);
    assert_eq!(
        accepted.entity.entity_type.as_deref(),
        Some(crate::maps::MAP_ENTITY_TYPE)
    );
    let descriptor = store
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "map")
        .unwrap();
    assert_eq!(
        descriptor.value["provider"]["id"],
        crate::maps::VECTOR_PROVIDER
    );
    let canonical = store.asset_bytes(accepted.source.id.clone()).unwrap();
    crate::maps::vector::require_canonical_bytes(
        std::path::Path::new("assets/maps/map.geojson"),
        &canonical,
        &std::collections::BTreeSet::new(),
    )
    .unwrap();
    let stored: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    assert_eq!(stored["features"][0]["properties"]["daenaLayerId"], "base");
    assert_eq!(stored["features"][0]["properties"]["kind"], "land");

    let layers_revision = store
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap()
        .revision;
    let created = store
        .create_vector_layer(
            accepted.entity.id.clone(),
            "Countries".into(),
            &layers_revision,
            None,
            None,
        )
        .unwrap();
    assert_eq!(created.layers.value["layers"][0]["kind"], "vector");
    assert!(created.asset.is_none());

    let source = store.asset(accepted.source.id.clone()).unwrap();
    let feature_id = Uuid::new_v4().to_string();
    let layer_id = created.layer_id.clone();
    let authored = serde_json::json!({
        "type": "FeatureCollection",
        "features": [
            serde_json::from_slice::<serde_json::Value>(&canonical).unwrap()["features"][0],
            {
                "type": "Feature",
                "id": feature_id,
                "properties": {"daenaLayerId": layer_id, "kind": "region", "name": "West"},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0], [4.0, 4.0]]]
                }
            }
        ]
    });
    let authored_bytes = serde_json::to_vec(&authored).unwrap();
    let upload_hash = format!("sha256:{:x}", Sha256::digest(&authored_bytes));
    let replaced = store
        .replace_vector_source(
            accepted.source.id.clone(),
            authored_bytes.clone(),
            upload_hash.clone(),
            &source.revision,
            None,
        )
        .unwrap();
    assert_ne!(replaced.source.content_hash, upload_hash);
    assert_eq!(
        replaced.source.content_hash,
        format!(
            "sha256:{:x}",
            Sha256::digest(
                crate::maps::vector::canonicalize_committed(
                    &authored_bytes,
                    &crate::maps::vector::layer_ids_from_layers_field(&created.layers.value)
                )
                .unwrap()
            )
        )
    );
    assert!(store
        .replace_vector_source(
            accepted.source.id.clone(),
            authored_bytes.clone(),
            upload_hash,
            "stale-revision",
            None,
        )
        .unwrap_err()
        .to_string()
        .contains("revision"));

    let place = store
        .create_entity(CreateEntity {
            name: "West".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store
        .upsert_map_location(
            place.id.clone(),
            crate::maps::LocationReference {
                id: Uuid::new_v4().to_string(),
                map_entity_id: accepted.entity.id.clone(),
                role: "region".into(),
                label: "West".into(),
                anchor: crate::maps::Anchor::ProviderFeature {
                    provider: crate::maps::VECTOR_PROVIDER.into(),
                    feature_kind: "geojson-feature".into(),
                    feature_id: feature_id.clone(),
                    fallback_point: crate::maps::Point(0.5, 0.5),
                },
                validity: crate::maps::Validity {
                    from: None,
                    to: None,
                },
            },
            None,
        )
        .unwrap();
    let linked = store
        .map_location_projection(accepted.entity.id.clone())
        .unwrap();
    assert_eq!(linked[0]["resolution"], "resolved");

    let count_mismatch = store.delete_vector_layer(
        accepted.entity.id.clone(),
        created.layer_id.clone(),
        &created.layers.revision,
        &replaced.source.revision,
        99,
        None,
    );
    assert!(count_mismatch
        .unwrap_err()
        .to_string()
        .contains("expectedFeatureCount"));
    assert_eq!(
        store.asset(accepted.source.id.clone()).unwrap().revision,
        replaced.source.revision
    );

    FAIL_NEXT_RUNTIME_ASSET_INSTALL.with(|flag| flag.set(true));
    let crashed = store.delete_vector_layer(
        accepted.entity.id.clone(),
        created.layer_id.clone(),
        &created.layers.revision,
        &replaced.source.revision,
        1,
        None,
    );
    assert!(crashed
        .unwrap_err()
        .to_string()
        .contains("install runtime asset"));
    assert_eq!(
        store.asset(accepted.source.id.clone()).unwrap().revision,
        replaced.source.revision
    );
    assert_eq!(
        store
            .list_fields(accepted.entity.id.clone())
            .unwrap()
            .into_iter()
            .find(|field| field.key == "layers")
            .unwrap()
            .value["layers"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let deleted = store
        .delete_vector_layer(
            accepted.entity.id.clone(),
            created.layer_id.clone(),
            &created.layers.revision,
            &replaced.source.revision,
            1,
            None,
        )
        .unwrap();
    assert_eq!(deleted.deleted_feature_count, 1);
    assert_eq!(deleted.layers.value["layers"].as_array().unwrap().len(), 0);

    store.flush_checkpoint("vector map export").unwrap();
    let checkpoint = crate::storage::read_json::<crate::storage::CheckpointManifest>(
        &root.join(crate::storage::CHECKPOINT_MANIFEST_FILE),
    )
    .unwrap();
    crate::storage::validate_checkpoint(&root, &checkpoint).unwrap();
    let before = canonical_files(&root);
    let source_hash = store
        .asset(accepted.source.id.clone())
        .unwrap()
        .content_hash;
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(canonical_files(&root), before);
    assert_eq!(
        rebuilt
            .asset(accepted.source.id.clone())
            .unwrap()
            .content_hash,
        source_hash
    );
    let layers = rebuilt
        .list_fields(accepted.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap();
    assert_eq!(layers.value["layers"].as_array().unwrap().len(), 0);
    let rebuilt_links = rebuilt
        .map_location_projection(accepted.entity.id.clone())
        .unwrap();
    assert_eq!(rebuilt_links[0]["resolution"], "unresolved");
    let feature_count: i64 = rusqlite::Connection::open(root.join(".daena/index.sqlite"))
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM map_feature_projection WHERE map_entity_id=?1",
            rusqlite::params![accepted.entity.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(feature_count, 1);
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn image_map_runtime_bytes_survive_an_interrupted_export() {
    let root = std::env::temp_dir().join(format!("daena-image-map-interrupt-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    store
        .export_worker
        .take()
        .unwrap()
        .stop_without_drain()
        .unwrap();
    store.suppress_sync.set(true);
    let png = crate::maps::encode_transparent_png(4, 3).unwrap();
    let imported = store
        .import_image_map(
            "Atlas".into(),
            png.clone(),
            "image/png".into(),
            "atlas.png".into(),
            None,
        )
        .unwrap();
    assert!(!root.join(&imported.preview.path).exists());
    drop(store);

    let reopened = ProjectStore::open_directory(&root).unwrap();
    reopened
        .flush_checkpoint("recover interrupted image map export")
        .unwrap();
    assert_eq!(
        std::fs::read(root.join(&imported.preview.path)).unwrap(),
        png
    );
    assert_eq!(
        std::fs::read(root.join(&imported.source.path)).unwrap(),
        crate::maps::empty_canonical_bytes()
    );
    assert_eq!(reopened.sync_summary().unwrap().state, "clean");
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn image_map_semantic_features_survive_checkpoint_rebuild_and_spatial_query() {
    let root = std::env::temp_dir().join(format!("daena-image-map-semantic-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let png = crate::maps::encode_transparent_png(8, 6).unwrap();
    let imported = store
        .import_image_map(
            "Atlas".into(),
            png,
            "image/png".into(),
            "atlas.png".into(),
            None,
        )
        .unwrap();
    let layers_revision = store
        .list_fields(imported.entity.id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap()
        .revision;
    let overlay = store
        .create_semantic_layer(
            imported.entity.id.clone(),
            "Routes".into(),
            &layers_revision,
            None,
            Some(serde_json::json!({"stroke": "#d5ab6c", "strokeWidth": 2})),
            Some(serde_json::json!({"roles": ["route"], "anchorKind": "path"})),
        )
        .unwrap();
    assert_eq!(overlay.layers.value["layers"][0]["kind"], "semantic");
    assert!(overlay.asset.is_none());

    let place = store
        .create_entity(CreateEntity {
            name: "Coast road".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    let location_id = Uuid::new_v4().to_string();
    store
        .upsert_map_location(
            place.id.clone(),
            crate::maps::LocationReference {
                id: location_id.clone(),
                map_entity_id: imported.entity.id.clone(),
                role: "route".into(),
                label: "Coast road".into(),
                anchor: crate::maps::Anchor::Path {
                    points: vec![
                        crate::maps::Point(0.1, 0.1),
                        crate::maps::Point(0.4, 0.2),
                        crate::maps::Point(0.7, 0.3),
                    ],
                },
                validity: crate::maps::Validity {
                    from: None,
                    to: None,
                },
            },
            None,
        )
        .unwrap();
    let open_area = store.upsert_map_location(
        place.id.clone(),
        crate::maps::LocationReference {
            id: Uuid::new_v4().to_string(),
            map_entity_id: imported.entity.id.clone(),
            role: "region".into(),
            label: "Broken ring".into(),
            anchor: crate::maps::Anchor::Area {
                rings: vec![vec![
                    crate::maps::Point(0.1, 0.1),
                    crate::maps::Point(0.2, 0.1),
                    crate::maps::Point(0.2, 0.2),
                ]],
            },
            validity: crate::maps::Validity {
                from: None,
                to: None,
            },
        },
        None,
    );
    assert!(open_area
        .unwrap_err()
        .to_string()
        .contains("invalid geometry"));

    let hits = store
        .query_map_locations(imported.entity.id.clone(), 0.35, 0.15, 0.45, 0.25)
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["id"], location_id);
    assert_eq!(hits[0]["anchor"]["kind"], "path");
    assert_eq!(hits[0]["entityId"], place.id);
    let miss = store
        .query_map_locations(imported.entity.id.clone(), 0.9, 0.9, 1.0, 1.0)
        .unwrap();
    assert!(miss.is_empty());

    store.flush_checkpoint("semantic features").unwrap();
    let before = canonical_files(&root);
    let map_id = imported.entity.id.clone();
    drop(store);
    std::fs::remove_dir_all(root.join(".daena")).unwrap();
    let rebuilt = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(canonical_files(&root), before);
    let projection = rebuilt.map_location_projection(map_id.clone()).unwrap();
    assert_eq!(projection[0]["anchor"]["kind"], "path");
    assert_eq!(
        projection[0]["anchor"]["points"].as_array().unwrap().len(),
        3
    );
    let layers = rebuilt
        .list_fields(map_id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap();
    assert_eq!(layers.value["layers"][0]["kind"], "semantic");
    rebuilt
        .delete_semantic_layer(
            map_id.clone(),
            overlay.layer_id.clone(),
            &layers.revision,
            None,
        )
        .unwrap();
    let leftover = rebuilt
        .list_fields(map_id.clone())
        .unwrap()
        .into_iter()
        .find(|field| field.key == "layers")
        .unwrap();
    assert_eq!(leftover.value["layers"].as_array().unwrap().len(), 0);
    assert_eq!(rebuilt.map_location_projection(map_id).unwrap().len(), 1);
    drop(rebuilt);
    std::fs::remove_dir_all(root).unwrap();
}
