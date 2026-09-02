// Module records, migrations, and schema overlays.
use super::*;

impl ProjectStore {
    pub const SCHEMA_OVERLAY_COUNT_LIMIT: usize = 256;

    pub(crate) fn runtime_ai_enabled(&self) -> Result<Option<bool>, CoreError> {
        self.connection
            .query_row(
                "SELECT value FROM project_meta WHERE key='ai_enabled'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|value| match value.as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(CoreError::Validation(
                    "runtime project AI setting is invalid".into(),
                )),
            })
            .transpose()
    }

    pub fn ai_enabled(&self) -> Result<bool, CoreError> {
        if let Some(enabled) = self.runtime_ai_enabled()? {
            return Ok(enabled);
        }
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| CoreError::NotFound("no project is open".to_string()))?;
        crate::storage::read_json::<crate::storage::ProjectManifest>(&root.join("project.json"))
            .map(|manifest| manifest.ai_enabled)
    }

    pub(crate) fn runtime_project_manifest(
        &self,
    ) -> Result<crate::storage::ProjectManifest, CoreError> {
        let root = self
            .root
            .as_ref()
            .ok_or_else(|| CoreError::NotFound("no project is open".to_string()))?;
        let path = root.join("project.json");
        let mut manifest = crate::storage::read_json::<crate::storage::ProjectManifest>(&path)?;
        if let Some(enabled) = self.runtime_ai_enabled()? {
            manifest.ai_enabled = enabled;
        }
        manifest.validate(&path)?;
        Ok(manifest)
    }

    /// Persists the project-level AI opt-in through the runtime authority and
    /// lets the checkpoint exporter render canonical `project.json`.
    pub fn set_ai_enabled(&self, enabled: bool) -> Result<ProjectInfo, CoreError> {
        if self.ai_enabled()? == enabled {
            return Ok(self.info().expect("root is present"));
        }
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO project_meta(key,value) VALUES ('ai_enabled',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [if enabled { "true" } else { "false" }],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(self.info().expect("root is present"))
    }

    /// Remove only data owned by a plugin. Code uninstall and disablement do
    /// not call this method; callers must present the explicit confirmation
    /// phrase and a backup is created before the destructive transaction.
    pub fn create_module_record(
        &self,
        module_id: &str,
        collection: &str,
        owner_entity_id: &str,
        value: serde_json::Value,
        request_id: Option<&str>,
    ) -> Result<ModuleRecord, CoreError> {
        validate_module_record_input(module_id, collection, owner_entity_id, &value)?;
        let fingerprint = digest_bytes(
            &serde_json::to_vec(&(module_id, collection, owner_entity_id, &value))
                .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut record) = self
            .committed_mutation_with_fingerprint::<ModuleRecord>(request_id, Some(&fingerprint))?
        {
            record.revision = self.revision_for_module_record_value(&record)?;
            return Ok(record);
        }
        self.ensure_live_module_record_owner(owner_entity_id)?;
        let id = Uuid::new_v4().to_string();
        let now = chrono_like_now();
        let encoded = serde_json::to_string(&value)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let result = ModuleRecord {
            module_id: module_id.into(),
            collection: collection.into(),
            id: id.clone(),
            owner_entity_id: owner_entity_id.into(),
            value: value.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
            revision: String::new(),
        };
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&serde_json::to_value(&result)?),
            &[format!("plugins/{module_id}.json")],
            &fingerprint,
        )?;
        transaction.execute(
            "INSERT INTO module_records(module_id,collection,id,owner_entity_id,value,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?6)",
            params![module_id, collection, id, owner_entity_id, encoded, now],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let mut record = result;
        record.revision = self.revision_for_module_record(&record.id)?;
        Ok(record)
    }

    pub fn list_module_records(
        &self,
        module_id: &str,
        collection: &str,
        owner_entity_id: &str,
        query: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ModuleRecord>, CoreError> {
        self.list_module_records_with(
            module_id,
            collection,
            owner_entity_id,
            ModuleRecordListParams {
                query,
                limit,
                offset,
                ..ModuleRecordListParams::default()
            },
        )
    }

    pub fn list_module_records_with(
        &self,
        module_id: &str,
        collection: &str,
        owner_entity_id: &str,
        params: ModuleRecordListParams<'_>,
    ) -> Result<Vec<ModuleRecord>, CoreError> {
        validate_module_record_scope(module_id, collection, owner_entity_id)?;
        self.ensure_live_module_record_owner(owner_entity_id)?;
        let limit = params.limit.clamp(1, 100) as i64;
        let offset = i64::try_from(params.offset)
            .map_err(|_| CoreError::Validation("record offset is too large".into()))?;
        let query = params.query.unwrap_or_default().trim();
        if query.len() > 200 {
            return Err(CoreError::Validation(
                "record search query exceeds 200 bytes".into(),
            ));
        }
        let status = optional_record_filter(params.status, "record status")?;
        let tag = optional_record_filter(params.tag, "record tag")?;
        let order = module_record_order_sql(
            params.sort.unwrap_or("lemma"),
            if query.is_empty() { "" } else { "r." },
        )?;
        let filters = module_record_filter_sql(if query.is_empty() { "" } else { "r." });
        let sql = if query.is_empty() {
            format!(
                "SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records WHERE module_id=:module AND collection=:collection AND owner_entity_id=:owner {filters} ORDER BY {order} LIMIT :limit OFFSET :offset"
            )
        } else {
            format!(
                "SELECT r.module_id,r.collection,r.id,r.owner_entity_id,r.value,r.created_at,r.updated_at FROM module_records r JOIN module_record_search s ON s.record_id=r.id WHERE s.module_id=:module AND s.collection=:collection AND s.owner_entity_id=:owner AND module_record_search MATCH :terms {filters} ORDER BY {order} LIMIT :limit OFFSET :offset"
            )
        };
        let terms = query
            .split_whitespace()
            .map(|term| format!("\"{}\"*", term.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let homonyms = i64::from(params.homonyms_only);
        let mut statement = self.connection.prepare(&sql)?;
        let read_record = |row: &rusqlite::Row<'_>| {
            let encoded: String = row.get(4)?;
            Ok(ModuleRecord {
                module_id: row.get(0)?,
                collection: row.get(1)?,
                id: row.get(2)?,
                owner_entity_id: row.get(3)?,
                value: decode_field_value(encoded),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                revision: String::new(),
            })
        };
        let rows = if query.is_empty() {
            statement.query_map(
                named_params! {
                    ":module": module_id,
                    ":collection": collection,
                    ":owner": owner_entity_id,
                    ":status": status,
                    ":tag": tag,
                    ":homonyms": homonyms,
                    ":limit": limit,
                    ":offset": offset,
                },
                read_record,
            )?
        } else {
            statement.query_map(
                named_params! {
                    ":module": module_id,
                    ":collection": collection,
                    ":owner": owner_entity_id,
                    ":terms": terms,
                    ":status": status,
                    ":tag": tag,
                    ":homonyms": homonyms,
                    ":limit": limit,
                    ":offset": offset,
                },
                read_record,
            )?
        };
        let mut records = rows.collect::<Result<Vec<_>, _>>()?;
        for record in &mut records {
            record.revision = self.revision_for_module_record_value(record)?;
        }
        Ok(records)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_module_record(
        &self,
        module_id: &str,
        collection: &str,
        id: &str,
        owner_entity_id: &str,
        value: serde_json::Value,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<ModuleRecord, CoreError> {
        validate_module_record_input(module_id, collection, owner_entity_id, &value)?;
        Uuid::parse_str(id)
            .map_err(|_| CoreError::Validation("module record ID must be a UUID".into()))?;
        let fingerprint = digest_bytes(
            &serde_json::to_vec(&(
                module_id,
                collection,
                id,
                owner_entity_id,
                &value,
                expected_revision,
            ))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(mut record) = self
            .committed_mutation_with_fingerprint::<ModuleRecord>(request_id, Some(&fingerprint))?
        {
            record.revision = self.revision_for_module_record_value(&record)?;
            return Ok(record);
        }
        let current = self.module_record(module_id, collection, id, owner_entity_id)?;
        Self::ensure_expected_revision(Some(expected_revision), current.revision, "module record")?;
        let now = chrono_like_now();
        let encoded = serde_json::to_string(&value)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let result = ModuleRecord {
            value: value.clone(),
            updated_at: now.clone(),
            revision: String::new(),
            ..current
        };
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&serde_json::to_value(&result)?),
            &[format!("plugins/{module_id}.json")],
            &fingerprint,
        )?;
        transaction.execute(
            "UPDATE module_records SET value=?1,updated_at=?2 WHERE id=?3 AND module_id=?4 AND collection=?5 AND owner_entity_id=?6",
            params![encoded, now, id, module_id, collection, owner_entity_id],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        let mut record = result;
        record.revision = self.revision_for_module_record(id)?;
        Ok(record)
    }

    pub fn delete_module_record(
        &self,
        module_id: &str,
        collection: &str,
        id: &str,
        owner_entity_id: &str,
        expected_revision: &str,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        validate_module_record_scope(module_id, collection, owner_entity_id)?;
        Uuid::parse_str(id)
            .map_err(|_| CoreError::Validation("module record ID must be a UUID".into()))?;
        let fingerprint = digest_bytes(
            &serde_json::to_vec(&(
                module_id,
                collection,
                id,
                owner_entity_id,
                expected_revision,
            ))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if self
            .committed_mutation_with_fingerprint::<serde_json::Value>(
                request_id,
                Some(&fingerprint),
            )?
            .is_some()
        {
            return Ok(());
        }
        let current = self.module_record(module_id, collection, id, owner_entity_id)?;
        Self::ensure_expected_revision(Some(expected_revision), current.revision, "module record")?;
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("plugins/{module_id}.json")],
            &fingerprint,
        )?;
        transaction.execute(
            "DELETE FROM module_records WHERE id=?1 AND module_id=?2 AND collection=?3 AND owner_entity_id=?4",
            params![id, module_id, collection, owner_entity_id],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub(crate) fn module_record(
        &self,
        module_id: &str,
        collection: &str,
        id: &str,
        owner_entity_id: &str,
    ) -> Result<ModuleRecord, CoreError> {
        validate_module_record_scope(module_id, collection, owner_entity_id)?;
        let mut record = self
            .connection
            .query_row(
                "SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records WHERE id=?1 AND module_id=?2 AND collection=?3 AND owner_entity_id=?4",
                params![id, module_id, collection, owner_entity_id],
                |row| {
                    let encoded: String = row.get(4)?;
                    Ok(ModuleRecord {
                        module_id: row.get(0)?,
                        collection: row.get(1)?,
                        id: row.get(2)?,
                        owner_entity_id: row.get(3)?,
                        value: decode_field_value(encoded),
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        revision: String::new(),
                    })
                },
            )
            .optional()?
            .ok_or_else(|| CoreError::NotFound("module record not found".into()))?;
        record.revision = self.revision_for_module_record(&record.id)?;
        Ok(record)
    }

    pub(crate) fn ensure_live_module_record_owner(
        &self,
        owner_entity_id: &str,
    ) -> Result<(), CoreError> {
        let exists = self
            .connection
            .query_row(
                "SELECT 1 FROM entities WHERE id=?1 AND deleted=0",
                params![owner_entity_id],
                |_| Ok(()),
            )
            .optional()?;
        if exists.is_none() {
            return Err(CoreError::NotFound(
                "module record owner entity not found".into(),
            ));
        }
        Ok(())
    }

    pub fn delete_plugin_data(
        &self,
        plugin_id: &str,
        confirmation: &str,
    ) -> Result<String, CoreError> {
        self.delete_plugin_data_with_request(plugin_id, confirmation, None)
    }

    pub fn delete_plugin_data_with_request(
        &self,
        plugin_id: &str,
        confirmation: &str,
        request_id: Option<&str>,
    ) -> Result<String, CoreError> {
        if let Some(backup) = self.committed_mutation::<String>(request_id)? {
            return Ok(backup);
        }
        if plugin_id.trim().is_empty() || confirmation != plugin_id {
            return Err(CoreError::Unauthorized {
                operation: "confirm plugin data deletion",
            });
        }
        let backup = self.backup()?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&result),
            &["entities/".into(), "plugins/".into(), "assets/".into()],
        )?;
        transaction.execute(
            "DELETE FROM entity_fields WHERE namespace IN (SELECT namespace FROM module_namespaces WHERE module_id=?1)",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM assets WHERE namespace IN (SELECT namespace FROM module_namespaces WHERE module_id=?1)",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_fields WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_records WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_namespaces WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_versions WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_state WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM module_package_versions WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.execute(
            "DELETE FROM migration_history WHERE module_id=?1",
            params![plugin_id],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(backup)
    }

    pub fn rebuild_search(&self) -> Result<(), CoreError> {
        self.connection.execute_batch(
            "DROP TRIGGER IF EXISTS entities_search_insert;
             DROP TRIGGER IF EXISTS entities_search_update;
             DROP TRIGGER IF EXISTS entities_search_deleted;
             DROP TRIGGER IF EXISTS documents_search_insert;
             DROP TRIGGER IF EXISTS documents_search_update;
             DROP TRIGGER IF EXISTS documents_search_delete;
             DROP TRIGGER IF EXISTS entity_fields_search_insert;
             DROP TRIGGER IF EXISTS entity_fields_search_update;
             DROP TRIGGER IF EXISTS entity_fields_search_delete;
             DROP TRIGGER IF EXISTS module_records_search_insert;
             DROP TRIGGER IF EXISTS module_records_search_update;
             DROP TRIGGER IF EXISTS module_records_search_delete;
             DROP TABLE IF EXISTS world_search;
             DROP TABLE IF EXISTS module_record_search;
             DROP TABLE IF EXISTS map_feature_search;
             CREATE VIRTUAL TABLE world_search USING fts5(entity_id UNINDEXED, source_path UNINDEXED, source_hash UNINDEXED, content, source_key UNINDEXED);
             CREATE VIRTUAL TABLE map_feature_search USING fts5(map_entity_id UNINDEXED, feature_id UNINDEXED, name, semantic_type, layer_id UNINDEXED, layer_name, content);
             INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT e.id,'entities/' || e.id || '/entity.json','',e.name || ' ' || COALESCE(e.entity_type,''),'entity' FROM entities e WHERE e.deleted=0;
             INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT d.entity_id,'entities/' || d.entity_id || '/document.md','',d.body,'document:' || d.id FROM documents d JOIN entities e ON e.id=d.entity_id WHERE e.deleted=0;
             INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT f.entity_id,'entities/' || f.entity_id || '/fields','',f.namespace || ' ' || f.key || ' ' || f.value,'field:' || f.namespace || '/' || f.key FROM entity_fields f JOIN entities e ON e.id=f.entity_id WHERE e.deleted=0;
             CREATE TRIGGER entities_search_insert AFTER INSERT ON entities BEGIN INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.id,'entities/' || new.id || '/entity.json','',new.name || ' ' || COALESCE(new.entity_type,''),'entity' WHERE new.deleted=0; END;
             CREATE TRIGGER entities_search_update AFTER UPDATE OF name,entity_type ON entities BEGIN DELETE FROM world_search WHERE entity_id=old.id AND source_key='entity'; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.id,'entities/' || new.id || '/entity.json','',new.name || ' ' || COALESCE(new.entity_type,''),'entity' WHERE new.deleted=0; END;
             CREATE TRIGGER entities_search_deleted AFTER UPDATE OF deleted ON entities WHEN old.deleted <> new.deleted BEGIN DELETE FROM world_search WHERE entity_id=old.id; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.id,'entities/' || new.id || '/entity.json','',new.name || ' ' || COALESCE(new.entity_type,''),'entity' WHERE new.deleted=0; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT d.entity_id,'entities/' || d.entity_id || '/document.md','',d.body,'document:' || d.id FROM documents d WHERE d.entity_id=new.id AND new.deleted=0; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT f.entity_id,'entities/' || f.entity_id || '/fields','',f.namespace || ' ' || f.key || ' ' || f.value,'field:' || f.namespace || '/' || f.key FROM entity_fields f WHERE f.entity_id=new.id AND new.deleted=0; END;
             CREATE TRIGGER documents_search_insert AFTER INSERT ON documents BEGIN INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/document.md','',new.body,'document:' || new.id FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER documents_search_update AFTER UPDATE OF body ON documents BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='document:' || old.id; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/document.md','',new.body,'document:' || new.id FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER documents_search_delete AFTER DELETE ON documents BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='document:' || old.id; END;
             CREATE TRIGGER entity_fields_search_insert AFTER INSERT ON entity_fields BEGIN INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/fields','',new.namespace || ' ' || new.key || ' ' || new.value,'field:' || new.namespace || '/' || new.key FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER entity_fields_search_update AFTER UPDATE OF namespace,key,value ON entity_fields BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='field:' || old.namespace || '/' || old.key; INSERT INTO world_search(entity_id,source_path,source_hash,content,source_key) SELECT new.entity_id,'entities/' || new.entity_id || '/fields','',new.namespace || ' ' || new.key || ' ' || new.value,'field:' || new.namespace || '/' || new.key FROM entities WHERE id=new.entity_id AND deleted=0; END;
             CREATE TRIGGER entity_fields_search_delete AFTER DELETE ON entity_fields BEGIN DELETE FROM world_search WHERE entity_id=old.entity_id AND source_key='field:' || old.namespace || '/' || old.key; END;"
        )?;
        self.connection.execute_batch(
            "CREATE VIRTUAL TABLE module_record_search USING fts5(module_id UNINDEXED, collection UNINDEXED, owner_entity_id UNINDEXED, record_id UNINDEXED, content);
             INSERT INTO module_record_search(module_id,collection,owner_entity_id,record_id,content) SELECT module_id,collection,owner_entity_id,id,value FROM module_records;
             CREATE TRIGGER module_records_search_insert AFTER INSERT ON module_records BEGIN INSERT INTO module_record_search(module_id,collection,owner_entity_id,record_id,content) VALUES (new.module_id,new.collection,new.owner_entity_id,new.id,new.value); END;
             CREATE TRIGGER module_records_search_update AFTER UPDATE ON module_records BEGIN DELETE FROM module_record_search WHERE record_id=old.id; INSERT INTO module_record_search(module_id,collection,owner_entity_id,record_id,content) VALUES (new.module_id,new.collection,new.owner_entity_id,new.id,new.value); END;
             CREATE TRIGGER module_records_search_delete AFTER DELETE ON module_records BEGIN DELETE FROM module_record_search WHERE record_id=old.id; END;",
        )?;
        self.rebuild_maps_projection()?;
        Ok(())
    }

    pub fn seed_example(&mut self) -> Result<usize, CoreError> {
        if self.root.is_none() {
            return self.seed_example_unchecked();
        }
        self.suppress_sync.set(true);
        let result = self.seed_example_unchecked();
        self.suppress_sync.set(false);
        result.and_then(|count| {
            self.notify_export_worker()?;
            Ok(count)
        })
    }

    pub(crate) fn seed_example_unchecked(&mut self) -> Result<usize, CoreError> {
        let tx = self.connection.transaction()?;
        tx.execute("DELETE FROM assets", [])?;
        tx.execute("DELETE FROM entity_fields", [])?;
        tx.execute("DELETE FROM documents", [])?;
        tx.execute("DELETE FROM relationships", [])?;
        tx.execute("DELETE FROM module_records", [])?;
        tx.execute("DELETE FROM map_projection", [])?;
        tx.execute("DELETE FROM map_location_projection", [])?;
        tx.execute("DELETE FROM entities", [])?;
        let now = chrono_like_now();
        let eldermere_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![eldermere_id, "Eldermere", "daena.lore:place", now])?;
        let lord_ashford_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![lord_ashford_id, "Lord Ashford", "daena.lore:person", now])?;
        let glass_coast_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![glass_coast_id, "The Glass Coast", "daena.lore:place", now])?;
        let silver_hand_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![silver_hand_id, "The Silver Hand", "daena.lore:faction", now])?;
        let amulet_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![amulet_id, "Amulet of Tides", "daena.lore:artifact", now])?;
        let highland_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![highland_id, "Highland Culture", "daena.lore:culture", now])?;
        let founding_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![founding_id, "Founding of Eldermere", "daena.timeline:event", now])?;
        let treaty_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![treaty_id, "The Treaty of Ashes", "daena.timeline:event", now])?;
        let rebellion_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![rebellion_id, "The Tide Rebellion", "daena.timeline:event", now])?;
        let mira_vale_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![mira_vale_id, "Mira Vale", "daena.lore:person", now])?;
        let sunken_archive_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![sunken_archive_id, "The Sunken Archive", "daena.lore:place", now])?;
        let ember_court_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![ember_court_id, "The Ember Court", "daena.lore:faction", now])?;
        let star_compass_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![star_compass_id, "Star Compass", "daena.lore:artifact", now])?;
        let riverborn_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![riverborn_id, "Riverborn Culture", "daena.lore:culture", now])?;
        let war_embers_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![war_embers_id, "The War of Embers", "daena.timeline:event", now])?;
        let first_tide_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![first_tide_id, "The First Tide", "daena.timeline:event", now])?;
        let archive_opening_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![archive_opening_id, "Opening of the Sunken Archive", "daena.timeline:event", now])?;
        let frostgate_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![frostgate_id, "Frostgate Pass", "daena.lore:place", now])?;
        let lantern_marsh_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![lantern_marsh_id, "Lantern Marsh", "daena.lore:place", now])?;
        let elian_rook_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![elian_rook_id, "Captain Elian Rook", "daena.lore:person", now])?;
        let sera_ashdown_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![sera_ashdown_id, "Sera Ashdown", "daena.lore:person", now])?;
        let tidewatch_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![tidewatch_id, "The Tidewatch", "daena.lore:faction", now])?;
        let crown_salt_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![crown_salt_id, "Crown of Salt", "daena.lore:artifact", now])?;
        let coastfolk_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![coastfolk_id, "Coastfolk Culture", "daena.lore:culture", now])?;
        let frostgate_battle_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO entities(id,name,entity_type,deleted,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?4)", params![frostgate_battle_id, "The Battle of Frostgate", "daena.timeline:event", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), eldermere_id, "markdown", "Eldermere is the ancient seat of power, a coastal fortress built upon the cliffs where the river meets the sea.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                lord_ashford_id,
                "markdown",
                "Lord Ashford rules Eldermere with a steady hand and a sharp mind.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), glass_coast_id, "markdown", "The Glass Coast is a stretch of shoreline where the sea glass glitters like scattered jewels.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                silver_hand_id,
                "markdown",
                "The Silver Hand is a sworn brotherhood dedicated to the protection of Eldermere.",
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                amulet_id,
                "markdown",
                "The Amulet of Tides grants its bearer the ability to breathe beneath the waves.",
                now
            ],
        )?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                highland_id,
                "markdown",
                "The Highland Culture values honor, craftsmanship, and the old songs.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), founding_id, "markdown", "The founding of Eldermere marked the unification of the coastal clans under one banner.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), treaty_id, "markdown", "The Treaty of Ashes ended the War of Embers and established the borders of the realm.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                rebellion_id,
                "markdown",
                "The Tide Rebellion was a brief but violent uprising against the crown.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), mira_vale_id, "markdown", "Mira Vale is a mapmaker who can read the coast by moonlight and remembers every vanished inlet.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sunken_archive_id, "markdown", "The Sunken Archive lies beneath the old river delta, its sealed chambers holding histories the crown chose to forget.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), ember_court_id, "markdown", "The Ember Court preserves the old fire rites and quietly contests the Silver Hand's claim to protect the realm.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                star_compass_id,
                "markdown",
                "The Star Compass points toward what has been lost rather than toward north.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), riverborn_id, "markdown", "The Riverborn Culture follows the delta's shifting channels and keeps songs for every flood season.", now])?;
        tx.execute(
            "INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                war_embers_id,
                "markdown",
                "The War of Embers began when the frontier forges refused the crown's new tribute.",
                now
            ],
        )?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), first_tide_id, "markdown", "The First Tide is remembered as the night the sea crossed the old boundary stones.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), archive_opening_id, "markdown", "Mira Vale opened the upper vault of the Sunken Archive and found a map bearing the Star Compass mark.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), frostgate_id, "markdown", "Frostgate Pass is the only safe road through the northern cliffs, guarded by old watchtowers and a newer border wall.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lantern_marsh_id, "markdown", "Lantern Marsh glows with blue reeds after dusk; its hidden channels lead from the delta to the western sea.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, "markdown", "Captain Elian Rook commands the coast patrols and distrusts any map that has not survived a flood season.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, "markdown", "Sera Ashdown is an archivist of the Ember Court who believes the forgotten histories can prevent another war.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, "markdown", "The Tidewatch keeps signal fires along the coast and answers to no single noble house.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, "markdown", "The Crown of Salt is a ceremonial relic worn by the first ruler to unite the river and coast clans.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, "markdown", "The Coastfolk Culture treats the tides as a calendar and settles disputes by reciting the names of drowned villages.", now])?;
        tx.execute("INSERT INTO documents(id,entity_id,format,body,updated_at) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), frostgate_battle_id, "markdown", "The Battle of Frostgate broke the northern siege and made the pass a symbol of the realm's uneasy unity.", now])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![eldermere_id, "Ancient coastal fortress and seat of power"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![eldermere_id, "The Elder Hold, The Clifftop"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![lord_ashford_id, "Ruler of Eldermere and keeper of the realm"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![lord_ashford_id, "Ash, The Lord"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![mira_vale_id, "Cartographer of the changing coast"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','aliases',?2)", params![mira_vale_id, "The Moonlit Mapmaker"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![sunken_archive_id, "Buried repository beneath the river delta"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![ember_court_id, "Keepers of the old fire rites"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![star_compass_id, "Artifact that points toward lost places"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','summary',?2)", params![riverborn_id, "Delta culture of navigators and singers"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![founding_id, "0001-01-01"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![founding_id, "0001-01-01"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![treaty_id, "0042-03-15"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![treaty_id, "0042-06-02"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![rebellion_id, "0067-07-22"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![rebellion_id, "0068-02-11"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![war_embers_id, "0038-09-04"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![war_embers_id, "0042-03-14"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![first_tide_id, "0021-11-19"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','startsAt',?2)", params![archive_opening_id, "0071-04-08"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'timeline','endsAt',?2)", params![archive_opening_id, "0071-04-10"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![eldermere_id, "Southern coast"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![eldermere_id, "Salt wind and mild winters"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![eldermere_id, "18000"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![glass_coast_id, "Western shore"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![glass_coast_id, "Stormy and bright"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![glass_coast_id, "4200"])?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![silver_hand_id, "Protect the realm's roads and rulers"],
        )?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![ember_court_id, "Preserve the old fire rites"],
        )?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![highland_id, "Honor, craft, and ancestral songs"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![highland_id, "High Cant"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![riverborn_id, "Adaptability, memory, and hospitality"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![riverborn_id, "River Cant"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![lord_ashford_id, "Ruler and treaty keeper"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![mira_vale_id, "Cartographer and coastal guide"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![frostgate_id, "Northern frontier"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![frostgate_id, "Cold, windy, and snowbound"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![frostgate_id, "900"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','region',?2)", params![lantern_marsh_id, "Eastern delta"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','climate',?2)", params![lantern_marsh_id, "Wet and misty"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','population',?2)", params![lantern_marsh_id, "2300"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![elian_rook_id, "Coast patrol captain"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','occupation',?2)", params![sera_ashdown_id, "Archivist and fire-rite scholar"])?;
        tx.execute(
            "INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','goal',?2)",
            params![tidewatch_id, "Keep the coast's signal network alive"],
        )?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','material',?2)", params![crown_salt_id, "Silver, pearl, and black coral"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','values',?2)", params![coastfolk_id, "Reciprocity, remembrance, and safe passage"])?;
        tx.execute("INSERT INTO entity_fields(entity_id,namespace,key,value) VALUES (?1,'lore','language',?2)", params![coastfolk_id, "Coastal Sign"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lord_ashford_id, eldermere_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), lord_ashford_id, silver_hand_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), amulet_id, mira_vale_id, "created_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), amulet_id, lord_ashford_id, "owned_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), silver_hand_id, lord_ashford_id, "led_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), silver_hand_id, eldermere_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), highland_id, eldermere_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, glass_coast_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), elian_rook_id, silver_hand_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, sunken_archive_id, "originates_from", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), sera_ashdown_id, ember_court_id, "affiliated_with", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, elian_rook_id, "led_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, lantern_marsh_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), tidewatch_id, frostgate_id, "based_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, sera_ashdown_id, "created_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), crown_salt_id, lord_ashford_id, "owned_by", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, glass_coast_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), coastfolk_id, lantern_marsh_id, "rooted_in", "{}"])?;
        tx.execute("INSERT INTO relationships(id,source_id,target_id,relationship_type,metadata) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), ember_court_id, sera_ashdown_id, "led_by", "{}"])?;
        tx.execute("INSERT OR IGNORE INTO module_versions(module_id,version) VALUES ('daena.lore',1), ('daena.timeline',1)", [])?;
        tx.execute("INSERT OR IGNORE INTO module_namespaces(module_id,namespace) VALUES ('daena.lore','lore'), ('daena.timeline','timeline')", [])?;
        tx.commit()?;
        self.rebuild_search()?;
        self.notify_export_worker()?;
        Ok(25)
    }

    pub fn get_module_version(&self, module_id: &str) -> Result<i64, CoreError> {
        self.connection
            .query_row(
                "SELECT COALESCE(version, 0) FROM module_versions WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .or_else(|e| {
                if e == rusqlite::Error::QueryReturnedNoRows {
                    Ok(0)
                } else {
                    Err(e)
                }
            })
            .map_err(CoreError::Database)
    }

    pub fn module_states(&self) -> Result<Vec<ModuleState>, CoreError> {
        let mut statement = self.connection.prepare("SELECT m.module_id, COALESCE(s.enabled, 1), m.version, p.package_version, o.overlay_json FROM module_versions m LEFT JOIN module_state s ON s.module_id = m.module_id LEFT JOIN module_package_versions p ON p.module_id = m.module_id LEFT JOIN module_schema_overlays o ON o.module_id = m.module_id ORDER BY m.module_id")?;
        let rows = statement.query_map([], |row| {
            let overlay_json: Option<String> = row.get(4)?;
            let schema_overlay = overlay_json.and_then(|json| {
                serde_json::from_str(&json)
                    .ok()
                    .filter(|value: &serde_json::Value| !value.is_null())
            });
            Ok(ModuleState {
                module_id: row.get(0)?,
                enabled: row.get::<_, i64>(1)? != 0,
                version: row.get(2)?,
                package_version: row.get(3)?,
                schema_overlay,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn validate_migration(
        &self,
        migration: &crate::migrations::Migration,
        current: i64,
    ) -> Result<(), CoreError> {
        crate::migrations::validate(migration, current)
    }

    pub fn set_module_enabled(&self, module_id: String, enabled: bool) -> Result<(), CoreError> {
        self.set_module_enabled_with_request(module_id, enabled, None)
    }

    pub fn set_module_enabled_with_request(
        &self,
        module_id: String,
        enabled: bool,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("plugins/{module_id}.json")],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
            params![module_id],
        )?;
        transaction.execute(
            "INSERT INTO module_state(module_id, enabled) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET enabled=excluded.enabled",
            params![module_id, i64::from(enabled)],
        )?;
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn is_module_enabled(&self, module_id: &str) -> Result<bool, CoreError> {
        Ok(self
            .connection
            .query_row(
                "SELECT enabled FROM module_state WHERE module_id=?1",
                params![module_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(1)
            != 0)
    }

    pub fn module_schema_overlay(
        &self,
        module_id: &str,
    ) -> Result<Option<serde_json::Value>, CoreError> {
        let overlay_json: Option<String> = self
            .connection
            .query_row(
                "SELECT overlay_json FROM module_schema_overlays WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(overlay_json.and_then(|json| serde_json::from_str(&json).ok()))
    }

    /// Opaque content revision for a module schema overlay (empty overlay included).
    pub fn revision_for_module_schema_overlay(&self, module_id: &str) -> Result<String, CoreError> {
        let value = self
            .module_schema_overlay(module_id)?
            .unwrap_or_else(|| serde_json::json!({}));
        self.revision_digest(&("module_schema_overlay", module_id, value))
    }

    /// Bounded live entity counts for the given types (`deleted=0` only).
    /// Returns `(counts, truncated)` when more than [`SCHEMA_OVERLAY_COUNT_LIMIT`] ids were requested.
    pub fn count_live_entities_by_types(
        &self,
        entity_types: &[String],
    ) -> Result<(BTreeMap<String, u64>, bool), CoreError> {
        let mut counts = BTreeMap::new();
        if entity_types.is_empty() {
            return Ok((counts, false));
        }
        let truncated = entity_types.len() > Self::SCHEMA_OVERLAY_COUNT_LIMIT;
        let limited: Vec<&String> = entity_types
            .iter()
            .take(Self::SCHEMA_OVERLAY_COUNT_LIMIT)
            .collect();
        let placeholders = std::iter::repeat_n("?", limited.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT entity_type, COUNT(*) FROM entities WHERE deleted=0 AND entity_type IN ({placeholders}) GROUP BY entity_type"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(limited.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (entity_type, count) = row?;
            counts.insert(entity_type, u64::try_from(count).unwrap_or_default());
        }
        for entity_type in limited {
            counts.entry(entity_type.clone()).or_insert(0);
        }
        Ok((counts, truncated))
    }

    /// Bounded live stored field-value counts by key (`deleted=0` entities only).
    /// Returns `(counts, truncated)` when more than [`SCHEMA_OVERLAY_COUNT_LIMIT`] keys were requested.
    pub fn count_live_field_values_by_keys(
        &self,
        field_keys: &[String],
    ) -> Result<(BTreeMap<String, u64>, bool), CoreError> {
        let mut counts = BTreeMap::new();
        if field_keys.is_empty() {
            return Ok((counts, false));
        }
        let truncated = field_keys.len() > Self::SCHEMA_OVERLAY_COUNT_LIMIT;
        let limited: Vec<&String> = field_keys
            .iter()
            .take(Self::SCHEMA_OVERLAY_COUNT_LIMIT)
            .collect();
        let placeholders = std::iter::repeat_n("?", limited.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT ef.key, COUNT(*) FROM entity_fields ef
             INNER JOIN entities e ON e.id = ef.entity_id AND e.deleted=0
             WHERE ef.key IN ({placeholders})
             GROUP BY ef.key"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(limited.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (key, count) = row?;
            counts.insert(key, u64::try_from(count).unwrap_or_default());
        }
        for key in limited {
            counts.entry(key.clone()).or_insert(0);
        }
        Ok((counts, truncated))
    }

    /// Reject removing custom types that still have live entities.
    pub fn ensure_module_schema_overlay_type_removals_resolved(
        &self,
        module_id: &str,
        candidate: Option<&serde_json::Value>,
    ) -> Result<(), CoreError> {
        let current_value = self
            .module_schema_overlay(module_id)?
            .unwrap_or_else(|| serde_json::json!({}));
        let current = daena_plugin_api::parse_module_overlay(&current_value)
            .map_err(CoreError::Validation)?;
        let next_value = candidate.cloned().unwrap_or_else(|| serde_json::json!({}));
        let next =
            daena_plugin_api::parse_module_overlay(&next_value).map_err(CoreError::Validation)?;
        let next_ids: BTreeSet<&str> = next
            .custom_entity_types
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        let removed: Vec<String> = current
            .custom_entity_types
            .iter()
            .map(|item| item.id.clone())
            .filter(|id| !next_ids.contains(id.as_str()))
            .collect();
        if removed.is_empty() {
            return Ok(());
        }
        let (counts, truncated) = self.count_live_entities_by_types(&removed)?;
        let mut unresolved = removed
            .into_iter()
            .filter(|id| counts.get(id).copied().unwrap_or(0) > 0)
            .collect::<Vec<_>>();
        if truncated {
            // Cap hit: refuse rather than risk orphaning uncounded types.
            return Err(CoreError::Validation(
                "cannot remove schema types while live entity counts are incomplete; reassign entities in smaller batches"
                    .into(),
            ));
        }
        unresolved.sort();
        if unresolved.is_empty() {
            return Ok(());
        }
        Err(CoreError::Validation(format!(
            "cannot remove schema types still used by entities: {}",
            unresolved.join(", ")
        )))
    }

    /// Read-only impact preview for a candidate overlay against the installed package.
    pub fn preview_module_schema_overlay(
        &self,
        module_id: &str,
        package: &PluginManifest,
        candidate: &daena_plugin_api::ModuleSchemaOverlay,
    ) -> Result<daena_plugin_api::SchemaOverlayPreviewResult, CoreError> {
        let current_value = self
            .module_schema_overlay(module_id)?
            .unwrap_or_else(|| serde_json::json!({}));
        let current = daena_plugin_api::parse_module_overlay(&current_value)
            .map_err(CoreError::Validation)?;
        let normalized = match daena_plugin_api::normalize_candidate_overlay(package, candidate) {
            Ok(overlay) => overlay,
            Err(errors) => {
                return Ok(
                    daena_plugin_api::assemble_schema_overlay_preview_with_bounds(
                        &daena_plugin_api::SchemaOverlayDiff::default(),
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        errors,
                        false,
                        false,
                    ),
                );
            }
        };
        let diff = daena_plugin_api::diff_module_schema_overlays(&current, &normalized);
        let (type_counts, types_truncated) =
            self.count_live_entities_by_types(&diff.type_ids_needing_counts())?;
        let (field_counts, fields_truncated) =
            self.count_live_field_values_by_keys(&diff.field_keys_needing_counts())?;
        Ok(
            daena_plugin_api::assemble_schema_overlay_preview_with_bounds(
                &diff,
                &type_counts,
                &field_counts,
                Vec::new(),
                types_truncated,
                fields_truncated,
            ),
        )
    }

    pub fn set_module_schema_overlay(
        &self,
        module_id: String,
        overlay: Option<serde_json::Value>,
    ) -> Result<String, CoreError> {
        self.set_module_schema_overlay_with_request(module_id, overlay, None, None)
    }

    pub fn set_module_schema_overlay_with_request(
        &self,
        module_id: String,
        overlay: Option<serde_json::Value>,
        expected_revision: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<String, CoreError> {
        let input_fingerprint = digest_bytes(
            &serde_json::to_vec(&(
                &module_id,
                overlay.as_ref().unwrap_or(&serde_json::Value::Null),
            ))
            .map_err(|error| CoreError::Serialization(error.to_string()))?,
        );
        if let Some(previous) = self
            .committed_mutation_with_fingerprint::<ModuleSchemaOverlayReceipt>(
                request_id,
                Some(&input_fingerprint),
            )?
        {
            return Ok(previous.revision);
        }
        if module_id.trim().is_empty() {
            return Err(CoreError::Validation("module id is required".into()));
        }
        self.ensure_module_schema_overlay_type_removals_resolved(&module_id, overlay.as_ref())?;
        let current_revision = self.revision_for_module_schema_overlay(&module_id)?;
        Self::ensure_expected_revision(
            expected_revision,
            current_revision,
            "module schema overlay",
        )?;
        let request_id = self.request_id(request_id)?;
        let receipt = ModuleSchemaOverlayReceipt {
            module_id: module_id.clone(),
            overlay: overlay.clone().unwrap_or(serde_json::Value::Null),
            revision: String::new(),
        };
        let result = serde_json::to_value(&receipt)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let transaction = self.begin_mutation_with_fingerprint(
            &request_id,
            Some(&result),
            &[format!("plugins/{module_id}.json")],
            &input_fingerprint,
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
            params![module_id],
        )?;
        match overlay {
            Some(value) if !value.is_null() => {
                let overlay_json = serde_json::to_string(&value)
                    .map_err(|error| CoreError::Validation(error.to_string()))?;
                transaction.execute(
                    "INSERT INTO module_schema_overlays(module_id, overlay_json) VALUES (?1, ?2) ON CONFLICT(module_id) DO UPDATE SET overlay_json=excluded.overlay_json",
                    params![module_id, overlay_json],
                )?;
            }
            _ => {
                transaction.execute(
                    "DELETE FROM module_schema_overlays WHERE module_id=?1",
                    params![module_id],
                )?;
            }
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        let revision = self.revision_for_module_schema_overlay(&module_id)?;
        let final_receipt = ModuleSchemaOverlayReceipt {
            module_id,
            overlay: receipt.overlay,
            revision: revision.clone(),
        };
        let final_value = serde_json::to_value(&final_receipt)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        self.write_mutation_result(&request_id, &final_value)?;
        Ok(revision)
    }

    pub fn set_module_package_version(
        &self,
        module_id: &str,
        package_version: Option<&str>,
    ) -> Result<(), CoreError> {
        self.set_module_package_version_with_request(module_id, package_version, None)
    }

    pub fn set_module_package_version_with_request(
        &self,
        module_id: &str,
        package_version: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(), CoreError> {
        if self
            .committed_mutation::<serde_json::Value>(request_id)?
            .is_some()
        {
            return Ok(());
        }
        let request_id = self.request_id(request_id)?;
        let transaction = self.begin_mutation(
            &request_id,
            Some(&serde_json::Value::Null),
            &[format!("plugins/{module_id}.json")],
        )?;
        match package_version {
            Some(version) => {
                transaction.execute(
                    "INSERT OR IGNORE INTO module_versions(module_id,version) VALUES (?1,0)",
                    params![module_id],
                )?;
                transaction.execute(
                    "INSERT INTO module_package_versions(module_id,package_version) VALUES (?1,?2) ON CONFLICT(module_id) DO UPDATE SET package_version=excluded.package_version",
                    params![module_id, version],
                )?;
            }
            None => {
                transaction.execute(
                    "DELETE FROM module_package_versions WHERE module_id=?1",
                    params![module_id],
                )?;
            }
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(())
    }

    pub fn module_package_version(&self, module_id: &str) -> Result<Option<String>, CoreError> {
        self.connection
            .query_row(
                "SELECT package_version FROM module_package_versions WHERE module_id=?1",
                params![module_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(CoreError::from)
    }

    pub fn apply_migration(
        &mut self,
        migration: &crate::migrations::Migration,
    ) -> Result<(), CoreError> {
        self.apply_migrations_with_request(std::slice::from_ref(migration), None)
            .map(|_| ())
    }

    /// Apply a plugin migration chain after one backup. If a later migration
    /// fails, restore the backup so the project remains usable at its prior
    /// data version.
    pub fn apply_migrations(
        &mut self,
        migrations: &[crate::migrations::Migration],
    ) -> Result<String, CoreError> {
        self.apply_migrations_with_request(migrations, None)
    }

    pub fn apply_migrations_with_request(
        &mut self,
        migrations: &[crate::migrations::Migration],
        request_id: Option<&str>,
    ) -> Result<String, CoreError> {
        if let Some(backup) = self.committed_mutation::<String>(request_id)? {
            return Ok(backup);
        }
        let backup = self
            .backup()
            .map_err(|error| CoreError::Validation(error.to_string()))?;
        let request_id = self.request_id(request_id)?;
        let result = serde_json::to_value(&backup)
            .map_err(|error| CoreError::Serialization(error.to_string()))?;
        let affected = migrations
            .iter()
            .map(|migration| format!("plugins/{}.json", migration.module_id))
            .collect::<Vec<_>>();
        let transaction = self.begin_mutation(&request_id, Some(&result), &affected)?;
        for migration in migrations {
            crate::migrations::apply_in_transaction(&transaction, migration)?;
        }
        transaction.commit()?;
        self.notify_export_worker()?;
        Ok(backup)
    }
}
