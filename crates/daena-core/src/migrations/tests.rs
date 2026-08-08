use super::*;

fn schema_connection() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE module_versions(module_id TEXT PRIMARY KEY, version INTEGER NOT NULL); CREATE TABLE module_namespaces(module_id TEXT NOT NULL, namespace TEXT NOT NULL, PRIMARY KEY(module_id, namespace)); CREATE TABLE module_fields(module_id TEXT NOT NULL, namespace TEXT NOT NULL, key TEXT NOT NULL, field_type TEXT NOT NULL, required INTEGER NOT NULL, PRIMARY KEY(module_id, namespace, key)); CREATE TABLE migration_history(module_id TEXT NOT NULL, migration_id TEXT NOT NULL, from_version INTEGER NOT NULL, to_version INTEGER NOT NULL, checksum TEXT NOT NULL, package_digest TEXT NOT NULL DEFAULT '', applied_at TEXT NOT NULL DEFAULT '', PRIMARY KEY(module_id, migration_id));").unwrap();
    connection
}

fn migration() -> Migration {
    Migration {
        id: "lore-v1".into(),
        module_id: "daena.lore".into(),
        from: 0,
        to: 1,
        operations: vec![Operation::CreateNamespace {
            namespace: "lore".into(),
        }],
        recovery: "backup".into(),
        package_digest: String::new(),
    }
}

fn apply(connection: &mut Connection, migration: &Migration) -> Result<(), CoreError> {
    let transaction = connection.transaction().unwrap();
    apply_in_transaction(&transaction, migration)?;
    transaction.commit().map_err(CoreError::from)
}

#[test]
fn rejects_invalid_field_operations() {
    let mut migration = migration();
    migration.operations = vec![Operation::AddField {
        namespace: "lore".into(),
        field: FieldDefinition {
            key: "".into(),
            field_type: "text".into(),
            required: false,
        },
    }];
    assert!(validate(&migration, 0).is_err());
}

#[test]
fn applies_once_and_tracks_version() {
    let mut connection = schema_connection();
    let migration = migration();
    apply(&mut connection, &migration).unwrap();
    assert!(apply(&mut connection, &migration).is_err());
    assert_eq!(
        connection
            .query_row(
                "SELECT version FROM module_versions WHERE module_id='daena.lore'",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
}

#[test]
fn rejects_missing_and_duplicate_schema_fields() {
    let mut connection = schema_connection();
    let missing = Migration {
        operations: vec![
            Operation::CreateNamespace {
                namespace: "lore".into(),
            },
            Operation::RenameField {
                namespace: "lore".into(),
                from: "missing".into(),
                to: "renamed".into(),
            },
        ],
        ..migration()
    };
    assert!(apply(&mut connection, &missing).is_err());

    connection
        .execute(
            "INSERT INTO module_namespaces(module_id, namespace) VALUES ('daena.lore', 'lore')",
            [],
        )
        .unwrap();
    connection
            .execute("INSERT INTO module_fields(module_id, namespace, key, field_type, required) VALUES ('daena.lore', 'lore', 'summary', 'text', 0)", [])
            .unwrap();
    let duplicate = Migration {
        operations: vec![Operation::AddField {
            namespace: "lore".into(),
            field: FieldDefinition {
                key: "summary".into(),
                field_type: "text".into(),
                required: false,
            },
        }],
        ..migration()
    };
    assert!(apply(&mut connection, &duplicate).is_err());
}
