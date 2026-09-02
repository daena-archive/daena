// Relationship and schema-constraint operations.
use super::*;

impl ProjectStore {
    /// Install the currently enabled plugin relationship schemas for this
    /// runtime session. The registry is derived state and is never persisted.
    pub fn set_relationship_metadata_schemas(
        &mut self,
        schemas: BTreeMap<String, Vec<MetadataFieldDefinition>>,
    ) -> Result<(), CoreError> {
        for (relationship_type, fields) in &schemas {
            if relationship_type.trim().is_empty() {
                return Err(CoreError::Validation(
                    "relationship metadata schema type cannot be empty".into(),
                ));
            }
            daena_plugin_api::validate_metadata_fields(
                "relationship",
                relationship_type,
                Some(fields),
            )
            .map_err(CoreError::Validation)?;
        }
        self.relationship_metadata_schemas = schemas;
        Ok(())
    }

    pub fn set_relationship_constraints(
        &mut self,
        constraints: BTreeMap<String, RelationshipConstraints>,
    ) -> Result<(), CoreError> {
        for relationship_type in constraints.keys() {
            if relationship_type.trim().is_empty() {
                return Err(CoreError::Validation(
                    "relationship constraint type cannot be empty".into(),
                ));
            }
        }
        self.relationship_constraints = constraints;
        Ok(())
    }

    pub(crate) fn constraints_for_relationship_type(
        &self,
        relationship_type: &str,
    ) -> RelationshipConstraints {
        self.relationship_constraints
            .get(relationship_type)
            .cloned()
            .unwrap_or_default()
    }

    pub(crate) fn metadata_fields_for_relationship_type(
        &self,
        relationship_type: &str,
    ) -> Option<&[MetadataFieldDefinition]> {
        self.relationship_metadata_schemas
            .get(relationship_type)
            .map(Vec::as_slice)
    }

    pub fn relationship_metadata_fields_for_type(
        &self,
        relationship_type: &str,
    ) -> Option<&[MetadataFieldDefinition]> {
        self.metadata_fields_for_relationship_type(relationship_type)
    }

    pub(crate) fn canonicalize_relationship_endpoints(
        &self,
        relationship_type: &str,
        source_id: &str,
        target_id: &str,
    ) -> Result<(String, String), CoreError> {
        if self
            .constraints_for_relationship_type(relationship_type)
            .unique
            == RelationshipUniqueness::Undirected
        {
            Self::canonicalize_undirected_endpoints(source_id, target_id)
        } else {
            Ok((source_id.to_owned(), target_id.to_owned()))
        }
    }

    pub(crate) fn canonicalize_undirected_endpoints(
        source_id: &str,
        target_id: &str,
    ) -> Result<(String, String), CoreError> {
        let source = Uuid::parse_str(source_id).map_err(|error| {
            CoreError::Validation(format!("relationship source is not a UUID: {error}"))
        })?;
        let target = Uuid::parse_str(target_id).map_err(|error| {
            CoreError::Validation(format!("relationship target is not a UUID: {error}"))
        })?;
        if source.as_bytes() <= target.as_bytes() {
            Ok((source_id.to_owned(), target_id.to_owned()))
        } else {
            Ok((target_id.to_owned(), source_id.to_owned()))
        }
    }

    pub(crate) fn validate_relationship_endpoints(
        &self,
        conn: &rusqlite::Connection,
        relationship_type: &str,
        source_id: &str,
        target_id: &str,
        exclude_id: Option<&str>,
    ) -> Result<(), CoreError> {
        let constraints = self.constraints_for_relationship_type(relationship_type);
        if !constraints.allow_self && source_id == target_id {
            return Err(CoreError::broker(
                "relationship.self",
                "relationship cannot target the same entity",
            ));
        }
        match constraints.unique {
            RelationshipUniqueness::None => {}
            RelationshipUniqueness::Directed => {
                if Self::relationship_pair_exists(
                    conn,
                    relationship_type,
                    source_id,
                    target_id,
                    false,
                    exclude_id,
                )? {
                    return Err(CoreError::broker(
                        "relationship.duplicate",
                        "a relationship already exists for these endpoints",
                    ));
                }
            }
            RelationshipUniqueness::Undirected => {
                if Self::relationship_pair_exists(
                    conn,
                    relationship_type,
                    source_id,
                    target_id,
                    true,
                    exclude_id,
                )? {
                    return Err(CoreError::broker(
                        "relationship.duplicate",
                        "a relationship already exists for these endpoints",
                    ));
                }
            }
        }
        if constraints.acyclic
            && Self::relationship_would_cycle(
                conn,
                relationship_type,
                source_id,
                target_id,
                exclude_id,
            )?
        {
            return Err(CoreError::broker(
                "relationship.cycle",
                "relationship would introduce a cycle",
            ));
        }
        Ok(())
    }

