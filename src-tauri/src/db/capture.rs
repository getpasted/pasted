use super::*;

impl DbState {
    pub fn save_clip(
        &self,
        content_type: &str,
        text_content: Option<&str>,
        html_content: Option<&str>,
        image_base64: Option<&str>,
        content_hash: &str,
        source: &str,
    ) -> Result<ClipItem> {
        self.save_clip_with_structure(
            ClipSaveInput {
                content_type,
                text_content,
                html_content,
                image_base64,
                content_hash,
                source,
            },
            None,
        )
    }

    fn save_clip_with_structure(
        &self,
        input: ClipSaveInput<'_>,
        structure: Option<&crate::content_inspection::StructuralMetadata>,
    ) -> Result<ClipItem> {
        let ClipSaveInput {
            content_type,
            text_content,
            html_content,
            image_base64,
            content_hash,
            source,
        } = input;
        if let Some(text) = text_content {
            ensure_resource_size(
                text,
                crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                "Clip text",
            )?;
        }
        if let Some(html) = html_content {
            ensure_resource_size(
                html,
                crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                "Clip HTML",
            )?;
        }
        if let Some(image) = image_base64 {
            ensure_safe_raster_data_url(image, "Clip image")?;
        }
        let conn = self.conn.lock();

        let existing: Result<i64> = conn.query_row(
            "SELECT id FROM clips WHERE content_hash = ?1",
            params![content_hash],
            |r| r.get(0),
        );

        if let Ok(id) = existing {
            conn.execute(
                "UPDATE clips SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), is_trashed = 0, trashed_at = NULL WHERE id = ?1",
                params![id],
            )?;
            let clip = self.get_clip_by_id_internal(&conn, id)?;
            drop(conn);
            self.persist_capture_structure(&clip, structure);
            return Ok(clip);
        }

        let ocr_status = if content_type == "image" {
            "never"
        } else {
            "not_applicable"
        };
        let ocr_input_hash = (content_type == "image").then_some(content_hash);
        conn.execute(
            "INSERT INTO clips
                (content_type, text_content, html_content, image_base64, content_hash, source,
                 ocr_status, ocr_input_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
            params![
                content_type,
                text_content,
                html_content,
                image_base64,
                content_hash,
                source,
                ocr_status,
                ocr_input_hash
            ],
        )?;

        let id = conn.last_insert_rowid();
        let _ = self.enforce_history_limit_internal(&conn);
        let _ = self.enforce_trash_limit_internal(&conn);
        let clip = self.get_clip_by_id_internal(&conn, id)?;
        drop(conn);
        self.persist_capture_structure(&clip, structure);
        Ok(clip)
    }

    fn persist_capture_structure(
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

    pub fn save_text_clip(&self, text: &str, source: &str) -> Result<ClipItem> {
        let include_classifiers =
            crate::features::is_enabled(self, crate::features::Feature::ContentClassification);
        let analysis = crate::analysis_execution::analyze_text(
            self,
            text,
            Some(source),
            crate::analysis_execution::AnalyzerOptions {
                policy: crate::analysis_contract::AnalysisPolicy::Capture,
                include_extractor: false,
                include_classifiers,
                include_suggestions: false,
            },
        )
        .ok();
        let classification_matches = analysis
            .as_ref()
            .map(|result| result.analysis.result.classification_matches.clone())
            .unwrap_or_default();
        let structure = analysis
            .as_ref()
            .and_then(|result| result.analysis.result.structure.as_ref());
        let content_hash = crate::clipboard_fingerprint::text(text);
        let clip = self.save_clip_with_structure(
            ClipSaveInput {
                content_type: "text",
                text_content: Some(text),
                html_content: None,
                image_base64: None,
                content_hash: &content_hash,
                source,
            },
            structure,
        )?;
        if include_classifiers {
            self.replace_analysis_classifications(
                clip.id,
                &clip.content_hash,
                &classification_matches,
                "original_text",
            )?;
            return self.get_clip_by_id(clip.id);
        }
        Ok(clip)
    }

    pub(crate) fn merge_external_text_clips(
        &self,
        source_label: &str,
        clips: &[ExternalTextClip],
    ) -> Result<(usize, usize, Option<usize>)> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
        let active_count_before: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM clips WHERE COALESCE(is_trashed, 0) = 0",
            [],
            |row| row.get(0),
        )?;
        let current_capacity = transaction
            .query_row(
                "SELECT value FROM settings WHERE key = 'keepClipCount'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1000);
        let mut imported_count = 0usize;
        let mut duplicate_count = 0usize;

        for clip in clips {
            let created_at = clip
                .created_at
                .as_deref()
                .map(|value| canonical_utc_timestamp(value, "External history"))
                .transpose()?;
            let changed = transaction.execute(
                "INSERT OR IGNORE INTO clips
                    (content_type, text_content, content_hash, source, ocr_status, created_at)
                 VALUES ('text', ?1, ?2, ?3, 'not_applicable', COALESCE(?4, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')))",
                params![clip.text, clip.content_hash, clip.source, created_at],
            )?;
            if changed == 1 {
                imported_count += 1;
            } else {
                duplicate_count += 1;
            }
        }

        let required_capacity =
            (active_count_before.max(0) as usize).saturating_add(imported_count);
        let history_capacity_adjusted_to =
            if current_capacity > 0 && required_capacity > current_capacity {
                transaction.execute(
                    "INSERT INTO settings (key, value) VALUES ('keepClipCount', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    [required_capacity.to_string()],
                )?;
                Some(required_capacity)
            } else {
                None
            };

        self.log_activity_internal(
            &transaction,
            "external_history_imported",
            &format!(
                "Imported {imported_count} clips from {source_label}; skipped {duplicate_count} duplicates"
            ),
        )?;
        transaction.commit()?;
        Ok((
            imported_count,
            duplicate_count,
            history_capacity_adjusted_to,
        ))
    }

    pub fn reattribute_image_capture(
        &self,
        clip_id: i64,
        content_hash: &str,
        source: &str,
    ) -> Result<Option<ClipItem>> {
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE clips SET source = ?1
             WHERE id = ?2 AND content_hash = ?3 AND content_type = 'image'
               AND COALESCE(is_trashed, 0) = 0",
            params![source, clip_id, content_hash],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        self.get_clip_by_id_internal(&conn, clip_id).map(Some)
    }
}
