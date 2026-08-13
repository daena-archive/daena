//! `gen-contract` — regenerate the four public contract schemas from the
//! canonical Rust types in this crate (built with `--features gen`).
//!
//! Emits, into `schemas/` at the repository root:
//!   - `plugin-manifest-v1.json`     (from `PluginManifest` + curated `$defs`)
//!   - `plugin-rpc-v1.json`          (request/response envelopes + method payload `$defs`)
//!   - `plugin-error-v1.json`        (from `RpcError`)
//!   - `capability-registry-v1.json` (from `CAPABILITY_REGISTRY`)
//!
//! schemars 0.8 emits draft-07-flavoured output using `definitions`; we use
//! draft-07 settings with a 2020-12 meta-schema and a `#/$defs/` definitions
//! path, so the emitted `$ref`s already point at `#/$defs/<Name>`.

use daena_plugin_api::rpc::{
    AiRequestIdPayload, AiRequestStartPayload, AssetListPayload, AssetReadBeginPayload,
    AssetRegisterPayload, AssetReplaceBeginPayload, AssetReplaceCommitPayload,
    AssetTransferCancelPayload, DocumentListPayload, DocumentSavePayload, EntityCreateDocument,
    EntityCreateField, EntityCreatePayload, EntityCreateRelationship, EntityDeletePayload,
    EntityGetPayload, EntityListPayload, EntityRecord, EntityUpdatePayload, EventPublishPayload,
    EventTypePayload, FieldListPayload, FieldReadPayload, FieldSetPayload, RecordCreatePayload,
    RecordDeletePayload, RecordListPayload, RecordUpdatePayload,
    MapsAssetCreateBeginPayload, MapsAssetCreateCommitPayload, MapsLocationsCreateAndLinkPayload,
    MapsLocationsListPayload, MapsLocationsUnlinkPayload, MapsLocationsUpsertPayload,
    MapsReconcileLinksPayload, MapsRecoveryExportBeginPayload, MapsRecoveryExportCommitPayload,
    MapsRecoveryListPayload, MapsRecoveryRestorePayload, PluginBootstrap,
    RelationshipCreatePayload, RelationshipDeletePayload, RelationshipListPayload,
    SearchQueryPayload, ServiceCallPayload,
};
use daena_plugin_api::{
    PluginManifest, RpcError, CAPABILITY_REGISTRY, DENIED_BY_DEFAULT_CAPABILITIES,
    RPC_METHOD_CATALOG,
};
use schemars::gen::{SchemaGenerator, SchemaSettings};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

const META: &str = "https://json-schema.org/draft/2020-12/schema";
const MANIFEST_ID: &str = "https://github.com/daena-archive/daena/schemas/plugin-manifest-v1.json";
const RPC_ID: &str = "https://github.com/daena-archive/daena/schemas/plugin-rpc-v1.json";
const ERROR_ID: &str = "https://github.com/daena-archive/daena/schemas/plugin-error-v1.json";
const CAPABILITY_ID: &str =
    "https://github.com/daena-archive/daena/schemas/capability-registry-v1.json";

fn settings() -> SchemaSettings {
    SchemaSettings::draft07().with(|s| {
        s.definitions_path = "#/$defs/".to_owned();
        s.meta_schema = Some(META.to_owned());
    })
}

fn ref_to(name: &str) -> Value {
    json!({"$ref": format!("#/$defs/{name}")})
}

/// Returns the mutable `$defs.<name>` schema object.
fn defs_entry<'a>(root: &'a mut Value, name: &str) -> &'a mut Map<String, Value> {
    root.get_mut("$defs")
        .and_then(|d| d.as_object_mut())
        .expect("$defs")
        .get_mut(name)
        .and_then(|d| d.as_object_mut())
        .expect(name)
}

