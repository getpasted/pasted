use serde::{Deserialize, Serialize};
use std::path::Path;

use super::{
    executable_label, resolve_executable, resource_path_is_available, validate_recipe,
    ExtractorRecipe,
};

pub const EXTRACTOR_DIAGNOSTIC_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorDiagnosticCode {
    InvalidRecipe,
    ExecutableNotConfigured,
    ExecutableUnavailable,
    ResourceNotConfigured,
    ResourceUnavailable,
}

impl ExtractorDiagnosticCode {
    pub const fn stable_name(&self) -> &'static str {
        match self {
            Self::InvalidRecipe => "invalid_recipe",
            Self::ExecutableNotConfigured => "executable_not_configured",
            Self::ExecutableUnavailable => "executable_unavailable",
            Self::ResourceNotConfigured => "resource_not_configured",
            Self::ResourceUnavailable => "resource_unavailable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorDiagnosticIssue {
    pub code: ExtractorDiagnosticCode,
    pub subject_id: String,
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExtractorDiagnosticReport {
    pub version: u32,
    pub is_available: bool,
    pub platform: String,
    pub architecture: String,
    pub package_managers: Vec<String>,
    pub issues: Vec<ExtractorDiagnosticIssue>,
}

pub fn diagnose(recipe: &ExtractorRecipe) -> ExtractorDiagnosticReport {
    let mut issues = Vec::new();
    if let Err(detail) = validate_recipe(recipe) {
        issues.push(ExtractorDiagnosticIssue {
            code: ExtractorDiagnosticCode::InvalidRecipe,
            subject_id: "recipe".into(),
            label: "Extractor recipe".into(),
            detail,
        });
    }
    diagnose_executables(recipe, &mut issues);
    diagnose_resources(recipe, &mut issues);
    ExtractorDiagnosticReport {
        version: EXTRACTOR_DIAGNOSTIC_VERSION,
        is_available: issues.is_empty(),
        platform: std::env::consts::OS.into(),
        architecture: std::env::consts::ARCH.into(),
        package_managers: available_package_managers(),
        issues,
    }
}

fn diagnose_executables(recipe: &ExtractorRecipe, issues: &mut Vec<ExtractorDiagnosticIssue>) {
    for step in &recipe.steps {
        if resolve_executable(&step.executable).is_some() {
            continue;
        }
        let not_configured = step.executable.path.is_none() && step.executable.discover.is_empty();
        issues.push(ExtractorDiagnosticIssue {
            code: if not_configured {
                ExtractorDiagnosticCode::ExecutableNotConfigured
            } else {
                ExtractorDiagnosticCode::ExecutableUnavailable
            },
            subject_id: step.id.clone(),
            label: executable_label(step),
            detail: if not_configured {
                "No executable path or discovery candidate is configured.".into()
            } else {
                "No configured executable or discovery candidate is available.".into()
            },
        });
    }
}

fn diagnose_resources(recipe: &ExtractorRecipe, issues: &mut Vec<ExtractorDiagnosticIssue>) {
    for resource in recipe.resources.iter().filter(|resource| resource.required) {
        let Some(path) = resource.path.as_deref() else {
            issues.push(ExtractorDiagnosticIssue {
                code: ExtractorDiagnosticCode::ResourceNotConfigured,
                subject_id: resource.id.clone(),
                label: resource.label.clone(),
                detail: "A required local resource has not been selected.".into(),
            });
            continue;
        };
        if !resource_path_is_available(resource, Path::new(path)) {
            issues.push(ExtractorDiagnosticIssue {
                code: ExtractorDiagnosticCode::ResourceUnavailable,
                subject_id: resource.id.clone(),
                label: resource.label.clone(),
                detail: "The configured local resource is unavailable or has the wrong kind."
                    .into(),
            });
        }
    }
}

fn available_package_managers() -> Vec<String> {
    ["brew", "winget", "apt-get", "dnf", "pacman", "zypper"]
        .into_iter()
        .filter(|name| crate::external_tools::find_executable(name, &[]).is_some())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod invalid_tests;
#[cfg(test)]
mod tests;
