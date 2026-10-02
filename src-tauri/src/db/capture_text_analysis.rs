use rusqlite::Result;

use super::DbState;

impl DbState {
    pub(super) fn persist_capture_text_analysis(
        &self,
        clip_id: i64,
        content_hash: &str,
        source: &str,
        matches: &[crate::content_classification::ClassificationMatch],
        include_classifiers: bool,
    ) -> Result<()> {
        if include_classifiers {
            self.replace_analysis_classifications(clip_id, content_hash, matches, "original_text")?;
        }
        if !crate::features::is_enabled(self, crate::features::Feature::SmartPaste) {
            return Ok(());
        }
        let detected = if include_classifiers {
            None
        } else {
            let classifiers = self.get_content_classifiers()?;
            Some(crate::content_classification::classify_with_classifiers(
                source,
                &classifiers,
            ))
        };
        let paste_matches = detected.as_deref().unwrap_or(matches);
        self.replace_paste_parts(
            clip_id,
            content_hash,
            source,
            &crate::smart_paste::parts::from_classifications(paste_matches),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::tests::setup_test_db;

    #[test]
    fn detection_is_independent_of_transformations_and_content_classification() {
        let db = setup_test_db();
        db.save_setting("enableTransformations", "false").unwrap();
        db.save_setting("enableContentClassification", "false")
            .unwrap();
        let clip = db
            .save_text_clip("Email: ada@example.com", "Tests")
            .unwrap();
        let parts = db.get_paste_parts(clip.id).unwrap().unwrap();
        assert!(!parts.parts.is_empty());
        assert!(db.get_analysis_classifications(clip.id).unwrap().is_empty());
        let context = crate::smart_paste::SmartPasteContext {
            application: "Tests".into(),
            label: Some("Email".into()),
            ..Default::default()
        };
        let selected =
            crate::smart_paste::select_value(&db, "Email: ada@example.com", &context).unwrap();
        assert_eq!(selected.value, "ada@example.com");
    }

    #[test]
    fn disabling_smart_paste_stops_new_detection_without_deleting_history() {
        let db = setup_test_db();
        let old = db
            .save_text_clip("Email: old@example.com", "Tests")
            .unwrap();
        assert!(db.get_paste_parts(old.id).unwrap().is_some());
        db.save_setting("enableSmartPaste", "false").unwrap();
        let new = db
            .save_text_clip("Email: new@example.com", "Tests")
            .unwrap();
        assert!(db.get_paste_parts(new.id).unwrap().is_none());
        assert!(db.get_paste_parts(old.id).unwrap().is_some());
        let context = crate::smart_paste::SmartPasteContext {
            application: "Tests".into(),
            label: Some("Email".into()),
            ..Default::default()
        };
        let blocked =
            crate::smart_paste::select_value(&db, "Email: old@example.com", &context).unwrap_err();
        assert_eq!(blocked.code, "feature_disabled");
    }
}