/// Sets `key: value` on `properties[prop]` inside `$defs.<def>`.
fn rule_on_prop(root: &mut Value, def: &str, prop: &str, key: &str, value: u64) {
    let props = defs_entry(root, def)
        .get_mut("properties")
        .and_then(|p| p.as_object_mut())
        .expect("properties");
    let target = props
        .get_mut(prop)
        .and_then(|p| p.as_object_mut())
        .expect(prop);
    target.insert(key.to_owned(), json!(value));
}

/// Replaces `properties[prop]` inside `$defs.<def>`.
fn set_prop(root: &mut Value, def: &str, prop: &str, schema: Value) {
    let props = defs_entry(root, def)
        .get_mut("properties")
        .and_then(|p| p.as_object_mut())
        .expect("properties");
    props.insert(prop.to_owned(), schema);
}

fn add_curated_defs(root: &mut Value) {
    let curated = json!({
        "identifier": {
            "type": "string",
            "pattern": r"^[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*)*$"
        },
        "serviceName": {
            "type": "string",
            "pattern": r"^[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*)*(?:\/[a-z0-9][a-z0-9_-]*)?$"
        },
        "semver": {
            "type": "string",
            "pattern": r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$"
        },
        "packagePath": {
            "type": "string",
            "pattern": r"^(?!/)(?!.*(?:^|/)\.\.?(?:/|$))(?!.*\\)[^\u0000]+$"
        },
        "namespace": {
            "type": "string",
            "pattern": r"^[a-z][a-z0-9]*(?:[._-][a-z0-9]+)*$",
            "maxLength": 64
        }
    });
    let defs = root
        .get_mut("$defs")
        .and_then(|d| d.as_object_mut())
        .expect("$defs");
    for (name, schema) in curated.as_object().expect("curated object") {
        defs.insert(name.clone(), schema.clone());
    }
}

// ---------------------------------------------------------------------------
// plugin-manifest-v1.json
// ---------------------------------------------------------------------------