    pub(crate) fn relationship_pair_exists(
        conn: &rusqlite::Connection,
        relationship_type: &str,
        source_id: &str,
        target_id: &str,
        undirected: bool,
        exclude_id: Option<&str>,
    ) -> Result<bool, CoreError> {
        let sql = if undirected {
            "SELECT 1 FROM relationships WHERE relationship_type=?1 AND ((source_id=?2 AND target_id=?3) OR (source_id=?3 AND target_id=?2)) AND (?4 IS NULL OR id!=?4) LIMIT 1"
        } else {
            "SELECT 1 FROM relationships WHERE relationship_type=?1 AND source_id=?2 AND target_id=?3 AND (?4 IS NULL OR id!=?4) LIMIT 1"
        };
        let found: Option<i64> = conn
            .query_row(
                sql,
                params![relationship_type, source_id, target_id, exclude_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(found.is_some())
    }

    pub(crate) fn relationship_would_cycle(
        conn: &rusqlite::Connection,
        relationship_type: &str,
        source_id: &str,
        target_id: &str,
        exclude_id: Option<&str>,
    ) -> Result<bool, CoreError> {
        if source_id == target_id {
            return Ok(true);
        }
        let mut statement = conn.prepare(
            "SELECT id, source_id, target_id FROM relationships WHERE relationship_type=?1",
        )?;
        let rows = statement.query_map(params![relationship_type], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for row in rows {
            let (id, source, target) = row?;
            if exclude_id == Some(id.as_str()) {
                continue;
            }
            adjacency.entry(source).or_default().push(target);
        }
        adjacency
            .entry(source_id.to_owned())
            .or_default()
            .push(target_id.to_owned());
        let mut stack = vec![target_id.to_owned()];
        let mut seen = BTreeSet::from([target_id.to_owned()]);
        while let Some(current) = stack.pop() {
            if current == source_id {
                return Ok(true);
            }
            if let Some(next) = adjacency.get(&current) {
                for child in next {
                    if seen.insert(child.clone()) {
                        stack.push(child.clone());
                    }
                }
            }
        }
        Ok(false)
    }

    pub fn create_relationship(&self, input: RelationshipInput) -> Result<Relationship, CoreError> {
        self.create_relationship_with_options(input, None, None)
    }

    pub fn create_relationship_with_request(
        &self,
        input: RelationshipInput,
        request_id: Option<&str>,
    ) -> Result<Relationship, CoreError> {
        self.create_relationship_with_options(input, None, request_id)
    }

    pub fn create_relationship_with_options(
        &self,
        input: RelationshipInput,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Relationship, CoreError> {
        if let Some(relationship) = self.committed_mutation::<Relationship>(request_id)? {
            let mut relationship = relationship;
            relationship.revision = self.revision_for_relationship_value(&relationship)?;
            return Ok(relationship);
        }
        if input.relationship_type.trim().is_empty() {
            return Err(CoreError::NotFound(
                "relationship type cannot be empty".into(),
            ));
        }
        let (source_id, target_id) = self.canonicalize_relationship_endpoints(
            &input.relationship_type,
            &input.source_id,
            &input.target_id,
        )?;
        Self::ensure_expected_revision(
            expected_revision,
            self.revision_for_entity(&source_id)?,
            "relationship source entity",
        )?;
        let id = Uuid::new_v4().to_string();
        let metadata = input.metadata.unwrap_or_else(|| "{}".into());
        let metadata_value = serde_json::from_str(&metadata).map_err(|error| {
            CoreError::Validation(format!("relationship metadata is invalid JSON: {error}"))
        })?;
        let metadata = serde_json::to_string(&validate_relationship_metadata(
            &input.relationship_type,
            &metadata_value,
            self.metadata_fields_for_relationship_type(&input.relationship_type),
        )?)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&Relationship {
            id: id.clone(),
            source_id: source_id.clone(),
            target_id: target_id.clone(),
            relationship_type: input.relationship_type.clone(),
            metadata: metadata.clone(),
            revision: String::new(),
        })
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            &[
                format!("entities/{source_id}/"),
                format!("entities/{target_id}/"),
            ],
        )?;
        for entity_id in [&source_id, &target_id] {
            let exists: Option<String> = transaction
                .query_row(
                    "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                    params![entity_id],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(CoreError::NotFound("relationship entity not found".into()));
            }
        }
        self.validate_relationship_endpoints(
            &transaction,
            &input.relationship_type,
            &source_id,
            &target_id,
            None,
        )?;
        transaction.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![id, source_id, target_id, input.relationship_type, metadata])?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_relationship(&id)?;
        Ok(Relationship {
            id,
            source_id,
            target_id,
            relationship_type: input.relationship_type,
            metadata,
            revision,
        })
    }

    pub fn update_relationship(
        &self,
        input: RelationshipUpdate,
    ) -> Result<Relationship, CoreError> {
        self.update_relationship_with_options(input, None, None)
    }

    pub fn update_relationship_with_options(
        &self,
        input: RelationshipUpdate,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<Relationship, CoreError> {
        if let Some(relationship) = self.committed_mutation::<Relationship>(request_id)? {
            let mut relationship = relationship;
            relationship.revision = self.revision_for_relationship_value(&relationship)?;
            return Ok(relationship);
        }

        let current = self.relationship(input.id.clone())?;
        Self::ensure_expected_revision(
            expected_revision,
            current.revision.clone(),
            "relationship",
        )?;

        let metadata = input.metadata.unwrap_or_else(|| current.metadata.clone());
        let metadata_value = serde_json::from_str(&metadata).map_err(|error| {
            CoreError::Validation(format!("relationship metadata is invalid JSON: {error}"))
        })?;
        let metadata = serde_json::to_string(&validate_relationship_metadata(
            &current.relationship_type,
            &metadata_value,
            self.metadata_fields_for_relationship_type(&current.relationship_type),
        )?)
        .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let requested_target = input
            .target_id
            .clone()
            .unwrap_or_else(|| current.target_id.clone());
        let (source_id, target_id) = self.canonicalize_relationship_endpoints(
            &current.relationship_type,
            &current.source_id,
            &requested_target,
        )?;
        let endpoints_changed = source_id != current.source_id || target_id != current.target_id;
        let updated = Relationship {
            id: current.id.clone(),
            source_id: source_id.clone(),
            target_id: target_id.clone(),
            relationship_type: current.relationship_type.clone(),
            metadata: metadata.clone(),
            revision: String::new(),
        };
        let result = serde_json::to_value(&updated)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let request_id = self.request_id(request_id)?;
        let affected_prefixes = if endpoints_changed {
            vec![
                format!("entities/{}/", current.source_id),
                format!("entities/{}/", current.target_id),
                format!("entities/{source_id}/"),
                format!("entities/{target_id}/"),
            ]
        } else {
            vec![format!("entities/{}/", current.source_id)]
        };
        let transaction = self.begin_mutation(&request_id, Some(&result), &affected_prefixes)?;
        if input.target_id.is_some() {
            let exists: Option<String> = transaction
                .query_row(
                    "SELECT id FROM entities WHERE id=?1 AND deleted=0",
                    params![requested_target],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                return Err(CoreError::NotFound(
                    "relationship target entity not found".into(),
                ));
            }
        }
        self.validate_relationship_endpoints(
            &transaction,
            &current.relationship_type,
            &source_id,
            &target_id,
            Some(&current.id),
        )?;
        if transaction.execute(
            "UPDATE relationships SET metadata=?1, source_id=?2, target_id=?3 WHERE id=?4",
            params![metadata, source_id, target_id, current.id],
        )? == 0
        {
            return Err(CoreError::NotFound("relationship not found".into()));
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_relationship(&updated.id)?;
        Ok(Relationship {
            revision,
            ..updated
        })
    }

    pub fn delete_relationship(&self, id: String) -> Result<(), CoreError> {
        self.delete_relationship_with_options(id, None, None)
    }

    pub fn delete_relationship_with_options(
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
            self.revision_for_relationship(&id)?,
            "relationship",
        )?;
        let (source_id, target_id): (String, String) = self.connection.query_row(
            "SELECT source_id,target_id FROM relationships WHERE id=?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[
                format!("entities/{source_id}/"),
                format!("entities/{target_id}/"),
            ],
        )?;
        if transaction.execute("DELETE FROM relationships WHERE id=?1", params![id])? == 0 {
            return Err(CoreError::NotFound("relationship not found".into()));
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn relationship(&self, id: String) -> Result<Relationship, CoreError> {
        self.connection
            .query_row(
                "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE id=?1",
                params![id],
                |row| {
                    Ok(Relationship {
                        id: row.get(0)?,
                        source_id: row.get(1)?,
                        target_id: row.get(2)?,
                        relationship_type: row.get(3)?,
                        metadata: row.get(4)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("relationship not found".into()))
            .and_then(|mut relationship| {
                relationship.revision = self.revision_for_relationship_value(&relationship)?;
                Ok(relationship)
            })
    }

    pub fn list_relationships(&self, entity_id: String) -> Result<Vec<Relationship>, CoreError> {
        self.list_relationships_unchecked(entity_id)
    }

    pub(crate) fn list_relationships_unchecked(
        &self,
        entity_id: String,
    ) -> Result<Vec<Relationship>, CoreError> {
        let mut statement = self.connection.prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id=?1 OR target_id=?1")?;
        let rows = statement.query_map(params![entity_id], |row| {
            Ok(Relationship {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                relationship_type: row.get(3)?,
                metadata: row.get(4)?,
                revision: String::new(),
            })
        })?;
        let mut relationships = rows.collect::<Result<Vec<_>, _>>()?;
        for relationship in &mut relationships {
            relationship.revision = self.revision_for_relationship_value(relationship)?;
        }
        Ok(relationships)
    }

    pub fn query_relationships(
        &self,
        query: RelationshipQuery,
    ) -> Result<RelationshipPage, CoreError> {
        let unique_ids = query.entity_ids.iter().collect::<BTreeSet<_>>();
        if query.entity_ids.is_empty()
            || query.entity_ids.len() > MAX_RELATIONSHIP_QUERY_ENTITIES
            || unique_ids.len() != query.entity_ids.len()
        {
            return Err(CoreError::Validation(format!(
                "relationship.query requires 1 to {MAX_RELATIONSHIP_QUERY_ENTITIES} unique entity ids"
            )));
        }
        let unique_types = query.relationship_types.iter().collect::<BTreeSet<_>>();
        if unique_types.len() != query.relationship_types.len() {
            return Err(CoreError::Validation(
                "relationship.query relationshipTypes must be unique".into(),
            ));
        }
        let limit = query
            .limit
            .unwrap_or(DEFAULT_RELATIONSHIP_QUERY_LIMIT)
            .clamp(1, MAX_RELATIONSHIP_QUERY_LIMIT);
        let offset = query.offset.unwrap_or(0);
        let mut sql = String::from(
            "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE ",
        );
        let mut params: Vec<SqlValue> = Vec::new();
        let entity_placeholders = std::iter::repeat_n("?", query.entity_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        match query.direction {
            RelationshipQueryDirection::Incoming => {
                sql.push_str(&format!("target_id IN ({entity_placeholders})"));
                params.extend(query.entity_ids.iter().cloned().map(SqlValue::Text));
            }
            RelationshipQueryDirection::Outgoing => {
                sql.push_str(&format!("source_id IN ({entity_placeholders})"));
                params.extend(query.entity_ids.iter().cloned().map(SqlValue::Text));
            }
            RelationshipQueryDirection::Any => {
                sql.push_str(&format!(
                    "(source_id IN ({entity_placeholders}) OR target_id IN ({entity_placeholders}))"
                ));
                params.extend(query.entity_ids.iter().cloned().map(SqlValue::Text));
                params.extend(query.entity_ids.iter().cloned().map(SqlValue::Text));
            }
        }
        if !query.relationship_types.is_empty() {
            let type_placeholders = std::iter::repeat_n("?", query.relationship_types.len())
                .collect::<Vec<_>>()
                .join(",");
            sql.push_str(&format!(" AND relationship_type IN ({type_placeholders})"));
            params.extend(query.relationship_types.iter().cloned().map(SqlValue::Text));
        }
        let count_sql = format!("SELECT COUNT(*) FROM ({sql}) counted");
        let total: u64 = self.connection.query_row(
            &count_sql,
            rusqlite::params_from_iter(params.iter()),
            |row| row.get(0),
        )?;
        sql.push_str(" ORDER BY id LIMIT ? OFFSET ?");
        params.push(SqlValue::Integer(i64::from(limit)));
        params.push(SqlValue::Integer(offset as i64));
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(Relationship {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                relationship_type: row.get(3)?,
                metadata: row.get(4)?,
                revision: String::new(),
            })
        })?;
        let mut items = rows.collect::<Result<Vec<_>, _>>()?;
        for relationship in &mut items {
            relationship.revision = self.revision_for_relationship_value(relationship)?;
        }
        Ok(RelationshipPage {
            items,
            total,
            offset,
            limit,
            has_more: offset.saturating_add(u64::from(limit)) < total,
        })
    }
}
