use super::*;

#[derive(Clone, Copy)]
pub(crate) struct SmartBinFeaturePolicy {
    pub(crate) clip_types: bool,
    pub(crate) content_types: bool,
    pub(crate) file_formats: bool,
    pub(crate) sources: bool,
}

pub(crate) fn load(conn: &Connection) -> Result<SmartBinFeaturePolicy> {
    conn.query_row(
        "SELECT
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableClipTypes' AND value IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableTypes' AND value IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableFileFormats' AND value IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableSources' AND value IN ('false', '0'))",
        [],
        |row| {
            Ok(SmartBinFeaturePolicy {
                clip_types: row.get(0)?,
                content_types: row.get(1)?,
                file_formats: row.get(2)?,
                sources: row.get(3)?,
            })
        },
    )
}