fn manifest_schema() -> Value {
    let mut gen = SchemaGenerator::new(settings());
    let _ = gen.subschema_for::<PluginManifest>();
    let defs = gen.take_definitions();
    let mut root = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["manifestVersion", "id", "name", "version", "publisher", "hostApi", "kind", "entrypoints", "capabilities", "dependencies", "namespaces", "schemas", "templates", "views", "commands", "services", "events", "migrations"],
        "properties": {
            "manifestVersion": {"const": 1},
            "id": ref_to("identifier"),
            "name": {"type": "string", "minLength": 1, "maxLength": 128},
            "version": ref_to("semver"),
            "publisher": ref_to("identifier"),
            "enabledByDefault": {"type": "boolean"},
            "stability": ref_to("PluginStability"),
            "hostApi": {"type": "string", "minLength": 1, "maxLength": 128},
            "kind": ref_to("PluginKind"),
            "entrypoints": ref_to("Entrypoints"),
            "capabilities": {"type": "array", "items": {"type": "string", "minLength": 1}, "uniqueItems": true},
            "dependencies": {"type": "object", "additionalProperties": ref_to("Dependency")},
            "namespaces": {"type": "array", "items": ref_to("namespace"), "uniqueItems": true},
            "schemas": {"type": "array", "items": ref_to("SchemaContribution")},
            "templates": {"type": "array", "items": ref_to("EntityTemplate")},
            "views": {"type": "array", "items": ref_to("View")},
            "commands": {"type": "array", "items": ref_to("Command")},
            "services": ref_to("Services"),
            "events": ref_to("Events"),
            "migrations": {"type": "array", "items": ref_to("Migration")}
        },
        "$schema": META,
        "$id": MANIFEST_ID,
        "title": "Daena Archive Plugin Manifest v1",
        "$defs": serde_json::to_value(&defs).expect("defs serialize")
    });

    add_curated_defs(&mut root);

    // Entrypoints: at least one package path.
    let entrypoints = defs_entry(&mut root, "Entrypoints");
    entrypoints.insert("minProperties".to_owned(), json!(1));
    let ep_props = entrypoints
        .get_mut("properties")
        .and_then(|p| p.as_object_mut())
        .expect("Entrypoints properties");
    ep_props.insert("ui".to_owned(), ref_to("packagePath"));
    ep_props.insert("wasm".to_owned(), ref_to("packagePath"));

    // Dependency.version is the hostApi-style constraint string.
    rule_on_prop(&mut root, "Dependency", "version", "minLength", 1);

    // SchemaContribution.
    set_prop(
        &mut root,
        "SchemaContribution",
        "namespace",
        ref_to("namespace"),
    );
    set_prop(
        &mut root,
        "SchemaContribution",
        "entityTypes",
        json!({"type": "array", "items": {"type": "string", "minLength": 1}, "uniqueItems": true}),
    );

    // FieldDefinition.
    set_prop(
        &mut root,
        "FieldDefinition",
        "type",
        json!({"enum": ["text", "number", "boolean", "date", "enum", "entity-ref", "relationship"]}),
    );
    rule_on_prop(&mut root, "FieldDefinition", "key", "pattern", 0);
    {
        let defs = defs_entry(&mut root, "FieldDefinition");
        let key = defs["properties"]["key"].as_object_mut().expect("key");
        key.insert("pattern".to_owned(), json!(r"^[a-z][a-zA-Z0-9_]*$"));
        let label = defs["properties"]["label"].as_object_mut().expect("label");
        label.insert("minLength".to_owned(), json!(1));
        let options = defs["properties"]["options"]
            .as_object_mut()
            .expect("options");
        options.insert("uniqueItems".to_owned(), json!(true));
        let shared = defs["properties"]["shared"]
            .as_object_mut()
            .expect("shared");
        shared.insert("default".to_owned(), json!(false));
    }
    set_prop(
        &mut root,
        "FieldDefinition",
        "entityTypes",
        json!({"type": "array", "items": {"type": "string", "minLength": 1}, "minItems": 1, "uniqueItems": true}),
    );
    set_prop(
        &mut root,
        "FieldDefinition",
        "relationshipType",
        json!({"type": "string", "pattern": r"^[a-z][a-z0-9_-]*$"}),
    );
    set_prop(
        &mut root,
        "FieldDefinition",
        "targetEntityTypes",
        json!({"type": "array", "items": {"type": "string", "minLength": 1}, "minItems": 1, "uniqueItems": true}),
    );

    // EntityTemplate.
    for prop in ["id", "name", "entityType"] {
        rule_on_prop(&mut root, "EntityTemplate", prop, "minLength", 1);
    }
    set_prop(
        &mut root,
        "EntityTemplate",
        "fields",
        json!({"type": "object"}),
    );

    // Migration.
    set_prop(
        &mut root,
        "Migration",
        "recovery",
        json!({"enum": ["backup", "preserve-data"]}),
    );
    rule_on_prop(&mut root, "Migration", "from", "minimum", 0);
    rule_on_prop(&mut root, "Migration", "to", "minimum", 1);

    // MigrationOperation + ViewComponent: owned namespaces, non-empty forms.
    for (kind, subschemas_key) in [("MigrationOperation", "oneOf"), ("ViewComponent", "oneOf")] {
        let items = defs_entry(&mut root, kind)
            .get_mut(subschemas_key)
            .and_then(|o| o.as_array_mut())
            .expect(subschemas_key);
        for item in items {
            let Some(props) = item.get_mut("properties").and_then(|p| p.as_object_mut()) else {
                continue;
            };
            let is_field_form = props.contains_key("fields");
            if let Some(ns) = props.get_mut("namespace") {
                *ns = ref_to("namespace");
                if is_field_form {
                    let fields = props
                        .get_mut("fields")
                        .and_then(|p| p.as_object_mut())
                        .expect("fields");
                    fields.insert("minItems".to_owned(), json!(1));
                    fields.insert("uniqueItems".to_owned(), json!(true));
                }
            }
        }
    }

    // Command.
    set_prop(&mut root, "Command", "action", ref_to("CommandAction"));
    set_prop(
        &mut root,
        "Command",
        "capabilities",
        json!({"type": "array", "items": {"type": "string", "minLength": 1}, "uniqueItems": true}),
    );
    for prop in ["id", "title"] {
        rule_on_prop(&mut root, "Command", prop, "minLength", 1);
    }
    {
        let defs = defs_entry(&mut root, "Command");
        let exposure = defs["properties"]["exposure"]
            .as_object_mut()
            .expect("exposure");
        exposure.insert("uniqueItems".to_owned(), json!(true));
    }

    // CommandSchema.
    set_prop(
        &mut root,
        "CommandSchema",
        "type",
        json!({"const": "object"}),
    );
    {
        let defs = defs_entry(&mut root, "CommandSchema");
        let required = defs["properties"]["required"]
            .as_object_mut()
            .expect("required");
        required.insert("uniqueItems".to_owned(), json!(true));
        let items = required["items"].as_object_mut().expect("required items");
        items.insert("minLength".to_owned(), json!(1));
    }

    // View.
    for prop in ["id", "title"] {
        rule_on_prop(&mut root, "View", prop, "minLength", 1);
    }
    {
        let variants = defs_entry(&mut root, "ViewRenderer")
            .get_mut("oneOf")
            .and_then(|value| value.as_array_mut())
            .expect("ViewRenderer variants");
        let host_surface = variants
            .iter_mut()
            .find(|variant| {
                variant["properties"]["type"]["enum"]
                    .as_array()
                    .is_some_and(|values| values.iter().any(|value| value == "host-surface"))
            })
            .expect("host-surface renderer variant");
        let properties = host_surface["properties"]
            .as_object_mut()
            .expect("host-surface renderer properties");
        properties
            .get_mut("id")
            .and_then(|value| value.as_object_mut())
            .expect("host-surface renderer id")
            .insert(
                "pattern".to_owned(),
                json!(r"^[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*)*/[a-z0-9][a-z0-9_.-]*$"),
            );
        properties
            .get_mut("major")
            .and_then(|value| value.as_object_mut())
            .expect("host-surface renderer major")
            .insert("minimum".to_owned(), json!(1));
    }

    // Service + Event.
    for kind in ["Service", "Event"] {
        set_prop(&mut root, kind, "name", ref_to("serviceName"));
    }
    rule_on_prop(&mut root, "Service", "major", "minimum", 1);
    rule_on_prop(&mut root, "Event", "version", "minimum", 1);

    root
}

