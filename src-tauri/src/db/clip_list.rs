use rusqlite::{params, Result, ToSql};
use serde::{Deserialize, Serialize};

use super::{ClipItem, ClipSearchRequest, DbState, DEFAULT_CLIP_SEARCH_PAGE_SIZE};

pub const CLIP_LIST_SCHEMA_VERSION: u32 = 1;
pub const MAX_CLIP_LIST_PREVIEW_CHARS: usize = 1_024;
const MAX_CLIP_LIST_FILE_NAMES: usize = 20;
const MAX_CLIP_LIST_FILE_NAME_CHARS: usize = 255;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipListItem {
    pub id: i64,
    pub name: Option<String>,
    pub content_type: String,
    pub content_types: Vec<String>,
    pub file_formats: Vec<String>,
    pub preview_text: Option<String>,
    pub preview_truncated: bool,
    pub file_names: Vec<String>,
    pub file_count: usize,
    pub content_hash: String,
    pub source: String,
    pub is_pinned: bool,
    pub is_protected: bool,
    pub is_explicitly_protected: Option<bool>,
    pub protecting_bin_ids: Vec<i64>,
    pub is_concealed: bool,
    pub is_explicitly_concealed: Option<bool>,
    pub is_explicitly_revealed: bool,
    pub concealing_bin_ids: Vec<i64>,
    pub concealing_content_types: Vec<String>,
    #[serde(rename = "hotkey")]
    pub shortcut: Option<String>,
    pub is_transformed: bool,
    pub pin_order: i32,
    pub bin_id: Option<i64>,
    pub bin_ids: Vec<i64>,
    pub note: Option<String>,
    pub is_trashed: bool,
    pub trashed_at: Option<String>,
    pub created_at: String,
    pub ocr_extractor_ref: Option<String>,
    pub ocr_extractor_name: Option<String>,
    pub ocr_engine_version: Option<String>,
}

fn bounded_preview(value: &str) -> (String, bool) {
    let mut characters = value.chars();
    let preview = characters
        .by_ref()
        .take(MAX_CLIP_LIST_PREVIEW_CHARS)
        .collect::<String>();
    (preview, characters.next().is_some())
}

fn file_names(value: &str) -> Vec<String> {
    let paths = serde_json::from_str::<Vec<String>>(value).unwrap_or_else(|_| {
        value
            .lines()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .collect()
    });
    paths
        .iter()
        .filter_map(|path| path.rsplit(['/', '\\']).find(|part| !part.is_empty()))
        .take(MAX_CLIP_LIST_FILE_NAMES)
        .map(|name| name.chars().take(MAX_CLIP_LIST_FILE_NAME_CHARS).collect())
        .collect()
}

fn note_preview(value: &str) -> String {
    let text = serde_json::from_str::<serde_json::Value>(value)
        .ok()
        .and_then(|parsed| parsed.as_array().cloned())
        .map(|notes| {
            notes
                .iter()
                .filter_map(|note| note.as_str().or_else(|| note.get("text")?.as_str()))
                .map(str::trim)
                .filter(|note| !note.is_empty())
                .collect::<Vec<_>>()
                .join(" • ")
        })
        .filter(|summary| !summary.is_empty())
        .unwrap_or_else(|| value.to_owned());
    bounded_preview(&text).0
}

