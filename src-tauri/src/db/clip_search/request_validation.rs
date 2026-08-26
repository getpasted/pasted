use super::super::*;

pub(super) fn validate(request: &ClipSearchRequest) -> Result<()> {
    if request.query.len() > MAX_CLIP_SEARCH_QUERY_BYTES {
        return Err(rusqlite::Error::InvalidParameterName(
            "Search query exceeds its safety limit".into(),
        ));
    }
    if request.offset > MAX_CLIP_SEARCH_OFFSET {
        return Err(rusqlite::Error::InvalidParameterName(
            "Search offset exceeds its safety limit".into(),
        ));
    }
    if request.limit > MAX_CLIP_SEARCH_PAGE_SIZE {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Search limit must not exceed {MAX_CLIP_SEARCH_PAGE_SIZE}"
        )));
    }
    if request.clip_ids.len() > MAX_CLIP_SEARCH_IDS || request.clip_ids.iter().any(|id| *id <= 0) {
        return Err(rusqlite::Error::InvalidParameterName(
            "Clip ID filters exceed their safety limit or contain an invalid ID".into(),
        ));
    }
    let requested_filter_count = request.clip_types.len()
        + request.content_types.len()
        + request.file_formats.len()
        + request.sources.len();
    if requested_filter_count > MAX_CLIP_SEARCH_FILTERS {
        return Err(rusqlite::Error::InvalidParameterName(
            "Search filters exceed their safety limit".into(),
        ));
    }
    let validate_filter =
        |value: &String| !value.trim().is_empty() && value.len() <= 256 && !value.contains('\0');
    if request
        .clip_types
        .iter()
        .chain(&request.content_types)
        .chain(&request.file_formats)
        .chain(&request.sources)
        .any(|value| !validate_filter(value))
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "Search filter is empty or exceeds its safety limit".into(),
        ));
    }
    Ok(())
}
