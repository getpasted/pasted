use rusqlite::Result;

use super::{ClipItem, ClipVersion, DbState};

impl DbState {
    #[cfg(test)]
    pub fn get_clip_versions(&self, clip_id: i64) -> Result<Vec<ClipVersion>> {
        self.get_clip_versions_page(clip_id, -1, 0)
    }

    pub fn get_clip_versions_page(
        &self,
        clip_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ClipVersion>> {
        self.get_clip_versions_page_internal(clip_id, limit, offset)
    }

    pub fn restore_clip_version(&self, clip_id: i64, version_id: i64) -> Result<ClipItem> {
        self.restore_clip_version_internal(clip_id, version_id)
    }

    pub fn delete_clip_version(&self, clip_id: i64, version_id: i64) -> Result<()> {
        self.delete_clip_version_internal(clip_id, version_id)
    }
}
