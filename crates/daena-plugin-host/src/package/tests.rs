use super::*;
use ed25519_dalek::{Signer, SigningKey};
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

fn manifest_with_svg_icon(version: &str) -> String {
    manifest(version).replace(
        r#""schemas":[]"#,
        r#""schemas":[{"namespace":"icons","entityTypes":[{"id":"note","name":"Note","icon":{"kind":"plugin-svg","path":"icons/note.svg"},"iconColor":{"kind":"preset","id":"slate"}}],"fields":[]}]"#,
    ).replace(r#""namespaces":[]"#, r#""namespaces":["icons"]"#)
}

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn public_key_b64(key: &SigningKey) -> String {
    BASE64.encode(key.verifying_key().as_bytes())
}

fn signed_archive(
    path: &Path,
    version: &str,
    extra: &[(&str, &[u8])],
    key: &SigningKey,
    key_id: Option<&str>,
) {
    signed_archive_for_publisher(path, version, extra, key, key_id, "com.example");
}

fn signed_archive_for_publisher(
    path: &Path,
    version: &str,
    extra: &[(&str, &[u8])],
    key: &SigningKey,
    key_id: Option<&str>,
    publisher: &str,
) {
    let mut files: PackageFiles = vec![(
        "manifest.json".into(),
        manifest(version)
            .replace(
                "\"publisher\":\"com.example\"",
                &format!("\"publisher\":\"{publisher}\""),
            )
            .into_bytes(),
    )];
    for (name, content) in extra {
        files.push(((*name).into(), content.to_vec()));
    }
    let placeholder = PackageSignature {
        algorithm: "ed25519".into(),
        publisher: Some(publisher.into()),
        key_id: key_id.map(str::to_string),
        public_key: public_key_b64(key),
        signature: String::new(),
        digest: String::new(),
    };
    files.push((
        SIGNATURE_FILE.into(),
        serde_json::to_vec(&placeholder).unwrap(),
    ));
    let digest = archive_digest(&files).unwrap();
    let signature = BASE64.encode(key.sign(digest.as_bytes()).to_bytes());
    let signed = PackageSignature {
        digest,
        signature,
        ..placeholder
    };
    files.last_mut().unwrap().1 = serde_json::to_vec(&signed).unwrap();
    let file = File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default();
    for (name, content) in &files {
        zip.start_file(name, options).unwrap();
        zip.write_all(content).unwrap();
    }
    zip.finish().unwrap();
}

fn pinned_policy(key: &SigningKey, key_id: &str) -> VerificationPolicy {
    VerificationPolicy {
        require_signature: true,
        trust: TrustSnapshot {
            publishers: BTreeMap::from([(
                "com.example".into(),
                PublisherIdentity {
                    keys: vec![PublisherKey {
                        key_id: key_id.into(),
                        public_key: public_key_b64(key),
                    }],
                },
            )]),
            ..TrustSnapshot::default()
        },
        ..VerificationPolicy::default()
    }
}

#[test]
fn passive_plugin_svg_profile_accepts_geometry_and_rejects_active_content() {
    validate_icon_svg(
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 4h16v16H4z"/></svg>"#,
    )
    .unwrap();

    for unsafe_svg in [
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><script/></svg>"#.as_slice(),
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path style="fill:red" d="M0 0"/></svg>"#.as_slice(),
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="url(https://example.test/a.svg)" d="M0 0"/></svg>"#.as_slice(),
        br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><text>secret</text></svg>"#.as_slice(),
    ] {
        assert!(validate_icon_svg(unsafe_svg).is_err());
    }
}

#[test]
fn package_install_requires_declared_svg_icons_to_exist_and_be_passive() {
    let dir = tempdir().unwrap();
    let missing_path = dir.path().join("missing-icon.daenaplugin");
    archive(
        &missing_path,
        &manifest_with_svg_icon("1.0.0"),
        &[("dist/index.html", b"ok")],
    );
    let missing_error = verify_and_extract(
        &missing_path,
        &dir.path().join("missing-install"),
        ArchiveLimits::default(),
        VerificationPolicy::with_unsigned_consent(),
    )
    .unwrap_err();
    assert!(missing_error.0.contains("manifest SVG icon is missing"));

    let active_path = dir.path().join("active-icon.daenaplugin");
    archive(
        &active_path,
        &manifest_with_svg_icon("1.0.1"),
        &[
            ("dist/index.html", b"ok"),
            (
                "icons/note.svg",
                br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><script/></svg>"#,
            ),
        ],
    );
    let active_error = verify_and_extract(
        &active_path,
        &dir.path().join("active-install"),
        ArchiveLimits::default(),
        VerificationPolicy::with_unsigned_consent(),
    )
    .unwrap_err();
    assert!(active_error.0.contains("invalid manifest SVG icon"));
}

#[test]
fn malformed_archives_fail_closed_without_panic() {
    let dir = tempdir().unwrap();
    let valid = dir.path().join("ok.daenaplugin");
    archive(&valid, &manifest("1.0.0"), &[("dist/index.html", b"ok")]);
    let bytes = std::fs::read(&valid).unwrap();
    for (index, chunk) in bytes.chunks(16).enumerate() {
        let mut mutated = bytes.clone();
        let offset = index * 16;
        mutated[offset] ^= 0xff;
        if chunk.len() > 1 {
            mutated[offset + chunk.len() - 1] ^= 0xaa;
        }
        let path = dir.path().join(format!("mut-{index}.daenaplugin"));
        std::fs::write(&path, &mutated).unwrap();
        let _ = verify_and_extract(
            &path,
            &dir.path().join(format!("out-{index}")),
            ArchiveLimits::default(),
            VerificationPolicy::with_unsigned_consent(),
        );
    }
    for len in [0_usize, 1, 4, bytes.len() / 2] {
        let path = dir.path().join(format!("trunc-{len}.daenaplugin"));
        std::fs::write(&path, &bytes[..len]).unwrap();
        assert!(verify_and_extract(
            &path,
            &dir.path().join(format!("trunc-out-{len}")),
            ArchiveLimits::default(),
            VerificationPolicy::with_unsigned_consent(),
        )
        .is_err());
    }
}

#[test]
fn rejects_wbplugin_extension() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.wbplugin");
    archive(&path, &manifest("1.0.0"), &[("dist/index.html", b"ok")]);
    let error = verify_and_extract(
        &path,
        dir.path(),
        ArchiveLimits::default(),
        VerificationPolicy::with_unsigned_consent(),
    )
    .unwrap_err();
    assert!(error.0.contains(".daenaplugin"));
    assert!(error.0.contains(".wbplugin is not accepted"));
}

