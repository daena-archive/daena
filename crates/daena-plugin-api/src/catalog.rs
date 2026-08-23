//! Data-driven RPC method catalog — the single source for method names, payload
//! types, revision requirements, and the capabilities each executable method
//! exercises. Consumed by the plugin host (`required_capabilities`), the
//! `gen-contract` binary, and the schema/TS generators.
//!
//! The catalog replaces the hand-written `required_capabilities` match in
//! `daena-plugin-host` and the ad-hoc payload checks it embedded. Capability
//! rules that depend on the request payload (owned vs shared namespaces, event
//! types, service identities) live in `RpcCapability::resolve`; the host only
//! supplies the authorization context.

use crate::RpcError;
use serde_json::Value;

/// Minimal namespace-ownership view the authorization context needs. The host
/// implements this for its `NamespaceOwnership`.
pub trait NamespaceView {
    fn owner(&self, namespace: &str) -> Option<&str>;
    fn field_is_shared(&self, namespace: &str, key: &str) -> bool;
    fn namespace_has_shared_fields(&self, namespace: &str) -> bool;
}

/// Everything `RpcCapability::resolve` needs beyond the raw payload.
pub struct RpcAuthorizationContext<'a> {
    pub plugin_id: &'a str,
    pub namespaces: &'a dyn NamespaceView,
}

fn deny(code: &str, message: &str) -> RpcError {
    RpcError {
        code: code.into(),
        message: message.into(),
        retryable: false,
        details: None,
    }
}

/// How a catalog method maps to the capabilities it requires.
pub enum RpcCapability {
    /// Fixed capability list, independent of the payload.
    Static(&'static [&'static str]),
    /// `entity.create` — `entity.write` plus conditional `document.write` and
    /// `field.write:self` / `relationship.write` derived from the payload.
    EntityCreate,
    /// `field.read` / `field.list` — `field.read:self` when the namespace is
    /// owned, otherwise `field.read:shared` for explicitly shared fields.
    FieldRead,
    /// `field.set` — `field.write:self` only; foreign namespaces are denied.
    FieldWrite,
    /// `event.publish` — `event.publish:<type>`.
    EventPublish,
    /// `event.subscribe` / `event.poll` — `event.subscribe:<type>`.
    EventSubscribe,
    /// `service.call` — `service.call:<name>@<major>`.
    ServiceCall,
    /// Asset methods that first require ownership of the payload namespace.
    OwnedNamespace(&'static [&'static str]),
    /// AI lifecycle methods retain the operation grant without exposing the
    /// provider or caller identity to the plugin.
    AiRequest,
    /// Lifecycle methods accept either text AI grant; the host still binds the
    /// request ID to the authorized session before dispatch.
    AnyStatic(&'static [&'static str]),
}

