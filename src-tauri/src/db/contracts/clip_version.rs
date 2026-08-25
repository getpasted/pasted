use serde::{Deserialize, Serialize};

use crate::db::clip_visual_labels::EffectiveVisualLabels;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipVersion {
    pub id: i64,
    pub clip_id: i64,
    pub text_content: String,
    pub action_kind: Option<String>,
    pub action_label: Option<String>,
    pub restores_organization: bool,
    pub visual_labels: Option<EffectiveVisualLabels>,
    pub is_current: bool,
    pub is_original: bool,
    pub created_at: String,
}
