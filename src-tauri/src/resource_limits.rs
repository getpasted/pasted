//! Central resource ceilings for data that can originate outside Pasted.
//!
//! These are deliberately generous enough for normal clipboard-manager use while
//! preventing a single clipboard item, import, or provider process from consuming
//! unbounded memory, disk, or execution time.

pub const DEFAULT_CLIP_CAPTURE_BYTES: usize = 100 * 1024 * 1024;
pub const MAX_CLIP_TEXT_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_CLIP_NOTE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_FILE_LIST_ITEMS: usize = 1_024;
pub const MAX_FILE_LIST_METADATA_BYTES: usize = 1024 * 1024;
pub const MAX_FILE_PREVIEW_INPUT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_FILE_PREVIEW_COUNT: usize = 8;
pub const MAX_FILE_PREVIEW_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MEDIA_PROBE_FILES: usize = 8;
pub const MAX_MEDIA_PROBE_OUTPUT_BYTES: u64 = 256 * 1024;
pub const MAX_TRANSCRIPTION_AUDIO_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SEARCHABLE_TEXT_QUERY_TERMS: usize = 8;
pub const MAX_SEARCHABLE_TEXT_MATCHES: i64 = 10_000;
pub const MAX_TEXT_FILE_PREVIEW_BYTES: usize = 64 * 1024;
pub const MAX_FILE_PREVIEW_CACHE_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_FILE_PREVIEW_CACHE_ITEMS: usize = 256;
pub const MAX_OCR_TEXT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMAGE_PIXELS: u64 = 24_000_000;
pub const MAX_ENCODED_IMAGE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_STORED_IMAGE_BASE64_BYTES: usize = 192 * 1024 * 1024;
pub const MAX_BACKUP_IMPORT_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_LIBRARY_ARCHIVE_ROWS: usize = 500_000;
pub const MAX_ACTIVITY_IMPORT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_ACTIVITY_IMPORT_ROWS: usize = 100_000;
pub const MAX_ACTIVITY_EVENT_TYPE_BYTES: usize = 128;
pub const MAX_ACTIVITY_DESCRIPTION_BYTES: usize = 16 * 1024;
pub const MAX_ACTIVITY_ATTRIBUTES_BYTES: usize = 64 * 1024;
pub const MAX_EXTERNAL_IMPORT_DATABASE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_EXTERNAL_IMPORT_ROWS: usize = 100_000;
pub const MAX_EXTERNAL_IMPORT_TEXT_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_TRANSFORM_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PROVIDER_PROMPT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_PROVIDER_RESULT_BYTES: u64 = 1024 * 1024;
pub const MAX_PROVIDER_WORKSPACE_BYTES: u64 = 8 * 1024 * 1024;
pub const PROVIDER_EXECUTION_TIMEOUT_SECS: u64 = 90;

#[cfg(test)]
pub const TEST_PNG_DATA_URL: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Wl2nQAAAABJRU5ErkJggg==";

pub fn configured_clip_capture_bytes(value: Option<&str>) -> usize {
    value
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|megabytes| (megabytes * 1024.0 * 1024.0) as usize)
        .unwrap_or(DEFAULT_CLIP_CAPTURE_BYTES)
        .clamp(1024 * 1024, MAX_CLIP_TEXT_BYTES)
}

pub fn image_dimensions_within_limit(width: u32, height: u32) -> bool {
    u64::from(width)
        .checked_mul(u64::from(height))
        .is_some_and(|pixels| pixels <= MAX_IMAGE_PIXELS)
}

pub fn validate_raster_data_url(value: &str) -> Result<(), String> {
    use base64::Engine;
    use image::ImageFormat;
    use std::io::Cursor;

    if value.len() > MAX_STORED_IMAGE_BASE64_BYTES {
        return Err("Raster image exceeds the stored-image safety limit".to_string());
    }
    let (header, payload) = value
        .split_once(',')
        .ok_or_else(|| "Raster image must be a Base64 data URL".to_string())?;
    let declared_format = match header {
        "data:image/png;base64" => ImageFormat::Png,
        "data:image/jpeg;base64" | "data:image/jpg;base64" => ImageFormat::Jpeg,
        "data:image/webp;base64" => ImageFormat::WebP,
        _ => {
            return Err("Raster image must use the PNG, JPEG, or WebP data URL format".to_string());
        }
    };
    if payload.is_empty() || payload.len() > MAX_STORED_IMAGE_BASE64_BYTES {
        return Err("Raster image payload is empty or oversized".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| "Raster image payload is not valid Base64".to_string())?;
    if bytes.is_empty() || bytes.len() > MAX_ENCODED_IMAGE_BYTES {
        return Err("Decoded raster image is empty or oversized".to_string());
    }
    let reader = image::ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|_| "Raster image format could not be determined".to_string())?;
    if reader.format() != Some(declared_format) {
        return Err("Raster image content does not match its declared format".to_string());
    }
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| "Raster image dimensions could not be read".to_string())?;
    if !image_dimensions_within_limit(width, height) {
        return Err("Raster image dimensions exceed the safety limit".to_string());
    }
    Ok(())
}

pub fn file_list_within_limit(paths: &[String]) -> bool {
    paths.len() <= MAX_FILE_LIST_ITEMS
        && paths
            .iter()
            .try_fold(0usize, |total, path| total.checked_add(path.len()))
            .is_some_and(|total| total <= MAX_FILE_LIST_METADATA_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_limit_rejects_oversized_and_overflowing_dimensions() {
        assert!(image_dimensions_within_limit(6_000, 4_000));
        assert!(!image_dimensions_within_limit(6_001, 4_000));
        assert!(!image_dimensions_within_limit(u32::MAX, u32::MAX));
    }

    #[test]
    fn raster_data_urls_require_bounded_matching_raster_content() {
        use base64::Engine;
        use std::io::Cursor;

        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let valid = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.into_inner())
        );
        assert!(validate_raster_data_url(&valid).is_ok());

        let svg = base64::engine::general_purpose::STANDARD
            .encode(br#"<svg onload="alert(1)" xmlns="http://www.w3.org/2000/svg"/>"#);
        assert!(validate_raster_data_url(&format!("data:image/svg+xml;base64,{svg}")).is_err());
        assert!(validate_raster_data_url(&format!("data:image/png;base64,{svg}")).is_err());
        assert!(validate_raster_data_url("https://example.invalid/image.png").is_err());
    }

    #[test]
    fn configured_capture_limit_is_bounded_and_has_a_safe_default() {
        assert_eq!(
            configured_clip_capture_bytes(None),
            DEFAULT_CLIP_CAPTURE_BYTES
        );
        assert_eq!(
            configured_clip_capture_bytes(Some("0")),
            DEFAULT_CLIP_CAPTURE_BYTES
        );
        assert_eq!(configured_clip_capture_bytes(Some("1")), 1024 * 1024);
        assert_eq!(
            configured_clip_capture_bytes(Some("9999")),
            MAX_CLIP_TEXT_BYTES
        );
    }

    #[test]
    fn file_lists_have_bounded_item_and_metadata_counts() {
        assert!(file_list_within_limit(&["/tmp/example.txt".to_string()]));
        assert!(!file_list_within_limit(&vec![
            "/tmp/file".to_string();
            MAX_FILE_LIST_ITEMS + 1
        ]));
        assert!(!file_list_within_limit(&[
            "x".repeat(MAX_FILE_LIST_METADATA_BYTES + 1)
        ]));
    }
}
