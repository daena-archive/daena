// Plugin broker RPC dispatch.
use super::*;

pub(super) fn payload_string(payload: &serde_json::Value, key: &str) -> Result<String, CoreError> {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| CoreError::Validation(format!("plugin RPC payload requires {key}")))
}

pub(super) fn optional_payload_string(
    payload: &serde_json::Value,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| payload.get(*key).and_then(serde_json::Value::as_str))
        .map(str::to_owned)
}

pub(super) fn required_payload_string(
    payload: &serde_json::Value,
    keys: &[&str],
    label: &str,
) -> Result<String, CoreError> {
    optional_payload_string(payload, keys).ok_or_else(|| {
        CoreError::Conflict(format!("revision-aware broker mutation requires {label}"))
    })
}

pub(super) fn payload_value<T: serde::de::DeserializeOwned>(
    payload: &serde_json::Value,
) -> Result<T, CoreError> {
    serde_json::from_value(payload.clone())
        .map_err(|error| CoreError::Validation(format!("invalid plugin RPC payload: {error}")))
}

pub(super) fn validate_broker_payload(
    method: &str,
    payload: &serde_json::Value,
) -> Result<(), CoreError> {
    daena_plugin_api::validate_rpc_payload(method, payload)
        .map_err(|error| CoreError::Validation(error.0))
}

#[derive(Debug, Clone)]
pub(super) enum RecordOwnerConstraint {
    Package(Vec<String>),
    EffectiveSchema {
        live_types: Vec<String>,
        unique_per_owner: bool,
    },
}

impl RecordOwnerConstraint {
    fn unique_per_owner(&self) -> bool {
        match self {
            Self::Package(_) => false,
            Self::EffectiveSchema {
                unique_per_owner, ..
            } => *unique_per_owner,
        }
    }
}

pub(super) fn record_owner_constraint_from_declaration(
    project: &ProjectStore,
    package: &PluginManifest,
    collection: &daena_plugin_api::RecordCollection,
) -> Result<RecordOwnerConstraint, CoreError> {
    match collection.owner_scope {
        daena_plugin_api::RecordOwnerScope::Package => Ok(RecordOwnerConstraint::Package(
            collection.owner_entity_types.clone(),
        )),
        daena_plugin_api::RecordOwnerScope::EffectiveSchema => {
            Ok(RecordOwnerConstraint::EffectiveSchema {
                live_types: live_record_owner_types(project, package)?,
                unique_per_owner: collection.unique_per_owner,
            })
        }
    }
}

fn live_record_owner_types(
    project: &ProjectStore,
    package: &PluginManifest,
) -> Result<Vec<String>, CoreError> {
    let types_of = |manifest: &PluginManifest| {
        manifest
            .schemas
            .iter()
            .flat_map(|schema| {
                schema
                    .entity_types
                    .iter()
                    .map(|entity_type| entity_type.id.clone())
            })
            .collect::<Vec<_>>()
    };
    if !supports_schema_overlay(package) {
        return Ok(types_of(package));
    }
    let overlay_value = project
        .module_schema_overlay(&package.id)?
        .unwrap_or_else(|| serde_json::json!({}));
    let overlay = parse_module_overlay(&overlay_value).map_err(CoreError::Validation)?;
    let merged = merge_module_manifest(package, &overlay).map_err(CoreError::Validation)?;
    Ok(types_of(&merged))
}

pub(super) fn validate_record_owner_entity_type(
    project: &ProjectStore,
    owner_entity_id: &str,
    method: &str,
    constraint: Option<&RecordOwnerConstraint>,
) -> Result<(), CoreError> {
    let constraint = constraint.ok_or_else(|| CoreError::Unauthorized {
        operation: "access undeclared module record collection",
    })?;
    let owner = project
        .get_entity(owner_entity_id)?
        .ok_or_else(|| CoreError::NotFound("module record owner entity not found".into()))?;
    let type_allowed = |allowed: &[String]| {
        owner
            .entity_type
            .as_ref()
            .is_some_and(|entity_type| allowed.contains(entity_type))
    };
    match constraint {
        RecordOwnerConstraint::Package(types) => {
            if !type_allowed(types) {
                return Err(CoreError::Unauthorized {
                    operation: "use disallowed module record owner entity type",
                });
            }
        }
        RecordOwnerConstraint::EffectiveSchema { live_types, .. } => {
            if method == "record.create" && !type_allowed(live_types) {
                return Err(CoreError::Unauthorized {
                    operation: "use disallowed module record owner entity type",
                });
            }
        }
    }
    Ok(())
}

