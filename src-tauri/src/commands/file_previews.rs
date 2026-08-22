use base64::Engine;
use std::io::Cursor;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

#[path = "file_preview_cache.rs"]
mod file_preview_cache;
#[cfg(test)]
use self::file_preview_cache::looks_like_pdf;
use self::file_preview_cache::{
    clip_file_preview_cache_key, flatten_image_on_white, pdf_preview_cache_key, read_bounded_file,
    read_preview_cache, render_pdf_first_page, write_preview_cache,
};
use crate::db::DbState;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileClipPreview {
    index: usize,
    data_url: Option<String>,
    text_content: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    availability: crate::file_reference_health::FileReferenceAvailability,
    cached: bool,
}

pub(super) fn parse_file_clip_paths(value: &str) -> Vec<String> {
    crate::content_inspection::parse_file_paths(value)
}

fn is_safe_preview_extension(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jpeg" | "jpg" | "pdf" | "png" | "txt" | "webp"
            )
        })
}

fn is_pdf_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

fn is_text_preview_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
}

fn text_file_preview(bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut end = text
        .len()
        .min(crate::resource_limits::MAX_TEXT_FILE_PREVIEW_BYTES);
    while !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let bounded = &text[..end];
    if bounded.contains('\0') {
        return None;
    }
    Some(bounded.trim_start_matches('\u{feff}').to_string())
}

fn collect_file_clip_previews(
    paths: &[String],
    mode: &str,
    configured_max_bytes: u64,
    cache_directory: Option<&std::path::Path>,
    only_index: Option<usize>,
    clip_content_hash: Option<&str>,
) -> Vec<FileClipPreview> {
    collect_file_clip_previews_with_health(
        paths,
        mode,
        configured_max_bytes,
        cache_directory,
        only_index,
        clip_content_hash,
        None,
    )
}