// ---------------------------------------------------------------------------
// plugin-rpc-v1.json + plugin-error-v1.json
// ---------------------------------------------------------------------------

/// Registers a payload type with the generator by its catalog `$defs` name.
fn register_payload(gen: &mut SchemaGenerator, payload_schema: &str) {
    let _ = match payload_schema {
        "EntityListPayload" => gen.subschema_for::<EntityListPayload>(),
        "EntityGetPayload" => gen.subschema_for::<EntityGetPayload>(),
        "EntityCreatePayload" => gen.subschema_for::<EntityCreatePayload>(),
        "EntityUpdatePayload" => gen.subschema_for::<EntityUpdatePayload>(),
        "EntityDeletePayload" => gen.subschema_for::<EntityDeletePayload>(),
        "DocumentListPayload" => gen.subschema_for::<DocumentListPayload>(),
        "DocumentSavePayload" => gen.subschema_for::<DocumentSavePayload>(),
        "FieldReadPayload" => gen.subschema_for::<FieldReadPayload>(),
        "FieldListPayload" => gen.subschema_for::<FieldListPayload>(),
        "FieldSetPayload" => gen.subschema_for::<FieldSetPayload>(),
        "RecordListPayload" => gen.subschema_for::<RecordListPayload>(),
        "RecordCreatePayload" => gen.subschema_for::<RecordCreatePayload>(),
        "RecordUpdatePayload" => gen.subschema_for::<RecordUpdatePayload>(),
        "RecordDeletePayload" => gen.subschema_for::<RecordDeletePayload>(),
        "RelationshipListPayload" => gen.subschema_for::<RelationshipListPayload>(),
        "RelationshipCreatePayload" => gen.subschema_for::<RelationshipCreatePayload>(),
        "RelationshipDeletePayload" => gen.subschema_for::<RelationshipDeletePayload>(),
        "AssetListPayload" => gen.subschema_for::<AssetListPayload>(),
        "AssetRegisterPayload" => gen.subschema_for::<AssetRegisterPayload>(),
        "AssetReadBeginPayload" => gen.subschema_for::<AssetReadBeginPayload>(),
        "AssetReplaceBeginPayload" => gen.subschema_for::<AssetReplaceBeginPayload>(),
        "AssetReplaceCommitPayload" => gen.subschema_for::<AssetReplaceCommitPayload>(),
        "AssetTransferCancelPayload" => gen.subschema_for::<AssetTransferCancelPayload>(),
        "SearchQueryPayload" => gen.subschema_for::<SearchQueryPayload>(),
        "MapsAssetCreateBeginPayload" => gen.subschema_for::<MapsAssetCreateBeginPayload>(),
        "MapsAssetCreateCommitPayload" => gen.subschema_for::<MapsAssetCreateCommitPayload>(),
        "MapsRecoveryExportBeginPayload" => gen.subschema_for::<MapsRecoveryExportBeginPayload>(),
        "MapsRecoveryExportCommitPayload" => gen.subschema_for::<MapsRecoveryExportCommitPayload>(),
        "MapsRecoveryListPayload" => gen.subschema_for::<MapsRecoveryListPayload>(),
        "MapsRecoveryRestorePayload" => gen.subschema_for::<MapsRecoveryRestorePayload>(),
        "MapsLocationsListPayload" => gen.subschema_for::<MapsLocationsListPayload>(),
        "MapsLocationsUpsertPayload" => gen.subschema_for::<MapsLocationsUpsertPayload>(),
        "MapsLocationsUnlinkPayload" => gen.subschema_for::<MapsLocationsUnlinkPayload>(),
        "MapsLocationsCreateAndLinkPayload" => {
            gen.subschema_for::<MapsLocationsCreateAndLinkPayload>()
        }
        "MapsReconcileLinksPayload" => gen.subschema_for::<MapsReconcileLinksPayload>(),
        "EventPublishPayload" => gen.subschema_for::<EventPublishPayload>(),
        "EventTypePayload" => gen.subschema_for::<EventTypePayload>(),
        "ServiceCallPayload" => gen.subschema_for::<ServiceCallPayload>(),
        "AiRequestStartPayload" => gen.subschema_for::<AiRequestStartPayload>(),
        "AiRequestIdPayload" => gen.subschema_for::<AiRequestIdPayload>(),
        other => panic!("catalog references unknown payload schema {other}"),
    };
}

