use super::super::*;

#[test]
fn clip_id_filters_accept_query_groups_and_structured_arrays() {
    let db = setup_test_db();
    let first = save_plain_test_clip(&db, "text", "First", "id-filter-first", "Tests");
    let second = save_plain_test_clip(&db, "text", "Second", "id-filter-second", "Tests");

    let by_query_ids = db
        .search_clips(&ClipSearchRequest {
            query: format!("id:{},{}", first.id, second.id),
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(by_query_ids.total_count, 2);

    let by_request_ids = db
        .search_clips(&ClipSearchRequest {
            clip_ids: vec![first.id],
            limit: 10,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(by_request_ids.total_count, 1);
    assert_eq!(by_request_ids.items[0].id, first.id);

    assert!(db
        .search_clips(&ClipSearchRequest {
            query: "id:0".into(),
            limit: 10,
            ..Default::default()
        })
        .unwrap()
        .items
        .is_empty());
}