fn collect_file_clip_previews_with_health(
    paths: &[String],
    mode: &str,
    configured_max_bytes: u64,
    cache_directory: Option<&std::path::Path>,
    only_index: Option<usize>,
    clip_content_hash: Option<&str>,
    health: Option<&[crate::file_reference_health::FileReferenceHealth]>,
) -> Vec<FileClipPreview> {
    if mode == "off" {
        return Vec::new();
    }
    let max_bytes = configured_max_bytes.clamp(
        1024 * 1024,
        crate::resource_limits::MAX_FILE_PREVIEW_INPUT_BYTES,
    );
    let mut previews = Vec::new();
    let mut encoded_total = 0usize;

    for (index, path) in paths.iter().enumerate() {
        if only_index.is_some_and(|requested| requested != index) {
            continue;
        }
        if previews.len() >= crate::resource_limits::MAX_FILE_PREVIEW_COUNT {
            break;
        }
        let path = std::path::Path::new(path);
        if mode == "safe" && !is_safe_preview_extension(path) {
            continue;
        }
        let clip_cache_key = clip_content_hash
            .filter(|_| !is_text_preview_path(path))
            .map(|content_hash| clip_file_preview_cache_key(content_hash, index));
        if let Some(cached) = clip_cache_key.as_deref().and_then(|key| {
            cache_directory.and_then(|directory| read_preview_cache(directory, key))
        }) {
            let dimensions = image::load_from_memory(&cached)
                .ok()
                .map(|decoded| (decoded.width(), decoded.height()));
            if let Some((width, height)) = dimensions {
                let Some(next_total) = encoded_total.checked_add(cached.len()) else {
                    break;
                };
                if next_total > crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES {
                    break;
                }
                encoded_total = next_total;
                previews.push(FileClipPreview {
                    index,
                    data_url: Some(format!(
                        "data:image/webp;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(cached)
                    )),
                    text_content: None,
                    width: Some(width),
                    height: Some(height),
                    availability:
                        crate::file_reference_health::FileReferenceAvailability::Available,
                    cached: true,
                });
                continue;
            }
        }
        if health.is_some_and(|items| {
            items.iter().any(|item| {
                item.index == index
                    && item.availability
                        != crate::file_reference_health::FileReferenceAvailability::Available
            })
        }) {
            continue;
        }
        let Some(bytes) = read_bounded_file(path, max_bytes) else {
            continue;
        };
        if is_text_preview_path(path) {
            let Some(text_content) = text_file_preview(&bytes) else {
                continue;
            };
            let Some(next_total) = encoded_total.checked_add(text_content.len()) else {
                break;
            };
            if next_total > crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES {
                break;
            }
            encoded_total = next_total;
            previews.push(FileClipPreview {
                index,
                data_url: None,
                text_content: Some(text_content),
                width: None,
                height: None,
                availability: crate::file_reference_health::FileReferenceAvailability::Available,
                cached: false,
            });
            continue;
        }
        let (preview_bytes, pdf_cache_key, was_cached) = if is_pdf_path(path) {
            let key = pdf_preview_cache_key(&bytes);
            if let Some(cached) =
                cache_directory.and_then(|directory| read_preview_cache(directory, &key))
            {
                (cached, Some(key), true)
            } else {
                let Some(rendered) = render_pdf_first_page(&bytes) else {
                    continue;
                };
                (rendered, Some(key), false)
            }
        } else {
            (bytes, None, false)
        };
        let Ok(reader) = image::ImageReader::new(Cursor::new(&preview_bytes)).with_guessed_format()
        else {
            continue;
        };
        let Ok((width, height)) = reader.into_dimensions() else {
            continue;
        };
        if !crate::resource_limits::image_dimensions_within_limit(width, height) {
            continue;
        }
        let (encoded, width, height) = if was_cached {
            (preview_bytes, width, height)
        } else {
            let Ok(mut decoded) = image::load_from_memory(&preview_bytes) else {
                continue;
            };
            if pdf_cache_key.is_some() {
                decoded = flatten_image_on_white(decoded);
            }
            let thumbnail = if decoded.width() > 1_600 || decoded.height() > 1_200 {
                decoded.thumbnail(1_600, 1_200)
            } else {
                decoded
            };
            let width = thumbnail.width();
            let height = thumbnail.height();
            let mut encoded = Cursor::new(Vec::new());
            if thumbnail
                .write_to(&mut encoded, image::ImageFormat::WebP)
                .is_err()
            {
                continue;
            }
            (encoded.into_inner(), width, height)
        };
        let Some(next_total) = encoded_total.checked_add(encoded.len()) else {
            break;
        };
        if next_total > crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES {
            break;
        }
        if !was_cached {
            if let (Some(directory), Some(key)) = (cache_directory, pdf_cache_key.as_deref()) {
                write_preview_cache(directory, key, &encoded);
            }
        }
        if let (Some(directory), Some(key)) = (cache_directory, clip_cache_key.as_deref()) {
            write_preview_cache(directory, key, &encoded);
        }
        encoded_total = next_total;
        previews.push(FileClipPreview {
            index,
            data_url: Some(format!(
                "data:image/webp;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(encoded)
            )),
            text_content: None,
            width: Some(width),
            height: Some(height),
            availability: crate::file_reference_health::FileReferenceAvailability::Available,
            cached: false,
        });
    }
    previews
}

fn attach_file_reference_health(
    paths: &[String],
    previews: Vec<FileClipPreview>,
    health: &[crate::file_reference_health::FileReferenceHealth],
    only_index: Option<usize>,
) -> Vec<FileClipPreview> {
    let mut previews = previews
        .into_iter()
        .map(|preview| (preview.index, preview))
        .collect::<std::collections::BTreeMap<_, _>>();
    health
        .iter()
        .filter(|item| only_index.is_none_or(|requested| requested == item.index))
        .filter(|item| item.index < paths.len())
        .map(|item| {
            let mut preview = previews.remove(&item.index).unwrap_or(FileClipPreview {
                index: item.index,
                data_url: None,
                text_content: None,
                width: None,
                height: None,
                availability: item.availability,
                cached: false,
            });
            preview.availability = item.availability;
            preview
        })
        .collect()
}

pub(crate) fn prefetch_file_clip_previews(
    app: &AppHandle,
    paths: &[String],
    content_hash: &str,
    mode: &str,
    max_size_mb: u64,
) {
    if mode == "off" {
        return;
    }
    let cache_directory = app
        .path()
        .app_cache_dir()
        .ok()
        .map(|directory| directory.join("file-previews/thumbnails"));
    let _ = collect_file_clip_previews(
        paths,
        mode,
        max_size_mb.saturating_mul(1024 * 1024),
        cache_directory.as_deref(),
        None,
        Some(content_hash),
    );
}