impl From<ClipItem> for ClipListItem {
    fn from(clip: ClipItem) -> Self {
        let (names, file_count) = if clip.content_type == "file" {
            let paths = clip
                .text_content
                .as_deref()
                .map(|value| {
                    serde_json::from_str::<Vec<String>>(value).unwrap_or_else(|_| {
                        value
                            .lines()
                            .map(str::trim)
                            .filter(|path| !path.is_empty())
                            .map(str::to_owned)
                            .collect()
                    })
                })
                .unwrap_or_default();
            (
                file_names(&serde_json::to_string(&paths).unwrap_or_default()),
                paths.len(),
            )
        } else {
            (Vec::new(), 0)
        };
        let (preview_text, preview_truncated) = if clip.content_type == "file" {
            (None, false)
        } else {
            clip.text_content
                .as_deref()
                .map(bounded_preview)
                .map_or((None, false), |(preview, truncated)| {
                    (Some(preview), truncated)
                })
        };
        Self {
            id: clip.id,
            name: clip.name,
            content_type: clip.content_type,
            content_types: clip.content_types,
            file_formats: clip.file_formats,
            preview_text,
            preview_truncated,
            file_names: names,
            file_count,
            content_hash: clip.content_hash,
            source: clip.source,
            is_pinned: clip.is_pinned,
            is_protected: clip.is_protected,
            is_explicitly_protected: clip.is_explicitly_protected,
            protecting_bin_ids: clip.protecting_bin_ids,
            is_concealed: clip.is_concealed,
            is_explicitly_concealed: clip.is_explicitly_concealed,
            is_explicitly_revealed: clip.is_explicitly_revealed,
            concealing_bin_ids: clip.concealing_bin_ids,
            concealing_content_types: clip.concealing_content_types,
            shortcut: clip.shortcut,
            is_transformed: clip.is_transformed,
            pin_order: clip.pin_order,
            bin_id: clip.bin_id,
            bin_ids: clip.bin_ids.unwrap_or_default(),
            note: clip.note.map(|note| note_preview(&note)),
            is_trashed: clip.is_trashed,
            trashed_at: clip.trashed_at,
            created_at: clip.created_at,
            ocr_extractor_ref: clip.ocr_extractor_ref,
            ocr_extractor_name: clip.ocr_extractor_name,
            ocr_engine_version: clip.ocr_engine_version,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClipListPage {
    pub schema_version: u32,
    pub items: Vec<ClipListItem>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct ClipCollectionPageRequest {
    pub collection: String,
    pub bin_id: Option<i64>,
    pub value: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

fn normalized_page(request: &ClipCollectionPageRequest) -> Result<(usize, usize)> {
    let limit = if request.limit == 0 {
        DEFAULT_CLIP_SEARCH_PAGE_SIZE
    } else {
        request.limit
    };
    if limit > super::MAX_CLIP_SEARCH_PAGE_SIZE || request.offset > super::MAX_CLIP_SEARCH_OFFSET {
        return Err(rusqlite::Error::InvalidParameterName(
            "Collection page exceeds its safety limit".into(),
        ));
    }
    Ok((limit, request.offset))
}

fn search_request(request: &ClipCollectionPageRequest, limit: usize) -> Result<ClipSearchRequest> {
    let mut search = ClipSearchRequest {
        limit,
        offset: request.offset,
        ..ClipSearchRequest::default()
    };
    match request.collection.as_str() {
        "protected" => search.query = "is:protected".into(),
        "named" => search.query = "is:named".into(),
        "noted" => search.query = "has:note".into(),
        "clipType" => search.clip_types.push(required_value(request)?),
        "contentType" => search.content_types.push(required_value(request)?),
        "fileFormat" => search.file_formats.push(required_value(request)?),
        "source" => search.sources.push(required_value(request)?),
        _ => {
            return Err(rusqlite::Error::InvalidParameterName(
                "Unknown clip collection".into(),
            ))
        }
    }
    Ok(search)
}

fn required_value(request: &ClipCollectionPageRequest) -> Result<String> {
    request
        .value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| rusqlite::Error::InvalidParameterName("Collection value is required".into()))
}

impl DbState {
    pub fn search_clip_list(&self, request: &ClipSearchRequest) -> Result<ClipListPage> {
        let result = self.search_clips_bounded(request)?;
        Ok(ClipListPage {
            schema_version: CLIP_LIST_SCHEMA_VERSION,
            items: result.items.into_iter().map(ClipListItem::from).collect(),
            total_count: result.total_count,
            limit: result.limit,
            offset: result.offset,
        })
    }

    pub fn get_clip_collection_page(
        &self,
        request: &ClipCollectionPageRequest,
    ) -> Result<ClipListPage> {
        let (limit, offset) = normalized_page(request)?;
        let (clips, total_count) = match request.collection.as_str() {
            "history" => (
                self.get_clips_page(None, false, Some(limit as i64), Some(offset as i64))?,
                self.get_total_clip_count()? as usize,
            ),
            "trash" => (
                self.get_trashed_clips_page(Some(limit as i64), Some(offset as i64))?,
                self.get_trashed_clip_count()? as usize,
            ),
            "pinned" => {
                let summary = self.get_clip_collection_summary()?;
                (
                    self.get_clips_page(None, true, Some(limit as i64), Some(offset as i64))?,
                    summary.pinned_count.max(0) as usize,
                )
            }
            "bin" => {
                let bin_id = request.bin_id.ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName("Bin ID is required".into())
                })?;
                let bin = self.get_bin(bin_id)?;
                (
                    self.get_clips_page(
                        Some(bin_id),
                        false,
                        Some(limit as i64),
                        Some(offset as i64),
                    )?,
                    bin.clip_count.unwrap_or(0).max(0) as usize,
                )
            }
            "concealed" => self.get_concealed_clip_page(limit, offset)?,
            "clipType" | "contentType" | "fileFormat" | "source" => self
                .get_exact_facet_clip_page(
                    &request.collection,
                    &required_value(request)?,
                    limit,
                    offset,
                )?,
            _ => {
                let result = self.search_clips(&search_request(request, limit)?)?;
                (result.items, result.total_count)
            }
        };
        Ok(ClipListPage {
            schema_version: CLIP_LIST_SCHEMA_VERSION,
            items: clips.into_iter().map(ClipListItem::from).collect(),
            total_count,
            limit,
            offset,
        })
    }

    fn get_concealed_clip_page(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<ClipItem>, usize)> {
        let conn = self.conn.lock();
        let total_count = conn
            .query_row(
                "SELECT COUNT(*) FROM effective_clip_concealment AS concealment
             JOIN clips ON clips.id = concealment.clip_id
             WHERE concealment.is_concealed = 1 AND COALESCE(clips.is_trashed, 0) = 0",
                [],
                |row| row.get::<_, i64>(0),
            )?
            .max(0) as usize;
        let mut statement = conn.prepare(
            "SELECT concealment.clip_id FROM effective_clip_concealment AS concealment
             JOIN clips ON clips.id = concealment.clip_id
             WHERE concealment.is_concealed = 1 AND COALESCE(clips.is_trashed, 0) = 0
             ORDER BY clips.created_at DESC, clips.id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let ids = statement
            .query_map(params![limit as i64, offset as i64], |row| {
                row.get::<_, i64>(0)
            })?
            .collect::<Result<Vec<_>>>()?;
        drop(statement);
        let clips = Self::get_clip_list_items_by_ids_internal(&conn, &ids)?;
        Ok((clips, total_count))
    }

    fn get_exact_facet_clip_page(
        &self,
        collection: &str,
        value: &str,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<ClipItem>, usize)> {
        let (predicate, parameters): (&str, Vec<Box<dyn ToSql>>) = match collection {
            "clipType" => (
                "LOWER(clips.content_type) = LOWER(?1)",
                vec![Box::new(value.to_owned())],
            ),
            "contentType" => (
                "(LOWER(clips.content_type) = LOWER(?1) OR EXISTS(
                    SELECT 1 FROM clip_analysis_classifications AS classifications
                    WHERE classifications.clip_id = clips.id
                      AND classifications.input_hash = clips.content_hash
                      AND LOWER(classifications.content_type) = LOWER(?1)
                ))",
                vec![Box::new(value.to_owned())],
            ),
            "fileFormat" => (
                "EXISTS(
                    SELECT 1 FROM clip_analysis_results AS results,
                         json_each(results.result_json, '$.formats') AS detected
                    WHERE results.clip_id = clips.id
                      AND results.participant_ref = ?2
                      AND results.content_hash = clips.content_hash
                      AND results.input_hash = clips.content_hash
                      AND results.format_version = ?3
                      AND LOWER(json_extract(detected.value, '$.format')) = LOWER(?1)
                )",
                vec![
                    Box::new(value.to_owned()),
                    Box::new(crate::content_inspection::FILE_FORMAT_INSPECTOR_REF),
                    Box::new(crate::analysis_contract::ANALYSIS_CONTRACT_VERSION),
                ],
            ),
            "source" => (
                "LOWER(clips.source) = LOWER(?1)",
                vec![Box::new(value.to_owned())],
            ),
            _ => unreachable!("validated exact facet collection"),
        };
        let parameter_refs = parameters
            .iter()
            .map(|parameter| parameter.as_ref())
            .collect::<Vec<_>>();
        let conn = self.conn.lock();
        let where_clause = format!("COALESCE(clips.is_trashed, 0) = 0 AND {predicate}");
        let total_count = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM clips WHERE {where_clause}"),
                parameter_refs.as_slice(),
                |row| row.get::<_, i64>(0),
            )?
            .max(0) as usize;
        let mut page_parameters = parameters;
        page_parameters.push(Box::new(limit as i64));
        page_parameters.push(Box::new(offset as i64));
        let page_parameter_refs = page_parameters
            .iter()
            .map(|parameter| parameter.as_ref())
            .collect::<Vec<_>>();
        let mut statement = conn.prepare(&format!(
            "SELECT clips.id FROM clips WHERE {where_clause}
             ORDER BY clips.created_at DESC, clips.id DESC LIMIT ?{} OFFSET ?{}",
            page_parameters.len() - 1,
            page_parameters.len(),
        ))?;
        let ids = statement
            .query_map(page_parameter_refs.as_slice(), |row| row.get::<_, i64>(0))?
            .collect::<Result<Vec<_>>>()?;
        drop(statement);
        Ok((
            Self::get_clip_list_items_by_ids_internal(&conn, &ids)?,
            total_count,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_item_bounds_text_and_removes_file_paths() {
        let mut clip = ClipItem {
            id: 1,
            name: None,
            content_type: "text".into(),
            content_types: Vec::new(),
            file_formats: Vec::new(),
            text_content: Some("x".repeat(MAX_CLIP_LIST_PREVIEW_CHARS + 1)),
            html_content: Some("<b>secret</b>".into()),
            image_base64: Some("data:image/png;base64,secret".into()),
            image_path: Some("/private/image.png".into()),
            content_hash: "hash".into(),
            source: "Tests".into(),
            is_pinned: false,
            is_protected: false,
            is_explicitly_protected: Some(false),
            protecting_bin_ids: Vec::new(),
            is_concealed: false,
            is_explicitly_concealed: Some(false),
            is_explicitly_revealed: false,
            concealing_bin_ids: Vec::new(),
            concealing_content_types: Vec::new(),
            shortcut: None,
            is_transformed: false,
            pin_order: 0,
            bin_id: None,
            bin_ids: Some(Vec::new()),
            note: None,
            is_trashed: false,
            trashed_at: None,
            created_at: "2026-09-14T00:00:00Z".into(),
            ocr_extractor_ref: None,
            ocr_extractor_name: None,
            ocr_engine_version: None,
        };
        let summary = ClipListItem::from(clip.clone());
        assert_eq!(
            summary.preview_text.unwrap().chars().count(),
            MAX_CLIP_LIST_PREVIEW_CHARS
        );
        assert!(summary.preview_truncated);

        clip.content_type = "file".into();
        clip.text_content = Some(r#"["/private/one.txt","C:\\secret\\two.png"]"#.into());
        let summary = ClipListItem::from(clip);
        assert_eq!(summary.file_names, vec!["one.txt", "two.png"]);
        assert_eq!(summary.file_count, 2);
        assert_eq!(summary.preview_text, None);
    }

    #[test]
    fn collection_pages_report_totals_and_defer_full_content() {
        let db = super::super::tests::setup_test_db();
        for index in 0..3 {
            db.save_clip(
                "text",
                Some(&format!(
                    "clip {index} {}",
                    "x".repeat(MAX_CLIP_LIST_PREVIEW_CHARS)
                )),
                Some("<strong>full HTML</strong>"),
                None,
                &format!("list-page-{index}"),
                "Tests",
            )
            .unwrap();
        }
        db.save_clip(
            "text",
            Some("different source"),
            None,
            None,
            "list-page-other-source",
            "Tests Extended",
        )
        .unwrap();

        let page = db
            .get_clip_collection_page(&ClipCollectionPageRequest {
                collection: "history".into(),
                limit: 2,
                ..ClipCollectionPageRequest::default()
            })
            .unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.total_count, 4);
        assert!(page.items.iter().any(|item| item.preview_truncated));
        let encoded = serde_json::to_string(&page).unwrap();
        assert!(!encoded.contains("full HTML"));
        assert!(!encoded.contains("image_base64"));

        let detail_id = page
            .items
            .iter()
            .find(|item| item.preview_truncated)
            .unwrap()
            .id;
        let detail = db.get_clip_by_id(detail_id).unwrap();
        assert_eq!(
            detail.html_content.as_deref(),
            Some("<strong>full HTML</strong>")
        );
        assert!(detail.text_content.unwrap().chars().count() > MAX_CLIP_LIST_PREVIEW_CHARS);

        let source_page = db
            .get_clip_collection_page(&ClipCollectionPageRequest {
                collection: "source".into(),
                value: Some("Tests".into()),
                limit: 10,
                ..ClipCollectionPageRequest::default()
            })
            .unwrap();
        assert_eq!(source_page.total_count, 3);
        assert!(source_page.items.iter().all(|item| item.source == "Tests"));
    }

    #[test]
    #[ignore = "manual comparative measurement for clip-list page tuning"]
    fn benchmark_clip_list_page_sizes() {
        let db = super::super::tests::setup_test_db();
        for index in 0..1_000 {
            db.save_clip(
                "text",
                Some(&format!("clip {index} {}", "x".repeat(4_096))),
                None,
                None,
                &format!("list-benchmark-{index}"),
                "Benchmark",
            )
            .unwrap();
        }
        for limit in [50, 75, 100, 250] {
            let first_page = db
                .get_clip_collection_page(&ClipCollectionPageRequest {
                    collection: "history".into(),
                    limit,
                    ..ClipCollectionPageRequest::default()
                })
                .unwrap();
            let first_page_bytes = serde_json::to_vec(&first_page).unwrap().len();
            let first_page_started = std::time::Instant::now();
            for _ in 0..20 {
                db.get_clip_collection_page(&ClipCollectionPageRequest {
                    collection: "history".into(),
                    limit,
                    ..ClipCollectionPageRequest::default()
                })
                .unwrap();
            }
            let average_first_page = first_page_started.elapsed() / 20;
            let drain_started = std::time::Instant::now();
            let mut encoded_bytes = 0;
            for offset in (0..1_000).step_by(limit) {
                let page = db
                    .get_clip_collection_page(&ClipCollectionPageRequest {
                        collection: "history".into(),
                        limit,
                        offset,
                        ..ClipCollectionPageRequest::default()
                    })
                    .unwrap();
                encoded_bytes += serde_json::to_vec(&page).unwrap().len();
            }
            eprintln!(
                "clip-list page size {limit}: {average_first_page:?} average first page, \
                 {first_page_bytes} first-page bytes; {:?} to drain 1,000 rows, \
                 {encoded_bytes} total encoded bytes",
                drain_started.elapsed(),
            );
        }
    }
}
