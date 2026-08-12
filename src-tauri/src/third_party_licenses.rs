use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

const LICENSE_DOCUMENT_JSON: &str = include_str!("../../THIRD_PARTY_LICENSES.json");

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseGenerator {
    cargo_about: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseSourceHashes {
    about_config: String,
    cargo_lock: String,
    generator_script: String,
    package_lock: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyComponent {
    ecosystem: String,
    name: String,
    version: String,
    license: String,
    repository: String,
    notice_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyNotice {
    id: String,
    labels: Vec<String>,
    spdx: Vec<String>,
    text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThirdPartyLicenseDocument {
    schema_version: u8,
    generator: LicenseGenerator,
    source_hashes: LicenseSourceHashes,
    component_count: usize,
    components: Vec<ThirdPartyComponent>,
    notices: Vec<ThirdPartyNotice>,
    notice_text: String,
}

static LICENSE_DOCUMENT: Lazy<ThirdPartyLicenseDocument> = Lazy::new(|| {
    serde_json::from_str(LICENSE_DOCUMENT_JSON)
        .expect("generated THIRD_PARTY_LICENSES.json must match the Rust data contract")
});

pub fn document() -> &'static ThirdPartyLicenseDocument {
    &LICENSE_DOCUMENT
}

impl ThirdPartyLicenseDocument {
    pub fn notice_text(&self) -> &str {
        &self.notice_text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_notice_is_consistent() {
        let document = document();
        assert_eq!(document.schema_version, 1);
        assert_eq!(document.component_count, document.components.len());
        assert!(document.component_count > 0);
        assert!(document
            .notice_text
            .contains("Pasted Third-Party Software Notices"));
        assert!(document
            .components
            .iter()
            .any(|component| component.name == "react"));
        assert!(document
            .components
            .iter()
            .any(|component| component.name == "tauri"));
    }
}