#[tauri::command]
pub async fn get_file_clip_previews(
    clip_id: i64,
    mode: String,
    max_size_mb: u64,
    only_index: Option<usize>,
    force_recheck: Option<bool>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<FileClipPreview>, String> {
    if !matches!(mode.as_str(), "off" | "safe" | "all") {
        return Err("Unknown file preview mode".to_string());
    }
    let cache_directory = app
        .path()
        .app_cache_dir()
        .ok()
        .map(|directory| directory.join("file-previews/thumbnails"));
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let clip = db
            .get_clip_by_id(clip_id)
            .map_err(|error| error.to_string())?;
        if clip.content_type != "file" {
            return Err("Clip is not a file list".to_string());
        }
        let paths = clip
            .text_content
            .as_deref()
            .map(parse_file_clip_paths)
            .filter(|paths| !paths.is_empty())
            .ok_or_else(|| "File clip has no valid path metadata".to_string())?;
        if !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File list exceeds Pasted's safety limit".to_string());
        }
        let health = crate::file_reference_health::resolve_file_reference_health(
            &db,
            clip.id,
            &paths,
            force_recheck.unwrap_or(false),
        )
        .map_err(|error| error.to_string())?;
        let configured_max_bytes = max_size_mb.saturating_mul(1024 * 1024);
        let previews = collect_file_clip_previews_with_health(
            &paths,
            &mode,
            configured_max_bytes,
            cache_directory.as_deref(),
            only_index,
            Some(&clip.content_hash),
            Some(&health),
        );
        Ok(attach_file_reference_health(
            &paths, previews, &health, only_index,
        ))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    fn minimal_pdf() -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R >>",
            "<< /Length 32 >>\nstream\n0.1 0.5 0.9 rg 20 20 160 160 re f\nendstream",
        ];
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n0 5\n0000000000 65535 f \n");
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n")
                .as_bytes(),
        );
        pdf
    }

    fn unique_test_directory(label: &str) -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("pasted_{label}_{}_{}", std::process::id(), nonce))
    }

    #[test]
    fn legacy_file_clip_paths_remain_previewable() {
        assert_eq!(
            parse_file_clip_paths("/Users/pasted/Downloads/old.txt"),
            vec!["/Users/pasted/Downloads/old.txt"]
        );
        assert_eq!(
            parse_file_clip_paths(
                "/Users/pasted/Downloads/first.txt\n/Users/pasted/Downloads/second.pdf"
            ),
            vec![
                "/Users/pasted/Downloads/first.txt",
                "/Users/pasted/Downloads/second.pdf"
            ]
        );
        assert_eq!(
            parse_file_clip_paths("\"/Users/pasted/Downloads/quoted.txt\""),
            vec!["/Users/pasted/Downloads/quoted.txt"]
        );
        assert_eq!(
            parse_file_clip_paths("file:///Users/pasted/Downloads/My%20File.txt"),
            vec!["/Users/pasted/Downloads/My File.txt"]
        );
    }

    #[test]
    fn file_previews_are_bounded_and_safe_mode_is_extension_allowlisted() {
        let root = unique_test_directory("file-previews");
        std::fs::create_dir_all(&root).unwrap();
        let png = root.join("preview.PNG");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([20, 40, 60, 255]))
            .save(&png)
            .unwrap();
        let disguised = root.join("preview.data");
        std::fs::copy(&png, &disguised).unwrap();
        let paths = vec![
            png.to_string_lossy().into_owned(),
            disguised.to_string_lossy().into_owned(),
        ];

        let safe = collect_file_clip_previews(&paths, "safe", 1024 * 1024, None, None, None);
        assert_eq!(safe.len(), 1);
        assert_eq!(safe[0].index, 0);
        assert!(safe[0]
            .data_url
            .as_deref()
            .is_some_and(|url| url.starts_with("data:image/webp;base64,")));

        let all = collect_file_clip_previews(&paths, "all", 1024 * 1024, None, None, None);
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].index, 1);

        let requested = collect_file_clip_previews(&paths, "all", 1024 * 1024, None, Some(1), None);
        assert_eq!(requested.len(), 1);
        assert_eq!(requested[0].index, 1);

        let oversized = root.join("oversized.png");
        std::fs::write(&oversized, vec![0u8; 1024 * 1024 + 1]).unwrap();
        assert!(read_bounded_file(&oversized, 1024 * 1024).is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn clip_thumbnail_cache_survives_an_ephemeral_source_file() {
        let root = unique_test_directory("ephemeral-file-preview");
        let cache = root.join("cache");
        std::fs::create_dir_all(&root).unwrap();
        let png = root.join("temporary.png");
        image::RgbaImage::from_pixel(3, 2, image::Rgba([80, 100, 120, 255]))
            .save(&png)
            .unwrap();
        let paths = vec![png.to_string_lossy().into_owned()];

        let initial = collect_file_clip_previews(
            &paths,
            "safe",
            1024 * 1024,
            Some(&cache),
            None,
            Some("files:ephemeral"),
        );
        assert_eq!(initial.len(), 1);
        std::fs::remove_file(&png).unwrap();
        let health = vec![crate::file_reference_health::FileReferenceHealth {
            index: 0,
            availability: crate::file_reference_health::FileReferenceAvailability::Missing,
            checked_at: "2026-08-22T00:00:00Z".into(),
        }];
        let restored = collect_file_clip_previews_with_health(
            &paths,
            "safe",
            1024 * 1024,
            Some(&cache),
            None,
            Some("files:ephemeral"),
            Some(&health),
        );
        let restored = attach_file_reference_health(&paths, restored, &health, None);
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].width, Some(3));
        assert_eq!(restored[0].height, Some(2));
        assert_eq!(restored[0].data_url, initial[0].data_url);
        assert_eq!(
            restored[0].availability,
            crate::file_reference_health::FileReferenceAvailability::Missing
        );
        assert!(restored[0].cached);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn text_file_previews_are_utf8_bounded_and_reject_binary_content() {
        let root = unique_test_directory("text-file-previews");
        std::fs::create_dir_all(&root).unwrap();
        let text = root.join("notes.TXT");
        std::fs::write(&text, "Hello from a copied text file.\nSecond line.").unwrap();
        let binary = root.join("binary.txt");
        std::fs::write(&binary, b"text\0binary").unwrap();
        let paths = vec![
            text.to_string_lossy().into_owned(),
            binary.to_string_lossy().into_owned(),
        ];

        let previews = collect_file_clip_previews(&paths, "safe", 1024 * 1024, None, None, None);
        assert_eq!(previews.len(), 1);
        assert_eq!(previews[0].index, 0);
        assert_eq!(
            previews[0].text_content.as_deref(),
            Some("Hello from a copied text file.\nSecond line.")
        );
        assert!(previews[0].data_url.is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pdf_previews_are_safe_allowlisted_and_require_a_pdf_header() {
        assert!(is_safe_preview_extension(std::path::Path::new(
            "document.PDF"
        )));
        assert!(is_pdf_path(std::path::Path::new("document.pdf")));
        assert!(looks_like_pdf(b"%PDF-1.7\n"));
        assert!(looks_like_pdf(
            &[b"prefix".as_slice(), b"%PDF-1.4"].concat()
        ));
        assert!(!looks_like_pdf(b"not a pdf"));
        assert!(!looks_like_pdf(
            &[vec![b' '; 1_025], b"%PDF-1.4".to_vec()].concat()
        ));

        let transparent = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            1,
            1,
            image::Rgba([20, 40, 60, 0]),
        ));
        assert_eq!(
            flatten_image_on_white(transparent)
                .to_rgba8()
                .get_pixel(0, 0),
            &image::Rgba([255, 255, 255, 255])
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn pdf_first_page_renderer_returns_a_bounded_png() {
        let rendered = render_pdf_first_page(&minimal_pdf()).expect("system PDF thumbnail");
        assert!(rendered.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(rendered.len() <= crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES);
    }

    #[test]
    #[cfg(unix)]
    fn pdf_preview_cache_is_content_addressed_and_private() {
        use std::os::unix::fs::PermissionsExt;

        let root = unique_test_directory("pdf-preview-cache");
        let key = pdf_preview_cache_key(b"%PDF-1.4\nfirst");
        assert_eq!(key, pdf_preview_cache_key(b"%PDF-1.4\nfirst"));
        assert_ne!(key, pdf_preview_cache_key(b"%PDF-1.4\nsecond"));

        write_preview_cache(&root, &key, b"cached-thumbnail");
        assert_eq!(
            read_preview_cache(&root, &key).as_deref(),
            Some(b"cached-thumbnail".as_slice())
        );
        assert_eq!(
            std::fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(root.join(format!("{key}.webp")))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