#[test]
fn rejects_traversal_and_missing_entrypoint() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.daenaplugin");
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
    let path = dir.path().join("unsigned.daenaplugin");
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
    let path = dir.path().join("too-many.daenaplugin");
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
        let path = dir.path().join(format!("{version}.daenaplugin"));
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
    let archive_path = dir.path().join("1.0.0.daenaplugin");
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

#[test]
fn cargo_fuzz_verify_archive_corpus_does_not_panic() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/corpus/verify_archive");
    let limits = ArchiveLimits {
        max_compressed_bytes: 64 * 1024,
        max_uncompressed_bytes: 256 * 1024,
        max_file_count: 64,
        max_path_length: 256,
        max_file_bytes: 64 * 1024,
    };
    let mut count = 0usize;
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|_| panic!("missing fuzz corpus {dir:?}")) {
        let path = entry.unwrap().path();
        if !path.is_file()
            || path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let result =
            verify_archive_bytes(&bytes, limits, &VerificationPolicy::with_unsigned_consent());
        let name = path.file_name().unwrap().to_string_lossy();
        if name == "valid-unsigned" {
            assert!(result.is_ok(), "{name} should verify: {result:?}");
        } else {
            assert!(result.is_err(), "{name} should fail closed");
        }
        count += 1;
    }
    assert!(count > 0, "fuzz corpus verify_archive is empty");
}

#[test]
fn publisher_key_rotation_accepts_current_and_previous_keys() {
    let dir = tempdir().unwrap();
    let previous = signing_key(1);
    let current = signing_key(2);
    let mut policy = pinned_policy(&current, "2026-02");
    policy.trust.publishers.get_mut("com.example").unwrap().keys = vec![
        PublisherKey {
            key_id: "2026-01".into(),
            public_key: public_key_b64(&previous),
        },
        PublisherKey {
            key_id: "2026-02".into(),
            public_key: public_key_b64(&current),
        },
    ];
    for (seed, key_id, key) in [("old", "2026-01", &previous), ("new", "2026-02", &current)] {
        let path = dir.path().join(format!("{seed}.daenaplugin"));
        signed_archive(
            &path,
            "1.0.0",
            &[("dist/index.html", b"ok")],
            key,
            Some(key_id),
        );
        let bytes = std::fs::read(&path).unwrap();
        let verified = verify_archive_bytes(&bytes, ArchiveLimits::default(), &policy).unwrap();
        assert!(verified.signed);
        assert_eq!(
            review_package(&verified, &policy.trust).trust_status,
            TrustStatus::TrustedPublisher
        );
    }
}

