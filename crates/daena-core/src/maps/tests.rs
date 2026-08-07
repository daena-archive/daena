use super::*;
use rusqlite::params;

#[test]
fn rejects_out_of_range_and_open_geometry() {
    assert!(point(&Point(1.1, 0.2)).is_err());
    assert!(anchor(
        &serde_json::json!({"kind":"area","rings":[[[0.1,0.1],[0.2,0.1],[0.2,0.2],[0.1,0.2]]]})
    )
    .is_err());
    assert!(anchor(&serde_json::json!({"kind":"path","points":[[0.1,0.1],[0.2,0.2]]})).is_ok());
}

#[test]
fn validates_descriptor_with_null_source_until_first_save() {
    let connection = Connection::open_in_memory().unwrap();
    connection
            .execute(
                "CREATE TABLE entities (id TEXT PRIMARY KEY, entity_type TEXT, deleted INTEGER NOT NULL DEFAULT 0)",
                [],
            )
            .unwrap();
    connection
            .execute(
                "CREATE TABLE assets (id TEXT PRIMARY KEY, entity_id TEXT NOT NULL, namespace TEXT NOT NULL)",
                [],
            )
            .unwrap();
    let map_id = Uuid::new_v4().to_string();
    let other_id = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO entities (id, entity_type) VALUES (?1, ?2), (?3, ?4)",
            params![map_id, MAP_ENTITY_TYPE, other_id, "place"],
        )
        .unwrap();
    let descriptor = |source_asset_id: Option<String>| {
        serde_json::json!({
            "schemaVersion": 1,
            "provider": {"id": FMG_PROVIDER, "adapterVersion": 1, "sourceFormat": "fmg-map"},
            "sourceAssetId": source_asset_id,
            "previewAssetId": null,
            "defaultView": {"center": [0.5, 0.5], "zoom": 1}
        })
    };

    assert!(validate_field(&connection, &map_id, "map", &descriptor(None)).is_ok());

    let foreign_asset = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO assets (id, entity_id, namespace) VALUES (?1, ?2, ?3)",
            params![foreign_asset, other_id, MAP_NAMESPACE],
        )
        .unwrap();
    assert!(
        validate_field(
            &connection,
            &map_id,
            "map",
            &descriptor(Some(foreign_asset.clone()))
        )
        .is_err(),
        "an asset owned by another entity must be rejected"
    );

    let owned_asset = Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO assets (id, entity_id, namespace) VALUES (?1, ?2, ?3)",
            params![owned_asset, map_id, MAP_NAMESPACE],
        )
        .unwrap();
    assert!(validate_field(&connection, &map_id, "map", &descriptor(Some(owned_asset))).is_ok());
}

#[test]
fn validates_dates_without_inventing_precision() {
    let year = serde_json::json!({"calendar":"gregorian","era":"CE","year":42,"precision":"year"});
    assert!(date(&year, "date").is_ok());
    let incomplete_month =
        serde_json::json!({"calendar":"gregorian","era":"CE","year":42,"precision":"month"});
    assert!(date(&incomplete_month, "date").is_err());
}

#[test]
fn rejects_inverted_validity_date_bounds() {
    let from_late =
        serde_json::json!({"calendar":"gregorian","era":"CE","year":2025,"precision":"year"});
    let to_early =
        serde_json::json!({"calendar":"gregorian","era":"CE","year":2020,"precision":"year"});
    let inverted = serde_json::json!({"from": from_late, "to": to_early});
    assert!(validity(&inverted, "validity").is_err());

    let valid = serde_json::json!({"from": to_early, "to": from_late});
    assert!(validity(&valid, "validity").is_ok());
}
