use super::*;

#[test]
fn repair_prompt_contains_host_diagnostics_but_not_local_paths() {
    let mut recipe = crate::content_extraction::EXTRACTOR_PRESETS[0].recipe();
    recipe.steps[0].executable.path = Some("/private/pasted-test-tool".into());
    let diagnostic = crate::extractor_recipe::diagnose(&recipe);
    let prompt = repair_prompt("Vision", "Describe images", &recipe, &diagnostic, None)
        .expect("repair prompt");
    assert!(prompt.contains(std::env::consts::OS));
    assert!(!prompt.contains("/private/pasted-test-tool"));
}
