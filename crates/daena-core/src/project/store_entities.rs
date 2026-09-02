// Entity, document, field, and search operations.
use super::*;

impl ProjectStore {
    pub fn create_entity(&self, input: CreateEntity) -> Result<Entity, CoreError> {
        self.create_entity_with_request(input, None)
    }

    pub fn create_entity_with_request(
        &self,
        input: CreateEntity,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&input)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut entity) = self
            .committed_mutation_with_fingerprint::<Entity>(request_id, Some(&input_fingerprint))?
        {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        if input.name.trim().is_empty() {
            return Err(CoreError::NotFound("entity name cannot be empty".into()));
        }
        let entity_type = input.entity_type.map(|value| value.trim().to_owned());
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: input.name.trim().into(),
            entity_type: entity_type.clone(),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("entities/{id}/")],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![id, input.name.trim(), entity_type, now],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_entity(&id)?;
        Ok(Entity {
            id,
            name: input.name.trim().into(),
            entity_type,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
            revision,
        })
    }

    pub fn create_entry(&self, input: CreateEntry) -> Result<Entity, CoreError> {
        self.create_entry_with_request(input, None)
    }

    pub fn create_entry_with_request(
        &self,
        input: CreateEntry,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        validate_document_format(
            input
                .document
                .as_ref()
                .and_then(|document| document.format.as_deref()),
            self.root.is_some(),
        )?;
        if let Some(mut entity) = self.committed_mutation::<Entity>(request_id)? {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        if input.name.trim().is_empty() {
            return Err(CoreError::NotFound("entity name cannot be empty".into()));
        }
        let format = input
            .document
            .as_ref()
            .and_then(|document| document.format.as_deref())
            .unwrap_or("markdown")
            .to_owned();
        validate_document_format(Some(&format), false)?;
        let encoded_fields = input
            .fields
            .iter()
            .map(|field| {
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(CoreError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                Ok((field, encode_field_value(&field.value)?))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        let entity_type = input.entity_type.map(|value| value.trim().to_owned());
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let mut relationship_rows = Vec::new();
        for relationship in &input.relationships {
            if relationship.relationship_type.trim().is_empty() {
                return Err(CoreError::Validation(
                    "relationship type cannot be empty".into(),
                ));
            }
            validate_relationship_metadata(
                &relationship.relationship_type,
                &serde_json::json!({}),
                self.metadata_fields_for_relationship_type(&relationship.relationship_type),
            )?;
            let mut target_ids = BTreeSet::new();
            for target_id in &relationship.target_ids {
                if !target_ids.insert(target_id) {
                    return Err(CoreError::Validation(
                        "duplicate relationship target".into(),
                    ));
                }
                let exists: Option<String> = self
                    .connection
                    .query_row(
                        "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                        params![target_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                if exists.is_none() {
                    return Err(CoreError::NotFound("relationship entity not found".into()));
                }
                relationship_rows.push((
                    target_id.clone(),
                    relationship.relationship_type.trim().to_owned(),
                ));
            }
        }
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: input.name.trim().into(),
            entity_type: entity_type.clone(),
            deleted: false,
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })?;
        let transaction =
            self.begin_mutation(&request_id, Some(&result), &[format!("entities/{id}/")])?;
        transaction.execute(
            "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
            params![id, input.name.trim(), entity_type, now],
        )?;
        if let Some(document) = input.document {
            transaction.execute(
                "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id, format, document.body, now],
            )?;
        }
        for (field, value) in encoded_fields {
            if field.namespace == crate::maps::MAP_NAMESPACE {
                crate::maps::validate_field(&transaction, &id, &field.key, &field.value)?;
            }
            transaction.execute(
                "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
                params![id, field.namespace, field.key, value],
            )?;
        }
        for (target_id, relationship_type) in relationship_rows {
            transaction.execute(
                "INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), id, target_id, relationship_type, "{}"],
            )?;
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_entity(&id)?;
        Ok(Entity {
            id,
            name: input.name.trim().into(),
            entity_type,
            deleted: false,
            created_at: now.clone(),
            updated_at: now,
            revision,
        })
    }

    /// Creates a group of normal entries, their fields, documents, and
    /// relationships in one receipt-backed transaction. The batch is used by
    /// explicit physical-event materialization so a retry cannot create a
    /// partial history. `CreateEntry` remains the generic input contract; no
    /// event-specific storage is hidden in the core.
    pub fn create_entries_with_request(
        &self,
        inputs: Vec<CreateEntry>,
        request_id: Option<&str>,
    ) -> Result<Vec<Entity>, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&inputs)
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut entities) = self.committed_mutation_with_fingerprint::<Vec<Entity>>(
            request_id,
            Some(&input_fingerprint),
        )? {
            self.populate_entity_revisions(&mut entities)?;
            return Ok(entities);
        }

        struct PreparedEntry {
            name: String,
            entity_type: Option<String>,
            document: Option<(String, String)>,
            fields: Vec<(String, String, serde_json::Value, String)>,
            relationships: Vec<(String, String)>,
        }

        let mut prepared = Vec::with_capacity(inputs.len());
        for input in inputs {
            validate_document_format(
                input
                    .document
                    .as_ref()
                    .and_then(|document| document.format.as_deref()),
                self.root.is_some(),
            )?;
            if input.name.trim().is_empty() {
                return Err(CoreError::NotFound("entity name cannot be empty".into()));
            }
            let format = input
                .document
                .as_ref()
                .and_then(|document| document.format.as_deref())
                .unwrap_or("markdown")
                .to_owned();
            validate_document_format(Some(&format), false)?;
            let mut fields = Vec::with_capacity(input.fields.len());
            for field in input.fields {
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(CoreError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                let encoded = encode_field_value(&field.value)?;
                fields.push((field.namespace, field.key, field.value, encoded));
            }
            let mut relationships = Vec::new();
            for relationship in input.relationships {
                if relationship.relationship_type.trim().is_empty() {
                    return Err(CoreError::Validation(
                        "relationship type cannot be empty".into(),
                    ));
                }
                validate_relationship_metadata(
                    &relationship.relationship_type,
                    &serde_json::json!({}),
                    self.metadata_fields_for_relationship_type(&relationship.relationship_type),
                )?;
                let mut target_ids = BTreeSet::new();
                for target_id in relationship.target_ids {
                    if !target_ids.insert(target_id.clone()) {
                        return Err(CoreError::Validation(
                            "duplicate relationship target".into(),
                        ));
                    }
                    let exists: Option<String> = self
                        .connection
                        .query_row(
                            "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                            params![target_id],
                            |row| row.get(0),
                        )
                        .optional()?;
                    if exists.is_none() {
                        return Err(CoreError::NotFound("relationship entity not found".into()));
                    }
                    relationships
                        .push((target_id, relationship.relationship_type.trim().to_owned()));
                }
            }
            prepared.push(PreparedEntry {
                name: input.name.trim().into(),
                entity_type: input.entity_type.map(|value| value.trim().to_owned()),
                document: input
                    .document
                    .map(|document| (format.clone(), document.body)),
                fields,
                relationships,
            });
        }

        let now = chrono_like_now();
        let mut entities = prepared
            .iter()
            .map(|entry| {
                let id = Uuid::new_v4().to_string();
                Entity {
                    id,
                    name: entry.name.clone(),
                    entity_type: entry.entity_type.clone(),
                    deleted: false,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    revision: String::new(),
                }
            })
            .collect::<Vec<_>>();
        let mut map_projection_ids = BTreeSet::new();
        for entry in &prepared {
            for (namespace, key, value, _) in &entry.fields {
                if namespace == crate::maps::MAP_NAMESPACE && key == "locations" {
                    if let Some(locations) =
                        value.get("locations").and_then(|value| value.as_array())
                    {
                        for location in locations {
                            if let Some(map_entity_id) =
                                location.get("mapEntityId").and_then(|value| value.as_str())
                            {
                                map_projection_ids.insert(map_entity_id.to_owned());
                            }
                        }
                    }
                }
            }
        }
        let result = serde_json::to_value(&entities)?;
        let request_id = self.request_id(request_id)?;
        let affected_prefixes = entities
            .iter()
            .map(|entity| format!("entities/{}/", entity.id))
            .collect::<Vec<_>>();
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &affected_prefixes,
            &input_fingerprint,
        )?;

        for (entry, entity) in prepared.iter().zip(entities.iter()) {
            transaction.execute(
                "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
                params![entity.id, entry.name, entry.entity_type, now],
            )?;
            if let Some((format, body)) = &entry.document {
                transaction.execute(
                    "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                    params![Uuid::new_v4().to_string(), entity.id, format, body, now],
                )?;
            }
            for (namespace, key, value, encoded) in &entry.fields {
                if namespace == crate::maps::MAP_NAMESPACE {
                    crate::maps::validate_field(&transaction, &entity.id, key, value)?;
                }
                transaction.execute(
                    "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
                    params![entity.id, namespace, key, encoded],
                )?;
            }
            for (target_id, relationship_type) in &entry.relationships {
                transaction.execute(
                    "INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)",
                    params![Uuid::new_v4().to_string(), entity.id, target_id, relationship_type, "{}"],
                )?;
            }
        }
        transaction.commit()?;
        if !map_projection_ids.is_empty() {
            self.refresh_maps_projection_for_entities(
                &map_projection_ids.into_iter().collect::<Vec<_>>(),
            )?;
        }
        self.notify_export_worker()?;
        for entity in &mut entities {
            entity.revision = self.revision_for_entity(&entity.id)?;
        }
        Ok(entities)
    }

    pub fn list_entities(&self) -> Result<Vec<Entity>, CoreError> {
        self.list_entities_where("WHERE deleted=0")
    }

    pub fn get_entity(&self, id: &str) -> Result<Option<Entity>, CoreError> {
        let mut entity = self
            .connection
            .query_row(
                "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1 AND deleted=0",
                params![id],
                |row| {
                    Ok(Entity {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        entity_type: row.get(2)?,
                        deleted: row.get::<_, i64>(3)? != 0,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?;
        if let Some(entity) = entity.as_mut() {
            entity.revision = self.revision_for_entity(&entity.id)?;
        }
        Ok(entity)
    }

    pub fn query_entities(&self, query: EntityListQuery) -> Result<EntityPage, CoreError> {
        if query
            .query
            .as_deref()
            .is_some_and(|value| value.chars().count() > 512)
        {
            return Err(CoreError::Validation(
                "entity query text cannot exceed 512 characters".into(),
            ));
        }
        if query.entity_types.len() > 256 || query.excluded_entity_types.len() > 256 {
            return Err(CoreError::Validation(
                "entity query cannot contain more than 256 entity types".into(),
            ));
        }
        let limit = query
            .limit
            .unwrap_or(DEFAULT_ENTITY_QUERY_LIMIT)
            .clamp(1, MAX_ENTITY_QUERY_LIMIT);
        let offset = query.offset.unwrap_or_default();
        let offset_i64 = i64::try_from(offset)
            .map_err(|_| CoreError::Validation("entity query offset is too large".into()))?;

        let archived_only = query.archived == Some(true);
        let search_terms = if archived_only {
            String::new()
        } else {
            query
                .query
                .as_deref()
                .unwrap_or_default()
                .split_whitespace()
                .filter_map(|term| {
                    let escaped = term.replace('"', "");
                    (!escaped.is_empty()).then(|| format!("\"{escaped}\"*"))
                })
                .collect::<Vec<_>>()
                .join(" AND ")
        };

        let mut params = Vec::<SqlValue>::new();
        let mut from = "FROM entities e".to_owned();
        let mut conditions = vec![if archived_only {
            "e.deleted=1".to_owned()
        } else {
            "e.deleted=0".to_owned()
        }];
        if archived_only {
            if let Some(text) = query.query.as_deref() {
                for term in text.split_whitespace() {
                    let escaped = term
                        .replace('\\', "\\\\")
                        .replace('%', "\\%")
                        .replace('_', "\\_");
                    if !escaped.is_empty() {
                        conditions.push("e.name LIKE ? ESCAPE '\\'".to_owned());
                        params.push(SqlValue::Text(format!("%{escaped}%")));
                    }
                }
            }
        } else if !search_terms.is_empty() {
            from.push_str(" JOIN (SELECT entity_id, MIN(rank) AS rank FROM world_search WHERE world_search MATCH ? GROUP BY entity_id) search ON search.entity_id=e.id");
            params.push(SqlValue::Text(search_terms.clone()));
        }

        let entity_types = query
            .entity_types
            .into_iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>();
        if !entity_types.is_empty() {
            conditions.push(format!(
                "e.entity_type IN ({})",
                std::iter::repeat_n("?", entity_types.len())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            params.extend(entity_types.into_iter().map(SqlValue::Text));
        }
        let excluded_entity_types = query
            .excluded_entity_types
            .into_iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect::<BTreeSet<_>>();
        if !excluded_entity_types.is_empty() {
            conditions.push(format!(
                "COALESCE(e.entity_type,'__uncategorized') NOT IN ({})",
                std::iter::repeat_n("?", excluded_entity_types.len())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            params.extend(excluded_entity_types.into_iter().map(SqlValue::Text));
        }
        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let total_i64 = self.connection.query_row(
            &format!("SELECT COUNT(*) {from} {where_clause}"),
            rusqlite::params_from_iter(params.iter()),
            |row| row.get::<_, i64>(0),
        )?;
        let total = u64::try_from(total_i64).unwrap_or_default();

        let mut type_count_statement = self.connection.prepare(&format!(
            "SELECT e.entity_type,COUNT(*) {from} {where_clause} GROUP BY e.entity_type ORDER BY COALESCE(e.entity_type,'')"
        ))?;
        let type_counts = type_count_statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                let count = row.get::<_, i64>(1)?;
                Ok(EntityTypeCount {
                    entity_type: row.get(0)?,
                    count: u64::try_from(count).unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let sort_field = query.sort_field.unwrap_or({
            if archived_only && search_terms.is_empty() {
                EntitySortField::UpdatedAt
            } else if search_terms.is_empty() {
                EntitySortField::Name
            } else {
                EntitySortField::Relevance
            }
        });
        let direction = match (query.sort_direction, archived_only, sort_field) {
            (Some(EntitySortDirection::Asc), _, _) => "ASC",
            (Some(EntitySortDirection::Desc), _, _) => "DESC",
            (None, true, EntitySortField::UpdatedAt) => "DESC",
            (None, _, _) => "ASC",
        };
        let order = match sort_field {
            EntitySortField::Name => format!("e.name COLLATE NOCASE {direction}, e.id {direction}"),
            EntitySortField::CreatedAt => {
                format!("e.created_at {direction}, e.id {direction}")
            }
            EntitySortField::UpdatedAt => {
                format!("e.updated_at {direction}, e.id {direction}")
            }
            EntitySortField::Relevance if !search_terms.is_empty() => {
                format!("search.rank {direction}, e.name COLLATE NOCASE ASC, e.id ASC")
            }
            EntitySortField::Relevance => "e.name COLLATE NOCASE ASC, e.id ASC".into(),
        };

        let mut page_params = params;
        page_params.push(SqlValue::Integer(i64::from(limit)));
        page_params.push(SqlValue::Integer(offset_i64));
        let mut statement = self.connection.prepare(&format!(
            "SELECT e.id,e.name,e.entity_type,e.deleted,e.created_at,e.updated_at {from} {where_clause} ORDER BY {order} LIMIT ? OFFSET ?"
        ))?;
        let rows = statement.query_map(rusqlite::params_from_iter(page_params.iter()), |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: String::new(),
            })
        })?;
        let mut items = rows.collect::<Result<Vec<_>, _>>()?;
        self.populate_entity_revisions(&mut items)?;
        let returned = u64::try_from(items.len()).unwrap_or_default();
        Ok(EntityPage {
            items,
            total,
            offset,
            limit,
            has_more: offset.saturating_add(returned) < total,
            type_counts,
        })
    }

    pub(crate) fn list_entities_where(&self, predicate: &str) -> Result<Vec<Entity>, CoreError> {
        let mut statement = self.connection.prepare(&format!("SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities {predicate} ORDER BY name"))?;
        let rows = statement.query_map([], |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: String::new(),
            })
        })?;
        let mut entities = rows.collect::<Result<Vec<_>, _>>()?;
        self.populate_entity_revisions(&mut entities)?;
        Ok(entities)
    }

    pub fn external_import_existing_targets(
        &self,
        entity_ids: &BTreeSet<String>,
    ) -> Result<BTreeMap<String, ImportExistingTarget>, CoreError> {
        let mut targets = BTreeMap::new();
        for entity_id in entity_ids {
            let entity: Option<(String, Option<String>)> = self
                .connection
                .query_row(
                    "SELECT id,entity_type FROM entities WHERE id=?1 AND deleted=0",
                    params![entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some((entity_id, entity_type)) = entity {
                targets.insert(
                    entity_id.clone(),
                    ImportExistingTarget {
                        revision: self.revision_for_entity(&entity_id)?,
                        entity_id,
                        entity_type,
                    },
                );
            }
        }
        Ok(targets)
    }

    pub fn external_import_duplicate_targets(
        &self,
        importer_id: &str,
        objects: &[(String, String)],
    ) -> Result<BTreeMap<String, Vec<String>>, CoreError> {
        let mut duplicates = BTreeMap::new();
        for (object_id, source_id) in objects {
            let key = external_import_source_key(importer_id, source_id);
            let mut statement = self.connection.prepare(
                "SELECT f.entity_id,f.value FROM entity_fields f JOIN entities e ON e.id=f.entity_id WHERE f.namespace=?1 AND f.key=?2 AND e.deleted=0 ORDER BY f.entity_id",
            )?;
            let rows = statement
                .query_map(params![EXTERNAL_IMPORT_SOURCE_NAMESPACE, key], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;
            let mut entity_ids = Vec::new();
            for row in rows {
                let (entity_id, encoded) = row?;
                let value = decode_field_value(encoded);
                if value.get("importerId").and_then(serde_json::Value::as_str) == Some(importer_id)
                    && value.get("sourceId").and_then(serde_json::Value::as_str) == Some(source_id)
                {
                    entity_ids.push(entity_id);
                }
            }
            if !entity_ids.is_empty() {
                duplicates.insert(object_id.clone(), entity_ids);
            }
        }
        Ok(duplicates)
    }

    pub fn commit_external_import(
        &self,
        plan: &ValidatedImportPlan,
        asset_source_root: Option<&Path>,
        acknowledge_warnings: bool,
        request_id: &str,
    ) -> Result<ExternalImportCommitReport, CoreError> {
        if plan.schema_version != VALIDATED_IMPORT_PLAN_SCHEMA_VERSION {
            return Err(CoreError::Validation(
                "unsupported validated import plan version".into(),
            ));
        }
        if !plan.warnings.is_empty() && !acknowledge_warnings {
            return Err(CoreError::Validation(
                "import warnings must be acknowledged before commit".into(),
            ));
        }
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&(plan, acknowledge_warnings))
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(report) = self
            .committed_mutation_with_fingerprint::<ExternalImportCommitReport>(
                Some(request_id),
                Some(&input_fingerprint),
            )?
        {
            return Ok(report);
        }
        let current_generation = self.content_generation()?;
        if current_generation != plan.content_generation {
            return Err(CoreError::Conflict(format!(
                "import plan is stale: expected project generation {}, current {}",
                plan.content_generation, current_generation
            )));
        }

        struct PreparedCreate {
            entity_id: String,
            object_index: usize,
            fields: Vec<(String, String, String)>,
            source_key: String,
            source_value: String,
        }
        struct PreparedAsset {
            id: String,
            entity_id: String,
            filename: String,
            content_hash: String,
            size: i64,
            mime_type: String,
            path: String,
        }
        struct PreparedRelationship {
            id: String,
            source_entity_id: String,
            target_entity_id: String,
            relationship_type: String,
        }
        let mut creates = Vec::new();
        let mut mapped_sources = Vec::new();
        let mut created_report = Vec::new();
        let mut mapped_report = Vec::new();
        let mut field_report = Vec::new();
        let mut decision_report = Vec::new();
        let mut skipped_source_paths = Vec::new();
        let mut affected_prefixes = Vec::new();
        for (object_index, object) in plan.objects.iter().enumerate() {
            let source_key = external_import_source_key(&plan.importer.id, &object.source_id);
            let source_value = encode_field_value(&serde_json::json!({
                "importerId": plan.importer.id,
                "importerVersion": plan.importer.version,
                "sourceId": object.source_id,
                "sourcePath": object.source_path,
                "contentHash": object.content_hash,
                "sourceContext": object.source_context,
            }))?;
            match &object.decision {
                ImportObjectDecision::Skip => {
                    skipped_source_paths.push(object.source_path.clone());
                    decision_report.push(ImportDecisionReport {
                        staged_object_id: object.staged_object_id.clone(),
                        source_path: object.source_path.clone(),
                        decision: "skip".into(),
                        entity_id: None,
                    });
                }
                ImportObjectDecision::MapToExisting {
                    entity_id,
                    expected_revision,
                } => {
                    let current_revision = self.revision_for_entity(entity_id)?;
                    Self::ensure_expected_revision(
                        Some(expected_revision),
                        current_revision,
                        "import map target",
                    )?;
                    mapped_sources.push((entity_id.clone(), source_key, source_value));
                    affected_prefixes.push(format!("entities/{entity_id}/"));
                    mapped_report.push(ImportedObjectReport {
                        staged_object_id: object.staged_object_id.clone(),
                        source_path: object.source_path.clone(),
                        entity_id: entity_id.clone(),
                        entity_type: object.entity_type.clone(),
                    });
                    decision_report.push(ImportDecisionReport {
                        staged_object_id: object.staged_object_id.clone(),
                        source_path: object.source_path.clone(),
                        decision: "map_to_existing".into(),
                        entity_id: Some(entity_id.clone()),
                    });
                }
                ImportObjectDecision::Create => {
                    if object.title.trim().is_empty() || object.entity_type.is_none() {
                        return Err(CoreError::Validation(
                            "validated create item requires a title and entity type".into(),
                        ));
                    }
                    validate_document_format(
                        object
                            .document
                            .as_ref()
                            .map(|document| document.format.as_str()),
                        self.root.is_some(),
                    )?;
                    let fields = object
                        .fields
                        .iter()
                        .map(|field| {
                            Ok((
                                field.namespace.clone(),
                                field.key.clone(),
                                encode_field_value(&field.value)?,
                            ))
                        })
                        .collect::<Result<Vec<_>, CoreError>>()?;
                    let entity_id = Uuid::new_v4().to_string();
                    affected_prefixes.push(format!("entities/{entity_id}/"));
                    created_report.push(ImportedObjectReport {
                        staged_object_id: object.staged_object_id.clone(),
                        source_path: object.source_path.clone(),
                        entity_id: entity_id.clone(),
                        entity_type: object.entity_type.clone(),
                    });
                    field_report.extend(object.fields.iter().map(|field| ImportedFieldReport {
                        staged_object_id: object.staged_object_id.clone(),
                        source_path: object.source_path.clone(),
                        entity_id: entity_id.clone(),
                        source_key: field.source_key.clone(),
                        namespace: field.namespace.clone(),
                        key: field.key.clone(),
                    }));
                    decision_report.push(ImportDecisionReport {
                        staged_object_id: object.staged_object_id.clone(),
                        source_path: object.source_path.clone(),
                        decision: "create".into(),
                        entity_id: Some(entity_id.clone()),
                    });
                    creates.push(PreparedCreate {
                        entity_id,
                        object_index,
                        fields,
                        source_key,
                        source_value,
                    });
                }
            }
        }
        let entity_by_staged_object = created_report
            .iter()
            .chain(mapped_report.iter())
            .map(|object| (object.staged_object_id.clone(), object.entity_id.clone()))
            .collect::<BTreeMap<_, _>>();
        let object_by_source_path = plan
            .objects
            .iter()
            .map(|object| (object.source_path.as_str(), object))
            .collect::<BTreeMap<_, _>>();
        let mut prepared_relationships = Vec::with_capacity(plan.relationships.len());
        let mut relationship_report = Vec::with_capacity(plan.relationships.len());
        for relationship in &plan.relationships {
            let source_entity_id = entity_by_staged_object
                .get(&relationship.source_staged_object_id)
                .ok_or_else(|| {
                    CoreError::Validation(
                        "validated import relationship source was not created or mapped".into(),
                    )
                })?
                .clone();
            let target_entity_id = entity_by_staged_object
                .get(&relationship.target_staged_object_id)
                .ok_or_else(|| {
                    CoreError::Validation(
                        "validated import relationship target was not created or mapped".into(),
                    )
                })?
                .clone();
            validate_relationship_metadata(
                &relationship.relationship_type,
                &serde_json::json!({}),
                self.metadata_fields_for_relationship_type(&relationship.relationship_type),
            )?;
            let id = Uuid::new_v4().to_string();
            affected_prefixes.push(format!("entities/{source_entity_id}/"));
            affected_prefixes.push(format!("entities/{target_entity_id}/"));
            relationship_report.push(ImportedRelationshipReport {
                relationship_id: id.clone(),
                source_entity_id: source_entity_id.clone(),
                target_entity_id: target_entity_id.clone(),
                relationship_type: relationship.relationship_type.clone(),
            });
            prepared_relationships.push(PreparedRelationship {
                id,
                source_entity_id,
                target_entity_id,
                relationship_type: relationship.relationship_type.clone(),
            });
        }
        let mut prepared_assets = Vec::new();
        let mut asset_report = Vec::new();
        let mut asset_install_guard = RuntimeAssetInstallGuard::default();
        for asset in &plan.assets {
            let entity_id = entity_by_staged_object
                .get(&asset.owner_staged_object_id)
                .ok_or_else(|| {
                    CoreError::Validation(
                        "validated import asset owner was not created or mapped".into(),
                    )
                })?
                .to_string();
            let root = asset_source_root.ok_or_else(|| {
                CoreError::Validation("import asset source root is unavailable".into())
            })?;
            let declared_size = i64::try_from(asset.size)
                .map_err(|_| CoreError::Validation("import asset is too large".into()))?;
            let project_root = self.root.as_deref().ok_or_else(|| {
                CoreError::Validation("asset import requires a directory project".into())
            })?;
            let runtime_path = runtime_asset_path(project_root, &asset.content_hash)?;
            let runtime_asset_existed = runtime_path.is_file();
            let (content_hash, size) = if is_docx_import_asset_source_path(&asset.source_path) {
                let container_path = asset
                    .source_path
                    .split_once("!/")
                    .map(|(container, _)| container)
                    .ok_or_else(|| {
                        CoreError::Validation(
                            "validated DOCX import asset container is unavailable".into(),
                        )
                    })?;
                let container = object_by_source_path.get(container_path).ok_or_else(|| {
                    CoreError::Validation(
                        "validated DOCX import asset container was not staged".into(),
                    )
                })?;
                let bytes = read_docx_import_asset_bytes(
                    root,
                    &plan.source.kind,
                    &asset.source_path,
                    asset.size,
                    &container.content_hash,
                )?;
                store_runtime_asset(project_root, bytes.as_slice(), Some(&asset.content_hash))?
            } else {
                match &plan.source.kind {
                    ImportSourceKind::Archive => {
                        let bytes = read_archive_asset_bytes(root, &asset.source_path, asset.size)?;
                        store_runtime_asset(
                            project_root,
                            bytes.as_slice(),
                            Some(&asset.content_hash),
                        )?
                    }
                    ImportSourceKind::Folder | ImportSourceKind::Vault => {
                        let source =
                            crate::storage::normalized_project_path(root, &asset.source_path)?;
                        let metadata =
                            std::fs::symlink_metadata(&source).map_err(|source| CoreError::Io {
                                operation: "read import asset metadata",
                                source,
                            })?;
                        if metadata.file_type().is_symlink() || !metadata.is_file() {
                            return Err(CoreError::Validation(
                                "import asset source must remain a regular file".into(),
                            ));
                        }
                        store_runtime_asset_file(project_root, &source, Some(&asset.content_hash))?
                    }
                    _ => {
                        return Err(CoreError::Validation(
                            "this import source kind cannot provide attachments".into(),
                        ));
                    }
                }
            };
            if !runtime_asset_existed {
                asset_install_guard.track(runtime_path);
            }
            if size != declared_size {
                return Err(CoreError::Conflict(format!(
                    "import asset '{}' changed size after analysis",
                    asset.source_path
                )));
            }
            let filename = validated_asset_filename(&asset.filename)?;
            let category = imported_asset_category(&filename, &asset.mime_type);
            let id = Uuid::new_v4().to_string();
            let path = format!("assets/{category}/{id}-{filename}");
            affected_prefixes.push(format!("entities/{entity_id}/"));
            affected_prefixes.push(path.clone());
            asset_report.push(ImportedAssetReport {
                staged_asset_id: asset.staged_asset_id.clone(),
                source_path: asset.source_path.clone(),
                asset_id: id.clone(),
                entity_id: entity_id.clone(),
                filename: filename.clone(),
                content_hash: content_hash.clone(),
            });
            prepared_assets.push(PreparedAsset {
                id,
                entity_id,
                filename,
                content_hash,
                size,
                mime_type: asset.mime_type.clone(),
                path,
            });
        }
        let missing_references = plan
            .objects
            .iter()
            .flat_map(|object| {
                object
                    .source_context
                    .links
                    .iter()
                    .filter(|link| link.resolution == StagedLinkResolution::Missing)
                    .map(|link| ImportMissingReferenceReport {
                        staged_object_id: object.staged_object_id.clone(),
                        source_path: object.source_path.clone(),
                        target: link.target.clone(),
                        kind: match link.kind {
                            StagedLinkKind::Internal => "internal",
                            StagedLinkKind::External => "external",
                            StagedLinkKind::Embed => "embed",
                        }
                        .into(),
                    })
            })
            .collect();
        let report = ExternalImportCommitReport {
            request_id: request_id.into(),
            plan_id: plan.plan_id.clone(),
            importer: plan.importer.clone(),
            source: plan.source.clone(),
            created: created_report,
            mapped: mapped_report,
            assets: asset_report,
            relationships: relationship_report,
            fields: field_report,
            decisions: decision_report,
            unsupported: plan.unsupported.clone(),
            missing_references,
            diagnostics: plan.diagnostics.clone(),
            skipped_source_paths,
            warnings: plan.warnings.clone(),
        };
        let result = serde_json::to_value(&report)?;
        let request_id = self.request_id(Some(request_id))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &affected_prefixes,
            &input_fingerprint,
        )?;
        let transaction_generation: i64 = transaction.query_row(
            "SELECT content_generation FROM runtime_meta WHERE key='runtime'",
            [],
            |row| row.get(0),
        )?;
        if transaction_generation != plan.content_generation {
            return Err(CoreError::Conflict(
                "import plan became stale before commit".into(),
            ));
        }
        let now = chrono_like_now();
        for prepared in &creates {
            let object = &plan.objects[prepared.object_index];
            transaction.execute(
                "INSERT INTO entities(id,name,entity_type,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",
                params![prepared.entity_id, object.title.trim(), object.entity_type, now],
            )?;
            if let Some(document) = &object.document {
                transaction.execute(
                    "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                    params![Uuid::new_v4().to_string(), prepared.entity_id, document.format, document.body, now],
                )?;
            }
            for (namespace, key, encoded) in &prepared.fields {
                transaction.execute(
                    "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
                    params![prepared.entity_id, namespace, key, encoded],
                )?;
            }
            transaction.execute(
                "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4)",
                params![
                    prepared.entity_id,
                    EXTERNAL_IMPORT_SOURCE_NAMESPACE,
                    prepared.source_key,
                    prepared.source_value
                ],
            )?;
        }
        for (entity_id, source_key, source_value) in &mapped_sources {
            transaction.execute(
                "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value",
                params![entity_id, EXTERNAL_IMPORT_SOURCE_NAMESPACE, source_key, source_value],
            )?;
        }
        for asset in &prepared_assets {
            transaction.execute(
                "INSERT INTO assets(id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    asset.id,
                    asset.entity_id,
                    EXTERNAL_IMPORT_SOURCE_NAMESPACE,
                    asset.filename,
                    asset.content_hash,
                    asset.size,
                    asset.mime_type,
                    asset.path,
                    now
                ],
            )?;
        }
        for relationship in &prepared_relationships {
            transaction.execute(
                "INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,'{}')",
                params![
                    relationship.id,
                    relationship.source_entity_id,
                    relationship.target_entity_id,
                    relationship.relationship_type
                ],
            )?;
        }
        transaction.commit()?;
        asset_install_guard.commit();
        self.notify_export_worker()?;
        Ok(report)
    }

    pub fn update_entity(
        &self,
        id: String,
        name: Option<String>,
        entity_type: Option<String>,
    ) -> Result<Entity, CoreError> {
        self.update_entity_with_options(id, name, entity_type, None, None)
    }

    pub fn update_entity_with_options(
        &self,
        id: String,
        name: Option<String>,
        entity_type: Option<String>,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Entity, CoreError> {
        if let Some(mut entity) = self.committed_mutation::<Entity>(request_id)? {
            entity.revision = self.revision_for_entity(&entity.id)?;
            return Ok(entity);
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        if let Some(value) = &name {
            if value.trim().is_empty() {
                return Err(CoreError::NotFound("entity name cannot be empty".into()));
            }
        }
        let current = self
            .connection
            .query_row(
                "SELECT name,entity_type,created_at FROM entities WHERE id=?1 AND deleted=0",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("entity not found".into()))?;
        let now = chrono_like_now();
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Entity {
            id: id.clone(),
            name: name.as_deref().map(str::trim).unwrap_or(&current.0).into(),
            entity_type: entity_type.clone().or(current.1.clone()),
            deleted: false,
            created_at: current.2.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        })?;
        let transaction =
            self.begin_mutation(&request_id, Some(&result), &[format!("entities/{id}/")])?;
        if transaction.execute("UPDATE entities SET name=COALESCE(?2,name), entity_type=COALESCE(?3,entity_type), updated_at=?4 WHERE id=?1 AND deleted=0", params![id, name, entity_type, now])? == 0 { return Err(CoreError::NotFound("entity not found".into())); }
        transaction.commit()?;
        let mut entity = self.connection.query_row(
            "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1",
            params![id],
            |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    deleted: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    revision: String::new(),
                })
            },
        )?;
        self.notify_export_worker()?;
        entity.revision = self.revision_for_entity(&entity.id)?;
        Ok(entity)
    }

    pub fn delete_entity(&self, id: String) -> Result<(), CoreError> {
        self.delete_entity_with_options(id, None, None)
    }

    pub fn delete_entity_with_options(
        &self,
        id: String,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{id}/")],
        )?;
        if transaction.execute(
            "UPDATE entities SET deleted=1, updated_at=?2 WHERE id=?1 AND deleted=0",
            params![id, chrono_like_now()],
        )? == 0
        {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&id))?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn restore_entity(&self, id: String) -> Result<(), CoreError> {
        self.restore_entity_with_options(id, None, None)
    }

    pub fn restore_entity_with_options(
        &self,
        id: String,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{id}/")],
        )?;
        if transaction.execute(
            "UPDATE entities SET deleted=0, updated_at=?2 WHERE id=?1 AND deleted=1",
            params![id, chrono_like_now()],
        )? == 0
        {
            return Err(CoreError::NotFound("archived entity not found".into()));
        }
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&id))?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn purge_entity(&self, id: String) -> Result<(), CoreError> {
        self.purge_entity_with_options(id, None, None)
    }

    pub fn purge_entity_with_options(
        &self,
        id: String,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let deleted: i64 = self
            .connection
            .query_row(
                "SELECT deleted FROM entities WHERE id=?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|_| CoreError::NotFound("archived entity not found".into()))?;
        if deleted == 0 {
            return Err(CoreError::Validation(
                "only archived entities can be permanently deleted".into(),
            ));
        }
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&id)?,
            "entity",
        )?;
        let related_entities = self
            .connection
            .prepare(
                "SELECT DISTINCT entity_id FROM (
                    SELECT source_id AS entity_id FROM relationships WHERE target_id=?1
                    UNION
                    SELECT target_id AS entity_id FROM relationships WHERE source_id=?1
                )",
            )?
            .query_map(params![id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{id}/")],
        )?;
        transaction.execute(
            "DELETE FROM module_records WHERE owner_entity_id=?1",
            params![id],
        )?;
        transaction.execute(
            "DELETE FROM relationships WHERE source_id=?1 OR target_id=?1",
            params![id],
        )?;
        transaction.execute("DELETE FROM documents WHERE entity_id=?1", params![id])?;
        transaction.execute("DELETE FROM entity_fields WHERE entity_id=?1", params![id])?;
        transaction.execute("DELETE FROM assets WHERE entity_id=?1", params![id])?;
        transaction.execute(
            "DELETE FROM map_projection WHERE map_entity_id=?1",
            params![id],
        )?;
        transaction.execute(
            "DELETE FROM map_location_projection WHERE entity_id=?1 OR map_entity_id=?1",
            params![id],
        )?;
        transaction.execute(
            "DELETE FROM map_feature_projection WHERE map_entity_id=?1",
            params![id],
        )?;
        transaction.execute(
            "DELETE FROM map_feature_search WHERE map_entity_id=?1",
            params![id],
        )?;
        transaction.execute("DELETE FROM world_search WHERE entity_id=?1", params![id])?;
        if transaction.execute(
            "DELETE FROM entities WHERE id=?1 AND deleted=1",
            params![id],
        )? == 0
        {
            return Err(CoreError::NotFound("archived entity not found".into()));
        }
        transaction.commit()?;
        let mut refresh_ids = related_entities;
        refresh_ids.push(id);
        refresh_ids.sort();
        refresh_ids.dedup();
        self.refresh_maps_projection_for_entities(&refresh_ids)?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn save_document(&self, input: SaveDocument) -> Result<(), CoreError> {
        self.save_document_with_options(input, None, None)
    }

    pub fn save_document_with_options(
        &self,
        input: SaveDocument,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let expected_revision = expected_revision.filter(|revision| !revision.is_empty());
        self.save_entry_with_options(
            SaveEntry {
                document: input,
                fields: Vec::new(),
            },
            expected_revision,
            request_id,
        )
    }

    pub fn save_entry(&self, input: SaveEntry) -> Result<(), CoreError> {
        self.save_entry_with_options(input, None, None)
    }

    pub fn save_entry_with_options(
        &self,
        input: SaveEntry,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        validate_document_format(input.document.format.as_deref(), self.root.is_some())?;
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let document = input.document;
        let format = document.format.unwrap_or_else(|| "markdown".into());
        validate_document_format(Some(&format), false)?;
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        let current_document_id: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC LIMIT 1",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(expected_revision) = expected_revision {
            let Some(document_id) = current_document_id.as_deref() else {
                return Err(CoreError::Conflict(
                    "document revision conflict: document does not exist".into(),
                ));
            };
            Self::ensure_expected_revision(
                Some(expected_revision),
                self.revision_for_document(document_id)?,
                "document",
            )?;
        }
        let encoded_fields = input
            .fields
            .iter()
            .map(|field| {
                if field.entity_id != document.entity_id {
                    return Err(CoreError::NotFound(
                        "field entity does not match document entity".into(),
                    ));
                }
                if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
                    return Err(CoreError::NotFound(
                        "field namespace and key are required".into(),
                    ));
                }
                Ok((field, encode_field_value(&field.value)?))
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        for (field, _) in &encoded_fields {
            if field.namespace == crate::maps::MAP_NAMESPACE {
                crate::maps::validate_field(
                    &self.connection,
                    &field.entity_id,
                    &field.key,
                    &field.value,
                )?;
            }
        }
        for (field, _) in &encoded_fields {
            if field.revision.is_empty() {
                continue;
            }
            let current: Option<String> = self
                .connection
                .query_row(
                    "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key=?3",
                    params![field.entity_id, field.namespace, field.key],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(current) = current else {
                return Err(CoreError::Conflict(
                    "field revision conflict: field does not exist".into(),
                ));
            };
            let current_field = FieldValue {
                entity_id: field.entity_id.clone(),
                namespace: field.namespace.clone(),
                key: field.key.clone(),
                value: decode_field_value(current),
                revision: String::new(),
            };
            Self::ensure_expected_revision(
                Some(&field.revision),
                self.revision_for_field(&current_field)?,
                "field",
            )?;
        }
        let now = chrono_like_now();
        let document_id: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC LIMIT 1",
                params![document.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{}/", document.entity_id)],
        )?;
        if let Some(document_id) = document_id {
            transaction.execute(
                "UPDATE documents SET format=?2, body=?3, updated_at=?4 WHERE id=?1",
                params![document_id, format, document.body, now],
            )?;
        } else {
            transaction.execute(
                "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
                params![Uuid::new_v4().to_string(), document.entity_id, format, document.body, now],
            )?;
        }
        for (field, value) in encoded_fields {
            transaction.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn list_documents(&self, entity_id: String) -> Result<Vec<Document>, CoreError> {
        self.list_documents_unchecked(entity_id)
    }

    pub(crate) fn list_documents_unchecked(
        &self,
        entity_id: String,
    ) -> Result<Vec<Document>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,entity_id,format,body,updated_at FROM documents WHERE entity_id=?1 ORDER BY updated_at DESC")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Document {
                id: row.get(0)?,
                entity_id: row.get(1)?,
                format: row.get(2)?,
                body: row.get(3)?,
                updated_at: row.get(4)?,
                revision: String::new(),
            })
        })?;
        let mut documents = rows.collect::<Result<Vec<_>, _>>()?;
        for document in &mut documents {
            document.revision = self.revision_for_document_value(document)?;
        }
        Ok(documents)
    }

    pub fn set_field(&self, field: FieldValue) -> Result<(), CoreError> {
        self.set_field_with_request(field, None)
    }

    pub fn set_field_with_request(
        &self,
        field: FieldValue,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let exists: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                params![field.entity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound("entity not found".into()));
        }
        if field.namespace.trim().is_empty() || field.key.trim().is_empty() {
            return Err(CoreError::NotFound(
                "field namespace and key are required".into(),
            ));
        }
        let current_physical_identity = if field.namespace == crate::maps::MAP_NAMESPACE
            && field.key == "map"
        {
            let current: Option<String> = self
                .connection
                .query_row(
                    "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key='map'",
                    params![field.entity_id, crate::maps::MAP_NAMESPACE],
                    |row| row.get(0),
                )
                .optional()?;
            current
                .map(decode_field_value)
                .filter(|value| {
                    value
                        .get("provider")
                        .and_then(|provider| provider.get("id"))
                        .and_then(serde_json::Value::as_str)
                        == Some(crate::maps::PHYSICAL_PROVIDER)
                })
                .map(|value| self.physical_identity_for_descriptor(&value))
                .transpose()?
                .flatten()
        } else {
            None
        };
        if field.namespace == crate::maps::MAP_NAMESPACE {
            crate::maps::validate_field(
                &self.connection,
                &field.entity_id,
                &field.key,
                &field.value,
            )?;
        }
        if let Some(current_identity) = current_physical_identity {
            let next_identity = self
                .physical_identity_for_descriptor(&field.value)?
                .ok_or_else(|| {
                    CoreError::Validation(
                        "physical identity fields are immutable; create a new map".into(),
                    )
                })?;
            if next_identity != current_identity {
                return Err(CoreError::Validation(
                    "physical identity fields are immutable; create a new map".into(),
                ));
            }
        }
        if !field.revision.is_empty() {
            let current: Option<String> = self
                .connection
                .query_row(
                    "SELECT value FROM entity_fields WHERE entity_id=?1 AND namespace=?2 AND key=?3",
                    params![field.entity_id, field.namespace, field.key],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(current) = current else {
                return Err(CoreError::Conflict(
                    "field revision conflict: field does not exist".into(),
                ));
            };
            let current_field = FieldValue {
                entity_id: field.entity_id.clone(),
                namespace: field.namespace.clone(),
                key: field.key.clone(),
                value: decode_field_value(current),
                revision: String::new(),
            };
            Self::ensure_expected_revision(
                Some(&field.revision),
                self.revision_for_field(&current_field)?,
                "field",
            )?;
        }
        let value = encode_field_value(&field.value)?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("entities/{}/", field.entity_id)],
        )?;
        transaction.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,?2,?3,?4) ON CONFLICT(entity_id,namespace,key) DO UPDATE SET value=excluded.value", params![field.entity_id, field.namespace, field.key, value])?;
        transaction.commit()?;
        self.refresh_maps_projection_for_entities(std::slice::from_ref(&field.entity_id))?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn list_fields(&self, entity_id: String) -> Result<Vec<FieldValue>, CoreError> {
        self.list_fields_unchecked(entity_id)
    }

    pub(crate) fn list_fields_unchecked(
        &self,
        entity_id: String,
    ) -> Result<Vec<FieldValue>, CoreError> {
        let mut s = self.connection.prepare(
            "SELECT entity_id,namespace,key,value FROM entity_fields WHERE entity_id=?1",
        )?;
        let rows = s.query_map(params![entity_id], |r| {
            let value: String = r.get(3)?;
            Ok(FieldValue {
                entity_id: r.get(0)?,
                namespace: r.get(1)?,
                key: r.get(2)?,
                value: decode_field_value(value),
                revision: String::new(),
            })
        })?;
        let mut fields = rows.collect::<Result<Vec<_>, _>>()?;
        for field in &mut fields {
            field.revision = self.revision_for_field(field)?;
        }
        Ok(fields)
    }

    pub fn get_entities(&self, ids: &[String]) -> Result<Vec<Entity>, CoreError> {
        if ids.is_empty() || ids.len() > MAX_ENTITY_GET_MANY {
            return Err(CoreError::Validation(format!(
                "entity.getMany requires 1 to {MAX_ENTITY_GET_MANY} ids"
            )));
        }
        let unique = ids.iter().collect::<BTreeSet<_>>();
        if unique.len() != ids.len() {
            return Err(CoreError::Validation(
                "entity.getMany ids must be unique".into(),
            ));
        }
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let mut statement = self.connection.prepare(&format!(
            "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id IN ({placeholders}) AND deleted=0"
        ))?;
        let rows = statement.query_map(rusqlite::params_from_iter(ids.iter()), |row| {
            Ok(Entity {
                id: row.get(0)?,
                name: row.get(1)?,
                entity_type: row.get(2)?,
                deleted: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                revision: String::new(),
            })
        })?;
        let mut found = rows
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|entity| (entity.id.clone(), entity))
            .collect::<BTreeMap<_, _>>();
        let mut entities = Vec::new();
        for id in ids {
            if let Some(mut entity) = found.remove(id) {
                entity.revision = self.revision_for_entity(&entity.id)?;
                entities.push(entity);
            }
        }
        Ok(entities)
    }

    pub fn search(&self, query: String) -> Result<Vec<Entity>, CoreError> {
        if query.trim().is_empty() {
            return self.list_entities();
        }
        Ok(self
            .query_entities(EntityListQuery {
                query: Some(query),
                sort_field: Some(EntitySortField::Relevance),
                limit: Some(100),
                ..EntityListQuery::default()
            })?
            .items)
    }

    pub fn search_map_features(
        &self,
        query: String,
        limit: usize,
    ) -> Result<Vec<MapFeatureSearchResult>, CoreError> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut statement = self.connection.prepare(
            "SELECT s.map_entity_id,e.name,s.feature_id,s.name,s.semantic_type,s.layer_id,s.layer_name,s.rank
             FROM map_feature_search s
             JOIN entities e ON e.id=s.map_entity_id AND e.deleted=0
             WHERE map_feature_search MATCH ?1
             ORDER BY s.rank,s.name,s.feature_id LIMIT ?2",
        )?;
        let rows = statement.query_map(params![terms, limit.min(100) as i64], |row| {
            Ok(MapFeatureSearchResult {
                map_entity_id: row.get(0)?,
                map_name: row.get(1)?,
                feature_id: row.get(2)?,
                name: row.get(3)?,
                semantic_type: row.get(4)?,
                layer_id: row.get(5)?,
                layer_name: row.get(6)?,
                rank: row.get(7)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(CoreError::from)
    }

    /// Return ranked FTS rows rather than collapsing matches to entities. The
    /// AI context builder adds authorization, byte ranges, and prompt framing.
    pub fn search_passages(
        &self,
        query: String,
        limit: usize,
    ) -> Result<Vec<SearchPassage>, CoreError> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let mut statement = self.connection.prepare(
            "SELECT entity_id,source_path,source_hash,content,rank FROM world_search WHERE world_search MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = statement.query_map(params![terms, limit as i64], |row| {
            Ok(SearchPassage {
                entity_id: row.get(0)?,
                source_path: row.get(1)?,
                source_hash: row.get(2)?,
                content: row.get(3)?,
                lexical_rank: row.get(4)?,
                source_kind: {
                    let path: String = row.get(1)?;
                    if path.ends_with("/document.md") {
                        "document".into()
                    } else if path.contains("/fields/") {
                        "field".into()
                    } else {
                        "entity".into()
                    }
                },
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map(|mut passages| {
                for (rank, passage) in passages.iter_mut().enumerate() {
                    passage.lexical_rank = rank as f64;
                }
                passages
            })
            .map_err(CoreError::from)
    }
}
