use crate::analysis_contract::{AnalysisPass, ParticipantContract, RepresentationKind};
use std::str::FromStr;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participant_contract: Option<ParticipantContract>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub type_relations: Vec<AnalysisTypeRelation>,
    pub capabilities: LibraryItemCapabilities,
}

#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisTypeRelationKind {
    Accepts,
    ClassifiesAs,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisTypeRelation {
    pub kind: AnalysisTypeRelationKind,
    pub type_id: String,
}

impl LibraryItem {
    pub fn analysis_pass(&self) -> Option<String> {
        match self.kind.as_str() {
            "inspector" => Some("inspect".into()),
            "extractor" => Some("extract".into()),
            "detector" => Some("classify".into()),
            "enricher" => Some("enrich".into()),
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

    pub fn participant_contract(&self) -> Option<ParticipantContract> {
        let pass = match self.kind.as_str() {
            "inspector" => AnalysisPass::Inspect,
            "extractor" => AnalysisPass::Extract,
            "detector" => AnalysisPass::Classify,
            "enricher" => AnalysisPass::Enrich,
            _ => return None,
        };
        let requires = representation_list(&self.input_contract);
        let mut provides = if self.kind == "detector" {
            vec![RepresentationKind::Classification]
        } else {
            representation_list(&self.output_contract)
        };
        if provides.contains(&RepresentationKind::SearchableText)
            && !provides.contains(&RepresentationKind::AnalyzableText)
        {
            provides.push(RepresentationKind::AnalyzableText);
        }
        if requires.is_empty() || provides.is_empty() {
            return None;
        }
        Some(ParticipantContract {
            stable_ref: self.stable_ref.clone(),
            name: self.name.clone(),
            pass,
            priority: self.sort_order.unwrap_or(0),
            requires,
            provides,
        })
    }

    pub fn type_relations(&self) -> Vec<AnalysisTypeRelation> {
        let mut relations = Vec::new();
        for requirement in representation_list(&self.input_contract) {
            let type_id = match requirement {
                RepresentationKind::ImageBytes => Some("image"),
                RepresentationKind::FileReferences => Some("file"),
                _ => None,
            };
            if let Some(type_id) = type_id {
                relations.push(AnalysisTypeRelation {
                    kind: AnalysisTypeRelationKind::Accepts,
                    type_id: type_id.into(),
                });
            }
        }
        if self.kind == "detector" {
            if let Some(type_id) = self
                .output_contract
                .strip_prefix("set_type:")
                .filter(|type_id| !type_id.is_empty())
            {
                relations.push(AnalysisTypeRelation {
                    kind: AnalysisTypeRelationKind::ClassifiesAs,
                    type_id: type_id.into(),
                });
            }
        }
        relations
    }
}

fn representation_list(contract: &str) -> Vec<RepresentationKind> {
    contract
        .split('+')
        .filter_map(|value| match value {
            "clip" => Some(RepresentationKind::ClipKind),
            "text" => Some(RepresentationKind::AnalyzableText),
            value => RepresentationKind::from_str(value).ok(),
        })
        .collect()
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

    #[test]
    fn analysis_items_expose_typed_participant_and_type_relations() {
        let mut extractor = item("extractor", true);
        extractor.stable_ref = "extractor:ocr".into();
        extractor.input_contract = "image".into();
        extractor.output_contract = "searchable_text".into();
        let contract = extractor.participant_contract().unwrap();
        assert_eq!(contract.pass, AnalysisPass::Extract);
        assert_eq!(contract.requires, vec![RepresentationKind::ImageBytes]);
        assert_eq!(
            contract.provides,
            vec![
                RepresentationKind::SearchableText,
                RepresentationKind::AnalyzableText
            ]
        );
        assert_eq!(
            extractor.type_relations(),
            vec![AnalysisTypeRelation {
                kind: AnalysisTypeRelationKind::Accepts,
                type_id: "image".into(),
            }]
        );

        let mut detector = item("detector", true);
        detector.input_contract = "text".into();
        detector.output_contract = "set_type:link".into();
        assert_eq!(
            detector.participant_contract().unwrap().provides,
            vec![RepresentationKind::Classification]
        );
        assert_eq!(
            detector.type_relations(),
            vec![AnalysisTypeRelation {
                kind: AnalysisTypeRelationKind::ClassifiesAs,
                type_id: "link".into(),
            }]
        );

        detector.input_contract = "unknown".into();
        assert!(detector.participant_contract().is_none());
    }
}
