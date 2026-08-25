use super::super::support::*;
use super::version_support::flagged_version_id;
use rusqlite::{params, Connection};

#[test]
fn clip_versions_have_safe_structured_restore_and_delete_contracts() {
    let database = temporary_path("clip-version-deletion", "db");
    let clip = success_json(&database, &["copy", "versioned CLI clip", "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID").to_string();
    let connection = Connection::open(&database).unwrap();
    for text in ["original CLI clip", "edited CLI clip"] {
        connection
            .execute(
                "INSERT INTO clip_versions (clip_id, text_content) VALUES (?1, ?2)",
                params![clip_id, text],
            )
            .unwrap();
    }

    let versions = success_json(&database, &["clip", "versions", &clip_id, "--json"]);
    let versions = versions.as_array().expect("version timeline");
    assert!(versions.iter().any(|version| version["is_current"] == true));
    let original = flagged_version_id(versions, "is_original", true);
    let deletable = flagged_version_id(versions, "is_current", false);

    let alias = success_json(&database, &["clip", "revisions", &clip_id, "--json"]);
    assert_eq!(alias.as_array().unwrap().len(), versions.len());
    let restored = success_json(
        &database,
        &["clip", "restore-version", &clip_id, &deletable, "--json"],
    );
    assert_eq!(restored["id"].to_string(), clip_id);
    assert!(
        !run(&database, &["clip", "delete-version", &clip_id, &deletable])
            .status
            .success()
    );
    let deleted = success_json(
        &database,
        &[
            "clip",
            "delete-version",
            &clip_id,
            &deletable,
            "--yes",
            "--json",
        ],
    );
    assert_eq!(deleted["deleted"], true);
    assert_eq!(deleted["versionId"].to_string(), deletable);
    let remaining = success_json(&database, &["clip", "versions", &clip_id, "--json"]);
    let original_id = original.parse::<i64>().unwrap();
    assert!(remaining
        .as_array()
        .unwrap()
        .iter()
        .any(|version| version["id"] == original_id));
    clean_database(&database);
}