impl RpcCapability {
    pub fn resolve(
        &self,
        payload: &Value,
        context: &RpcAuthorizationContext,
    ) -> Result<Vec<String>, RpcError> {
        match self {
            RpcCapability::Static(capabilities) => Ok(capabilities
                .iter()
                .map(|capability| capability.to_string())
                .collect()),
            RpcCapability::AnyStatic(capabilities) => Ok(vec![capabilities.join("|")]),
            RpcCapability::AiRequest => match payload.get("operation").and_then(Value::as_str) {
                Some("generate_text") => Ok(vec!["ai.text.generate".into()]),
                Some("generate_structured") => Ok(vec!["ai.text.generate-structured".into()]),
                _ => Err(deny("payload.invalid", "unsupported AI operation")),
            },
            RpcCapability::EntityCreate => {
                let mut capabilities = vec!["entity.write".into()];
                if payload.get("document").is_some() {
                    capabilities.push("document.write".into());
                }
                if let Some(fields) = payload.get("fields") {
                    let fields = fields
                        .as_array()
                        .ok_or_else(|| deny("payload.invalid", "entity fields must be an array"))?;
                    for field in fields {
                        let namespace =
                            field
                                .get("namespace")
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    deny("payload.invalid", "entity fields require namespace")
                                })?;
                        if context.namespaces.owner(namespace) != Some(context.plugin_id) {
                            return Err(deny("namespace.denied", "plugin does not own namespace"));
                        }
                    }
                    if !fields.is_empty() {
                        capabilities.push("field.write:self".into());
                    }
                }
                if let Some(relationships) = payload.get("relationships") {
                    let relationships = relationships.as_array().ok_or_else(|| {
                        deny("payload.invalid", "entity relationships must be an array")
                    })?;
                    if !relationships.is_empty() {
                        capabilities.push("relationship.write".into());
                    }
                }
                Ok(capabilities)
            }
            RpcCapability::FieldRead => {
                let namespace = payload
                    .get("namespace")
                    .and_then(Value::as_str)
                    .ok_or_else(|| deny("payload.invalid", "operation requires namespace"))?;
                if context.namespaces.owner(namespace) == Some(context.plugin_id) {
                    return Ok(vec!["field.read:self".into()]);
                }
                if let Some(key) = payload.get("key").and_then(Value::as_str) {
                    if !context.namespaces.field_is_shared(namespace, key) {
                        return Err(deny("namespace.denied", "field is not explicitly shared"));
                    }
                } else if !context.namespaces.namespace_has_shared_fields(namespace) {
                    return Err(deny(
                        "namespace.denied",
                        "namespace has no explicitly shared fields",
                    ));
                }
                Ok(vec!["field.read:shared".into()])
            }
            RpcCapability::FieldWrite => {
                let namespace = payload
                    .get("namespace")
                    .and_then(Value::as_str)
                    .ok_or_else(|| deny("payload.invalid", "operation requires namespace"))?;
                if context.namespaces.owner(namespace) != Some(context.plugin_id) {
                    return Err(deny(
                        "namespace.denied",
                        "plugin may only read explicitly shared fields",
                    ));
                }
                Ok(vec!["field.write:self".into()])
            }
            RpcCapability::EventPublish | RpcCapability::EventSubscribe => {
                let event_type = payload
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| deny("payload.invalid", "event operations require type"))?;
                let action = match self {
                    RpcCapability::EventPublish => "publish",
                    _ => "subscribe",
                };
                Ok(vec![format!("event.{action}:{event_type}")])
            }
            RpcCapability::ServiceCall => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| deny("payload.invalid", "service operations require name"))?;
                let major = payload
                    .get("major")
                    .and_then(Value::as_u64)
                    .and_then(|major| u32::try_from(major).ok())
                    .ok_or_else(|| deny("payload.invalid", "service operations require major"))?;
                Ok(vec![format!("service.call:{name}@{major}")])
            }
            RpcCapability::OwnedNamespace(capabilities) => {
                let namespace = payload
                    .get("namespace")
                    .and_then(Value::as_str)
                    .ok_or_else(|| deny("payload.invalid", "operation requires namespace"))?;
                if context.namespaces.owner(namespace) != Some(context.plugin_id) {
                    return Err(deny("namespace.denied", "plugin does not own namespace"));
                }
                Ok(capabilities
                    .iter()
                    .map(|capability| capability.to_string())
                    .collect())
            }
        }
    }
}

/// A single executable RPC method.
pub struct RpcMethodDef {
    pub name: &'static str,
    /// Schemars schema name for the payload type (the `$defs` key).
    pub payload_schema: &'static str,
    /// Whether the payload's required keys include `expectedRevision`.
    pub requires_revision: bool,
    pub capability: RpcCapability,
}

