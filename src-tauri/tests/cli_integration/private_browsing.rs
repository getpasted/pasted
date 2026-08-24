use super::support::*;

#[test]
fn private_browser_policy_has_gui_parity_and_stable_json_output() {
    let database = temporary_path("private-browser-policy", "db");
    let defaults = success_json(&database, &["private-browsing", "status", "--json"]);
    assert_eq!(defaults["enabled"], false);
    assert_eq!(defaults["unavailablePolicy"], "capture");
    assert_eq!(
        defaults["supportedBrowsers"].as_array().map(Vec::len),
        Some(6)
    );

    let enabled = success_json(&database, &["private-browsing", "enable", "--json"]);
    assert_eq!(enabled["enabled"], true);
    let strict = success_json(
        &database,
        &["private-browsing", "fallback", "exclude-browser", "--json"],
    );
    assert_eq!(strict["unavailablePolicy"], "exclude_browser");
    assert_eq!(
        success_json(
            &database,
            &[
                "settings",
                "get",
                "privateBrowserUnavailablePolicy",
                "--json",
            ],
        )["value"],
        "exclude_browser"
    );
    let invalid = run(
        &database,
        &["private-browsing", "fallback", "guess", "--json"],
    );
    assert_eq!(invalid.status.code(), Some(2));
    clean_database(&database);
}
