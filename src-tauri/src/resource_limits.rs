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
pub const MAX_TEXT_FILE_PREVIEW_BYTES: usize = 64 * 1024;
pub const MAX_FILE_PREVIEW_CACHE_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_FILE_PREVIEW_CACHE_ITEMS: usize = 256;
pub const MAX_OCR_TEXT_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMAGE_PIXELS: u64 = 24_000_000;
pub const MAX_ENCODED_IMAGE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_STORED_IMAGE_BASE64_BYTES: usize = 192 * 1024 * 1024;
pub const MAX_BACKUP_IMPORT_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_TRANSFORM_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PROVIDER_PROMPT_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_PROVIDER_RESULT_BYTES: u64 = 1024 * 1024;
pub const MAX_PROVIDER_WORKSPACE_BYTES: u64 = 8 * 1024 * 1024;
pub const PROVIDER_EXECUTION_TIMEOUT_SECS: u64 = 90;

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
