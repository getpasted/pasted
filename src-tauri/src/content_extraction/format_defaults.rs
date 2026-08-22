use super::{APPLE_VISION_OCR_REF, TESSERACT_OCR_REF, WHISPER_TRANSCRIPTION_REF};

pub(super) fn for_builtin(stable_ref: &str) -> Vec<String> {
    let formats: &[&str] = match stable_ref {
        APPLE_VISION_OCR_REF | TESSERACT_OCR_REF => {
            &["bmp", "gif", "heif", "jpg", "png", "tif", "webp"]
        }
        WHISPER_TRANSCRIPTION_REF => &["aac", "flac", "m4a", "mp3", "ogg", "wav"],
        _ => &["*"],
    };
    formats.iter().map(|format| (*format).into()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_extractors_declare_logical_file_format_defaults() {
        assert_eq!(
            for_builtin(WHISPER_TRANSCRIPTION_REF),
            ["aac", "flac", "m4a", "mp3", "ogg", "wav"]
        );
        for stable_ref in [APPLE_VISION_OCR_REF, TESSERACT_OCR_REF] {
            let accepted = for_builtin(stable_ref);
            assert!(accepted.contains(&"png".to_string()));
            assert!(accepted.contains(&"jpg".to_string()));
            assert!(accepted.contains(&"heif".to_string()));
            assert!(!accepted.contains(&"pdf".to_string()));
        }
    }
}
