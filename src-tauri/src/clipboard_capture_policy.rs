use std::path::Path;
use std::time::{Duration, Instant};

use crate::db::DbState;

pub(crate) const FILE_IMAGE_STABILIZATION_ATTEMPTS: usize = 20;
pub(crate) const FILE_IMAGE_STABILIZATION_INTERVAL: Duration = Duration::from_millis(100);

const COMPOSITE_CAPTURE_WINDOW: Duration = Duration::from_secs(2);

pub(crate) struct RecentImageCapture {
    pub(crate) clip_id: i64,
    pub(crate) content_hash: String,
    captured_at: Instant,
}

impl RecentImageCapture {
    pub(crate) fn new(clip_id: i64, content_hash: String) -> Self {
        Self {
            clip_id,
            content_hash,
            captured_at: Instant::now(),
        }
    }

    pub(crate) fn is_current(&self) -> bool {
        self.captured_at.elapsed() <= COMPOSITE_CAPTURE_WINDOW
    }
}

pub(crate) fn configured_capture_bytes(db: &DbState) -> usize {
    let configured = db.get_setting("maxClipSizeMb").ok().flatten();
    capture_bytes_for_setting(configured.as_deref())
}

fn capture_bytes_for_setting(configured: Option<&str>) -> usize {
    crate::resource_limits::configured_clip_capture_bytes(configured)
}

pub(crate) fn is_image_file_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "avif"
                    | "bmp"
                    | "gif"
                    | "heic"
                    | "heif"
                    | "ico"
                    | "jpeg"
                    | "jpg"
                    | "png"
                    | "tif"
                    | "tiff"
                    | "webp"
            )
        })
}

fn is_file_manager_source(source: Option<&str>) -> bool {
    source.is_some_and(|source| {
        matches!(
            source.trim().to_ascii_lowercase().as_str(),
            "finder"
                | "file explorer"
                | "windows explorer"
                | "explorer"
                | "files"
                | "nautilus"
                | "dolphin"
                | "thunar"
                | "nemo"
                | "caja"
                | "pcmanfm"
        )
    })
}

pub(crate) fn inferred_screenshot_source(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    if name.contains("cleanshot") {
        return Some("CleanShot X");
    }
    (name.starts_with("screenshot ") || name.starts_with("screen shot ")).then_some("Screenshot")
}

pub(crate) fn resolved_capture_source<'a>(
    active_app: Option<&'a str>,
    inferred_source: Option<&'a str>,
) -> Option<&'a str> {
    if inferred_source == Some("CleanShot X") {
        return inferred_source;
    }
    if is_file_manager_source(active_app) {
        active_app
    } else {
        inferred_source.or(active_app)
    }
}

pub(crate) fn composite_image_source(inferred_source: Option<&str>) -> &str {
    inferred_source.unwrap_or("Screenshot")
}

/// Resolve the common composite clipboard payload where one image file is
/// accompanied by bitmap bytes. Ordinary file-manager copies retain their file
/// identity; high-confidence screenshot and ambiguous producers prefer the bitmap
/// so previews, image paste, and OCR continue to work.
pub(crate) fn prefer_bitmap_for_image_file(
    bitmap_available: bool,
    active_app: Option<&str>,
    inferred_source: Option<&str>,
) -> bool {
    bitmap_available
        && (!is_file_manager_source(active_app) || inferred_source == Some("CleanShot X"))
}

pub(crate) fn is_pasted_source(source: Option<&str>) -> bool {
    source.is_some_and(|source| {
        matches!(
            source.trim().to_ascii_lowercase().as_str(),
            "pasted" | "pasted-app"
        )
    })
}

pub(crate) fn should_prefer_composite_image(
    image_file: bool,
    delayed_match: bool,
    bitmap_available: bool,
    active_app: Option<&str>,
    inferred_source: Option<&str>,
) -> bool {
    image_file
        && (delayed_match
            || prefer_bitmap_for_image_file(bitmap_available, active_app, inferred_source))
}

pub(crate) fn should_coalesce_recent_image(
    single_image_file: bool,
    file_matches_recent_image: bool,
    active_app: Option<&str>,
    inferred_source: Option<&str>,
    recent_image_is_current: bool,
) -> bool {
    single_image_file
        && (file_matches_recent_image || is_pasted_source(active_app) || inferred_source.is_some())
        && recent_image_is_current
}

#[cfg(target_os = "macos")]
pub(crate) fn clipboard_change_marker() -> Option<i64> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let pasteboard: *mut Object = msg_send![objc::class!(NSPasteboard), generalPasteboard];
        if pasteboard.is_null() {
            return None;
        }
        let change_count: isize = msg_send![pasteboard, changeCount];
        i64::try_from(change_count).ok()
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn clipboard_change_marker() -> Option<i64> {
    None
}

pub(crate) fn already_processed_change(marker: Option<i64>, processed: Option<i64>) -> bool {
    marker.is_some() && marker == processed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screenshot_sources_override_ambiguous_producers() {
        assert_eq!(
            inferred_screenshot_source(Path::new("/Users/pasted/Desktop/CleanShot 2026-08-11.png")),
            Some("CleanShot X")
        );
        assert_eq!(
            inferred_screenshot_source(Path::new("Screenshot 2026-08-11.png")),
            Some("Screenshot")
        );
        assert_eq!(
            resolved_capture_source(Some("Finder"), Some("CleanShot X")),
            Some("CleanShot X")
        );
        assert_eq!(
            resolved_capture_source(Some("Finder"), None),
            Some("Finder")
        );
        assert_eq!(composite_image_source(None), "Screenshot");
    }

    #[test]
    fn image_file_preference_distinguishes_file_manager_and_screenshot_copies() {
        assert!(!should_prefer_composite_image(
            true,
            false,
            true,
            Some("Finder"),
            None
        ));
        assert!(should_prefer_composite_image(
            true,
            false,
            true,
            Some("Finder"),
            Some("CleanShot X")
        ));
        assert!(should_prefer_composite_image(
            true,
            true,
            false,
            Some("File Explorer"),
            None
        ));
        assert!(!should_prefer_composite_image(
            false, true, true, None, None
        ));
    }

    #[test]
    fn duplicate_and_recent_image_decisions_are_explicit() {
        assert!(!already_processed_change(Some(42), None));
        assert!(!already_processed_change(Some(43), Some(42)));
        assert!(already_processed_change(Some(42), Some(42)));
        assert!(!already_processed_change(None, None));

        assert!(should_coalesce_recent_image(
            true,
            true,
            Some("Preview"),
            None,
            true
        ));
        assert!(should_coalesce_recent_image(
            true,
            false,
            Some("Pasted"),
            None,
            true
        ));
        assert!(!should_coalesce_recent_image(
            true,
            true,
            Some("Preview"),
            None,
            false
        ));
        assert!(!should_coalesce_recent_image(false, true, None, None, true));
    }

    #[test]
    fn configured_capture_size_uses_the_shared_bounded_policy() {
        assert_eq!(capture_bytes_for_setting(Some("25")), 25 * 1024 * 1024);
        assert_eq!(
            capture_bytes_for_setting(Some("not-a-number")),
            capture_bytes_for_setting(None)
        );
        assert_eq!(
            capture_bytes_for_setting(Some("999999")),
            crate::resource_limits::MAX_CLIP_TEXT_BYTES
        );
    }
}