pub(super) fn shared_field_keys_for_request(
    host: &PluginHost,
    plugin_id: &str,
    method: &str,
    payload: &serde_json::Value,
) -> Result<Option<std::collections::BTreeSet<String>>, String> {
    if method != "field.list" {
        return Ok(None);
    }
    let namespace = payload
        .get("namespace")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "field list payload requires namespace".to_string())?;
    let shared_only = payload
        .get("sharedOnly")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !shared_only && host.namespaces.owner(namespace) == Some(plugin_id) {
        Ok(None)
    } else {
        Ok(Some(host.namespaces.shared_field_keys(namespace)))
    }
}

pub(super) fn dispatch_module_rpc(
    core: &mut CoreService,
    plugin_id: Option<&str>,
    shared_field_keys: Option<std::collections::BTreeSet<String>>,
    record_owner_entity_types: Option<RecordOwnerConstraint>,
    method: &str,
    payload: serde_json::Value,
    request_id: Option<&str>,
) -> Result<serde_json::Value, CoreError> {
    validate_broker_payload(method, &payload)?;
    let project = core.project_mut(AuthorityContext::plugin())?;
    match method {
        "entity.list" => {
            let entity_type = payload
                .get("entityType")
                .and_then(serde_json::Value::as_str);
            let entities = if let Some(entity_type) = entity_type {
                let mut entities = Vec::new();
                let mut offset = 0_u64;
                loop {
                    let page = project.query_entities(EntityListQuery {
                        entity_types: vec![entity_type.to_owned()],
                        offset: Some(offset),
                        limit: Some(daena_core::MAX_ENTITY_QUERY_LIMIT),
                        ..EntityListQuery::default()
                    })?;
                    entities.extend(page.items);
                    if !page.has_more {
                        break;
                    }
                    offset = offset.saturating_add(u64::from(page.limit));
                }
                entities
            } else {
                project.list_entities()?
            };
            serde_json::to_value(entities).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.query" => {
            let payload: daena_plugin_api::EntityQueryPayload = serde_json::from_value(payload)
                .map_err(|error| {
                    CoreError::Validation(format!("invalid entity.query payload: {error}"))
                })?;
            let sort_field = match payload.sort_field.as_deref() {
                None => None,
                Some("name") => Some(EntitySortField::Name),
                Some("createdAt") | Some("created_at") => Some(EntitySortField::CreatedAt),
                Some("updatedAt") | Some("updated_at") => Some(EntitySortField::UpdatedAt),
                Some("relevance") => Some(EntitySortField::Relevance),
                Some(other) => {
                    return Err(CoreError::Validation(format!(
                        "unsupported entity sort field: {other}"
                    )))
                }
            };
            let sort_direction = match payload.sort_direction.as_deref() {
                None => None,
                Some("asc") => Some(EntitySortDirection::Asc),
                Some("desc") => Some(EntitySortDirection::Desc),
                Some(other) => {
                    return Err(CoreError::Validation(format!(
                        "unsupported entity sort direction: {other}"
                    )))
                }
            };
            let page = project.query_entities(EntityListQuery {
                query: payload.query,
                entity_types: payload.entity_types,
                excluded_entity_types: payload.excluded_entity_types,
                sort_field,
                sort_direction,
                offset: payload.offset,
                limit: payload.limit,
                ..EntityListQuery::default()
            })?;
            let record = daena_plugin_api::EntityPageRecord {
                items: page
                    .items
                    .into_iter()
                    .map(|entity| daena_plugin_api::EntityRecord {
                        id: entity.id,
                        name: entity.name,
                        entity_type: entity.entity_type,
                        deleted: entity.deleted,
                        created_at: entity.created_at,
                        updated_at: entity.updated_at,
                        revision: entity.revision,
                    })
                    .collect(),
                total: page.total,
                offset: page.offset,
                limit: page.limit,
                has_more: page.has_more,
                type_counts: page
                    .type_counts
                    .into_iter()
                    .map(|count| daena_plugin_api::EntityTypeCountRecord {
                        entity_type: count.entity_type,
                        count: count.count,
                    })
                    .collect(),
            };
            serde_json::to_value(record).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.get" => {
            let id = payload_string(&payload, "id")?;
            let entity = project.get_entity(&id)?;
            serde_json::to_value(entity).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.getMany" => {
            let payload: daena_plugin_api::EntityGetManyPayload = serde_json::from_value(payload)
                .map_err(|error| {
                CoreError::Validation(format!("invalid entity.getMany payload: {error}"))
            })?;
            serde_json::to_value(project.get_entities(&payload.ids)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.create" => {
            let fields = payload
                .get("fields")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| CoreError::Validation(format!("invalid entity fields: {error}")))?
                .unwrap_or_default();
            let document = payload
                .get("document")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::Validation(format!("invalid entity document: {error}"))
                })?;
            let relationships = payload
                .get("relationships")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(|error| {
                    CoreError::Validation(format!("invalid entity relationships: {error}"))
                })?
                .unwrap_or_default();
            let input = daena_core::CreateEntry {
                name: payload_string(&payload, "name")?,
                entity_type: payload
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                document,
                fields,
                relationships,
            };
            serde_json::to_value(project.create_entry_with_request(input, request_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.update" => {
            let id = payload_string(&payload, "id")?;
            let name = payload
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let entity_type = payload
                .get("type")
                .and_then(|value| {
                    if value.is_null() {
                        None
                    } else {
                        value.as_str()
                    }
                })
                .map(str::to_owned);
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            serde_json::to_value(project.update_entity_with_options(
                id,
                name,
                entity_type,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "entity.delete" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            project.delete_entity_with_options(
                payload_string(&payload, "id")?,
                Some(&expected_revision),
                request_id,
            )?;
            Ok(serde_json::Value::Null)
        }
        "document.list" => {
            serde_json::to_value(project.list_documents(payload_string(&payload, "entityId")?)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "document.save" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input = daena_core::SaveDocument {
                entity_id: payload_string(&payload, "entityId")?,
                body: payload_string(&payload, "body")?,
                format: payload
                    .get("format")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            project.save_document_with_options(input, Some(&expected_revision), request_id)?;
            Ok(serde_json::Value::Null)
        }
        "field.read" | "field.list" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let namespace = payload_string(&payload, "namespace")?;
            let mut fields = project
                .list_fields(entity_id)?
                .into_iter()
                .filter(|field| field.namespace == namespace)
                .filter(|field| {
                    shared_field_keys
                        .as_ref()
                        .is_none_or(|keys| keys.contains(field.key.as_str()))
                })
                .collect::<Vec<_>>();
            if method == "field.read" {
                let key = payload_string(&payload, "key")?;
                fields.retain(|field| field.key == key);
            }
            serde_json::to_value(fields).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "field.set" => {
            let field = daena_core::FieldValue {
                entity_id: payload_string(&payload, "entityId")?,
                namespace: payload_string(&payload, "namespace")?,
                key: payload_string(&payload, "key")?,
                value: payload
                    .get("value")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
                revision: required_payload_string(
                    &payload,
                    &["expectedRevision", "expected_revision", "revision"],
                    "expectedRevision",
                )?,
            };
            project.set_field_with_request(field, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "record.list" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "access module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                method,
                record_owner_entity_types.as_ref(),
            )?;
            let limit = usize::try_from(
                payload
                    .get("limit")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(50),
            )
            .unwrap_or(usize::MAX);
            let offset = usize::try_from(
                payload
                    .get("offset")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default(),
            )
            .unwrap_or(usize::MAX);
            serde_json::to_value(
                project.list_module_records_with(
                    module_id,
                    &payload_string(&payload, "collection")?,
                    &payload_string(&payload, "ownerEntityId")?,
                    daena_core::ModuleRecordListParams {
                        query: payload.get("query").and_then(serde_json::Value::as_str),
                        limit,
                        offset,
                        sort: payload.get("sort").and_then(serde_json::Value::as_str),
                        status: payload.get("status").and_then(serde_json::Value::as_str),
                        tag: payload.get("tag").and_then(serde_json::Value::as_str),
                        homonyms_only: payload
                            .get("homonymsOnly")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                    },
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "record.create" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "create module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                method,
                record_owner_entity_types.as_ref(),
            )?;
            serde_json::to_value(
                project.create_module_record_with(
                    module_id,
                    &payload_string(&payload, "collection")?,
                    &payload_string(&payload, "ownerEntityId")?,
                    payload
                        .get("value")
                        .cloned()
                        .ok_or_else(|| CoreError::Validation("record value is required".into()))?,
                    request_id,
                    record_owner_entity_types
                        .as_ref()
                        .is_some_and(RecordOwnerConstraint::unique_per_owner),
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "record.update" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "update module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                method,
                record_owner_entity_types.as_ref(),
            )?;
            serde_json::to_value(
                project.update_module_record(
                    module_id,
                    &payload_string(&payload, "collection")?,
                    &payload_string(&payload, "id")?,
                    &payload_string(&payload, "ownerEntityId")?,
                    payload
                        .get("value")
                        .cloned()
                        .ok_or_else(|| CoreError::Validation("record value is required".into()))?,
                    &required_payload_string(
                        &payload,
                        &["expectedRevision", "expected_revision", "revision"],
                        "expectedRevision",
                    )?,
                    request_id,
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "record.delete" => {
            let module_id = plugin_id.ok_or_else(|| CoreError::Unauthorized {
                operation: "delete module records without plugin identity",
            })?;
            validate_record_owner_entity_type(
                project,
                &payload_string(&payload, "ownerEntityId")?,
                method,
                record_owner_entity_types.as_ref(),
            )?;
            project.delete_module_record(
                module_id,
                &payload_string(&payload, "collection")?,
                &payload_string(&payload, "id")?,
                &payload_string(&payload, "ownerEntityId")?,
                &required_payload_string(
                    &payload,
                    &["expectedRevision", "expected_revision", "revision"],
                    "expectedRevision",
                )?,
                request_id,
            )?;
            Ok(serde_json::Value::Null)
        }
        "relationship.list" => {
            serde_json::to_value(project.list_relationships(payload_string(&payload, "entityId")?)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.query" => {
            let payload: daena_plugin_api::RelationshipQueryPayload =
                serde_json::from_value(payload).map_err(|error| {
                    CoreError::Validation(format!("invalid relationship.query payload: {error}"))
                })?;
            let direction = match payload.direction {
                daena_plugin_api::RelationshipQueryDirection::Incoming => {
                    daena_core::RelationshipQueryDirection::Incoming
                }
                daena_plugin_api::RelationshipQueryDirection::Outgoing => {
                    daena_core::RelationshipQueryDirection::Outgoing
                }
                daena_plugin_api::RelationshipQueryDirection::Any => {
                    daena_core::RelationshipQueryDirection::Any
                }
            };
            let page = project.query_relationships(daena_core::RelationshipQuery {
                entity_ids: payload.entity_ids,
                relationship_types: payload.relationship_types,
                direction,
                offset: payload.offset,
                limit: payload.limit,
            })?;
            let record = daena_plugin_api::RelationshipPageRecord {
                items: page
                    .items
                    .into_iter()
                    .map(|relationship| daena_plugin_api::RelationshipRecord {
                        id: relationship.id,
                        source_id: relationship.source_id,
                        target_id: relationship.target_id,
                        relationship_type: relationship.relationship_type,
                        metadata: relationship.metadata,
                        revision: relationship.revision,
                    })
                    .collect(),
                total: page.total,
                offset: page.offset,
                limit: page.limit,
                has_more: page.has_more,
            };
            serde_json::to_value(record).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.create" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input: RelationshipInput = payload_value(&payload)?;
            serde_json::to_value(project.create_relationship_with_options(
                input,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.update" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input = daena_core::RelationshipUpdate {
                id: payload_string(&payload, "id")?,
                metadata: payload
                    .get("metadata")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                target_id: payload
                    .get("target_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            serde_json::to_value(project.update_relationship_with_options(
                input,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "relationship.delete" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            project.delete_relationship_with_options(
                payload_string(&payload, "id")?,
                Some(&expected_revision),
                request_id,
            )?;
            Ok(serde_json::Value::Null)
        }
        "asset.list" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let assets = project.list_assets(entity_id)?;
            serde_json::to_value(assets).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.register" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let input: AssetInput = payload_value(&payload)?;
            serde_json::to_value(project.register_asset_with_options(
                input,
                Some(&expected_revision),
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.update" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let asset_id = payload_string(&payload, "assetId")?;
            let namespace = payload_string(&payload, "namespace")?;
            if project.asset(asset_id.clone())?.namespace != namespace {
                return Err(CoreError::Unauthorized {
                    operation: "update asset outside owned namespace",
                });
            }
            let input = AssetMetadataUpdate {
                asset_id,
                filename: payload
                    .get("filename")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                role: payload
                    .get("role")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                reference_scope: payload
                    .get("referenceScope")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
            };
            serde_json::to_value(project.update_asset_metadata_with_request(
                input,
                &expected_revision,
                request_id,
            )?)
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "asset.delete" => {
            let expected_revision = required_payload_string(
                &payload,
                &["expectedRevision", "expected_revision", "revision"],
                "expectedRevision",
            )?;
            let asset_id = payload_string(&payload, "assetId")?;
            let namespace = payload_string(&payload, "namespace")?;
            if project.asset(asset_id.clone())?.namespace != namespace {
                return Err(CoreError::Unauthorized {
                    operation: "delete asset outside owned namespace",
                });
            }
            project.delete_asset_with_request(asset_id, &expected_revision, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "search.query" => serde_json::to_value(project.search(payload_string(&payload, "query")?)?)
            .map_err(|error| CoreError::Validation(error.to_string())),
        "maps.recovery.list" => {
            let entity_id = payload_string(&payload, "mapEntityId")?;
            serde_json::to_value(project.list_map_recovery_copies(&entity_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.recovery.restore" => {
            let entity_id = payload_string(&payload, "mapEntityId")?;
            let file_name = payload_string(&payload, "fileName")?;
            serde_json::to_value(
                project.restore_map_recovery_copy(&entity_id, &file_name, request_id)?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.locations.list" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            serde_json::to_value(project.map_location_projection(map_entity_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.locations.upsert" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let location = payload
                .get("location")
                .cloned()
                .ok_or_else(|| CoreError::Validation("location is required".into()))?;
            let location: daena_core::maps::LocationReference = serde_json::from_value(location)
                .map_err(|error| CoreError::Validation(format!("invalid location: {error}")))?;
            project.upsert_map_location(entity_id, location, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "maps.layer.create" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            let name = payload_string(&payload, "name")?;
            let expected_revision = payload_string(&payload, "expectedRevision")?;
            let kind = payload
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let provider = project.map_provider_id(&map_entity_id)?;
            let create_vector = match kind.as_deref() {
                Some("vector") => true,
                Some("raster") => false,
                Some(other) => {
                    return Err(CoreError::Validation(format!(
                        "maps: unsupported layer kind {other}"
                    )));
                }
                None => provider == daena_core::maps::VECTOR_PROVIDER,
            };
            if create_vector {
                serde_json::to_value(project.create_vector_layer(
                    map_entity_id,
                    name,
                    &expected_revision,
                    request_id,
                    None,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            } else {
                serde_json::to_value(project.create_raster_layer(
                    map_entity_id,
                    name,
                    &expected_revision,
                    request_id,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            }
        }
        "maps.layer.delete" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            let layer_id = payload_string(&payload, "layerId")?;
            let expected_revision = payload_string(&payload, "expectedRevision")?;
            let kind = project
                .map_layer_kind(&map_entity_id, &layer_id)?
                .ok_or_else(|| CoreError::NotFound("layer not found".into()))?;
            if kind == "vector" {
                let expected_source_revision = payload_string(&payload, "expectedSourceRevision")?;
                let expected_feature_count = payload
                    .get("expectedFeatureCount")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        CoreError::Validation("expectedFeatureCount is required".into())
                    })?;
                serde_json::to_value(project.delete_vector_layer(
                    map_entity_id,
                    layer_id,
                    &expected_revision,
                    &expected_source_revision,
                    expected_feature_count,
                    request_id,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            } else {
                serde_json::to_value(project.delete_raster_layer(
                    map_entity_id,
                    layer_id,
                    &expected_revision,
                    request_id,
                )?)
                .map_err(|error| CoreError::Validation(error.to_string()))
            }
        }
        "maps.layer.update" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            let layer_id = payload_string(&payload, "layerId")?;
            let expected_revision = payload_string(&payload, "expectedRevision")?;
            serde_json::to_value(
                project.update_map_layer(
                    map_entity_id,
                    layer_id,
                    daena_core::RasterLayerUpdate {
                        name: payload
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .map(str::to_owned),
                        order: payload.get("order").and_then(serde_json::Value::as_i64),
                        default_visible: payload
                            .get("defaultVisible")
                            .and_then(serde_json::Value::as_bool),
                        opacity: payload.get("opacity").and_then(serde_json::Value::as_f64),
                        locked: payload.get("locked").and_then(serde_json::Value::as_bool),
                        style: payload.get("style").cloned(),
                        selector: payload.get("selector").cloned(),
                    },
                    &expected_revision,
                    request_id,
                )?,
            )
            .map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.locations.unlink" => {
            let entity_id = payload_string(&payload, "entityId")?;
            let location_id = payload_string(&payload, "locationId")?;
            project.unlink_map_location(entity_id, location_id, request_id)?;
            Ok(serde_json::Value::Null)
        }
        "maps.locations.create_and_link" => {
            let name = payload_string(&payload, "name")?;
            let entity_type = payload_string(&payload, "entityType")?;
            if entity_type == daena_core::maps::MAP_ENTITY_TYPE {
                return Err(CoreError::Validation(
                    "create_and_link cannot create map entities".into(),
                ));
            }
            let location = payload
                .get("location")
                .cloned()
                .ok_or_else(|| CoreError::Validation("location is required".into()))?;
            let mut location: daena_core::maps::LocationReference =
                serde_json::from_value(location)
                    .map_err(|error| CoreError::Validation(format!("invalid location: {error}")))?;
            let created = project.create_entry_with_request(
                daena_core::CreateEntry {
                    name,
                    entity_type: Some(entity_type),
                    document: None,
                    fields: Vec::new(),
                    relationships: Vec::new(),
                },
                request_id,
            )?;
            location.label.clone_from(&created.name);
            project.upsert_map_location(created.id.clone(), location, None)?;
            serde_json::to_value(created).map_err(|error| CoreError::Validation(error.to_string()))
        }
        "maps.reconcile.links" => {
            let map_entity_id = payload_string(&payload, "mapEntityId")?;
            serde_json::to_value(project.reconcile_map_links(map_entity_id)?)
                .map_err(|error| CoreError::Validation(error.to_string()))
        }
        _ => Err(CoreError::Validation(format!(
            "unknown plugin RPC method: {method}"
        ))),
    }
}

/// Main-window workspace editing is a trusted shell operation. Plugin identity
/// is required and authorized through the host; third-party webviews use the
/// session-bound `plugin_rpc` surface above.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(super) async fn trusted_module_rpc(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    plugins: tauri::State<'_, SharedPluginHost>,
    settings: tauri::State<'_, SharedSettings>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    plugin_id: String,
    method: String,
    payload: serde_json::Value,
    request_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let project_id = current_info(state.inner())?
        .map(|info| info.root)
        .ok_or_else(|| "project is not open".to_string())?;
    let event_method = method.clone();
    let granted_capabilities = {
        let mut host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        host.authorize_bundled(&plugin_id, &project_id, &method, payload.clone())
            .map_err(|error| error.to_string())?;
        host.ensure_bundled_session(&plugin_id, &project_id)
            .map_err(|error| error.to_string())?
            .grants
    };
    if method.starts_with("ai.request.") {
        return dispatch_host_rpc(
            plugins.inner(),
            &plugin_id,
            &project_id,
            &method,
            payload,
            AiBrokerContext {
                app: Some(app),
                core: Some(state.inner().clone()),
                settings: Some(settings.inner().clone()),
                ai_runtime: ai_runtime.inner().clone(),
                session_id: format!("bundled:{plugin_id}"),
                caller: daena_ai::AiCaller::authorized_plugin(
                    plugin_id.clone(),
                    project_id.clone(),
                    granted_capabilities.into_iter().collect(),
                    vec![format!("project:{project_id}")],
                    0,
                    "pending",
                ),
            },
        );
    }
    let record_owner_declaration = if method.starts_with("record.") {
        let collection = payload
            .get("collection")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "record collection is required".to_string())?;
        let host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        host.record_owner_declaration(&project_id, &plugin_id, collection)
    } else {
        None
    };
    let shared_field_keys = {
        let host = plugins
            .lock()
            .map_err(|_| "plugin host lock poisoned".to_string())?;
        shared_field_keys_for_request(&host, &plugin_id, &method, &payload)?
    };
    let rpc_plugin_id = plugin_id.clone();
    let result = with_core(state, move |core| {
        let record_owner_entity_types = match record_owner_declaration.as_ref() {
            Some((collection, package)) => Some(record_owner_constraint_from_declaration(
                core.project(AuthorityContext::plugin())?,
                package,
                collection,
            )?),
            None => None,
        };
        dispatch_module_rpc(
            core,
            Some(rpc_plugin_id.as_str()),
            shared_field_keys,
            record_owner_entity_types,
            &method,
            payload,
            request_id.as_deref(),
        )
    })
    .await?;
    publish_core_mutation_event(plugins.inner(), &project_id, &event_method, &result)?;
    Ok(result)
}

pub(super) fn publish_core_mutation_event(
    plugins: &SharedPluginHost,
    project_id: &str,
    method: &str,
    result: &serde_json::Value,
) -> Result<(), String> {
    if !matches!(
        method,
        "entity.create"
            | "entity.update"
            | "entity.delete"
            | "document.save"
            | "field.set"
            | "relationship.create"
            | "relationship.update"
            | "relationship.delete"
            | "asset.register"
            | "asset.update"
            | "asset.delete"
    ) {
        return Ok(());
    }
    plugins
        .lock()
        .map_err(|_| "plugin host lock poisoned".to_string())?
        .publish_core_event(
            project_id,
            "daena.core/entity-changed",
            1,
            serde_json::json!({"method": method, "result": result}),
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub(super) async fn project_open(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let core = state.inner().clone();
    let request_runtime = ai_runtime.clone();
    let sync_plugins = plugins.clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            cancel_ai_requests_for(&plugins, &request_runtime, &previous_project, None)
                .map_err(CoreError::Conflict)?;
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        core.open_without_flush(trusted_shell(), path)?;
        Ok(())
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
pub(super) async fn project_open_directory(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<ProjectInfo, String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let core = state.inner().clone();
    let request_runtime = ai_runtime.clone();
    let sync_plugins = plugins.clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            cancel_ai_requests_for(&plugins, &request_runtime, &previous_project, None)
                .map_err(CoreError::Conflict)?;
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        let info = core.open_directory_without_flush(trusted_shell(), path)?;
        Ok(info)
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
pub(super) async fn project_new(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    path: String,
) -> Result<ProjectInfo, String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    flush_project_checkpoint(state.clone(), "project lifecycle transition").await?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let core = state.inner().clone();
    let request_runtime = ai_runtime.clone();
    let sync_plugins = plugins.clone();
    let result = with_core(state, move |core| {
        if let Some(previous_project) = core.info().map(|info| info.root) {
            cancel_ai_requests_for(&plugins, &request_runtime, &previous_project, None)
                .map_err(CoreError::Conflict)?;
            plugins
                .lock()
                .map_err(|_| CoreError::Conflict("plugin host lock poisoned".into()))?
                .deactivate_project(&previous_project);
        }
        let info = core.open_directory_without_flush(trusted_shell(), path)?;
        Ok(info)
    })
    .await;
    if result.is_ok() {
        begin_current_physical_session(jobs.inner(), &core)?;
        sync_project_usage_and_wait(core.clone(), sync_plugins).await?;
        flush_project_checkpoint_for_shared_core(core.clone(), "project startup synchronization")
            .await?;
        schedule_ai_index_refresh(&core, &ai_runtime);
        start_project_watcher(&app, &core, watcher.inner())?;
    }
    result
}

#[tauri::command]
pub(super) async fn project_close(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedCore>,
    jobs: tauri::State<'_, SharedPhysicalJobs>,
    plugins: tauri::State<'_, SharedPluginHost>,
    ai_runtime: tauri::State<'_, ai::SharedAiRuntime>,
    watcher: tauri::State<'_, SharedProjectWatcher>,
    image_jobs: tauri::State<'_, image_generation::SharedImageGeneration>,
) -> Result<(), String> {
    cancel_physical_jobs(jobs.inner())?;
    cancel_external_import_jobs()?;
    let plugins = plugins.inner().clone();
    let ai_runtime = ai_runtime.inner().clone();
    let app = app.clone();
    let core = state.inner().clone();
    let jobs = jobs.inner().clone();
    let watcher = watcher.inner().clone();
    let image_jobs = image_jobs.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        close_project_for_app(
            &app,
            &core,
            &jobs,
            &plugins,
            &ai_runtime,
            &watcher,
            &image_jobs,
        )
    })
    .await
    .map_err(|error| format!("project close worker failed: {error}"))?
}