/// The canonical executable RPC methods. Capability-alias names
/// (`entity.read`, `entity.write`, `document.read`, `document.write`,
/// `relationship.read`, `relationship.write`, `asset.read`, `field.write`,
/// `service.provide`) are intentionally absent — they are grouping keys only
/// (see `docs/PLUGIN_PLATFORM_PLAN.md`, "Contract reconciliation and
/// generation record", § "Resolved contract decisions").
pub const RPC_METHOD_CATALOG: &[RpcMethodDef] = &[
    RpcMethodDef {
        name: "entity.list",
        payload_schema: "EntityListPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["entity.read"]),
    },
    RpcMethodDef {
        name: "entity.query",
        payload_schema: "EntityQueryPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["entity.read"]),
    },
    RpcMethodDef {
        name: "entity.get",
        payload_schema: "EntityGetPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["entity.read"]),
    },
    RpcMethodDef {
        name: "entity.create",
        payload_schema: "EntityCreatePayload",
        requires_revision: false,
        capability: RpcCapability::EntityCreate,
    },
    RpcMethodDef {
        name: "entity.update",
        payload_schema: "EntityUpdatePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["entity.write"]),
    },
    RpcMethodDef {
        name: "entity.delete",
        payload_schema: "EntityDeletePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["entity.delete"]),
    },
    RpcMethodDef {
        name: "document.list",
        payload_schema: "DocumentListPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["document.read"]),
    },
    RpcMethodDef {
        name: "document.save",
        payload_schema: "DocumentSavePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["document.write"]),
    },
    RpcMethodDef {
        name: "field.read",
        payload_schema: "FieldReadPayload",
        requires_revision: false,
        capability: RpcCapability::FieldRead,
    },
    RpcMethodDef {
        name: "field.list",
        payload_schema: "FieldListPayload",
        requires_revision: false,
        capability: RpcCapability::FieldRead,
    },
    RpcMethodDef {
        name: "field.set",
        payload_schema: "FieldSetPayload",
        requires_revision: true,
        capability: RpcCapability::FieldWrite,
    },
    RpcMethodDef {
        name: "record.list",
        payload_schema: "RecordListPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["record.read:self"]),
    },
    RpcMethodDef {
        name: "record.create",
        payload_schema: "RecordCreatePayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["record.write:self"]),
    },
    RpcMethodDef {
        name: "record.update",
        payload_schema: "RecordUpdatePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["record.write:self"]),
    },
    RpcMethodDef {
        name: "record.delete",
        payload_schema: "RecordDeletePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["record.write:self"]),
    },
    RpcMethodDef {
        name: "relationship.list",
        payload_schema: "RelationshipListPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["relationship.read"]),
    },
    RpcMethodDef {
        name: "relationship.create",
        payload_schema: "RelationshipCreatePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["relationship.write"]),
    },
    RpcMethodDef {
        name: "relationship.update",
        payload_schema: "RelationshipUpdatePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["relationship.write"]),
    },
    RpcMethodDef {
        name: "relationship.delete",
        payload_schema: "RelationshipDeletePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["relationship.write"]),
    },
    RpcMethodDef {
        name: "asset.list",
        payload_schema: "AssetListPayload",
        requires_revision: false,
        capability: RpcCapability::OwnedNamespace(&["asset.read:self"]),
    },
    RpcMethodDef {
        name: "asset.register",
        payload_schema: "AssetRegisterPayload",
        requires_revision: true,
        capability: RpcCapability::OwnedNamespace(&["asset.register"]),
    },
    RpcMethodDef {
        name: "asset.update",
        payload_schema: "AssetMetadataUpdatePayload",
        requires_revision: true,
        capability: RpcCapability::OwnedNamespace(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "asset.delete",
        payload_schema: "AssetDeletePayload",
        requires_revision: true,
        capability: RpcCapability::OwnedNamespace(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "asset.read.begin",
        payload_schema: "AssetReadBeginPayload",
        requires_revision: false,
        capability: RpcCapability::OwnedNamespace(&["asset.read:self"]),
    },
    RpcMethodDef {
        name: "asset.replace.begin",
        payload_schema: "AssetReplaceBeginPayload",
        requires_revision: true,
        capability: RpcCapability::OwnedNamespace(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "asset.replace.commit",
        payload_schema: "AssetReplaceCommitPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "asset.transfer.cancel",
        payload_schema: "AssetTransferCancelPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&[]),
    },
    RpcMethodDef {
        name: "search.query",
        payload_schema: "SearchQueryPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["search.query"]),
    },
    RpcMethodDef {
        name: "maps.asset.create.begin",
        payload_schema: "MapsAssetCreateBeginPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "maps.asset.create.commit",
        payload_schema: "MapsAssetCreateCommitPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "maps.image.import.begin",
        payload_schema: "MapsImageImportBeginPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&[
            "entity.write",
            "asset.write:self",
            "field.write:self",
        ]),
    },
    RpcMethodDef {
        name: "maps.image.import.commit",
        payload_schema: "MapsImageImportCommitPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&[
            "entity.write",
            "asset.write:self",
            "field.write:self",
        ]),
    },
    RpcMethodDef {
        name: "maps.vector.create.begin",
        payload_schema: "MapsVectorCreateBeginPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&[
            "entity.write",
            "asset.write:self",
            "field.write:self",
        ]),
    },
    RpcMethodDef {
        name: "maps.vector.create.commit",
        payload_schema: "MapsVectorCreateCommitPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&[
            "entity.write",
            "asset.write:self",
            "field.write:self",
        ]),
    },
    RpcMethodDef {
        name: "maps.physical.create.begin",
        payload_schema: "MapsPhysicalCreateBeginPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&[
            "entity.write",
            "asset.write:self",
            "field.write:self",
        ]),
    },
    RpcMethodDef {
        name: "maps.physical.create.commit",
        payload_schema: "MapsPhysicalCreateCommitPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&[
            "entity.write",
            "asset.write:self",
            "field.write:self",
        ]),
    },
    RpcMethodDef {
        name: "maps.vector.replace.begin",
        payload_schema: "MapsVectorReplaceBeginPayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "maps.vector.replace.commit",
        payload_schema: "MapsVectorReplaceCommitPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "maps.layer.create",
        payload_schema: "MapsLayerCreatePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["asset.write:self", "field.write:self"]),
    },
    RpcMethodDef {
        name: "maps.layer.delete",
        payload_schema: "MapsLayerDeletePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["asset.write:self", "field.write:self"]),
    },
    RpcMethodDef {
        name: "maps.layer.update",
        payload_schema: "MapsLayerUpdatePayload",
        requires_revision: true,
        capability: RpcCapability::Static(&["field.write:self"]),
    },
    RpcMethodDef {
        name: "maps.recovery.export.begin",
        payload_schema: "MapsRecoveryExportBeginPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "maps.recovery.export.commit",
        payload_schema: "MapsRecoveryExportCommitPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "maps.recovery.restore",
        payload_schema: "MapsRecoveryRestorePayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.write:self"]),
    },
    RpcMethodDef {
        name: "maps.recovery.list",
        payload_schema: "MapsRecoveryListPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.read:self"]),
    },
    RpcMethodDef {
        name: "maps.locations.list",
        payload_schema: "MapsLocationsListPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.read:self"]),
    },
    RpcMethodDef {
        name: "maps.locations.upsert",
        payload_schema: "MapsLocationsUpsertPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["field.write:self"]),
    },
    RpcMethodDef {
        name: "maps.locations.unlink",
        payload_schema: "MapsLocationsUnlinkPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["field.write:self"]),
    },
    RpcMethodDef {
        name: "maps.locations.create_and_link",
        payload_schema: "MapsLocationsCreateAndLinkPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["entity.write", "field.write:self"]),
    },
    RpcMethodDef {
        name: "maps.reconcile.links",
        payload_schema: "MapsReconcileLinksPayload",
        requires_revision: false,
        capability: RpcCapability::Static(&["asset.read:self"]),
    },
    RpcMethodDef {
        name: "event.publish",
        payload_schema: "EventPublishPayload",
        requires_revision: false,
        capability: RpcCapability::EventPublish,
    },
    RpcMethodDef {
        name: "event.subscribe",
        payload_schema: "EventTypePayload",
        requires_revision: false,
        capability: RpcCapability::EventSubscribe,
    },
    RpcMethodDef {
        name: "event.poll",
        payload_schema: "EventTypePayload",
        requires_revision: false,
        capability: RpcCapability::EventSubscribe,
    },
    RpcMethodDef {
        name: "service.call",
        payload_schema: "ServiceCallPayload",
        requires_revision: false,
        capability: RpcCapability::ServiceCall,
    },
    RpcMethodDef {
        name: "ai.request.start",
        payload_schema: "AiRequestStartPayload",
        requires_revision: false,
        capability: RpcCapability::AiRequest,
    },
    RpcMethodDef {
        name: "ai.request.poll",
        payload_schema: "AiRequestIdPayload",
        requires_revision: false,
        capability: RpcCapability::AnyStatic(&["ai.text.generate", "ai.text.generate-structured"]),
    },
    RpcMethodDef {
        name: "ai.request.cancel",
        payload_schema: "AiRequestIdPayload",
        requires_revision: false,
        capability: RpcCapability::AnyStatic(&["ai.text.generate", "ai.text.generate-structured"]),
    },
    RpcMethodDef {
        name: "ai.request.result",
        payload_schema: "AiRequestIdPayload",
        requires_revision: false,
        capability: RpcCapability::AnyStatic(&["ai.text.generate", "ai.text.generate-structured"]),
    },
    RpcMethodDef {
        name: "ai.request.citations",
        payload_schema: "AiRequestIdPayload",
        requires_revision: false,
        capability: RpcCapability::AnyStatic(&["ai.text.generate", "ai.text.generate-structured"]),
    },
];

