use rusqlite::{params, Connection, Result};

use super::{
    AnalyticsSummary, ClipTypeStat, DailyStat, DbState, FileFormatStat, SourceStat, TypeStat,
};

pub(super) const MAX_ANALYTICS_FILE_FORMATS: usize = 24;

impl DbState {
    pub fn get_analytics_summary(&self) -> Result<AnalyticsSummary> {
        let conn = self.conn.lock();

        let (total_clips, total_chars): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(LENGTH(text_content)), 0) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap_or((0, 0));

        let mut source_statement = conn.prepare(
            "SELECT source, COUNT(*) FROM clips WHERE (is_trashed IS NULL OR is_trashed = 0) GROUP BY source ORDER BY COUNT(*) DESC, source COLLATE NOCASE ASC LIMIT 8"
        )?;
        let top_sources = source_statement
            .query_map([], |row| {
                Ok(SourceStat {
                    name: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();

        let mut clip_type_statement = conn.prepare(
            "SELECT CASE WHEN content_type = 'image' THEN 'image' WHEN content_type = 'file' THEN 'file' ELSE 'text' END AS clip_type,
                    COUNT(*)
             FROM clips
             WHERE (is_trashed IS NULL OR is_trashed = 0)
             GROUP BY clip_type
             ORDER BY CASE
               WHEN content_type = 'image' THEN 1
               WHEN content_type = 'file' THEN 2
               ELSE 0
             END"
        )?;
        let clip_types = clip_type_statement
            .query_map([], |row| {
                Ok(ClipTypeStat {
                    clip_type: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();

        let mut file_format_statement = conn.prepare(
            "SELECT LOWER(json_extract(detected.value, '$.format')) AS file_format,
                    COUNT(DISTINCT results.clip_id)
             FROM clip_analysis_results AS results
             JOIN clips ON clips.id = results.clip_id,
                  json_each(results.result_json, '$.formats') AS detected
             WHERE results.participant_ref = ?1
               AND results.content_hash = clips.content_hash
               AND results.input_hash = clips.content_hash
               AND results.format_version = ?2
               AND COALESCE(clips.is_trashed, 0) = 0
             GROUP BY file_format
             ORDER BY COUNT(DISTINCT results.clip_id) DESC, file_format COLLATE NOCASE ASC
             LIMIT ?3",
        )?;
        let file_formats = file_format_statement
            .query_map(
                params![
                    crate::content_inspection::FILE_FORMAT_INSPECTOR_REF,
                    crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                    MAX_ANALYTICS_FILE_FORMATS as i64,
                ],
                |row| {
                    Ok(FileFormatStat {
                        file_format: row.get(0)?,
                        count: row.get(1)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>>>()?;

        let mut content_type_statement = conn.prepare(
            "SELECT content_type, COUNT(DISTINCT clip_id)
             FROM (
                SELECT classifications.clip_id, classifications.content_type
                FROM clip_analysis_classifications AS classifications
                JOIN clips ON clips.id = classifications.clip_id
                WHERE classifications.input_hash = clips.content_hash
                  AND (clips.is_trashed IS NULL OR clips.is_trashed = 0)
                UNION
                SELECT id AS clip_id, content_type
                FROM clips
                WHERE (is_trashed IS NULL OR is_trashed = 0)
                  AND content_type NOT IN ('text', 'image', 'file')
             )
             GROUP BY content_type
             ORDER BY COUNT(DISTINCT clip_id) DESC, content_type COLLATE NOCASE ASC",
        )?;
        let content_types = content_type_statement
            .query_map([], |row| {
                Ok(TypeStat {
                    content_type: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|row| row.ok())
            .collect();

        let reference_time = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let daily_activity =
            Self::get_daily_activity_for_calendar(&conn, &reference_time, "localtime")?;

        Ok(AnalyticsSummary {
            total_clips,
            total_chars,
            top_sources,
            clip_types,
            file_formats,
            content_types,
            daily_activity,
        })
    }

    pub(super) fn get_daily_activity_for_calendar(
        conn: &Connection,
        reference_time: &str,
        calendar_modifier: &str,
    ) -> Result<Vec<DailyStat>> {
        let mut daily_statement = conn.prepare(
            "WITH RECURSIVE recent_days(day) AS (
                SELECT date(?1, ?2, '-13 days')
                UNION ALL
                SELECT date(day, '+1 day')
                FROM recent_days
                WHERE day < date(?1, ?2)
             )
             SELECT recent_days.day, COUNT(clips.id)
             FROM recent_days
             LEFT JOIN clips
               ON date(clips.created_at, ?2) = recent_days.day
              AND (clips.is_trashed IS NULL OR clips.is_trashed = 0)
             GROUP BY recent_days.day
             ORDER BY recent_days.day DESC",
        )?;
        let daily_activity = daily_statement
            .query_map(params![reference_time, calendar_modifier], |row| {
                Ok(DailyStat {
                    date: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .filter_map(|row| row.ok())
            .collect::<Vec<_>>();
        Ok(daily_activity)
    }
}
