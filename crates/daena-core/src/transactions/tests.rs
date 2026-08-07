use super::*;

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("daena-{name}-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn writer_lock_is_exclusive_and_released() {
    let root = test_root("writer-lock");
    ensure_transaction_directories(&root).unwrap();
    let first = WriterLock::acquire(&root).unwrap();
    assert!(matches!(
        WriterLock::acquire(&root),
        Err(CoreError::Conflict(message)) if message.contains("writer lock")
    ));
    drop(first);
    let second = WriterLock::acquire(&root).unwrap();
    drop(second);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn committed_request_ids_are_idempotent() {
    let root = test_root("idempotent");
    fs::write(root.join("record.txt"), b"old").unwrap();
    let request_id = Uuid::new_v4().to_string();
    let mut transaction = match FileTransaction::begin(&root, &request_id).unwrap() {
        TransactionStart::Ready(transaction) => transaction,
        TransactionStart::AlreadyCommitted => panic!("request is unexpectedly committed"),
    };
    transaction.stage_bytes("record.txt", b"new").unwrap();
    transaction
        .commit_with_result(&serde_json::json!({"value": "new"}))
        .unwrap();
    assert_eq!(fs::read(root.join("record.txt")).unwrap(), b"new");
    assert_eq!(
        committed_result(&root, &request_id).unwrap(),
        Some(serde_json::json!({"value": "new"}))
    );
    assert!(matches!(
        FileTransaction::begin(&root, &request_id).unwrap(),
        TransactionStart::AlreadyCommitted
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_rolls_forward_after_every_durable_transaction_step() {
    for failure in [
        FailurePoint::AfterJournal,
        FailurePoint::AfterReplacement(0),
        FailurePoint::AfterReplacement(1),
        FailurePoint::AfterReceipt,
    ] {
        let root = test_root("recovery-step");
        fs::write(root.join("first.txt"), b"first-old").unwrap();
        fs::write(root.join("second.txt"), b"second-old").unwrap();
        let request_id = Uuid::new_v4().to_string();
        let mut transaction = match FileTransaction::begin(&root, &request_id).unwrap() {
            TransactionStart::Ready(transaction) => transaction,
            TransactionStart::AlreadyCommitted => panic!("request is unexpectedly committed"),
        };
        transaction.stage_bytes("first.txt", b"first-new").unwrap();
        transaction
            .stage_bytes("second.txt", b"second-new")
            .unwrap();
        transaction.commit_with_failure(failure).unwrap_err();
        drop(transaction);
        recover_transactions(&root).unwrap();
        assert_eq!(fs::read(root.join("first.txt")).unwrap(), b"first-new");
        assert_eq!(fs::read(root.join("second.txt")).unwrap(), b"second-new");
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn recovery_rolls_forward_after_durable_journal() {
    let root = test_root("recovery");
    fs::write(root.join("first.txt"), b"first-old").unwrap();
    fs::write(root.join("second.txt"), b"second-old").unwrap();
    let request_id = Uuid::new_v4().to_string();
    let mut transaction = match FileTransaction::begin(&root, &request_id).unwrap() {
        TransactionStart::Ready(transaction) => transaction,
        TransactionStart::AlreadyCommitted => panic!("request is unexpectedly committed"),
    };
    transaction.stage_bytes("first.txt", b"first-new").unwrap();
    transaction
        .stage_bytes("second.txt", b"second-new")
        .unwrap();
    transaction
        .commit_with_failure(FailurePoint::AfterReplacement(0))
        .unwrap_err();
    drop(transaction);

    recover_transactions(&root).unwrap();
    assert_eq!(fs::read(root.join("first.txt")).unwrap(), b"first-new");
    assert_eq!(fs::read(root.join("second.txt")).unwrap(), b"second-new");
    assert!(!root.join(TRANSACTION_ROOT).join(&request_id).exists());
    assert!(root
        .join(COMMITTED_ROOT)
        .join(format!("{request_id}.json"))
        .is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_never_leaves_a_mixed_remove_and_replace_mutation() {
    for failure in [
        FailurePoint::AfterJournal,
        FailurePoint::AfterReplacement(0),
        FailurePoint::AfterReplacement(1),
        FailurePoint::AfterReceipt,
    ] {
        let root = test_root("recovery-remove-replace");
        fs::write(root.join("removed.txt"), b"old").unwrap();
        fs::write(root.join("changed.txt"), b"old").unwrap();
        let request_id = Uuid::new_v4().to_string();
        let mut transaction = match FileTransaction::begin(&root, &request_id).unwrap() {
            TransactionStart::Ready(transaction) => transaction,
            TransactionStart::AlreadyCommitted => panic!("request is unexpectedly committed"),
        };
        transaction.stage_remove("removed.txt").unwrap();
        transaction.stage_bytes("changed.txt", b"new").unwrap();
        transaction.commit_with_failure(failure).unwrap_err();
        drop(transaction);

        recover_transactions(&root).unwrap();
        assert!(!root.join("removed.txt").exists());
        assert_eq!(fs::read(root.join("changed.txt")).unwrap(), b"new");
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn map_asset_recovery_never_mixes_source_bytes_and_metadata() {
    for failure in [
        FailurePoint::AfterJournal,
        FailurePoint::AfterReplacement(0),
        FailurePoint::AfterReplacement(1),
        FailurePoint::AfterReceipt,
    ] {
        let root = test_root("recovery-map-asset");
        let source = root.join("assets/maps/world.map");
        let metadata = root.join("entities/map/asset.json");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(metadata.parent().unwrap()).unwrap();
        fs::write(&source, b"old-map-source").unwrap();
        fs::write(&metadata, br#"{"contentHash":"old-hash","size":15}"#).unwrap();

        let request_id = Uuid::new_v4().to_string();
        let mut transaction = match FileTransaction::begin(&root, &request_id).unwrap() {
            TransactionStart::Ready(transaction) => transaction,
            TransactionStart::AlreadyCommitted => panic!("request is unexpectedly committed"),
        };
        transaction
            .stage_bytes("assets/maps/world.map", b"new-map-source")
            .unwrap();
        transaction
            .stage_bytes(
                "entities/map/asset.json",
                br#"{"contentHash":"new-hash","size":15}"#,
            )
            .unwrap();
        transaction.commit_with_failure(failure).unwrap_err();
        drop(transaction);

        recover_transactions(&root).unwrap();
        assert_eq!(fs::read(source).unwrap(), b"new-map-source");
        assert_eq!(
            fs::read_to_string(metadata).unwrap(),
            r#"{"contentHash":"new-hash","size":15}"#
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn incomplete_staging_without_a_journal_is_discarded() {
    let root = test_root("staging-cleanup");
    let directory = root.join(TRANSACTION_ROOT).join(Uuid::new_v4().to_string());
    fs::create_dir_all(directory.join("new")).unwrap();
    fs::write(directory.join("new/record.txt"), b"orphan").unwrap();
    recover_transactions(&root).unwrap();
    assert!(!directory.exists());
    fs::remove_dir_all(root).unwrap();
}
