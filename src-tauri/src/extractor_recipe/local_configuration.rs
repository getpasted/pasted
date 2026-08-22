use super::ExtractorRecipe;

pub fn reset_preserving_local_paths(
    current: &ExtractorRecipe,
    defaults: &ExtractorRecipe,
) -> ExtractorRecipe {
    let mut reset = defaults.clone();
    for step in &mut reset.steps {
        let current_path = current
            .steps
            .iter()
            .find(|candidate| candidate.id == step.id)
            .and_then(|candidate| candidate.executable.path.clone());
        if current_path.is_some() {
            step.executable.path = current_path;
        }
    }
    for resource in &mut reset.resources {
        let current_path = current
            .resources
            .iter()
            .find(|candidate| candidate.id == resource.id)
            .and_then(|candidate| candidate.path.clone());
        if current_path.is_some() {
            resource.path = current_path;
        }
    }
    reset
}