pub fn rpc_method(name: &str) -> Option<&'static RpcMethodDef> {
    RPC_METHOD_CATALOG.iter().find(|entry| entry.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_unique_entries_and_known_revision_methods() {
        let mut names = std::collections::BTreeSet::new();
        for entry in RPC_METHOD_CATALOG {
            assert!(names.insert(entry.name), "duplicate method {}", entry.name);
            assert!(!entry.payload_schema.is_empty());
            assert!(!entry.name.is_empty());
        }
        assert_eq!(RPC_METHOD_CATALOG.len(), 59);
        let revision_methods = RPC_METHOD_CATALOG
            .iter()
            .filter(|entry| entry.requires_revision)
            .map(|entry| entry.name)
            .collect::<Vec<_>>();
        assert_eq!(
            revision_methods,
            vec![
                "entity.update",
                "entity.delete",
                "document.save",
                "field.set",
                "record.update",
                "record.delete",
                "relationship.create",
                "relationship.update",
                "relationship.delete",
                "asset.register",
                "asset.update",
                "asset.delete",
                "asset.replace.begin",
                "maps.vector.replace.begin",
                "maps.layer.create",
                "maps.layer.delete",
                "maps.layer.update",
            ]
        );
    }

    #[test]
    fn capability_aliases_are_excluded_from_the_catalog() {
        for alias in [
            "entity.read",
            "entity.write",
            "document.read",
            "document.write",
            "relationship.read",
            "relationship.write",
            "asset.read",
            "field.write",
            "service.provide",
        ] {
            assert!(
                rpc_method(alias).is_none(),
                "{alias} is a capability alias, not an executable method"
            );
        }
    }

    struct NullNamespace;
    impl NamespaceView for NullNamespace {
        fn owner(&self, _namespace: &str) -> Option<&str> {
            None
        }
        fn field_is_shared(&self, _namespace: &str, _key: &str) -> bool {
            false
        }
        fn namespace_has_shared_fields(&self, _namespace: &str) -> bool {
            false
        }
    }

    #[test]
    fn static_capabilities_are_returned_unmodified() {
        let payload = serde_json::json!({});
        let context = RpcAuthorizationContext {
            plugin_id: "com.example.one",
            namespaces: &NullNamespace,
        };
        assert_eq!(
            RpcCapability::Static(&["entity.read"])
                .resolve(&payload, &context)
                .unwrap(),
            vec!["entity.read".to_string()]
        );
        assert!(RpcCapability::Static(&[])
            .resolve(&payload, &context)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn event_and_service_capabilities_are_parameterized_by_payload() {
        let context = RpcAuthorizationContext {
            plugin_id: "com.example.one",
            namespaces: &NullNamespace,
        };
        assert_eq!(
            RpcCapability::EventPublish
                .resolve(&serde_json::json!({"type": "daena.core/event@1"}), &context)
                .unwrap(),
            vec!["event.publish:daena.core/event@1".to_string()]
        );
        assert_eq!(
            RpcCapability::EventSubscribe
                .resolve(&serde_json::json!({"type": "daena.core/event@1"}), &context)
                .unwrap(),
            vec!["event.subscribe:daena.core/event@1".to_string()]
        );
        assert_eq!(
            RpcCapability::ServiceCall
                .resolve(
                    &serde_json::json!({"name": "com.example.calculate", "major": 1}),
                    &context,
                )
                .unwrap(),
            vec!["service.call:com.example.calculate@1".to_string()]
        );
    }
}
