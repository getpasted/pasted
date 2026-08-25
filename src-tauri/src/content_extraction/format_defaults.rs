use super::{
    APPLE_VISION_LABELS_REF, APPLE_VISION_OCR_REF, LLAMA_CPP_LABELS_REF, TESSERACT_OCR_REF,
    WHISPER_TRANSCRIPTION_REF,
};

pub(super) fn for_builtin(stable_ref: &str) -> Vec<String> {
    let formats: &[&str] = match stable_ref {
        APPLE_VISION_OCR_REF | APPLE_VISION_LABELS_REF => {
            &["bmp", "gif", "heif", "jpg", "png", "tif", "webp"]
        }
        LLAMA_CPP_LABELS_REF => &["bmp", "jpg", "png", "webp"],
        TESSERACT_OCR_REF => &["bmp", "gif", "jpg", "png", "tif", "webp"],
        WHISPER_TRANSCRIPTION_REF => &["aac", "flac", "m4a", "mp3", "ogg", "wav"],
        _ => &["*"],
    };
    formats.iter().map(|format| (*format).into()).collect()
}
