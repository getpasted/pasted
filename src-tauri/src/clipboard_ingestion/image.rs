use base64::Engine;
use sha2::{Digest, Sha256};
use tauri::Emitter;

use super::CaptureContext;
use crate::app_exclusions::ExcludedCaptureKind;
use crate::clipboard_capture_policy::RecentImageCapture;

pub(crate) fn ingest_image(
    context: &CaptureContext<'_>,
    image: arboard::ImageData<'_>,
    ocr: &crate::ocr::OcrService,
    last_hash: &mut String,
    recent_image: &mut Option<RecentImageCapture>,
    reattribute_source: Option<&str>,
    capture_limit: usize,
) {
    let (Ok(width), Ok(height)) = (u32::try_from(image.width), u32::try_from(image.height)) else {
        context.report_ignored("Ignored clipboard image with invalid dimensions");
        return;
    };
    if !crate::resource_limits::image_dimensions_within_limit(width, height) {
        let mut hasher = Sha256::new();
        hasher.update(image.bytes.as_ref());
        let hash = format!("{:x}", hasher.finalize());
        if context.begin_hash(last_hash, &hash) {
            context.report_ignored("Ignored clipboard image larger than 24 megapixels");
        }
        return;
    }

    let raw_bytes = image.bytes.to_vec();
    let hash = crate::clipboard_fingerprint::image_rgba(&raw_bytes);
    if !context.begin_hash(last_hash, &hash) {
        return;
    }
    if context.queue.consume_internal_clipboard_write(&hash) {
        return;
    }
    if let (Some(recent), Some(source)) = (recent_image.as_ref(), reattribute_source) {
        if let Ok(Some(updated)) =
            context
                .db
                .reattribute_image_capture(recent.clip_id, &recent.content_hash, source)
        {
            let _ = context.app.emit("clip-added", updated);
        }
        return;
    }
    if context.ignore_excluded(ExcludedCaptureKind::Image) {
        return;
    }

    let Some(image_bytes) = rgba_to_encoded_image(width, height, &raw_bytes) else {
        context.report_failed();
        return;
    };
    let capture_limit = capture_limit.min(crate::resource_limits::MAX_ENCODED_IMAGE_BYTES);
    if image_bytes.len() > capture_limit {
        context.report_ignored(&format!(
            "Ignored clipboard image larger than the configured {} MB limit",
            capture_limit / 1024 / 1024
        ));
        return;
    }
    let data_url = format!(
        "data:image/webp;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&image_bytes)
    );
    match context
        .db
        .save_clip("image", None, None, Some(&data_url), &hash, context.source)
    {
        Ok(clip) => {
            *recent_image = Some(RecentImageCapture::new(clip.id, clip.content_hash.clone()));
            let _ = context.app.emit("clip-added", clip.clone());
            context.report_success(clip.id);
            if crate::features::is_enabled(context.db, crate::features::Feature::Ocr) {
                let _ = ocr.enqueue(crate::ocr::OcrTask {
                    clip_id: clip.id,
                    content_hash: clip.content_hash,
                    image_bytes,
                });
            }
        }
        Err(error) => {
            eprintln!("[Pasted Monitor] Failed to save image clip: {error}");
            context.report_failed();
        }
    }
}

fn rgba_to_encoded_image(width: u32, height: u32, rgba_data: &[u8]) -> Option<Vec<u8>> {
    use image::{ImageBuffer, Rgba};

    let buffer: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(width, height, rgba_data.to_vec())?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    if buffer
        .write_to(&mut cursor, image::ImageFormat::WebP)
        .is_ok()
    {
        return Some(cursor.into_inner());
    }
    let mut fallback = std::io::Cursor::new(Vec::new());
    buffer
        .write_to(&mut fallback, image::ImageFormat::Png)
        .ok()?;
    Some(fallback.into_inner())
}

#[cfg(test)]
mod tests {
    use super::rgba_to_encoded_image;

    #[test]
    fn invalid_rgba_payloads_fail_before_persistence() {
        assert!(rgba_to_encoded_image(2, 2, &[0; 4]).is_none());
    }

    #[test]
    fn valid_rgba_payloads_encode_to_bounded_raster_data() {
        let bytes = rgba_to_encoded_image(1, 1, &[10, 20, 30, 255]).unwrap();
        assert!(!bytes.is_empty());
        assert!(bytes.len() <= crate::resource_limits::MAX_ENCODED_IMAGE_BYTES);
    }
}
