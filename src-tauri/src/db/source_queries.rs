use super::*;

impl DbState {
    pub fn get_distinct_sources(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT source FROM clips WHERE source IS NOT NULL AND source != '' ORDER BY source ASC"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        let mut sources = Vec::new();
        for r in rows {
            sources.push(r?);
        }
        Ok(sources)
    }
}
