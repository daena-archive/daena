use super::*;
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
    assert_eq!(metadata.0, 5);
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
    assert!(run_git(&root, &["push", "-q", "-u", "origin", &branch])
        .status
        .success());

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

    let reset = store.git_reset_hard(&base).unwrap();
    assert!(reset.diverged_from_upstream);
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
        .list_module_records(
            "daena.language",
            "lexemes",
            &other.id,
            None,
            50,
            0,
        )
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
        by_status.iter().map(|record| &record.value).collect::<Vec<_>>()
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
            .list_module_records(
                "daena.language",
                "lexemes",
                &language.id,
                None,
                50,
                0,
            )
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
fn language_grammar_topics_round_trip_and_sort_by_section_title() {
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
                "title": "Verb stems",
                "section": "verb",
                "body": "See [[sol]](lexeme:PLACEHOLDER).",
                "links": [{ "id": "l1", "kind": "lexeme", "lexemeId": lexeme.id, "label": "sol" }]
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
                "title": "Basic order",
                "section": "word-order",
                "body": "SVO.",
                "links": []
            }),
            Some(&Uuid::new_v4().to_string()),
        )
        .unwrap();
    let topics = store
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
    assert_eq!(
        topics
            .iter()
            .map(|record| record.value["title"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["Basic order", "Verb stems"]
    );
    assert_eq!(
        store
            .list_module_records(
                "daena.language",
                "grammar",
                &language.id,
                Some("SVO"),
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
    assert_eq!(
        rebuilt
            .list_module_records("daena.language", "grammar", &language.id, None, 50, 0)
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
    let target = ProjectStore::in_memory().unwrap();
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
    let second_revision = store.list_documents(entity.id).unwrap()[0]
        .revision
        .clone();

    assert_eq!(first_revision, second_revision);
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
    let destination = std::env::temp_dir().join(format!("daena-markdown-destination-{}", Uuid::new_v4()));
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
    let destination = std::env::temp_dir().join(format!("daena-markdown-collision-{}", Uuid::new_v4()));
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
    assert!(export.join(format!("Twin-{}.md", &second.id[..8])).is_file());
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
        .filter(|relationship| relationship.relationship_type == crate::maps::DETAIL_MAP_RELATIONSHIP)
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
        .filter(|relationship| relationship.relationship_type == crate::maps::DETAIL_MAP_RELATIONSHIP)
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
            "selector": {"entityTypes": ["place"]}
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
