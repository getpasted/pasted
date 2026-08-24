use super::super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchGrammarFixture {
    query: String,
    sources: Vec<String>,
    clip_types: Vec<String>,
    content_types: Vec<String>,
    file_formats: Vec<String>,
    terms: Vec<String>,
    requires_note: bool,
    requires_named: bool,
    requires_pinned: bool,
    requires_protected: bool,
    requires_trashed: bool,
    incomplete: bool,
    regex: Option<String>,
    regex_fallback: Option<String>,
}

#[test]
fn native_and_frontend_search_grammar_share_public_fixtures() {
    let fixtures: Vec<SearchGrammarFixture> = serde_json::from_str(include_str!(
        "../../../../../contracts/search/v1/grammar.json"
    ))
    .unwrap();
    for fixture in fixtures {
        let parsed = parse_clip_search(&fixture.query);
        assert_eq!(parsed.sources, fixture.sources, "{}", fixture.query);
        assert_eq!(parsed.clip_types, fixture.clip_types, "{}", fixture.query);
        assert_eq!(
            parsed.content_types, fixture.content_types,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.file_formats, fixture.file_formats,
            "{}",
            fixture.query
        );
        assert_eq!(parsed.terms, fixture.terms, "{}", fixture.query);
        assert_eq!(
            parsed.requires_note, fixture.requires_note,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_named, fixture.requires_named,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_pinned, fixture.requires_pinned,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_protected, fixture.requires_protected,
            "{}",
            fixture.query
        );
        assert_eq!(
            parsed.requires_trashed, fixture.requires_trashed,
            "{}",
            fixture.query
        );
        assert_eq!(parsed.incomplete, fixture.incomplete, "{}", fixture.query);
        assert_eq!(parsed.regex, fixture.regex, "{}", fixture.query);
        assert_eq!(
            parsed.regex_fallback, fixture.regex_fallback,
            "{}",
            fixture.query
        );
    }
}

#[test]
#[ignore = "run explicitly against a disposable copy of a real Pasted database"]
fn real_database_library_item_migration_smoke_test() {
    let path = std::env::var("PASTED_MIGRATION_TEST_DB")
        .expect("PASTED_MIGRATION_TEST_DB must point to a disposable database copy");
    let db = DbState::new(PathBuf::from(path)).unwrap();
    let items = db.get_library_items(None, true).unwrap();
    assert!(items.iter().any(|item| item.item.kind == "classifier"));
    assert!(items.iter().any(|item| item.item.kind == "extractor"));
    assert!(items.iter().any(|item| item.item.kind == "operation"));
    assert!(items.iter().any(|item| item.item.kind == "capture"));
}
