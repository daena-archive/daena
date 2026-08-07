use super::*;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;

fn archive(path: &Path, manifest: &str, extra: &[(&str, &[u8])]) {
    let file = File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(manifest.as_bytes()).unwrap();
    for (name, content) in extra {
        zip.start_file(*name, options).unwrap();
        zip.write_all(content).unwrap();
    }
    zip.finish().unwrap();
}

fn manifest(version: &str) -> String {
    format!(
        r#"{{"manifestVersion":1,"id":"com.example.test","name":"Test","version":"{version}","publisher":"com.example","hostApi":">=1.0.0 <2.0.0","kind":"declarative","entrypoints":{{"ui":"dist/index.html"}},"capabilities":[],"dependencies":{{}},"namespaces":[],"schemas":[],"templates":[],"views":[],"commands":[],"services":{{"provides":[],"consumes":[]}},"events":{{"publishes":[],"subscribes":[]}},"migrations":[]}}"#
    )
}

#[test]
fn rejects_traversal_and_missing_entrypoint() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.wbplugin");
    archive(&path, &manifest("1.0.0"), &[("../evil", b"x")]);
    assert!(verify_and_extract(
        &path,
        dir.path(),
        ArchiveLimits::default(),
        VerificationPolicy::with_unsigned_consent()
    )
    .is_err());
}

#[test]
fn unsigned_packages_require_explicit_consent() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("unsigned.wbplugin");
    archive(&path, &manifest("1.0.0"), &[("dist/index.html", b"ok")]);
    let error = verify_and_extract(
        &path,
        dir.path(),
        ArchiveLimits::default(),
        VerificationPolicy::default(),
    )
    .unwrap_err();
    assert!(error.0.contains("explicit install consent"));
    assert!(verify_and_extract(
        &path,
        dir.path(),
        ArchiveLimits::default(),
        VerificationPolicy::with_unsigned_consent(),
    )
    .is_ok());
}

#[test]
fn archive_entry_limit_is_checked_before_extraction() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("too-many.wbplugin");
    archive(&path, &manifest("1.0.0"), &[("dist/index.html", b"ok")]);
    let limits = ArchiveLimits {
        max_file_count: 1,
        ..ArchiveLimits::default()
    };
    let error = verify_and_extract(
        &path,
        dir.path(),
        limits,
        VerificationPolicy::with_unsigned_consent(),
    )
    .unwrap_err();
    assert!(error.0.contains("too many entries"));
}

#[test]
fn host_api_ranges_follow_semver_zero_major_rules() {
    assert!(host_api_compatible("^0.1.0", "0.1.9"));
    assert!(!host_api_compatible("^0.1.0", "0.2.0"));
    assert!(!host_api_compatible("not-a-range", "1.0.0"));
}

#[test]
fn signature_metadata_is_part_of_the_canonical_digest() {
    let files = vec![
        ("manifest.json".into(), b"manifest".to_vec()),
        (
            SIGNATURE_FILE.into(),
            br#"{"algorithm":"ed25519","publicKey":"key","signature":"sig","digest":"digest"}"#
                .to_vec(),
        ),
    ];
    let first = archive_digest(&files).unwrap();
    let changed = vec![
        files[0].clone(),
        (
            SIGNATURE_FILE.into(),
            br#"{"algorithm":"rsa","publicKey":"key","signature":"sig","digest":"digest"}"#
                .to_vec(),
        ),
    ];
    assert_ne!(first, archive_digest(&changed).unwrap());
}
#[test]
fn installs_atomically_and_retains_versions() {
    let dir = tempdir().unwrap();
    let mut catalog = PackageCatalog::default();
    for version in ["1.0.0", "1.1.0"] {
        let path = dir.path().join(format!("{version}.wbplugin"));
        archive(&path, &manifest(version), &[("dist/index.html", b"ok")]);
        catalog
            .install(
                path,
                dir.path().join("installed"),
                ArchiveLimits::default(),
                VerificationPolicy::with_unsigned_consent(),
            )
            .unwrap();
    }
    assert_eq!(catalog.list("com.example.test").count(), 2);
    assert_eq!(
        catalog
            .active_candidate("com.example.test")
            .unwrap()
            .version,
        "1.1.0"
    );
}

#[test]
fn rediscovery_rehashes_installed_packages_after_restart() {
    let dir = tempdir().unwrap();
    let archive_path = dir.path().join("1.0.0.wbplugin");
    archive(
        &archive_path,
        &manifest("1.0.0"),
        &[("dist/index.html", b"ok")],
    );
    let install_root = dir.path().join("installed");
    let mut first = PackageCatalog::default();
    first
        .install(
            &archive_path,
            &install_root,
            ArchiveLimits::default(),
            VerificationPolicy::with_unsigned_consent(),
        )
        .unwrap();
    let state_path = dir.path().join("plugin-state.json");
    first.persist(&state_path).unwrap();

    let mut restarted = PackageCatalog::load(&state_path).unwrap();
    let rejected = restarted
        .rediscover(
            &install_root,
            ArchiveLimits::default(),
            &VerificationPolicy::default(),
        )
        .unwrap();
    assert!(rejected.is_empty(), "{rejected:?}");
    assert_eq!(restarted.list("com.example.test").count(), 1);

    fs::write(
        install_root.join("com.example.test/1.0.0/dist/index.html"),
        b"tampered",
    )
    .unwrap();
    let mut tampered = PackageCatalog::load(&state_path).unwrap();
    let rejected = tampered
        .rediscover(
            &install_root,
            ArchiveLimits::default(),
            &VerificationPolicy::default(),
        )
        .unwrap();
    assert_eq!(rejected.len(), 1);
    assert!(tampered.list("com.example.test").next().is_none());
}

#[test]
fn failed_code_removal_keeps_catalog_entry() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("not-a-directory");
    fs::write(&root, b"plugin").unwrap();
    let mut catalog = PackageCatalog::default();
    catalog.versions.insert(
        "com.example.test".into(),
        [(
            "1.0.0".into(),
            InstalledVersion {
                plugin_id: "com.example.test".into(),
                version: "1.0.0".into(),
                digest: "digest".into(),
                root,
                publisher: "com.example".into(),
                signed: false,
                installed_at: 0,
                unsigned_consent: false,
            },
        )]
        .into_iter()
        .collect(),
    );
    assert!(catalog.remove_version("com.example.test", "1.0.0").is_err());
    assert!(catalog.get("com.example.test", "1.0.0").is_some());
}
#[test]
fn capability_escalation_requires_consent() {
    let consent = CapabilityConsent::compare(
        &["entity.read".into()],
        &["entity.read".into(), "entity.write".into()],
    );
    assert!(consent.requires_renewal);
}
#[test]
fn migration_selection_is_contiguous_and_hashed() {
    let mut value: PluginManifest = parse_manifest(&manifest("1.0.0")).unwrap();
    value.namespaces.push("data".into());
    value.migrations = vec![daena_plugin_api::Migration {
        id: "v1".into(),
        from: 0,
        to: 1,
        recovery: "backup".into(),
        operations: vec![daena_plugin_api::MigrationOperation::CreateNamespace {
            namespace: "data".into(),
        }],
    }];
    let plan = select_migrations(&value, 0).unwrap();
    assert_eq!(plan.to, 1);
    assert_eq!(plan.checksums.len(), 1);
    assert!(plan.requires_backup);
}
