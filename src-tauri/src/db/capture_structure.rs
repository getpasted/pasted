use super::*;

impl DbState {
    pub(super) fn persist_capture_structure(
        &self,
        clip: &ClipItem,
        structure: Option<&crate::content_inspection::StructuralMetadata>,
    ) {
        let persisted = structure.is_some_and(|metadata| {
            let stored_origin =
                crate::content_inspection::origin_kind(&clip.content_type, Some(&clip.source));
            if metadata.origin != stored_origin {
                return false;
            }
            let input_hash = crate::inspection_execution::inspection_input_hash(clip);
            self.record_structural_inspection(clip.id, &clip.content_hash, &input_hash, metadata)
                .unwrap_or(false)
        });
        if !persisted {
            let _ = crate::inspection_execution::inspect_clip_with_policy(
                self,
                clip.id,
                true,
                crate::analysis_contract::AnalysisPolicy::Capture,
            );
        }
    }
}
