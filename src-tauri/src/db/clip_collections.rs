use rusqlite::{params, Result};
use serde::{Deserialize, Serialize};

use super::{ClipTypeStat, DbState, FileFormatStat, SourceStat, TypeStat};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipCollectionSummary {
    pub active_count: i64,
    pub trash_count: i64,
    pub pinned_count: i64,
    pub protected_count: i64,
    pub concealed_count: i64,
    pub noted_count: i64,
    pub clip_type_counts: Vec<ClipTypeStat>,
    pub file_format_counts: Vec<FileFormatStat>,
    pub type_counts: Vec<TypeStat>,
    pub source_counts: Vec<SourceStat>,
}

impl DbState {
    pub fn get_clip_collection_summary(&self) -> Result<ClipCollectionSummary> {
        let conn = self.conn.lock();
        let (active_count, trash_count, pinned_count, protected_count, concealed_count, noted_count) = conn.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN is_trashed = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 AND is_pinned = 1 THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 AND id IN (
                    SELECT clip_id FROM effective_clip_protection WHERE is_protected = 1
                ) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 AND id IN (
                    SELECT clip_id FROM effective_clip_concealment WHERE is_concealed = 1
                ) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN COALESCE(is_trashed, 0) = 0 AND TRIM(COALESCE(note, '')) != '' THEN 1 ELSE 0 END), 0)
             FROM clips",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )?;
        let type_counts = conn
            .prepare(
                "SELECT content_type, COUNT(DISTINCT clip_id)
                 FROM (
                    SELECT classifications.clip_id, classifications.content_type
                    FROM clip_analysis_classifications AS classifications
                    JOIN clips ON clips.id = classifications.clip_id
                    WHERE classifications.input_hash = clips.content_hash
                      AND COALESCE(clips.is_trashed, 0) = 0
                    UNION
                    SELECT id AS clip_id, content_type
                    FROM clips
                    WHERE COALESCE(is_trashed, 0) = 0
                      AND content_type NOT IN ('text', 'image', 'file')
                 )
                 GROUP BY content_type ORDER BY content_type",
            )?
            .query_map([], |row| {
                Ok(TypeStat {
                    content_type: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        let clip_type_counts = conn
            .prepare(
                "SELECT CASE
                    WHEN content_type IN ('image', 'file') THEN content_type
                    ELSE 'text'
                 END AS clip_type, COUNT(*)
                 FROM clips
                 WHERE COALESCE(is_trashed, 0) = 0
                 GROUP BY clip_type
                 ORDER BY CASE clip_type WHEN 'text' THEN 1 WHEN 'image' THEN 2 ELSE 3 END",
            )?
            .query_map([], |row| {
                Ok(ClipTypeStat {
                    clip_type: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        let file_format_counts = conn
            .prepare(
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
                 GROUP BY file_format ORDER BY file_format COLLATE NOCASE",
            )?
            .query_map(
                params![
                    crate::content_inspection::FILE_FORMAT_INSPECTOR_REF,
                    crate::analysis_contract::ANALYSIS_CONTRACT_VERSION,
                ],
                |row| {
                    Ok(FileFormatStat {
                        file_format: row.get(0)?,
                        count: row.get(1)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>>>()?;
        let source_counts = conn
            .prepare(
                "SELECT source, COUNT(*) FROM clips
                 WHERE COALESCE(is_trashed, 0) = 0
                 GROUP BY source ORDER BY COUNT(*) DESC, source",
            )?
            .query_map([], |row| {
                Ok(SourceStat {
                    name: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(ClipCollectionSummary {
            active_count,
            trash_count,
            pinned_count,
            protected_count,
            concealed_count,
            noted_count,
            clip_type_counts,
            file_format_counts,
            type_counts,
            source_counts,
        })
    }
}
