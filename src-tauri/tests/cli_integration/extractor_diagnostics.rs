use super::support::*;

#[test]
fn extractor_preflight_reports_all_missing_dependencies_without_local_paths() {
    let database = temporary_path("extractor-preflight", "db");
    let report = success_json(
        &database,
        &[
            "extractor",
            "preflight",
            "extractor:whisper-transcription",
            "--json",
        ],
    );
    assert_eq!(report["version"], 1);
    assert_eq!(report["isAvailable"], false);
    assert!(report["issues"].as_array().is_some_and(|issues| issues
        .iter()
        .any(|issue| issue["code"] == "resource_not_configured" && issue["subjectId"] == "model")));
    assert!(!report.to_string().contains("/Users/"));
    clean_database(&database);
}
