use std::path::Path;

use crate::content_extraction::{ExtractorRuntimeDependency, ExtractorRuntimeStatus};

use super::{resolve_executable, resource_path_is_available, ExtractorRecipe};

pub fn runtime_status(recipe: &ExtractorRecipe) -> ExtractorRuntimeStatus {
    build_runtime_status(recipe, true)
}

pub fn runtime_status_summary(recipe: &ExtractorRecipe) -> ExtractorRuntimeStatus {
    build_runtime_status(recipe, false)
}

fn build_runtime_status(recipe: &ExtractorRecipe, probe_versions: bool) -> ExtractorRuntimeStatus {
    let primary = recipe.steps.first();
    let location = primary
        .and_then(|step| resolve_executable(&step.executable))
        .map(|path| path.to_string_lossy().into_owned());
    let version =
        primary.and_then(|step| probe_versions.then(|| probe_step_version(step)).flatten());
    let mut dependencies = recipe
        .steps
        .iter()
        .skip(1)
        .map(|step| {
            let path = resolve_executable(&step.executable);
            ExtractorRuntimeDependency {
                name: step.id.clone(),
                location: path
                    .as_deref()
                    .map(|path| path.to_string_lossy().into_owned()),
                version: probe_versions.then(|| probe_step_version(step)).flatten(),
                is_available: path.is_some(),
                unavailable_reason: path
                    .is_none()
                    .then(|| format!("{} is unavailable.", step.id)),
            }
        })
        .collect::<Vec<_>>();
    dependencies.extend(recipe.resources.iter().map(|resource| {
        let available = resource
            .path
            .as_deref()
            .is_some_and(|path| resource_path_is_available(resource, Path::new(path)));
        ExtractorRuntimeDependency {
            name: resource.label.clone(),
            location: resource.path.clone(),
            version: None,
            is_available: available || !resource.required,
            unavailable_reason: (resource.required && !available)
                .then(|| format!("{} is unavailable.", resource.label)),
        }
    }));
    ExtractorRuntimeStatus {
        method: "recipe".into(),
        location,
        version,
        uses_automatic_discovery: primary.is_some_and(|step| step.executable.path.is_none()),
        dependencies,
    }
}

fn probe_step_version(step: &super::ExtractorCommandStep) -> Option<String> {
    if step.executable.version_arguments.is_empty() {
        return None;
    }
    let path = resolve_executable(&step.executable)?;
    let arguments = step
        .executable
        .version_arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    crate::external_tools::probe_version(&path, &arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_reports_readiness_without_runtime_versions() {
        let mut recipe = crate::content_extraction::EXTRACTOR_PRESETS[0].recipe();
        recipe.steps[0].executable.path = Some(
            std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        );
        recipe.steps[0].executable.discover.clear();
        recipe.steps[0].executable.version_arguments = vec!["--version".into()];
        let summary = runtime_status_summary(&recipe);

        assert!(summary.location.is_some());
        assert!(summary.version.is_none());
        assert!(summary
            .dependencies
            .iter()
            .all(|dependency| dependency.version.is_none()));
    }
}
