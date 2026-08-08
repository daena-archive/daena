use crate::error::CoreError;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub key: String,
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Operation {
    CreateNamespace {
        namespace: String,
    },
    AddField {
        namespace: String,
        field: FieldDefinition,
    },
    RenameField {
        namespace: String,
        from: String,
        to: String,
    },
    DropField {
        namespace: String,
        key: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migration {
    pub id: String,
    pub module_id: String,
    pub from: i64,
    pub to: i64,
    pub operations: Vec<Operation>,
    pub recovery: String,
    #[serde(default)]
    pub package_digest: String,
}

pub fn validate(migration: &Migration, current: i64) -> Result<(), CoreError> {
    if migration.id.trim().is_empty() || migration.module_id.trim().is_empty() {
        return Err("migration and module IDs are required".into());
    }
    if migration.from != current || migration.to <= migration.from {
        return Err(format!(
            "invalid migration version {} -> {} (current {})",
            migration.from, migration.to, current
        )
        .into());
    }
    if migration.recovery != "backup" && migration.recovery != "preserve-data" {
        return Err("recovery must be backup or preserve-data".into());
    }
    for operation in &migration.operations {
        let namespace = match operation {
            Operation::CreateNamespace { namespace }
            | Operation::AddField { namespace, .. }
            | Operation::RenameField { namespace, .. }
            | Operation::DropField { namespace, .. } => namespace,
        };
        if namespace.is_empty() || namespace.contains('.') {
            return Err("namespace must be a non-empty local name".into());
        }
        match operation {
            Operation::AddField { field, .. }
                if field.key.trim().is_empty() || field.field_type.trim().is_empty() =>
            {
                return Err("field key and type are required".into());
            }
            Operation::RenameField { from, to, .. }
                if from.trim().is_empty() || to.trim().is_empty() || from == to =>
            {
                return Err("field rename requires distinct non-empty keys".into());
            }
            Operation::DropField { key, .. } if key.trim().is_empty() => {
                return Err("field key is required".into());
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_schema(connection: &Connection, migration: &Migration) -> Result<(), CoreError> {
    let mut namespaces = HashSet::new();
    let mut namespace_statement =
        connection.prepare("SELECT module_id, namespace FROM module_namespaces")?;
    let namespace_rows = namespace_statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in namespace_rows {
        namespaces.insert(row?);
    }

    let mut fields = HashSet::new();
    let mut field_statement =
        connection.prepare("SELECT module_id, namespace, key FROM module_fields")?;
    let field_rows = field_statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in field_rows {
        fields.insert(row?);
    }

    for operation in &migration.operations {
        match operation {
            Operation::CreateNamespace { namespace } => {
                namespaces.insert((migration.module_id.clone(), namespace.clone()));
            }
            Operation::AddField { namespace, field } => {
                let namespace_key = (migration.module_id.clone(), namespace.clone());
                let field_key = (
                    migration.module_id.clone(),
                    namespace.clone(),
                    field.key.clone(),
                );
                if !namespaces.contains(&namespace_key) {
                    return Err(format!("namespace does not exist: {namespace}").into());
                }
                if !fields.insert(field_key) {
                    return Err(format!("field already exists: {namespace}.{}", field.key).into());
                }
            }
            Operation::RenameField {
                namespace,
                from,
                to,
            } => {
                let from_key = (migration.module_id.clone(), namespace.clone(), from.clone());
                let to_key = (migration.module_id.clone(), namespace.clone(), to.clone());
                if !namespaces.contains(&(migration.module_id.clone(), namespace.clone())) {
                    return Err(format!("namespace does not exist: {namespace}").into());
                }
                if !fields.remove(&from_key) {
                    return Err(format!("field does not exist: {namespace}.{from}").into());
                }
                if !fields.insert(to_key) {
                    return Err(format!("field already exists: {namespace}.{to}").into());
                }
            }
            Operation::DropField { namespace, key } => {
                let field_key = (migration.module_id.clone(), namespace.clone(), key.clone());
                if !namespaces.contains(&(migration.module_id.clone(), namespace.clone())) {
                    return Err(format!("namespace does not exist: {namespace}").into());
                }
                if !fields.remove(&field_key) {
                    return Err(format!("field does not exist: {namespace}.{key}").into());
                }
            }
        }
    }
    Ok(())
}

pub fn apply_in_transaction(
    connection: &Transaction<'_>,
    migration: &Migration,
) -> Result<(), CoreError> {
    let current: i64 = connection
        .query_row(
            "SELECT COALESCE(version, 0) FROM module_versions WHERE module_id=?1",
            params![migration.module_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0);
    validate(migration, current)?;
    validate_schema(connection, migration)?;
    let checksum = serde_json::to_string(migration).map_err(|e| e.to_string())?;
    let previous: Option<String> = connection
        .query_row(
            "SELECT checksum FROM migration_history WHERE module_id=?1 AND migration_id=?2",
            params![migration.module_id, migration.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some(previous) = previous {
        if previous != checksum {
            return Err("migration ID already exists with different contents".into());
        }
        return Err("migration has already been applied".into());
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
            params![migration.module_id],
        )
        .map_err(|e| e.to_string())?;
    for operation in &migration.operations {
        match operation {
            Operation::CreateNamespace { namespace } => {
                connection.execute(
                    "INSERT OR IGNORE INTO module_namespaces(module_id,namespace) VALUES (?1,?2)",
                    params![migration.module_id, namespace],
                )
                .map_err(|e| e.to_string())?;
            }
            Operation::AddField { namespace, field } => {
                connection.execute("INSERT INTO module_fields(module_id,namespace,key,field_type,required) VALUES (?1,?2,?3,?4,?5)", params![migration.module_id, namespace, field.key, field.field_type, field.required as i64]).map_err(|e| e.to_string())?;
            }
            Operation::RenameField {
                namespace,
                from,
                to,
            } => {
                connection.execute("UPDATE module_fields SET key=?4 WHERE module_id=?1 AND namespace=?2 AND key=?3", params![migration.module_id, namespace, from, to]).map_err(|e| e.to_string())?;
            }
            Operation::DropField { namespace, key } => {
                if migration.recovery != "preserve-data" {
                    return Err("drop-field requires preserve-data recovery".into());
                }
                connection
                    .execute(
                        "DELETE FROM module_fields WHERE module_id=?1 AND namespace=?2 AND key=?3",
                        params![migration.module_id, namespace, key],
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    connection
        .execute(
            "UPDATE module_versions SET version=?2 WHERE module_id=?1",
            params![migration.module_id, migration.to],
        )
        .map_err(|e| e.to_string())?;
    connection.execute(
        "INSERT INTO migration_history(module_id,migration_id,from_version,to_version,checksum,package_digest,applied_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            migration.module_id,
            migration.id,
            migration.from,
            migration.to,
            checksum,
            migration.package_digest,
            migration_applied_at(),
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

fn migration_applied_at() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod tests;
