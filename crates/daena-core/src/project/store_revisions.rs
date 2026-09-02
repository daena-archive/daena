// Opaque revision computation.
use super::*;

impl ProjectStore {
    pub(crate) fn revision_for_entity(&self, entity_id: &str) -> Result<String, CoreError> {
        let entity = self.connection.query_row(
            "SELECT id,name,entity_type,deleted,created_at,updated_at FROM entities WHERE id=?1",
            params![entity_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )?;
        let documents = self
            .connection
            .prepare(
                "SELECT id,format,body,updated_at FROM documents WHERE entity_id=?1 ORDER BY id",
            )?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let fields = self
            .connection
            .prepare("SELECT namespace,key,value FROM entity_fields WHERE entity_id=?1 ORDER BY namespace,key")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let relationships = self
            .connection
            .prepare("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id=?1 OR target_id=?1 ORDER BY id")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let assets = self
            .connection
            .prepare("SELECT id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets WHERE entity_id=?1 ORDER BY id")?
            .query_map(params![entity_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, Option<String>>(10)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let revision = self.revision_digest(&(entity, documents, fields, relationships, assets))?;
        Ok(revision)
    }

    pub(crate) fn populate_entity_revisions(
        &self,
        entities: &mut [Entity],
    ) -> Result<(), CoreError> {
        if entities.is_empty() {
            return Ok(());
        }

        let ids = entities
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        // Search returns at most 100 entities. Restrict those supporting reads;
        // full entity lists are cheaper as four sequential table scans than as
        // hundreds of indexed point queries.
        let restricted = ids.len() <= 100;
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(",");

        let mut documents: BTreeMap<String, Vec<(String, String, String, String)>> =
            BTreeMap::new();
        let sql = if restricted {
            format!("SELECT entity_id,id,format,body,updated_at FROM documents WHERE entity_id IN ({placeholders}) ORDER BY entity_id,id")
        } else {
            "SELECT entity_id,id,format,body,updated_at FROM documents ORDER BY entity_id,id".into()
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            rusqlite::params_from_iter(ids.iter().take(if restricted { ids.len() } else { 0 })),
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ),
                ))
            },
        )?;
        for row in rows {
            let (entity_id, document) = row?;
            documents.entry(entity_id).or_default().push(document);
        }

        let mut fields: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
        let sql = if restricted {
            format!("SELECT entity_id,namespace,key,value FROM entity_fields WHERE entity_id IN ({placeholders}) ORDER BY entity_id,namespace,key")
        } else {
            "SELECT entity_id,namespace,key,value FROM entity_fields ORDER BY entity_id,namespace,key".into()
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            rusqlite::params_from_iter(ids.iter().take(if restricted { ids.len() } else { 0 })),
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ),
                ))
            },
        )?;
        for row in rows {
            let (entity_id, field) = row?;
            fields.entry(entity_id).or_default().push(field);
        }

        type RelationshipRevision = (String, String, String, String, String);
        let mut relationships: BTreeMap<String, Vec<RelationshipRevision>> = BTreeMap::new();
        let sql = if restricted {
            format!("SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE source_id IN ({placeholders}) OR target_id IN ({placeholders}) ORDER BY id")
        } else {
            "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships ORDER BY id".into()
        };
        let relationship_params =
            ids.iter()
                .chain(ids.iter())
                .take(if restricted { ids.len() * 2 } else { 0 });
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(relationship_params), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let relationship = row?;
            relationships
                .entry(relationship.1.clone())
                .or_default()
                .push(relationship.clone());
            if relationship.2 != relationship.1 {
                relationships
                    .entry(relationship.2.clone())
                    .or_default()
                    .push(relationship);
            }
        }

        type AssetRevision = (
            String,
            String,
            String,
            String,
            i64,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
        );
        let mut assets: BTreeMap<String, Vec<AssetRevision>> = BTreeMap::new();
        let sql = if restricted {
            format!("SELECT entity_id,id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets WHERE entity_id IN ({placeholders}) ORDER BY entity_id,id")
        } else {
            "SELECT entity_id,id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets ORDER BY entity_id,id".into()
        };
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            rusqlite::params_from_iter(ids.iter().take(if restricted { ids.len() } else { 0 })),
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    (
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, Option<String>>(11)?,
                    ),
                ))
            },
        )?;
        for row in rows {
            let (entity_id, asset) = row?;
            assets.entry(entity_id).or_default().push(asset);
        }

        for entity in entities {
            let entity_value = (
                &entity.id,
                &entity.name,
                &entity.entity_type,
                i64::from(entity.deleted),
                &entity.created_at,
                &entity.updated_at,
            );
            entity.revision = self.revision_digest(&(
                entity_value,
                documents.remove(&entity.id).unwrap_or_default(),
                fields.remove(&entity.id).unwrap_or_default(),
                relationships.remove(&entity.id).unwrap_or_default(),
                assets.remove(&entity.id).unwrap_or_default(),
            ))?;
        }
        Ok(())
    }

    pub(crate) fn revision_for_document(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,entity_id,format,body FROM documents WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    pub(crate) fn revision_for_document_value(
        &self,
        document: &Document,
    ) -> Result<String, CoreError> {
        self.revision_digest(&(
            &document.id,
            &document.entity_id,
            &document.format,
            &document.body,
        ))
    }

    pub(crate) fn revision_for_field(&self, field: &FieldValue) -> Result<String, CoreError> {
        self.revision_digest(&(
            &field.entity_id,
            &field.namespace,
            &field.key,
            encode_field_value(&field.value)?,
        ))
    }

    pub(crate) fn revision_for_relationship(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,source_id,target_id,relationship_type,metadata FROM relationships WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    pub(crate) fn revision_for_relationship_value(
        &self,
        relationship: &Relationship,
    ) -> Result<String, CoreError> {
        self.revision_digest(&(
            &relationship.id,
            &relationship.source_id,
            &relationship.target_id,
            &relationship.relationship_type,
            &relationship.metadata,
        ))
    }

    pub(crate) fn revision_for_asset(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT id,entity_id,namespace,filename,content_hash,size,mime_type,path,created_at,role,reference_scope,provenance FROM assets WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<String>>(11)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    pub(crate) fn revision_for_asset_value(&self, asset: &Asset) -> Result<String, CoreError> {
        self.revision_digest(&(
            &asset.id,
            &asset.entity_id,
            &asset.namespace,
            &asset.filename,
            &asset.content_hash,
            asset.size,
            &asset.mime_type,
            &asset.path,
            &asset.created_at,
            &asset.role,
            &asset.reference_scope,
            encode_asset_provenance(&asset.provenance)?,
        ))
    }

    pub(crate) fn revision_for_module_record(&self, id: &str) -> Result<String, CoreError> {
        let value = self.connection.query_row(
            "SELECT module_id,collection,id,owner_entity_id,value,created_at,updated_at FROM module_records WHERE id=?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )?;
        self.revision_digest(&value)
    }

    pub(crate) fn revision_for_module_record_value(
        &self,
        record: &ModuleRecord,
    ) -> Result<String, CoreError> {
        self.revision_digest(&(
            &record.module_id,
            &record.collection,
            &record.id,
            &record.owner_entity_id,
            encode_field_value(&record.value)?,
            &record.created_at,
            &record.updated_at,
        ))
    }

    pub(crate) fn revision_digest<T: Serialize>(&self, value: &T) -> Result<String, CoreError> {
        revision_digest(&(&self.database_epoch, value))
    }
}