/// Iterates the data-driven `RPC_METHOD_CATALOG`, registering each payload type
/// in catalog order and returning the `(method, schema name)` pairs that drive
/// `x-methods`, the method `enum`, and the request `allOf` conditions.
fn rpc_methods(gen: &mut SchemaGenerator) -> Vec<(&'static str, String)> {
    let mut methods: Vec<(&'static str, String)> = Vec::new();
    for entry in RPC_METHOD_CATALOG {
        register_payload(gen, entry.payload_schema);
        methods.push((entry.name, entry.payload_schema.to_string()));
    }
    methods
}

fn rpc_schema() -> Value {
    let mut gen = SchemaGenerator::new(settings());
    let methods = rpc_methods(&mut gen);

    // Shared (non-method) definitions.
    let _ = gen.subschema_for::<EntityCreateField>();
    let _ = gen.subschema_for::<EntityCreateRelationship>();
    let _ = gen.subschema_for::<EntityCreateDocument>();
    let _ = gen.subschema_for::<PluginBootstrap>();
    let _ = gen.subschema_for::<EntityRecord>();
    let _ = gen.subschema_for::<RpcError>();

    let defs = gen.take_definitions();
    let mut defs_value = serde_json::to_value(&defs).expect("defs serialize");

    // Rename the RpcError definition to the published `error` name.
    let error = defs_value
        .as_object_mut()
        .expect("defs object")
        .remove("RpcError")
        .expect("RpcError definition missing");
    defs_value
        .as_object_mut()
        .expect("defs object")
        .insert("error".to_owned(), error);

    // Error enrichment: code pattern + message bounds.
    {
        let props = defs_value
            .get_mut("error")
            .and_then(|e| e.as_object_mut())
            .expect("error")
            .get_mut("properties")
            .and_then(|p| p.as_object_mut())
            .expect("error props");
        let code = props
            .get_mut("code")
            .and_then(|p| p.as_object_mut())
            .expect("code");
        code.insert(
            "pattern".to_owned(),
            json!(r"^[a-z][a-z0-9]*(?:\.[a-z][a-z0-9-]*)+$"),
        );
        let message = props
            .get_mut("message")
            .and_then(|p| p.as_object_mut())
            .expect("message");
        message.insert("minLength".to_owned(), json!(1));
        message.insert("maxLength".to_owned(), json!(512));
    }

    let method_names: Vec<String> = methods.iter().map(|(name, _)| name.to_string()).collect();

    // x-methods catalog, driven by RPC_METHOD_CATALOG. `requires_revision`
    // gates the `requiredPayload` emission; the payload shape itself stays
    // authoritative for the exact (alphabetically-sorted) key list.
    let mut xmethods = Map::new();
    for (method, def_name) in &methods {
        let mut entry = Map::new();
        entry.insert("requestId".to_owned(), json!("envelope"));
        entry.insert("payload".to_owned(), json!(def_name));
        let catalog_entry = RPC_METHOD_CATALOG
            .iter()
            .find(|candidate| candidate.name == *method)
            .expect("catalog entry");
        if catalog_entry.requires_revision {
            let def = defs_value
                .get(def_name)
                .and_then(|d| d.as_object())
                .expect("payload def");
            if let Some(required) = def.get("required").and_then(|r| r.as_array()) {
                entry.insert("requiredPayload".to_owned(), Value::Array(required.clone()));
            }
        }
        if *method == "entity.create" {
            entry.insert("expectedRevision".to_owned(), json!(false));
        }
        xmethods.insert(method.to_string(), Value::Object(entry));
    }

    let request_conditions: Vec<Value> = methods
        .iter()
        .map(|(method, def_name)| {
            json!({
                "if": {"properties": {"method": {"const": method}}},
                "then": {"properties": {"payload": {"$ref": format!("#/$defs/{def_name}")}}}
            })
        })
        .collect();

    let request = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["rpcVersion", "sessionId", "requestId", "method", "payload"],
        "properties": {
            "rpcVersion": {"const": 1},
            "sessionId": {"type": "string", "minLength": 1, "maxLength": 128},
            "requestId": {"type": "string", "minLength": 1, "maxLength": 128},
            "method": {"enum": method_names},
            "payload": {}
        },
        "allOf": request_conditions
    });

    let response = json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["rpcVersion", "requestId", "ok"],
        "properties": {
            "rpcVersion": {"const": 1},
            "requestId": {"type": "string", "minLength": 1, "maxLength": 128},
            "ok": {"type": "boolean"},
            "result": {},
            "error": {"$ref": "#/$defs/error"}
        },
        "allOf": [
            {
                "if": {"properties": {"ok": {"const": true}}},
                "then": {"required": ["result"], "not": {"required": ["error"]}}
            },
            {
                "if": {"properties": {"ok": {"const": false}}},
                "then": {"required": ["error"], "not": {"required": ["result"]}}
            }
        ]
    });

    let defs_object = defs_value.as_object_mut().expect("defs object");
    defs_object.insert("request".to_owned(), request);
    defs_object.insert("response".to_owned(), response);

    json!({
        "$schema": META,
        "$id": RPC_ID,
        "title": "Daena Archive Plugin RPC v1",
        "x-methods": Value::Object(xmethods),
        "oneOf": [
            {"$ref": "#/$defs/request"},
            {"$ref": "#/$defs/response"}
        ],
        "$defs": defs_value
    })
}