#[test]
fn revoked_key_and_digest_fail_closed() {
    let dir = tempdir().unwrap();
    let key = signing_key(3);
    let path = dir.path().join("signed.daenaplugin");
    signed_archive(
        &path,
        "1.0.0",
        &[("dist/index.html", b"ok")],
        &key,
        Some("2026-01"),
    );
    let bytes = std::fs::read(&path).unwrap();
    let verified = verify_archive_bytes(
        &bytes,
        ArchiveLimits::default(),
        &VerificationPolicy::with_unsigned_consent(),
    )
    .unwrap();
    let mut revoked_key = VerificationPolicy::with_unsigned_consent();
    revoked_key.trust.revocations.keys.push(RevokedKey {
        publisher: "com.example".into(),
        key_id: Some("2026-01".into()),
        public_key: Some(public_key_b64(&key)),
    });
    let key_error =
        verify_archive_bytes(&bytes, ArchiveLimits::default(), &revoked_key).unwrap_err();
    assert!(key_error.0.contains("signing key is revoked"));
    let mut revoked_digest = VerificationPolicy::with_unsigned_consent();
    revoked_digest
        .trust
        .revocations
        .digests
        .push(verified.digest.clone());
    let digest_error =
        verify_archive_bytes(&bytes, ArchiveLimits::default(), &revoked_digest).unwrap_err();
    assert!(digest_error.0.contains("digest is revoked"));
}

#[test]
fn local_file_and_registry_bytes_produce_the_same_review() {
    let dir = tempdir().unwrap();
    let key = signing_key(4);
    let path = dir.path().join("plugin.daenaplugin");
    signed_archive(
        &path,
        "1.2.0",
        &[("dist/index.html", b"ok")],
        &key,
        Some("2026-02"),
    );
    let policy = pinned_policy(&key, "2026-02");
    let local = verify_and_extract(
        &path,
        &dir.path().join("local"),
        ArchiveLimits::default(),
        policy.clone(),
    )
    .unwrap();
    let registry = verify_archive_bytes(
        &std::fs::read(&path).unwrap(),
        ArchiveLimits::default(),
        &policy,
    )
    .unwrap();
    assert_eq!(local.digest, registry.digest);
    assert_eq!(local.signed, registry.signed);
    assert_eq!(local.manifest.publisher, registry.manifest.publisher);
    assert_eq!(
        review_manifest(
            &local.manifest,
            &local.digest,
            local.signature.as_ref(),
            local.signed,
            &policy.trust
        ),
        review_package(&registry, &policy.trust)
    );
}

#[test]
fn unsigned_local_install_still_works_with_consent() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("unsigned.daenaplugin");
    archive(&path, &manifest("1.0.0"), &[("dist/index.html", b"ok")]);
    let bytes = std::fs::read(&path).unwrap();
    let policy = VerificationPolicy::from_install_root(dir.path(), true).unwrap();
    let verified = verify_archive_bytes(&bytes, ArchiveLimits::default(), &policy).unwrap();
    assert!(!verified.signed);
    assert_eq!(
        review_package(&verified, &policy.trust).trust_status,
        TrustStatus::Unsigned
    );
}

#[test]
fn trust_snapshot_unknown_schema_and_unknown_publisher_fail_closed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(TRUST_SNAPSHOT_FILE);
    std::fs::write(
        &path,
        r#"{"schemaVersion":2,"publishers":{},"revocations":{}}"#,
    )
    .unwrap();
    let error = TrustSnapshot::load(&path).unwrap_err();
    assert!(error
        .0
        .contains("unsupported trust snapshot schema version"));
    let key = signing_key(5);
    let archive_path = dir.path().join("other.daenaplugin");
    signed_archive_for_publisher(
        &archive_path,
        "1.0.0",
        &[("dist/index.html", b"ok")],
        &key,
        Some("2026-01"),
        "com.other",
    );
    let policy = pinned_policy(&signing_key(9), "pinned");
    let error = verify_archive_bytes(
        &std::fs::read(&archive_path).unwrap(),
        ArchiveLimits::default(),
        &policy,
    )
    .unwrap_err();
    assert!(error.0.contains("publisher is not trusted"));
}

#[test]
fn key_revocation_matches_public_key_even_without_key_id() {
    let dir = tempdir().unwrap();
    let key = signing_key(6);
    let path = dir.path().join("signed.daenaplugin");
    signed_archive(&path, "1.0.0", &[("dist/index.html", b"ok")], &key, None);
    let mut policy = VerificationPolicy::with_unsigned_consent();
    policy.trust.revocations.keys.push(RevokedKey {
        publisher: "com.example".into(),
        key_id: Some("2026-01".into()),
        public_key: Some(public_key_b64(&key)),
    });
    let error = verify_archive_bytes(
        &std::fs::read(&path).unwrap(),
        ArchiveLimits::default(),
        &policy,
    )
    .unwrap_err();
    assert!(error.0.contains("signing key is revoked"));
}

