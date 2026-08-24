use super::super::*;

#[test]
fn legacy_semantic_clip_types_become_preserved_content_type_matches() {
    let db = setup_test_db();
    let clip = save_plain_test_clip(
        &db,
        "link",
        "https://example.com",
        "legacy-link-hash",
        "Test",
    );
    {
        let conn = db.conn.lock();
        migrate_legacy_semantic_clip_types(&conn).unwrap();
    }

    let migrated = db.get_clip_by_id(clip.id).unwrap();
    assert_eq!(migrated.content_type, "text");
    assert_eq!(migrated.content_types, vec!["link"]);
    let matches = db.get_analysis_classifications(clip.id).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].classifier_ref, "url");
    assert_eq!(matches[0].start_offset, None);
}

#[test]
fn legacy_source_app_column_migrates_without_losing_filters_or_search() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pasted_source_migration_{nanos}.db"));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE clips (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content_type TEXT NOT NULL,
                    text_content TEXT,
                    html_content TEXT,
                    image_base64 TEXT,
                    content_hash TEXT UNIQUE NOT NULL,
                    source_app TEXT DEFAULT 'Unknown',
                    is_pinned INTEGER DEFAULT 0,
                    bin_id INTEGER,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 CREATE TABLE bins (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    icon TEXT DEFAULT 'Folder',
                    color TEXT DEFAULT 'default',
                    smart_rule TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 INSERT INTO clips
                    (content_type, text_content, content_hash, source_app)
                 VALUES ('text', 'migration-search-token', 'legacy-source-hash', 'Safari');
                 INSERT INTO bins (name, smart_rule)
                 VALUES ('Safari', '{\"type\":\"source_app\",\"value\":\"Safari\"}');",
        )
        .unwrap();
    drop(connection);

    let db = DbState::new(path).unwrap();
    let conn = db.conn.lock();
    assert!(column_exists(&conn, "clips", "source").unwrap());
    assert!(!column_exists(&conn, "clips", "source_app").unwrap());
    let migrated_rule: String = conn
        .query_row(
            "SELECT smart_rule FROM bins WHERE name = 'Safari'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(migrated_rule, r#"{"type":"source","value":"Safari"}"#);
    drop(conn);

    let clips = search_test_clips(&db, "migration-search-token");
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].source, "Safari");
    assert_eq!(db.get_clips(Some(1), false).unwrap().len(), 1);

    let backup = db.export_backup_json().unwrap();
    assert!(backup.contains("\"source\": \"Safari\""));
    assert!(!backup.contains("\"source_app\""));

    let mut legacy_backup: serde_json::Value = serde_json::from_str(&backup).unwrap();
    for clip in legacy_backup["clips"].as_array_mut().unwrap() {
        let object = clip.as_object_mut().unwrap();
        let source = object.remove("source").unwrap();
        object.insert("source_app".to_string(), source);
    }
    let destination = setup_test_db();
    destination
        .import_backup_json(&serde_json::to_string(&legacy_backup).unwrap())
        .unwrap();
    assert!(destination
        .get_clips(None, false)
        .unwrap()
        .iter()
        .any(|clip| clip.source == "Safari"));
}

#[test]
fn legacy_classification_preferences_migrate_once_into_classifier_records() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pasted_classifier_migration_{nanos}.db"));
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO settings (key, value) VALUES ('detectColors', 'false');",
        )
        .unwrap();
    drop(connection);

    let db = DbState::new(path).unwrap();
    let classifiers = db.get_content_classifiers().unwrap();
    assert!(
        !classifiers
            .iter()
            .find(|classifier| classifier.stable_ref == "color")
            .unwrap()
            .enabled
    );
    assert!(
        classifiers
            .iter()
            .find(|classifier| classifier.stable_ref == "url")
            .unwrap()
            .enabled
    );

    let color = classifiers
        .iter()
        .find(|classifier| classifier.stable_ref == "color")
        .unwrap();
    db.update_content_classifier(
        color.id,
        &crate::content_classification::ClassifierInput {
            name: color.name.clone(),
            content_type: color.content_type.clone(),
            description: color.description.clone(),
            patterns: color.patterns.clone(),
            validator: color.validator.clone(),
            enabled: true,
            priority: color.priority,
        },
    )
    .unwrap();
    let reopened = DbState::new(db.database_path()).unwrap();
    assert!(
        reopened
            .get_content_classifiers()
            .unwrap()
            .iter()
            .find(|classifier| classifier.stable_ref == "color")
            .unwrap()
            .enabled
    );
}

#[test]
fn legacy_analysis_terminology_migrates_without_losing_classifier_configuration() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("pasted_analysis_terms_{nanos}.db"));
    let connection = Connection::open(&path).unwrap();
    connection
            .execute_batch(
                "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO settings (key, value) VALUES ('enableContentDetection', 'false');
                 CREATE TABLE schema_migrations (key TEXT PRIMARY KEY, applied_at DATETIME DEFAULT CURRENT_TIMESTAMP);
                 INSERT INTO schema_migrations (key) VALUES ('contentDetectorRegistryV1');
                 CREATE TABLE content_detectors (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    stable_ref TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    content_type TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    patterns_json TEXT NOT NULL,
                    validator TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    priority INTEGER NOT NULL DEFAULT 100,
                    is_builtin INTEGER NOT NULL DEFAULT 0,
                    is_deleted INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 INSERT INTO content_detectors
                    (stable_ref, name, content_type, patterns_json, enabled, priority)
                 VALUES ('custom:legacy-classifier', 'Legacy Classifier', 'prose', '[\"legacy\"]', 0, 42);
                 CREATE TABLE content_classifiers (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    stable_ref TEXT NOT NULL UNIQUE,
                    name TEXT NOT NULL,
                    content_type TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    patterns_json TEXT NOT NULL,
                    validator TEXT,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    priority INTEGER NOT NULL DEFAULT 100,
                    is_builtin INTEGER NOT NULL DEFAULT 0,
                    is_deleted INTEGER NOT NULL DEFAULT 0,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 INSERT INTO content_classifiers
                    (stable_ref, name, content_type, patterns_json, enabled, priority)
                 VALUES ('custom:current-classifier', 'Current Classifier', 'prose', '[\"current\"]', 1, 41);",
            )
            .unwrap();
    drop(connection);

    let db = DbState::new(path).unwrap();
    let classifiers = db.get_content_classifiers().unwrap();
    let migrated = classifiers
        .iter()
        .find(|classifier| classifier.stable_ref == "custom:legacy-classifier")
        .unwrap();
    assert_eq!(migrated.name, "Legacy Classifier");
    assert!(!migrated.enabled);
    assert_eq!(migrated.priority, 42);
    assert!(classifiers
        .iter()
        .any(|classifier| classifier.stable_ref == "custom:current-classifier"));
    assert_eq!(
        db.get_setting("enableContentClassification")
            .unwrap()
            .as_deref(),
        Some("false")
    );
    assert_eq!(db.get_setting("enableContentDetection").unwrap(), None);

    let conn = db.conn.lock();
    assert!(table_exists(&conn, "content_classifiers").unwrap());
    assert!(!table_exists(&conn, "content_detectors").unwrap());
    let migrated_key: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE key = 'contentClassifierRegistryV1')",
                [],
                |row| row.get(0),
            )
            .unwrap();
    assert!(migrated_key);
}