fn error_schema() -> Value {
    let mut gen = SchemaGenerator::new(settings());
    let _ = gen.subschema_for::<RpcError>();
    let defs = gen.take_definitions();
    let mut error: Map<String, Value> = serde_json::to_value(&defs)
        .expect("defs serialize")
        .as_object()
        .expect("defs object")
        .get("RpcError")
        .expect("RpcError definition missing")
        .as_object()
        .expect("error object")
        .clone();
    let props = error
        .get_mut("properties")
        .and_then(|p| p.as_object_mut())
        .expect("props");
    let code = props
        .get_mut("code")
        .and_then(|p| p.as_object_mut())
        .expect("code");
    code.insert(
        "pattern".to_owned(),
        json!(r"^[a-z][a-z0-9]*(?:\.[a-z][a-z0-9-]*)+$"),
    );
    let message = props
        .get_mut("message")
        .and_then(|p| p.as_object_mut())
        .expect("message");
    message.insert("minLength".to_owned(), json!(1));
    message.insert("maxLength".to_owned(), json!(512));
    error.insert("$schema".to_owned(), json!(META));
    error.insert("$id".to_owned(), json!(ERROR_ID));
    error.insert(
        "title".to_owned(),
        json!("Daena Archive Plugin RPC Error v1"),
    );
    Value::Object(error)
}

