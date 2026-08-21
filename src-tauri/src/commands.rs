use std::sync::Arc;
use tauri::AppHandle;

use crate::db::DbState;

pub(crate) mod activity;
pub(crate) mod analysis;
pub(crate) mod app_lock;
pub(crate) mod backups;
pub(crate) mod bins;
pub(crate) mod capture;
pub(crate) mod cli_installation;
pub(crate) mod clip_metadata;
pub(crate) mod clip_policies;
pub(crate) mod clipboard;
pub(crate) mod clips;
pub(crate) mod content_registry;
pub(crate) mod extraction;
pub(crate) mod extractors;
pub(crate) mod factory_reset;
pub(crate) mod file_previews;
pub(crate) mod hotkeys;
pub(crate) mod hud;
pub(crate) mod imports;
pub(crate) mod intelligence;
pub(crate) mod library_access;
pub(crate) mod manual_transforms;
pub(crate) mod platform;
pub(crate) mod queue;
pub(crate) mod retention;
pub(crate) mod search_indexes;
pub(crate) mod settings;
pub(crate) mod source_apps;
pub(crate) mod storage;
pub(crate) mod transformations;

pub(crate) use backups::*;
pub(crate) use bins::*;
pub(crate) use clipboard::*;
pub(crate) use extraction::*;
pub(crate) use factory_reset::*;
pub(crate) use hotkeys::*;
pub(crate) use hud::*;
pub(crate) use imports::*;
pub(crate) use intelligence::*;
pub(crate) use manual_transforms::*;
pub(crate) use source_apps::*;
pub(crate) use transformations::*;

fn refresh_native_app_menu(app: &AppHandle, db: &Arc<DbState>) {
    if let Err(error) = crate::app_menu::install(app, db) {
        eprintln!("Could not refresh the native app menu: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn ocr_text_never_replaces_an_image_clips_copy_fingerprint() {
        let rgba = vec![12, 34, 56, 255];
        let image = image::RgbaImage::from_raw(1, 1, rgba.clone()).unwrap();
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let image_base64 = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(encoded.into_inner())
        );
        let clip = crate::db::ClipItem {
            id: 1,
            name: None,
            content_type: "image".to_string(),
            content_types: Vec::new(),
            file_formats: Vec::new(),
            text_content: Some("recognized OCR text".to_string()),
            html_content: None,
            image_base64: Some(image_base64),
            image_path: None,
            content_hash: "stored-image-hash".to_string(),
            source: "Screenshot".to_string(),
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
            bin_ids: None,
            note: None,
            is_trashed: false,
            trashed_at: None,
            created_at: "2026-08-11T00:00:00Z".to_string(),
            ocr_extractor_ref: None,
            ocr_extractor_name: None,
            ocr_engine_version: None,
        };

        assert_eq!(
            crate::clipboard_actions::internal_fingerprint(&clip).unwrap(),
            crate::clipboard_fingerprint::image_rgba(&rgba)
        );
    }

    #[test]
    fn intelligence_credentials_must_remain_references() {
        for reference in [
            "env:OPENAI_API_KEY",
            "env:_LOCAL_MODEL_TOKEN",
            "op://Private/OpenAI/credential",
            "keychain:pasted.openai",
        ] {
            assert!(
                crate::intelligence_connections::validate_credential_reference(Some(reference))
                    .is_ok()
            );
        }
        for value in [
            "sk-proj-literal-secret",
            "env:NOT VALID",
            "env:123_INVALID",
            "op://",
            " keychain:pasted.openai",
            "",
        ] {
            assert!(
                crate::intelligence_connections::validate_credential_reference(Some(value))
                    .is_err()
            );
        }
        assert!(crate::intelligence_connections::validate_credential_reference(None).is_ok());
    }

    #[test]
    fn test_print_parsed_shortcuts() {
        let strings = vec![
            "Command+1",
            "Command+Digit1",
            "Super+Digit1",
            "Command+C",
            "Command+KeyC",
            "Super+KeyC",
            "Alt+Shift+V",
            "Alt+Shift+KeyV",
            "Control+Alt+C",
            "Control+Alt+KeyC",
        ];
        for s in strings {
            let parsed = crate::keyboard_shortcuts::parse(s);
            println!("parse_shortcut_str('{s}') = {:?}", parsed);
        }
    }

    #[test]
    fn test_accessibility_status_check() {
        let status = check_accessibility_permission();
        println!(
            "Accessibility test status: trusted={}, dev_mode={}",
            status.is_trusted, status.is_dev_mode
        );
        assert_eq!(status.is_dev_mode, cfg!(debug_assertions));
    }

    #[test]
    fn file_clip_metadata_reports_availability_without_crawling_directories() {
        let root = std::env::temp_dir().join(format!(
            "pasted_file_metadata_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let directory = root.join("Folder");
        std::fs::create_dir_all(&directory).unwrap();
        let file = root.join("first.txt");
        std::fs::write(&file, b"pasted").unwrap();
        let missing = root.join("missing.mp4");
        let paths = vec![
            file.to_string_lossy().into_owned(),
            directory.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
        ];

        let inspection = crate::content_inspection::inspect_files(paths.clone(), None).unwrap();
        let structure = inspection.result.files.unwrap();
        let observations = crate::content_inspection::observe_files(&paths);
        assert_eq!(structure.item_count, 3);
        assert_eq!(observations.available_count, 2);
        assert_eq!(observations.file_count, 1);
        assert_eq!(observations.directory_count, 1);
        assert_eq!(observations.total_size_bytes, 6);
        assert_eq!(structure.extensions, vec!["TXT", "MP4"]);

        std::fs::remove_dir_all(root).unwrap();
    }
}
