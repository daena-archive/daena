use super::*;
use std::fs;

#[test]
fn project_manifest_is_byte_stable_and_lexicographically_ordered() {
    let manifest = ProjectManifest {
        format_version: 3,
        id: "6f21a771-eec6-4833-9a56-89b5cfc8f126".into(),
        name: "Eldermere".into(),
        created_at: "2026-08-05T10:30:00Z".into(),
    };

    assert_eq!(
        canonical_json_bytes(&manifest).unwrap(),
        br#"{
  "createdAt": "2026-08-05T10:30:00Z",
  "formatVersion": 3,
  "id": "6f21a771-eec6-4833-9a56-89b5cfc8f126",
  "name": "Eldermere"
}
"#
    );
}

#[test]
fn manifest_rejects_unknown_fields_and_duplicate_keys() {
    let path = Path::new("project.json");
    assert!(parse_json::<ProjectManifest>(path, br#"{"formatVersion":3,"id":"6f21a771-eec6-4833-9a56-89b5cfc8f126","name":"E","createdAt":"now","extra":true}"#).is_err());
    assert!(parse_json::<ProjectManifest>(path, br#"{"formatVersion":3,"formatVersion":3,"id":"6f21a771-eec6-4833-9a56-89b5cfc8f126","name":"E","createdAt":"now"}"#).is_err());
}

#[test]
fn checkpoint_manifest_is_deterministic_and_rejects_tampering() {
    let root = std::env::temp_dir().join(format!("daena-checkpoint-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let project = ProjectManifest::new("Checkpoint");
    write_json(&root.join("project.json"), &project).unwrap();
    let checkpoint = build_checkpoint_manifest(&root, 7).unwrap();
    assert_eq!(checkpoint.content_generation, 7);
    assert_eq!(checkpoint.files.len(), 1);
    write_checkpoint_manifest(&root, &checkpoint).unwrap();
    validate_checkpoint(&root, &checkpoint).unwrap();
    let replacement = build_checkpoint_manifest(&root, 8).unwrap();
    write_checkpoint_manifest(&root, &replacement).unwrap();
    assert_eq!(
        read_json::<CheckpointManifest>(&root.join(CHECKPOINT_MANIFEST_FILE)).unwrap(),
        replacement
    );
    assert!(!fs::read_dir(&root).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".checkpoint.json.tmp-")
    }));
    std::fs::write(root.join("project.json"), b"tampered").unwrap();
    assert!(validate_checkpoint(&root, &replacement).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn entity_rejects_document_traversal() {
    let entity = EntityFile {
        id: "018f89df-b93e-7ad0-a07f-08b1441d1550".into(),
        name: "The Glass Coast".into(),
        entity_type: Some("place".into()),
        deleted: false,
        created_at: "now".into(),
        updated_at: "now".into(),
        document: Some(EntityDocumentRef {
            id: "018f89e1-3d7b-73bb-b7c1-c83de04102e1".into(),
            path: "../document.md".into(),
        }),
    };
    let error = entity
        .validate(Path::new("entities/id/entity.json"))
        .unwrap_err();
    assert!(error.to_string().contains("[entity.document-path]"));
}

#[test]
fn markdown_normalizes_line_endings_without_changing_canonical_bytes() {
    assert_eq!(
        canonical_markdown("# Title\r\n\r\nBody\n"),
        "# Title\n\nBody\n"
    );
    assert_eq!(
        canonical_markdown_bytes(Path::new("document.md"), b"Body\n").unwrap(),
        b"Body\n"
    );
}

#[test]
fn json_round_trips_through_a_canonical_file() {
    let root = std::env::temp_dir().join(format!("daena-codec-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("project.json");
    let manifest = ProjectManifest::new("Test");
    write_json(&path, &manifest).unwrap();
    assert_eq!(read_json::<ProjectManifest>(&path).unwrap(), manifest);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn canonical_scan_ignores_os_metadata_entries() {
    let root = std::env::temp_dir().join(format!("daena-metadata-{}", Uuid::new_v4()));
    fs::create_dir_all(root.join("entities")).unwrap();
    fs::create_dir_all(root.join("plugins")).unwrap();
    fs::write(root.join("entities/.DS_Store"), b"metadata").unwrap();
    fs::write(root.join("plugins/Thumbs.db"), b"metadata").unwrap();
    write_json(&root.join("project.json"), &ProjectManifest::new("Test")).unwrap();

    let canonical = read_canonical_project(&root).unwrap();
    assert!(canonical.snapshot.entities.is_empty());
    assert!(canonical.snapshot.modules.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn repository_rejects_symlinked_canonical_directories_before_creation() {
    let root = std::env::temp_dir().join(format!("daena-symlink-root-{}", Uuid::new_v4()));
    let outside = std::env::temp_dir().join(format!("daena-symlink-target-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("assets")).unwrap();

    let error = FilesystemRepository::open(&root).unwrap_err();
    assert!(error.to_string().contains("[path.symlink]"));
    assert!(!outside.join("images").exists());

    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}