// ---------------------------------------------------------------------------
// capability-registry-v1.json
// ---------------------------------------------------------------------------

fn capability_registry_schema() -> Value {
    let capabilities: Vec<Value> = CAPABILITY_REGISTRY
        .iter()
        .map(|entry| {
            let mut item = Map::new();
            item.insert("id".to_owned(), json!(entry.id));
            item.insert("resource".to_owned(), json!(entry.resource));
            item.insert("operations".to_owned(), json!(entry.operations));
            if let Some(confirmation) = entry.confirmation {
                item.insert("confirmation".to_owned(), json!(confirmation));
            }
            Value::Object(item)
        })
        .collect();
    json!({
        "$schema": META,
        "$id": CAPABILITY_ID,
        "version": 1,
        "capabilities": capabilities,
        "deniedByDefault": DENIED_BY_DEFAULT_CAPABILITIES
    })
}

// ---------------------------------------------------------------------------
// entrypoint
// ---------------------------------------------------------------------------

fn write_json(relative: &str, value: &Value) {
    let out = match std::env::var_os("DAENA_SCHEMA_OUT_DIR") {
        Some(dir) => PathBuf::from(dir).join(relative),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("schemas")
            .join(relative),
    };
    std::fs::create_dir_all(out.parent().expect("parent dir")).expect("create schemas dir");
    let bytes = serde_json::to_vec_pretty(value).expect("pretty JSON");
    std::fs::write(&out, bytes).expect("write schema");
    println!("wrote {}", out.display());
}

