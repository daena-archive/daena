use super::*;
use std::collections::BTreeMap;

static EXPORT_FAILURE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
fn external_edit_before_mutation_fails_closed() {
    let root = std::env::temp_dir().join(format!("daena-baseline-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Baseline owner".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_exports().unwrap();
    let entity_path = root.join("entities").join(&entity.id).join("entity.json");
    let mut file = std::fs::read_to_string(&entity_path).unwrap();
    file = file.replace("Baseline owner", "External edit");
    std::fs::write(entity_path, file).unwrap();
    assert!(matches!(
        store.update_entity(entity.id, Some("Local edit".into()), None),
        Err(CoreError::Validation(_)) | Err(CoreError::Conflict(_))
    ));
    drop(store);
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
    store.flush_exports().unwrap();
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
fn runtime_recovery_backup_contains_consistent_database_and_staged_payload_area() {
    let root = std::env::temp_dir().join(format!("daena-recovery-backup-{}", Uuid::new_v4()));
    let output = std::env::temp_dir().join(format!("daena-recovery-output-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    store.suppress_sync.set(true);
    store
        .create_entity(CreateEntity {
            name: "Recovery owner".into(),
            entity_type: None,
        })
        .unwrap();
    store.suppress_sync.set(false);

    let artifact = store.recovery_backup_to(&output).unwrap();
    let database = Connection::open(Path::new(&artifact).join("index.sqlite")).unwrap();
    let count: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM entities WHERE name='Recovery owner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
    let pending: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM sync_batches WHERE state NOT IN ('completed','superseded')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(pending > 0);
    assert!(Path::new(&artifact).join("index.sqlite").is_file());

    drop(database);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn runtime_recovery_backup_restores_runtime_state_and_restarts_exporter() {
    let root = std::env::temp_dir().join(format!("daena-recovery-restore-{}", Uuid::new_v4()));
    let output =
        std::env::temp_dir().join(format!("daena-recovery-restore-output-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Before recovery".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_exports().unwrap();
    let staged_payload = root.join(".daena/sync/pending-request/input/payload.bin");
    std::fs::create_dir_all(staged_payload.parent().unwrap()).unwrap();
    std::fs::write(&staged_payload, b"pending payload").unwrap();
    let artifact = store.recovery_backup_to(&output).unwrap();
    std::fs::remove_file(&staged_payload).unwrap();
    store
        .update_entity(entity.id.clone(), Some("After recovery".into()), None)
        .unwrap();

    store.restore_recovery_backup(&artifact).unwrap();
    assert_eq!(store.list_entities().unwrap()[0].name, "Before recovery");
    assert_eq!(std::fs::read(&staged_payload).unwrap(), b"pending payload");
    assert!(store.export_worker.is_some());

    drop(store);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(output).unwrap();
}

#[test]
fn rebuild_from_files_refuses_dirty_runtime_without_explicit_divergence_review() {
    let root = std::env::temp_dir().join(format!("daena-rebuild-guard-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    store.suppress_sync.set(true);
    store
        .create_entity(CreateEntity {
            name: "Dirty rebuild".into(),
            entity_type: None,
        })
        .unwrap();
    store.suppress_sync.set(false);
    assert!(matches!(
        store.rebuild_from_files(),
        Err(CoreError::Conflict(message)) if message.contains("explicit divergence review")
    ));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn portable_restore_archives_dirty_runtime_before_replacement() {
    let root = std::env::temp_dir().join(format!("daena-restore-archive-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    store.suppress_sync.set(true);
    store
        .create_entity(CreateEntity {
            name: "Pending runtime".into(),
            entity_type: None,
        })
        .unwrap();
    store.suppress_sync.set(false);
    let payload = store.export_json().unwrap();
    store.restore_payload(&payload).unwrap();
    assert!(std::fs::read_dir(root.join(".daena/backups"))
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with("daena-recovery-")));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn blocked_rebuild_archives_and_discards_obsolete_runtime_queue() {
    let root = std::env::temp_dir().join(format!("daena-rebuild-discard-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Portable winner".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_exports().unwrap();
    store
        .connection
        .execute(
            "UPDATE runtime_meta SET reconciliation_state='blocked',sync_state='failed',dirty_count=1 WHERE key='runtime'",
            [],
        )
        .unwrap();
    let report = store.rebuild_from_files().unwrap();
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("discarded")));
    let (sync_state, dirty_count): (String, i64) = store
        .connection
        .query_row(
            "SELECT sync_state,dirty_count FROM runtime_meta WHERE key='runtime'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(sync_state, "clean");
    assert_eq!(dirty_count, 0);
    assert_eq!(store.list_entities().unwrap()[0].name, "Portable winner");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unindexed_external_file_before_mutation_fails_closed() {
    let root = std::env::temp_dir().join(format!("daena-unindexed-baseline-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Unindexed owner".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_exports().unwrap();
    std::fs::write(
        root.join("entities").join(&entity.id).join("unexpected.md"),
        "external\n",
    )
    .unwrap();
    let result = store.save_document(SaveDocument {
        entity_id: entity.id,
        body: "new body\n".into(),
        format: Some("markdown".into()),
    });
    assert!(
        matches!(result, Err(CoreError::Conflict(message)) if message.contains("unexpected.md"))
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ignored_os_metadata_does_not_block_mutation_preflight() {
    let root = std::env::temp_dir().join(format!("daena-ignored-metadata-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Metadata owner".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_exports().unwrap();
    let entity_root = root.join("entities").join(&entity.id);
    for name in [".DS_Store", "Thumbs.db", "desktop.ini"] {
        std::fs::write(entity_root.join(name), b"ignored metadata").unwrap();
    }
    store
        .save_document(SaveDocument {
            entity_id: entity.id,
            body: "metadata remains harmless\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
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
fn interrupted_export_resumes_applied_items_after_reopen() {
    let _guard = EXPORT_FAILURE_TEST_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("daena-recovery-{}", Uuid::new_v4()));
    let request_id = Uuid::new_v4().to_string();
    let store = ProjectStore::open_directory(&root).unwrap();
    crate::sync::set_test_export_failure_after(Some(&request_id), 1);
    store
        .create_entity_with_request(
            CreateEntity {
                name: "Recovery owner".into(),
                entity_type: None,
            },
            Some(&request_id),
        )
        .unwrap();
    let _ = store.flush_exports();
    crate::sync::set_test_export_failure_after(None, 0);
    store
        .connection
        .execute(
            "UPDATE sync_batches SET state='exporting', last_error=NULL WHERE request_id=?1",
            params![request_id],
        )
        .unwrap();
    drop(store);

    let reopened = ProjectStore::open_directory(&root).unwrap();
    let entities = reopened.list_entities().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].name, "Recovery owner");
    let receipt_state: String = reopened
        .connection
        .query_row(
            "SELECT state FROM mutation_receipts WHERE request_id=?1",
            params![request_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(receipt_state, "completed");
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_worker_flushes_a_durable_pending_batch() {
    let root = std::env::temp_dir().join(format!("daena-worker-flush-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Worker flush owner".into(),
            entity_type: None,
        })
        .unwrap();
    let entity_path = root.join("entities").join(&entity.id).join("entity.json");
    store
        .connection
        .execute(
            "UPDATE sync_batches SET state='exporting', completed_at=NULL, last_error=NULL WHERE request_id=(SELECT request_id FROM sync_batches ORDER BY created_at DESC LIMIT 1)",
            [],
        )
        .unwrap();
    store
        .connection
        .execute(
            "UPDATE sync_items SET state='pending', last_error=NULL WHERE batch_id=(SELECT id FROM sync_batches ORDER BY created_at DESC LIMIT 1)",
            [],
        )
        .unwrap();

    store.flush_exports().unwrap();
    assert!(entity_path.is_file());
    let state: String = store
        .connection
        .query_row(
            "SELECT state FROM sync_batches ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "completed");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn scoped_flush_barrier_does_not_drain_unrelated_revisions() {
    let root = std::env::temp_dir().join(format!("daena-scoped-flush-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let first_request = Uuid::new_v4().to_string();
    let second_request = Uuid::new_v4().to_string();
    store.suppress_sync.set(true);
    store
        .create_entity_with_request(
            CreateEntity {
                name: "Scoped flush one".into(),
                entity_type: None,
            },
            Some(&first_request),
        )
        .unwrap();
    store
        .create_entity_with_request(
            CreateEntity {
                name: "Scoped flush two".into(),
                entity_type: None,
            },
            Some(&second_request),
        )
        .unwrap();
    store.suppress_sync.set(false);

    store
        .flush_with_barrier(FlushBarrier {
            revision_set: Some(vec![first_request.clone()]),
            operation_reason: "test scoped git preflight".into(),
        })
        .unwrap();
    let first_state: String = store
        .connection
        .query_row(
            "SELECT state FROM sync_batches WHERE request_id=?1",
            params![first_request],
            |row| row.get(0),
        )
        .unwrap();
    let second_state: String = store
        .connection
        .query_row(
            "SELECT state FROM sync_batches WHERE request_id=?1",
            params![second_request],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(first_state, "completed");
    assert_eq!(second_state, "exporting");

    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn export_worker_retries_a_failed_batch_within_bound() {
    let _guard = EXPORT_FAILURE_TEST_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("daena-worker-retry-{}", Uuid::new_v4()));
    let request_id = Uuid::new_v4().to_string();
    let store = ProjectStore::open_directory(&root).unwrap();
    crate::sync::set_test_export_failure_after(Some(&request_id), 1);
    store
        .create_entity_with_request(
            CreateEntity {
                name: "Retry owner".into(),
                entity_type: None,
            },
            Some(&request_id),
        )
        .unwrap();
    let _ = store.flush_exports();
    crate::sync::set_test_export_failure_after(None, 0);
    store.flush_exports().unwrap();
    let entity_id: String = store
        .connection
        .query_row(
            "SELECT id FROM entities WHERE name='Retry owner'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let state: String = store
        .connection
        .query_row(
            "SELECT state FROM sync_batches WHERE request_id=?1",
            params![request_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "completed");
    assert!(root
        .join("entities")
        .join(entity_id)
        .join("entity.json")
        .exists());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn queued_export_does_not_change_runtime_revisions() {
    let root = std::env::temp_dir().join(format!("daena-revision-stability-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Stable revision owner".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Stable body\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    let before_flush = store.list_documents(entity.id.clone()).unwrap()[0]
        .revision
        .clone();
    store.flush_exports().unwrap();
    let after_flush = store.list_documents(entity.id).unwrap()[0].revision.clone();
    assert_eq!(before_flush, after_flush);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rapid_document_edits_preserve_newest_after_worker_debounce() {
    let root = std::env::temp_dir().join(format!("daena-document-debounce-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Debounce owner".into(),
            entity_type: None,
        })
        .unwrap();
    for index in 0..8 {
        store
            .save_document(SaveDocument {
                entity_id: entity.id.clone(),
                body: format!("revision-{index}\n"),
                format: Some("markdown".into()),
            })
            .unwrap();
    }
    store.flush_exports().unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("entities").join(entity.id).join("document.md")).unwrap(),
        "revision-7\n"
    );
    let superseded: i64 = store
        .connection
        .query_row(
            "SELECT COUNT(*) FROM sync_batches WHERE state='superseded'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(superseded > 0, "rapid edits should supersede stale batches");
    drop(store);
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
fn interrupted_asset_export_resumes_from_persisted_input() {
    let _guard = EXPORT_FAILURE_TEST_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("daena-asset-recovery-{}", Uuid::new_v4()));
    let source = root.with_extension("source.bin");
    std::fs::write(&source, b"before").unwrap();
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Asset recovery owner".into(),
            entity_type: None,
        })
        .unwrap();
    let asset = store
        .register_asset_file(AssetFileInput {
            entity_id: entity.id,
            namespace: "core".into(),
            source_path: source.to_string_lossy().into_owned(),
            filename: "payload.bin".into(),
            mime_type: "application/octet-stream".into(),
        })
        .unwrap();
    store.flush_exports().unwrap();
    let request_id = Uuid::new_v4().to_string();
    let bytes = b"after-persisted".to_vec();
    crate::sync::set_test_export_failure_after(Some(&request_id), 1);
    store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: asset.id.clone(),
                content_hash: format!("sha256:{}", digest_bytes(&bytes)),
                size: bytes.len() as i64,
                mime_type: "application/octet-stream".into(),
            },
            bytes.clone(),
            &asset.revision,
            Some(&request_id),
        )
        .unwrap();
    crate::sync::set_test_export_failure_after(None, 0);
    store
        .connection
        .execute(
            "UPDATE sync_batches SET state='exporting',last_error=NULL WHERE request_id=?1",
            params![request_id],
        )
        .unwrap();
    drop(store);

    let reopened = ProjectStore::open_directory(&root).unwrap();
    let path = reopened.asset(asset.id).unwrap().path;
    assert_eq!(std::fs::read(root.join(path)).unwrap(), bytes);
    drop(reopened);
    std::fs::remove_file(source).unwrap();
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
fn external_markdown_edits_refresh_clean_index_and_invalid_edits_preserve_it() {
    let root = std::env::temp_dir().join(format!("daena-external-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Watched record".into(),
            entity_type: Some("place".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Before\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();

    std::fs::write(
        root.join("entities").join(&entity.id).join("document.md"),
        "# After\n",
    )
    .unwrap();
    let report = store.reconcile_external_changes().unwrap();
    assert!(report.changed);
    assert!(report
        .paths
        .iter()
        .any(|path| path == &format!("entities/{}/document.md", entity.id)));
    assert_eq!(
        store.list_documents(entity.id.clone()).unwrap()[0].body,
        "# After\n"
    );

    std::fs::write(
        root.join("entities").join(&entity.id).join("entity.json"),
        "{not valid json",
    )
    .unwrap();
    let invalid = store.reconcile_external_changes().unwrap();
    assert!(!invalid.changed);
    assert!(!invalid.diagnostics.is_empty());
    assert_eq!(store.info().unwrap().index_status, "diagnostic");
    let sync = &store.info().unwrap().sync;
    assert_eq!(sync.reconciliation_state, "failed");
    assert!(!sync.reconciliation_diagnostics.is_empty());
    assert_eq!(
        store.list_documents_unchecked(entity.id.clone()).unwrap()[0].body,
        "# After\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_document_and_assets_manifest_deletion_removes_runtime_rows() {
    let root = std::env::temp_dir().join(format!("daena-single-file-delete-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Delete canonical files".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "remove me".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();
    std::fs::remove_file(root.join("entities").join(&entity.id).join("document.md")).unwrap();
    assert!(store.reconcile_external_changes().unwrap().changed);
    assert!(store
        .list_documents_unchecked(entity.id.clone())
        .unwrap()
        .is_empty());
    assert!(store
        .search("remove me".into())
        .unwrap()
        .iter()
        .all(|match_| match_.id != entity.id));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn two_sided_external_change_is_reported_without_overwriting_database() {
    let root = std::env::temp_dir().join(format!("daena-two-sided-conflict-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Conflict owner".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "baseline\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();

    store
        .connection
        .execute(
            "UPDATE documents SET body='database side\n' WHERE entity_id=?1",
            params![entity.id],
        )
        .unwrap();
    store
        .connection
        .execute(
            "UPDATE source_files SET content_hash='sha256:database-side' WHERE path=?1",
            params![format!("entities/{}/document.md", entity.id)],
        )
        .unwrap();
    std::fs::write(
        root.join("entities").join(&entity.id).join("document.md"),
        "disk side\n",
    )
    .unwrap();

    let report = store.reconcile_external_changes().unwrap();
    assert!(!report.changed);
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("database and disk both changed")));
    assert_eq!(
        store.list_documents_unchecked(entity.id.clone()).unwrap()[0].body,
        "database side\n"
    );
    assert_eq!(store.info().unwrap().sync.conflicts.len(), 1);
    drop(store);
    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(reopened.info().unwrap().sync.conflicts.len(), 1);
    reopened
        .resolve_reconciliation_conflict_use_disk(&format!("entities/{}/document.md", entity.id))
        .unwrap();
    assert_eq!(
        reopened
            .list_documents_unchecked(entity.id.clone())
            .unwrap()[0]
            .body,
        "disk side\n"
    );
    assert!(std::fs::read_dir(root.join(".daena/conflicts"))
        .unwrap()
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().contains("database-")));
    assert!(reopened.info().unwrap().sync.conflicts.is_empty());
    reopened
        .connection
        .execute(
            "UPDATE documents SET body='database again\n' WHERE entity_id=?1",
            params![entity.id],
        )
        .unwrap();
    reopened
        .connection
        .execute(
            "UPDATE source_files SET content_hash='sha256:database-again' WHERE path=?1",
            params![format!("entities/{}/document.md", entity.id)],
        )
        .unwrap();
    std::fs::write(
        root.join("entities").join(&entity.id).join("document.md"),
        "disk again\n",
    )
    .unwrap();
    let _ = reopened.reconcile_external_changes().unwrap();
    reopened
        .resolve_reconciliation_conflict_use_database(&format!(
            "entities/{}/document.md",
            entity.id
        ))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("entities").join(&entity.id).join("document.md"))
            .unwrap(),
        "database again\n"
    );
    reopened
        .connection
        .execute(
            "UPDATE documents SET body='database third\n' WHERE entity_id=?1",
            params![entity.id],
        )
        .unwrap();
    reopened
        .connection
        .execute(
            "UPDATE source_files SET content_hash='sha256:database-third' WHERE path=?1",
            params![format!("entities/{}/document.md", entity.id)],
        )
        .unwrap();
    std::fs::write(
        root.join("entities").join(&entity.id).join("document.md"),
        "disk third\n",
    )
    .unwrap();
    let _ = reopened.reconcile_external_changes().unwrap();
    reopened
        .resolve_reconciliation_conflict_manual_document(
            &format!("entities/{}/document.md", entity.id),
            "manually resolved",
        )
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.join("entities").join(&entity.id).join("document.md"))
            .unwrap(),
        "manually resolved\n"
    );
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_field_file_reconciles_without_snapshot_reimport() {
    let root = std::env::temp_dir().join(format!("daena-external-field-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Field owner".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .set_field(FieldValue {
            entity_id: entity.id.clone(),
            namespace: "lore".into(),
            key: "summary".into(),
            value: serde_json::json!("before"),
            revision: String::new(),
        })
        .unwrap();
    store.flush_exports().unwrap();
    let fields_dir = root.join("entities").join(&entity.id).join("fields");
    let field_path = std::fs::read_dir(&fields_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .unwrap();
    std::fs::write(&field_path, r#"{"summary":"after"}"#).unwrap();

    let report = store.reconcile_external_changes().unwrap();
    assert!(report.changed);
    assert_eq!(
        store.list_fields_unchecked(entity.id).unwrap()[0].value,
        serde_json::json!("after")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_relationship_file_reconciles_targeted_source_records() {
    let root = std::env::temp_dir().join(format!("daena-external-relationship-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let source = store
        .create_entity(CreateEntity {
            name: "Source".into(),
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
        .create_relationship(RelationshipInput {
            source_id: source.id.clone(),
            target_id: target.id,
            relationship_type: "knows".into(),
            metadata: Some("{}".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();
    let path = root
        .join("entities")
        .join(&source.id)
        .join("relationships.json");
    let contents = std::fs::read_to_string(&path)
        .unwrap()
        .replace("knows", "influences");
    std::fs::write(path, contents).unwrap();

    let report = store.reconcile_external_changes().unwrap();
    assert!(report.changed, "{report:?}");
    assert_eq!(
        store.list_relationships(source.id).unwrap()[0].relationship_type,
        "influences"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_asset_payload_and_manifest_reconcile_targeted_metadata() {
    let root = std::env::temp_dir().join(format!("daena-external-asset-{}", Uuid::new_v4()));
    let source_path = root.with_extension("input");
    std::fs::write(&source_path, b"before").unwrap();
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
            namespace: "core".into(),
            source_path: source_path.to_string_lossy().into_owned(),
            filename: "payload.bin".into(),
            mime_type: "application/octet-stream".into(),
        })
        .unwrap();
    store.flush_exports().unwrap();
    let payload = root.join(&asset.path);
    let old_manifest = std::fs::read_to_string(
        root.join("entities")
            .join(&asset.entity_id)
            .join("assets.json"),
    )
    .unwrap();
    let new_bytes = b"after-payload";
    let new_hash = format!("sha256:{}", digest_bytes(new_bytes));
    std::fs::write(&payload, new_bytes).unwrap();
    let new_manifest = old_manifest
        .replace(&asset.content_hash, &new_hash)
        .replace(
            &format!("\"size\":{}", asset.size),
            &format!("\"size\":{}", new_bytes.len()),
        )
        .replace(
            &format!("\"size\": {}", asset.size),
            &format!("\"size\": {}", new_bytes.len()),
        );
    std::fs::write(
        root.join("entities")
            .join(&asset.entity_id)
            .join("assets.json"),
        new_manifest,
    )
    .unwrap();

    let report = store.reconcile_external_changes().unwrap();
    assert!(report.changed, "{report:?}");
    assert_eq!(store.asset(asset.id).unwrap().content_hash, new_hash);
    std::fs::remove_file(source_path).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn external_entity_deletion_reconciles_runtime_rows_and_relationships() {
    let root = std::env::temp_dir().join(format!("daena-external-delete-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let source = store
        .create_entity(CreateEntity {
            name: "Delete source".into(),
            entity_type: None,
        })
        .unwrap();
    let target = store
        .create_entity(CreateEntity {
            name: "Delete target".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .create_relationship(RelationshipInput {
            source_id: source.id.clone(),
            target_id: target.id.clone(),
            relationship_type: "points-to".into(),
            metadata: Some("{}".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();
    std::fs::remove_dir_all(root.join("entities").join(&source.id)).unwrap();

    let report = store.reconcile_external_changes().unwrap();
    assert!(report.changed);
    assert!(store
        .list_entities()
        .unwrap()
        .iter()
        .all(|entity| entity.id != source.id));
    assert!(store.list_relationships(target.id).unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rapid_external_document_writes_reconcile_newest_body() {
    let root = std::env::temp_dir().join(format!("daena-rapid-external-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Rapid external owner".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "baseline\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();
    let path = root.join("entities").join(&entity.id).join("document.md");
    for index in 0..8 {
        std::fs::write(&path, format!("external-{index}\n")).unwrap();
    }
    let report = store.reconcile_external_changes().unwrap();
    assert!(report.changed);
    assert_eq!(
        store.list_documents_unchecked(entity.id).unwrap()[0].body,
        "external-7\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn exporter_owned_files_are_ignored_by_reconciliation_after_flush() {
    let root = std::env::temp_dir().join(format!("daena-export-race-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Exporter target".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id,
            body: "exported".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();
    let report = store.reconcile_external_changes().unwrap();
    assert!(
        !report.changed,
        "export-owned files were reimported: {report:?}"
    );
    assert!(report.diagnostics.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_external_intermediate_state_recovers_on_valid_save() {
    let root = std::env::temp_dir().join(format!("daena-invalid-intermediate-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Before invalid state".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_exports().unwrap();
    let path = root.join("entities").join(&entity.id).join("entity.json");
    let valid = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, "{not valid json").unwrap();
    let invalid = store.reconcile_external_changes().unwrap();
    assert!(!invalid.changed);
    assert!(!invalid.diagnostics.is_empty());
    let repaired = valid.replace("Before invalid state", "After recovery");
    std::fs::write(&path, repaired).unwrap();
    let recovered = store.reconcile_external_changes().unwrap();
    assert!(recovered.changed);
    assert_eq!(store.list_entities().unwrap()[0].name, "After recovery");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn widespread_external_changes_enter_project_divergence_mode() {
    let root = std::env::temp_dir().join(format!("daena-divergence-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entities = (0..33)
        .map(|index| {
            store
                .create_entity(CreateEntity {
                    name: format!("Divergence {index}"),
                    entity_type: None,
                })
                .unwrap()
        })
        .collect::<Vec<_>>();
    store.flush_exports().unwrap();
    for (index, entity) in entities.iter().enumerate() {
        let path = root.join("entities").join(&entity.id).join("entity.json");
        let contents = std::fs::read_to_string(&path)
            .unwrap()
            .replace(&format!("Divergence {index}"), &format!("Changed {index}"));
        std::fs::write(path, contents).unwrap();
    }
    let report = store.reconcile_external_changes().unwrap();
    assert!(!report.changed);
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.contains("project divergence")));
    assert_eq!(store.info().unwrap().sync.reconciliation_state, "blocked");
    std::fs::remove_dir_all(root).unwrap();
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
    assert!(!store
        .canonical_source_hashes(".daena/conflicts/%")
        .unwrap()
        .iter()
        .any(|(source, _)| source.contains("conflicts")));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn git_unmerged_canonical_files_are_diagnostic_before_scanning() {
    let root = std::env::temp_dir().join(format!("daena-git-conflict-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let run_git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap();
        output
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
    let base_commit = run_git(&["commit", "-qm", "base"]);
    assert!(
        base_commit.status.success(),
        "base commit failed: {}",
        String::from_utf8_lossy(&base_commit.stderr)
    );
    assert!(run_git(&["checkout", "-qb", "feature"]).status.success());

    let mut manifest: crate::storage::ProjectManifest =
        crate::storage::read_json(&root.join("project.json")).unwrap();
    manifest.name = "Feature project".into();
    crate::storage::write_json(&root.join("project.json"), &manifest).unwrap();
    assert!(run_git(&["add", "project.json"]).status.success());
    assert!(run_git(&["commit", "-qm", "feature"]).status.success());
    assert!(run_git(&["checkout", "-q", "-"]).status.success());

    manifest.name = "Main project".into();
    crate::storage::write_json(&root.join("project.json"), &manifest).unwrap();
    assert!(run_git(&["add", "project.json"]).status.success());
    assert!(run_git(&["commit", "-qm", "main"]).status.success());
    assert!(!run_git(&["merge", "feature"]).status.success());

    let report = store.reconcile_external_changes().unwrap();
    assert_eq!(report.paths, vec!["project.json"]);
    assert!(report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.starts_with("git.unmerged: project.json")));
    let commit = store.git_commit("must not commit unresolved merge".into(), None);
    assert!(matches!(commit, Err(CoreError::Conflict(_))));
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
    store.flush_exports().unwrap();
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
fn directory_mutations_treat_canonical_files_as_authority_over_a_stale_index() {
    let root = std::env::temp_dir().join(format!("daena-canonical-first-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Canonical name".into(),
            entity_type: None,
        })
        .unwrap();

    // Simulate a stale disposable projection. A repository-first update
    // must seed its proposal from canonical files, not this SQLite row.
    store
        .connection
        .execute(
            "UPDATE entities SET name='SQLite-only name' WHERE id=?1",
            params![entity.id],
        )
        .unwrap();
    let updated = store
        .update_entity_with_options(
            entity.id.clone(),
            Some("Canonical update".into()),
            None,
            None,
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    assert_eq!(updated.name, "Canonical update");

    drop(store);
    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(
        reopened.list_entities().unwrap()[0].name,
        "Canonical update"
    );
    drop(reopened);
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
    store.flush_exports().unwrap();
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
fn directory_mutation_records_completed_export_batch() {
    let root = std::env::temp_dir().join(format!("daena-export-batch-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    store
        .create_entity(CreateEntity {
            name: "Exported record".into(),
            entity_type: None,
        })
        .unwrap();
    store.flush_exports().unwrap();

    let batch_state: String = store
        .connection
        .query_row(
            "SELECT state FROM sync_batches ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let sync_state: String = store
        .connection
        .query_row(
            "SELECT sync_state FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(batch_state, "completed");
    assert_eq!(sync_state, "clean");
    assert!(root.join(".daena/sync").is_dir());
    assert!(!root.join(".daena/transactions").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unchanged_export_item_is_applied() {
    let root = std::env::temp_dir().join(format!("daena-unchanged-export-{}", Uuid::new_v4()));
    let store = ProjectStore::open_directory(&root).unwrap();
    let entity = store
        .create_entity(CreateEntity {
            name: "Unchanged export".into(),
            entity_type: None,
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id.clone(),
            body: "Same body\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store
        .save_document(SaveDocument {
            entity_id: entity.id,
            body: "Same body\n".into(),
            format: Some("markdown".into()),
        })
        .unwrap();
    store.flush_exports().unwrap();

    let batch_state: String = store
        .connection
        .query_row(
            "SELECT state FROM sync_batches ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let sync_state: String = store
        .connection
        .query_row(
            "SELECT sync_state FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(batch_state, "completed");
    assert_eq!(sync_state, "clean");
    drop(store);
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
        .import_json_with_mode(&source.export_json().unwrap(), false)
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
    let mut target = ProjectStore::in_memory().unwrap();
    target.import_json_with_mode(&payload, false).unwrap();
    target.import_json_with_mode(&payload, false).unwrap();
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
    store.flush_exports().unwrap();
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
        .import_json_with_mode(&source.export_json().unwrap(), false)
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

    assert_eq!(store.seed_example().unwrap(), 26);
    assert_eq!(store.seed_example().unwrap(), 26);
    let entities = store.list_entities().unwrap();
    assert_eq!(entities.len(), 26);
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
    let map = entities
        .iter()
        .find(|entity| entity.entity_type.as_deref() == Some(crate::maps::MAP_ENTITY_TYPE))
        .expect("seed example ships a map entity");
    assert_eq!(map.name, "The Known Coast");
    assert_eq!(store.list_map_recovery_copies(&map.id).unwrap().len(), 0);
    assert_eq!(
        store
            .map_locations_for_entity(
                entities
                    .iter()
                    .find(|e| e.name == "Eldermere")
                    .unwrap()
                    .id
                    .clone()
            )
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn seed_example_survives_reopen() {
    let root = std::env::temp_dir().join(format!("daena-seed-example-{}", Uuid::new_v4()));
    let mut store = ProjectStore::open_directory(&root).unwrap();

    assert_eq!(store.seed_example().unwrap(), 26);
    let entities = store.list_entities().unwrap();
    assert_eq!(entities.len(), 26);
    let map = entities
        .iter()
        .find(|entity| entity.entity_type.as_deref() == Some(crate::maps::MAP_ENTITY_TYPE))
        .expect("seed example ships a map entity");
    assert_eq!(map.name, "The Known Coast");
    assert_eq!(
        store
            .map_locations_for_entity(
                entities
                    .iter()
                    .find(|e| e.name == "Eldermere")
                    .unwrap()
                    .id
                    .clone()
            )
            .unwrap()
            .len(),
        1
    );

    drop(store);
    let reopened = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(reopened.list_entities().unwrap().len(), 26);
    assert_eq!(
        reopened
            .list_entities()
            .unwrap()
            .iter()
            .filter(|entity| entity.entity_type.as_deref() == Some(crate::maps::MAP_ENTITY_TYPE))
            .count(),
        1
    );
    drop(reopened);

    let mut again = ProjectStore::open_directory(&root).unwrap();
    assert_eq!(again.seed_example().unwrap(), 26);
    assert_eq!(again.list_entities().unwrap().len(), 26);
    drop(again);
    std::fs::remove_dir_all(&root).unwrap();
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
    assert_eq!(manifest.format_version, 2);
    assert_eq!(manifest.name, root.file_name().unwrap().to_string_lossy());
    assert_eq!(
        std::fs::read_to_string(root.join(".gitignore")).unwrap(),
        ".daena/\n"
    );
    assert!(root.join("assets/images").is_dir());
    assert!(root.join("assets/videos").is_dir());
    assert!(root.join("assets/maps").is_dir());
    assert!(root.join("assets/files").is_dir());
    drop(store);
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
    store.flush_exports().unwrap();
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
    first.flush_exports().unwrap();
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
    first.flush_exports().unwrap();
    let canonical_before = canonical_files(&root);
    let search_before = first
        .search("Canonical prose".into())
        .unwrap()
        .into_iter()
        .map(|entity| entity.id)
        .collect::<Vec<_>>();
    let source_count_before: i64 = first
        .connection
        .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))
        .unwrap();
    let source_hash: String = first
        .connection
        .query_row(
            "SELECT content_hash FROM source_files WHERE path=?1",
            params![format!("entities/{}/document.md", source.id)],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        source_hash,
        format!(
            "sha256:{}",
            digest_bytes(
                &std::fs::read(root.join("entities").join(&source.id).join("document.md")).unwrap()
            )
        )
    );
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
    let source_count_after: i64 = reopened
        .connection
        .query_row("SELECT COUNT(*) FROM source_files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(source_count_after, source_count_before);
    assert!(!root.join(".daena/index.sqlite.next").exists());
    let document_path = root.join("entities").join(&source.id).join("document.md");
    std::fs::write(&document_path, b"# External change\n").unwrap();
    assert!(reopened.search("Canonical prose".into()).is_ok());
    drop(reopened);
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

    let location_a = Uuid::new_v4().to_string();
    let location_b = Uuid::new_v4().to_string();
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
                        }
                    ]
                }),
                revision: String::new(),
            })
            .unwrap();

    store.flush_exports().unwrap();
    let canonical_before = canonical_files(&root);
    let projection_before = store.map_locations_for_entity(place.id.clone()).unwrap();
    assert_eq!(projection_before.len(), 2);
    assert!(projection_before
        .iter()
        .all(|location| location["resolution"] == "resolved"));
    let anchor_kinds = projection_before
        .iter()
        .map(|location| location["anchorKind"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(anchor_kinds, BTreeSet::from(["point", "provider-feature"]));
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
        rebuilt.map_locations_for_entity(place.id).unwrap(),
        projection_before
    );
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
    let invalid = store.set_field(FieldValue {
        entity_id: place.id,
        namespace: crate::maps::MAP_NAMESPACE.into(),
        key: "locations".into(),
        value: serde_json::json!({
            "schemaVersion": 1,
            "locations": [{
                "id": Uuid::new_v4(),
                "mapEntityId": Uuid::new_v4(),
                "role": "origin",
                "label": "Nowhere",
                "anchor": {"kind": "point", "point": [1.5, 0.5]},
                "validity": {"from": null, "to": null}
            }]
        }),
        revision: String::new(),
    });
    assert!(invalid.is_err());
    assert!(invalid.unwrap_err().to_string().contains("maps:"));
}

#[test]
fn map_location_projection_and_reconcile_track_feature_resolution() {
    let root = std::env::temp_dir().join(format!("daena-map-resolve-{}", Uuid::new_v4()));
    let source = std::env::temp_dir().join(format!("daena-map-resolve-src-{}.map", Uuid::new_v4()));
    std::fs::write(
        &source,
        br#"{"features": [{"kind": "burg", "id": "7", "x": 120, "y": 80}]}"#,
    )
    .unwrap();

    let store = ProjectStore::open_directory(&root).unwrap();
    let map = store
        .create_entity(CreateEntity {
            name: "Resolved map".into(),
            entity_type: Some(crate::maps::MAP_ENTITY_TYPE.into()),
        })
        .unwrap();
    let place = store
        .create_entity(CreateEntity {
            name: "Harbor".into(),
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

    let resolved_id = Uuid::new_v4().to_string();
    let missing_id = Uuid::new_v4().to_string();
    store
            .set_field(FieldValue {
                entity_id: place.id.clone(),
                namespace: crate::maps::MAP_NAMESPACE.into(),
                key: "locations".into(),
                value: serde_json::json!({
                    "schemaVersion": 1,
                    "locations": [
                        {
                            "id": resolved_id,
                            "mapEntityId": map.id,
                            "role": "birthplace",
                            "label": "Old Harbor",
                            "anchor": {"kind": "provider-feature", "provider": "azgaar-fmg", "featureKind": "burg", "featureId": "7", "fallbackPoint": [0.6, 0.4]},
                            "validity": {"from": null, "to": null}
                        },
                        {
                            "id": missing_id,
                            "mapEntityId": map.id,
                            "role": "haven",
                            "label": "Lost Harbor",
                            "anchor": {"kind": "provider-feature", "provider": "azgaar-fmg", "featureKind": "burg", "featureId": "999", "fallbackPoint": [0.2, 0.8]},
                            "validity": {"from": null, "to": null}
                        }
                    ]
                }),
                revision: String::new(),
            })
            .unwrap();

    store.flush_exports().unwrap();
    let projection = store.map_location_projection(map.id.clone()).unwrap();
    assert_eq!(projection.len(), 2);
    let resolved = projection
        .iter()
        .find(|location| location["id"] == serde_json::Value::String(resolved_id.clone()))
        .unwrap();
    assert_eq!(resolved["resolution"], "resolved");
    assert_eq!(resolved["featureKind"], "burg");
    assert_eq!(resolved["featureId"], "7");
    let missing = projection
        .iter()
        .find(|location| location["id"] == serde_json::Value::String(missing_id.clone()))
        .unwrap();
    assert_eq!(missing["resolution"], "unresolved");
    assert_eq!(missing["featureId"], "999");
    assert!(missing["bounds"].is_array());

    let reconciled = store.reconcile_map_links(map.id.clone()).unwrap();
    assert_eq!(reconciled.len(), 2);
    let by_id: BTreeMap<&str, bool> = reconciled
        .iter()
        .map(|row| {
            (
                row["locationId"].as_str().unwrap(),
                row["resolved"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(by_id.get(resolved_id.as_str()), Some(&true));
    assert_eq!(by_id.get(missing_id.as_str()), Some(&false));

    // Renumbering the feature must flip resolution without touching the
    // canonical location field: no silent retargeting. The asset file is a
    // project copy, so replace it through the revision-aware mutation.
    let renumbered = br#"{"features": [{"kind": "burg", "id": "999", "x": 120, "y": 80}]}"#;
    store
        .replace_asset_bytes_with_request(
            AssetReplaceInput {
                asset_id: asset.id.clone(),
                content_hash: format!("sha256:{}", digest_bytes(renumbered)),
                size: renumbered.len() as i64,
                mime_type: "application/x-fmg-map".into(),
            },
            renumbered.to_vec(),
            &asset.revision,
            None,
        )
        .unwrap();
    store.flush_exports().unwrap();
    let after = store.reconcile_map_links(map.id.clone()).unwrap();
    let by_id_after: BTreeMap<&str, bool> = after
        .iter()
        .map(|row| {
            (
                row["locationId"].as_str().unwrap(),
                row["resolved"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(by_id_after.get(missing_id.as_str()), Some(&true));
    assert_eq!(by_id_after.get(resolved_id.as_str()), Some(&false));
    assert_eq!(
        store.map_locations(place.id).unwrap().len(),
        2,
        "canonical locations must be untouched by reconciliation"
    );
    drop(store);
    std::fs::remove_file(source).unwrap();
    std::fs::remove_dir_all(root).unwrap();
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
    store.flush_exports().unwrap();
    std::fs::remove_file(&source).unwrap();
    let before = store.list_map_recovery_copies(&map.id).unwrap();
    assert!(before.is_empty());
    let first_path = store.save_map_recovery_copy(&map.id, b"draft-v1").unwrap();
    let second_path = store.save_map_recovery_copy(&map.id, b"draft-v2").unwrap();
    assert!(first_path.starts_with(".daena/conflicts/maps/") && first_path.ends_with(".map"));
    assert!(second_path.starts_with(".daena/conflicts/maps/") && second_path.ends_with(".map"));
    assert_eq!(std::fs::read(root.join(&second_path)).unwrap(), b"draft-v2");
    assert!(!store
        .canonical_source_hashes(".daena/conflicts/%")
        .unwrap()
        .iter()
        .any(|(source, _)| source.contains("conflicts")));

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
    store.flush_exports().unwrap();
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

    store.flush_exports().unwrap();
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
