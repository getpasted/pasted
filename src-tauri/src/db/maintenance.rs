use super::*;

impl DbState {
    pub fn clear_history(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND clips.id NOT IN (SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1)",
            [],
        )?;
        Ok(())
    }

    pub fn rescan_file_formats(&self) -> Result<FileFormatRescanReport> {
        let clips = {
            let conn = self.conn.lock();
            let mut statement = conn.prepare(
                "SELECT id, content_hash, text_content
                 FROM clips
                 WHERE content_type = 'file' AND COALESCE(is_trashed, 0) = 0
                 ORDER BY id ASC",
            )?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        let mut changed_count = 0usize;
        let mut missing_count = 0usize;
        let mut failed_count = 0usize;
        for (clip_id, content_hash, payload) in &clips {
            let paths = payload
                .as_deref()
                .map(crate::content_inspection::parse_file_paths)
                .unwrap_or_default();
            if paths.is_empty() || !crate::resource_limits::file_list_within_limit(&paths) {
                failed_count += 1;
                continue;
            }
            let inspection = crate::content_inspection::inspect_file_formats(&paths);
            if inspection.unavailable_count == paths.len() {
                missing_count += 1;
                continue;
            }
            let existing = self.get_file_format_inspection(*clip_id, content_hash)?;
            if existing.as_ref() != Some(&inspection)
                && self.record_file_format_inspection(*clip_id, content_hash, &inspection)?
            {
                changed_count += 1;
            }
        }
        let report = FileFormatRescanReport {
            scanned_count: clips.len(),
            changed_count,
            unchanged_count: clips
                .len()
                .saturating_sub(changed_count)
                .saturating_sub(missing_count)
                .saturating_sub(failed_count),
            missing_count,
            failed_count,
        };
        let _ = self.log_activity(
            "file_format_history_rescanned",
            &format!(
                "Rescanned {} file clips; updated {}; missing {}; failed {}",
                report.scanned_count,
                report.changed_count,
                report.missing_count,
                report.failed_count
            ),
        );
        Ok(report)
    }
}