fn main() {
    let manifest = manifest_schema();
    let rpc = rpc_schema();
    let error = error_schema();
    let capability = capability_registry_schema();

    // Structural sanity checks mirroring scripts/validate-plugin-contract.mjs.
    let methods = rpc["x-methods"].as_object().expect("x-methods").len();
    assert!(methods >= 20, "expected >= 20 RPC methods, found {methods}");
    let request = rpc["$defs"]["request"].as_object().expect("request def");
    assert!(
        request["properties"]["method"]["enum"].is_array(),
        "method enum missing"
    );
    assert!(request["allOf"].is_array(), "request allOf missing");
    let defs = rpc["$defs"].as_object().expect("defs");
    for (method, contract) in rpc["x-methods"].as_object().expect("x-methods") {
        let payload = contract["payload"].as_str().expect("payload name");
        assert!(
            defs.contains_key(payload),
            "{method}: missing payload definition"
        );
        assert!(
            request["properties"]["method"]["enum"]
                .as_array()
                .expect("enum")
                .iter()
                .any(|v| v.as_str() == Some(method)),
            "{method}: missing method enum entry"
        );
    }
    assert!(manifest["$id"] == json!(MANIFEST_ID));
    assert!(rpc["$id"] == json!(RPC_ID));
    assert!(error["$id"] == json!(ERROR_ID));
    assert!(capability["$id"] == json!(CAPABILITY_ID));
    assert!(capability["version"] == json!(1));
    assert!(
        capability["deniedByDefault"]
            .as_array()
            .expect("deniedByDefault")
            .len()
            > 0
    );

    write_json("plugin-manifest-v1.json", &manifest);
    write_json("plugin-rpc-v1.json", &rpc);
    write_json("plugin-error-v1.json", &error);
    write_json("capability-registry-v1.json", &capability);
    println!("contract schemas regenerated");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins `RPC_METHOD_CATALOG` to the payload types it names: every
    /// `payload_schema` must be a real registered definition and
    /// `requires_revision` must equal whether that definition's required keys
    /// include `expectedRevision`. Guards the catalog against drifting from the
    /// typed payload structs (and vice versa).
    #[test]
    fn catalog_matches_emitted_payload_definitions() {
        let mut gen = SchemaGenerator::new(settings());
        let methods = rpc_methods(&mut gen);
        let defs = serde_json::to_value(gen.take_definitions()).expect("defs serialize");
        assert_eq!(methods.len(), RPC_METHOD_CATALOG.len());
        for (method, def_name) in &methods {
            let entry = RPC_METHOD_CATALOG
                .iter()
                .find(|candidate| candidate.name == *method)
                .expect("catalog entry");
            assert_eq!(
                entry.payload_schema, *def_name,
                "{method}: payload schema mismatch"
            );
            let has_revision = defs
                .get(def_name)
                .and_then(|definition| definition.get("required"))
                .and_then(|required| required.as_array())
                .is_some_and(|required| {
                    required
                        .iter()
                        .any(|item| item.as_str() == Some("expectedRevision"))
                });
            assert_eq!(
                entry.requires_revision, has_revision,
                "{method}: requires_revision does not match the payload schema"
            );
        }
    }

    /// Drift guard: the committed `schemas/*.json` must equal what this
    /// generator emits. Any Rust type or curated-rule change fails here until
    /// `npm run gen:plugin-contract` is re-run.
    #[test]
    fn committed_schemas_match_generation() {
        let schemas_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("schemas");
        let schemas: Vec<(&str, Value)> = vec![
            ("plugin-manifest-v1.json", manifest_schema()),
            ("plugin-rpc-v1.json", rpc_schema()),
            ("plugin-error-v1.json", error_schema()),
            ("capability-registry-v1.json", capability_registry_schema()),
        ];
        for (name, value) in schemas {
            let generated = serde_json::to_vec_pretty(&value).expect("pretty JSON");
            let committed = std::fs::read(schemas_dir.join(name)).expect("committed schema");
            assert_eq!(
                committed, generated,
                "{name} is stale; re-run `npm run gen:plugin-contract`"
            );
        }
    }
}
