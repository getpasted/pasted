use super::*;

pub const DEFAULT_SEARCH_HISTORY_LIMIT: usize = 100;
pub const DEFAULT_SEARCH_HISTORY_AGE_DAYS: i64 = 0;
pub const MAX_SEARCH_HISTORY_LIMIT: usize = 10_000;
pub const MAX_SEARCH_HISTORY_AGE_DAYS: i64 = 36_500;
pub const MAX_SEARCH_HISTORY_PAGE_SIZE: usize = 500;
const SEARCH_HISTORY_LIMIT_SETTING: &str = "searchHistoryLimit";
const SEARCH_HISTORY_AGE_SETTING: &str = "searchHistoryAgeDays";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHistoryEntry {
    pub id: i64,
    pub request: ClipSearchRequest,
    pub result_count: usize,
    pub use_count: usize,
    pub last_used_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHistoryPage {
    pub items: Vec<SearchHistoryEntry>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

fn invalid_input(message: impl Into<String>) -> rusqlite::Error {
    rusqlite::Error::InvalidParameterName(message.into())
}

fn canonical_search_request(request: &ClipSearchRequest) -> ClipSearchRequest {
    let mut canonical = request.clone();
    canonical.query = canonical.query.trim().to_string();
    canonical.clip_ids.sort_unstable();
    canonical.clip_ids.dedup();
    let canonicalize_filters = |values: &mut Vec<String>| {
        for value in values.iter_mut() {
            *value = value.trim().to_lowercase();
        }
        values.sort();
        values.dedup();
    };
    canonicalize_filters(&mut canonical.clip_types);
    canonicalize_filters(&mut canonical.content_types);
    canonicalize_filters(&mut canonical.file_formats);
    canonicalize_filters(&mut canonical.sources);
    canonical.limit = 0;
    canonical.offset = 0;
    canonical
}

fn configured_retention_policy(conn: &Connection) -> Result<(usize, i64)> {
    let configured = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [SEARCH_HISTORY_LIMIT_SETTING],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let limit = match configured {
        None => DEFAULT_SEARCH_HISTORY_LIMIT,
        Some(value) => value.trim().parse::<usize>().map_err(|_| {
            invalid_input(format!(
                "Search history limit must be between 0 and {MAX_SEARCH_HISTORY_LIMIT}"
            ))
        })?,
    };
    let age_days = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [SEARCH_HISTORY_AGE_SETTING],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .map(|value| {
            value.trim().parse::<i64>().map_err(|_| {
                invalid_input(format!(
                    "Search history age must be between 0 and {MAX_SEARCH_HISTORY_AGE_DAYS} days"
                ))
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_SEARCH_HISTORY_AGE_DAYS);
    validate_retention_policy(limit, age_days)?;
    Ok((limit, age_days))
}

fn validate_retention_policy(keep_count: usize, keep_age_days: i64) -> Result<()> {
    if keep_count > MAX_SEARCH_HISTORY_LIMIT {
        return Err(invalid_input(format!(
            "Search history limit must be between 0 and {MAX_SEARCH_HISTORY_LIMIT}"
        )));
    }
    if !(0..=MAX_SEARCH_HISTORY_AGE_DAYS).contains(&keep_age_days) {
        return Err(invalid_input(format!(
            "Search history age must be between 0 and {MAX_SEARCH_HISTORY_AGE_DAYS} days"
        )));
    }
    Ok(())
}

pub(super) fn prune_search_history(
    conn: &Connection,
    keep_count: usize,
    keep_age_days: i64,
    reference_time: chrono::DateTime<chrono::Utc>,
) -> Result<usize> {
    validate_retention_policy(keep_count, keep_age_days)?;
    let mut deleted = 0;
    if keep_age_days > 0 {
        let cutoff = (reference_time - chrono::Duration::days(keep_age_days))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        deleted += conn.execute(
            "DELETE FROM search_history WHERE julianday(last_used_at) < julianday(?1)",
            [cutoff],
        )?;
    }
    if keep_count > 0 {
        deleted += conn.execute(
            "DELETE FROM search_history
             WHERE id IN (
                SELECT id FROM search_history
                ORDER BY last_used_at DESC, id DESC
                LIMIT -1 OFFSET ?1
             )",
            [i64::try_from(keep_count)
                .map_err(|_| invalid_input("Search history limit is invalid"))?],
        )?;
    }
    Ok(deleted)
}

fn history_entry_from_row(row: &Row<'_>) -> Result<SearchHistoryEntry> {
    let request_json = row.get::<_, String>(1)?;
    let request = serde_json::from_str(&request_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let result_count = usize::try_from(row.get::<_, i64>(2)?)
        .map_err(|_| invalid_input("Search history result count is invalid"))?;
    let use_count = usize::try_from(row.get::<_, i64>(3)?)
        .map_err(|_| invalid_input("Search history use count is invalid"))?;
    Ok(SearchHistoryEntry {
        id: row.get(0)?,
        request,
        result_count,
        use_count,
        last_used_at: row.get(4)?,
    })
}

impl DbState {
    pub fn record_search_history(
        &self,
        request: &ClipSearchRequest,
        result_count: usize,
    ) -> Result<SearchHistoryEntry> {
        super::clip_search::validate_search_request(request)?;
        let result_count = i64::try_from(result_count)
            .map_err(|_| invalid_input("Search result count exceeds its safety limit"))?;
        let canonical = canonical_search_request(request);
        if canonical.query.is_empty()
            && canonical.clip_ids.is_empty()
            && canonical.clip_types.is_empty()
            && canonical.content_types.is_empty()
            && canonical.file_formats.is_empty()
            && canonical.sources.is_empty()
            && !canonical.trash
        {
            return Err(invalid_input("Search history requires a query or filter"));
        }
        let canonical_json = serde_json::to_string(&canonical)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let request_json = canonical_json.clone();
        let reference_time = chrono::Utc::now();
        let now = reference_time.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let (keep_count, keep_age_days) = configured_retention_policy(&tx)?;
        tx.execute(
            "INSERT INTO search_history
                (canonical_request_json, request_json, result_count, use_count, created_at, last_used_at)
             VALUES (?1, ?2, ?3, 1, ?4, ?4)
             ON CONFLICT(canonical_request_json) DO UPDATE SET
                request_json = excluded.request_json,
                result_count = excluded.result_count,
                use_count = search_history.use_count + 1,
                last_used_at = excluded.last_used_at",
            params![canonical_json, request_json, result_count, now],
        )?;
        let id = tx.query_row(
            "SELECT id FROM search_history WHERE canonical_request_json = ?1",
            [&canonical_json],
            |row| row.get::<_, i64>(0),
        )?;
        prune_search_history(&tx, keep_count, keep_age_days, reference_time)?;
        let entry = tx.query_row(
            "SELECT id, request_json, result_count, use_count, last_used_at
             FROM search_history WHERE id = ?1",
            [id],
            history_entry_from_row,
        )?;
        tx.commit()?;
        Ok(entry)
    }

    pub fn list_search_history(&self, limit: usize, offset: usize) -> Result<SearchHistoryPage> {
        if limit == 0 || limit > MAX_SEARCH_HISTORY_PAGE_SIZE {
            return Err(invalid_input(format!(
                "Search history page size must be between 1 and {MAX_SEARCH_HISTORY_PAGE_SIZE}"
            )));
        }
        if offset > MAX_CLIP_SEARCH_OFFSET {
            return Err(invalid_input(
                "Search history offset exceeds its safety limit",
            ));
        }
        let limit_i64 = i64::try_from(limit)
            .map_err(|_| invalid_input("Search history page size is invalid"))?;
        let offset_i64 =
            i64::try_from(offset).map_err(|_| invalid_input("Search history offset is invalid"))?;
        let conn = self.conn.lock();
        let total_count = usize::try_from(conn.query_row(
            "SELECT COUNT(*) FROM search_history",
            [],
            |row| row.get::<_, i64>(0),
        )?)
        .map_err(|_| invalid_input("Search history count is invalid"))?;
        let mut statement = conn.prepare(
            "SELECT id, request_json, result_count, use_count, last_used_at
             FROM search_history
             ORDER BY last_used_at DESC, id DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = statement.query_map(params![limit_i64, offset_i64], history_entry_from_row)?;
        let items = rows.collect::<Result<Vec<_>>>()?;
        Ok(SearchHistoryPage {
            items,
            total_count,
            limit,
            offset,
        })
    }

    pub fn delete_search_history(&self, id: i64) -> Result<bool> {
        if id <= 0 {
            return Err(invalid_input("Search history ID must be positive"));
        }
        let conn = self.conn.lock();
        Ok(conn.execute("DELETE FROM search_history WHERE id = ?1", [id])? > 0)
    }

    pub fn clear_search_history(&self) -> Result<usize> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM search_history", [])
    }

    pub fn configure_search_history_retention(
        &self,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<usize> {
        let keep_count = usize::try_from(keep_count)
            .map_err(|_| invalid_input("Search history limit must not be negative"))?;
        validate_retention_policy(keep_count, keep_age_days)?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SEARCH_HISTORY_LIMIT_SETTING, keep_count.to_string()],
        )?;
        tx.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SEARCH_HISTORY_AGE_SETTING, keep_age_days.to_string()],
        )?;
        let deleted = prune_search_history(&tx, keep_count, keep_age_days, chrono::Utc::now())?;
        tx.commit()?;
        Ok(deleted)
    }

    pub fn enforce_search_history_retention(
        &self,
        keep_count: i64,
        keep_age_days: i64,
    ) -> Result<usize> {
        let keep_count = usize::try_from(keep_count)
            .map_err(|_| invalid_input("Search history limit must not be negative"))?;
        validate_retention_policy(keep_count, keep_age_days)?;
        let conn = self.conn.lock();
        prune_search_history(&conn, keep_count, keep_age_days, chrono::Utc::now())
    }
}
