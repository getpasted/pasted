#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub stable_ref: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub group_label: Option<String>,
    pub icon: String,
    pub enabled: Option<bool>,
    pub is_builtin: bool,
    pub is_archived: bool,
    pub sort_order: Option<i64>,
    pub revision: i64,
    pub input_contract: String,
    pub output_contract: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemCapabilities {
    pub can_edit: bool,
    pub can_duplicate: bool,
    pub can_delete: bool,
    pub can_disable: bool,
    pub can_restore: bool,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemView {
    #[serde(flatten)]
    pub item: LibraryItem,
    pub analysis_pass: Option<String>,
    pub capabilities: LibraryItemCapabilities,
}

impl LibraryItem {
    pub fn analysis_pass(&self) -> Option<String> {
        match self.kind.as_str() {
            "extractor" => Some("extract".into()),
            "detector" => Some("classify".into()),
            _ => None,
        }
    }

    pub fn capabilities(&self) -> LibraryItemCapabilities {
        match self.kind.as_str() {
            "extractor" => LibraryItemCapabilities {
                can_edit: true,
                can_duplicate: true,
                can_delete: true,
                can_disable: true,
                can_restore: self.is_builtin,
            },
            "detector" => LibraryItemCapabilities {
                can_edit: true,
                can_duplicate: true,
                can_delete: true,
                can_disable: true,
                can_restore: self.is_builtin,
            },
            "operation" => LibraryItemCapabilities {
                can_edit: !self.is_builtin,
                can_duplicate: !self.is_builtin,
                can_delete: !self.is_builtin,
                can_disable: !self.is_builtin,
                can_restore: false,
            },
            "transform" => LibraryItemCapabilities {
                can_edit: true,
                can_duplicate: true,
                can_delete: true,
                can_disable: false,
                can_restore: false,
            },
            _ => LibraryItemCapabilities {
                can_edit: false,
                can_duplicate: false,
                can_delete: false,
                can_disable: false,
                can_restore: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: &str, is_builtin: bool) -> LibraryItem {
        LibraryItem {
            stable_ref: format!("{kind}:test"),
            kind: kind.into(),
            name: "Test".into(),
            description: String::new(),
            group_label: None,
            icon: String::new(),
            enabled: Some(true),
            is_builtin,
            is_archived: false,
            sort_order: Some(1),
            revision: 1,
            input_contract: String::new(),
            output_contract: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn extractor_capabilities_match_the_shared_lifecycle_surface() {
        let shipped = item("extractor", true).capabilities();
        assert!(shipped.can_edit);
        assert!(shipped.can_duplicate);
        assert!(shipped.can_delete);
        assert!(shipped.can_disable);
        assert!(shipped.can_restore);

        let custom = item("extractor", false).capabilities();
        assert!(custom.can_duplicate);
        assert!(custom.can_delete);
        assert!(!custom.can_restore);
    }
}
