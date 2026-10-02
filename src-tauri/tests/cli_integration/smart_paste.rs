use super::super::support::*;

#[test]
fn lists_and_selects_persistent_detected_parts() {
    let database = temporary_path("smart-paste", "db");
    let source = "Zoë Example\nzoe@example.com\nChicago, IL";
    let clip = success_json(&database, &["copy", source, "--json"]);
    let clip_id = clip["id"].as_i64().expect("clip ID").to_string();

    let parts = success_json(
        &database,
        &["smart-paste", "parts", "--clip", &clip_id, "--json"],
    );
    let email = parts
        .as_array()
        .and_then(|items| items.iter().find(|part| part["kind"] == "email"))
        .expect("persistent email part");
    assert_eq!(email["value"], "zoe@example.com");
    assert_eq!(email["startOffset"], 12);
    assert_eq!(email["endOffset"], 27);

    let selected = success_json(
        &database,
        &[
            "smart-paste",
            "--context",
            "Email address",
            "--text",
            source,
            "--json",
        ],
    );
    assert_eq!(selected["value"], "zoe@example.com");

    success_json(
        &database,
        &["settings", "set", "enableSmartPaste", "false", "--json"],
    );
    assert!(!run(
        &database,
        &[
            "smart-paste",
            "--context",
            "Email address",
            "--text",
            source,
            "--json"
        ],
    )
    .status
    .success());
    assert!(!run(
        &database,
        &["smart-paste", "parts", "--clip", &clip_id, "--json"],
    )
    .status
    .success());
    let new_clip = success_json(&database, &["copy", "new@example.com", "--json"]);
    let db = pasted_lib::db::DbState::new(database.clone()).expect("open shared database");
    assert!(db
        .get_paste_parts(new_clip["id"].as_i64().expect("new clip ID"))
        .expect("read paste parts")
        .is_none());
    assert!(db
        .get_paste_parts(clip_id.parse().expect("old clip ID"))
        .expect("read original paste parts")
        .is_some());
    drop(db);
    clean_database(&database);
}