#[test]
fn key_id_only_revocation_is_rejected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join(TRUST_SNAPSHOT_FILE);
    std::fs::write(
        &path,
        r#"{"schemaVersion":1,"publishers":{},"revocations":{"keys":[{"publisher":"com.example","keyId":"2026-01"}],"digests":[],"packages":[]}}"#,
    )
    .unwrap();
    let error = TrustSnapshot::load(&path).unwrap_err();
    assert!(error.0.contains("key revocation must include publicKey"));
}

#[test]
fn revoked_package_identity_fails_closed_for_unsigned_bytes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("unsigned.daenaplugin");
    archive(&path, &manifest("1.0.0"), &[("dist/index.html", b"ok")]);
    let bytes = std::fs::read(&path).unwrap();
    let mut policy = VerificationPolicy::with_unsigned_consent();
    policy.trust.revocations.packages.push(RevokedPackage {
        plugin_id: "com.example.test".into(),
        version: Some("1.0.0".into()),
    });
    let identity_error =
        verify_archive_bytes(&bytes, ArchiveLimits::default(), &policy).unwrap_err();
    assert!(identity_error.0.contains("package identity is revoked"));
    let digest = verify_archive_bytes(
        &bytes,
        ArchiveLimits::default(),
        &VerificationPolicy::with_unsigned_consent(),
    )
    .unwrap()
    .digest;
    let mut revoked_digest = VerificationPolicy::with_unsigned_consent();
    revoked_digest.trust.revocations.digests.push(digest);
    let digest_error =
        verify_archive_bytes(&bytes, ArchiveLimits::default(), &revoked_digest).unwrap_err();
    assert!(digest_error.0.contains("digest is revoked"));
}

#[test]
fn rotated_key_survives_after_previous_key_is_revoked() {
    let dir = tempdir().unwrap();
    let previous = signing_key(7);
    let current = signing_key(8);
    let mut policy = pinned_policy(&current, "2026-02");
    policy.trust.publishers.get_mut("com.example").unwrap().keys = vec![PublisherKey {
        key_id: "2026-02".into(),
        public_key: public_key_b64(&current),
    }];
    policy.trust.revocations.keys.push(RevokedKey {
        publisher: "com.example".into(),
        key_id: Some("2026-01".into()),
        public_key: Some(public_key_b64(&previous)),
    });
    let old_path = dir.path().join("old.daenaplugin");
    signed_archive(
        &old_path,
        "1.0.0",
        &[("dist/index.html", b"ok")],
        &previous,
        Some("2026-01"),
    );
    let new_path = dir.path().join("new.daenaplugin");
    signed_archive(
        &new_path,
        "1.1.0",
        &[("dist/index.html", b"ok")],
        &current,
        Some("2026-02"),
    );
    let old_error = verify_archive_bytes(
        &std::fs::read(&old_path).unwrap(),
        ArchiveLimits::default(),
        &policy,
    )
    .unwrap_err();
    assert!(old_error.0.contains("signing key is revoked"));
    let verified = verify_archive_bytes(
        &std::fs::read(&new_path).unwrap(),
        ArchiveLimits::default(),
        &policy,
    )
    .unwrap();
    assert_eq!(
        review_package(&verified, &policy.trust).trust_status,
        TrustStatus::TrustedPublisher
    );
}

#[test]
fn from_install_root_applies_on_disk_revocations() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("unsigned.daenaplugin");
    archive(&path, &manifest("1.0.0"), &[("dist/index.html", b"ok")]);
    let bytes = std::fs::read(&path).unwrap();
    let digest = verify_archive_bytes(
        &bytes,
        ArchiveLimits::default(),
        &VerificationPolicy::with_unsigned_consent(),
    )
    .unwrap()
    .digest;
    std::fs::write(
        dir.path().join(TRUST_SNAPSHOT_FILE),
        format!(
            r#"{{"schemaVersion":1,"publishers":{{}},"revocations":{{"keys":[],"digests":["{digest}"],"packages":[]}}}}"#
        ),
    )
    .unwrap();
    let policy = VerificationPolicy::from_install_root(dir.path(), true).unwrap();
    let error = verify_archive_bytes(&bytes, ArchiveLimits::default(), &policy).unwrap_err();
    assert!(error.0.contains("digest is revoked"));
}
