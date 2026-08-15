use arboard::Clipboard;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

use crate::bin_assignment::BinAssignmentOutcome;
use crate::db::{
    Bin, ClipItem, ClipMutationSummary, ContentDetectionRescanReport, DbState, FactoryResetReport,
    FullBackupInspection, IntelligenceConnection, IntelligenceConnectionUpdate,
    LibraryArchiveInspection, Pipeline, PipelineStepInput, SavedTransform,
    TransformClipApplication, TransformDefinition,
};
use crate::features::{self, Feature};
use crate::installation_diagnostics::InstallationDiagnostics;
use crate::library_storage::{self, LibraryLocationInfo};
use crate::sequential_paste::{SequentialQueueState, SequentialStatus};
use crate::third_party_licenses::ThirdPartyLicenseDocument;

fn refresh_native_app_menu(app: &AppHandle, db: &Arc<DbState>) {
    if let Err(error) = crate::app_menu::install(app, db) {
        eprintln!("Could not refresh the native app menu: {error}");
    }
}

fn emit_window_appearance_change(app: &AppHandle, key: &str, value: &str) {
    let _ = app.emit(
        "app-setting-changed",
        serde_json::json!({ "key": key, "value": value }),
    );

    // Retain the narrower event while older windows and integrations migrate.
    if matches!(key, "themeMode" | "textSize") {
        let _ = app.emit(
            "window-appearance-changed",
            serde_json::json!({ "key": key, "value": value }),
        );
    }
}

#[tauri::command]
pub fn set_linux_native_menu_theme(app: AppHandle, dark: bool) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        app.run_on_main_thread(move || {
            if let Err(error) = crate::linux_native_theme::apply_menu_theme(dark) {
                eprintln!("Could not apply the native Linux menu theme: {error}");
            }
        })
        .map_err(|error| error.to_string())?;
    }

    #[cfg(not(target_os = "linux"))]
    let _ = (app, dark);

    Ok(())
}

#[tauri::command]
pub fn set_overlay_cursor(app: AppHandle, pointing: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.run_on_main_thread(move || unsafe {
            use objc::runtime::Object;
            use objc::{msg_send, sel, sel_impl};

            let cursor: *mut Object = if pointing {
                msg_send![objc::class!(NSCursor), pointingHandCursor]
            } else {
                msg_send![objc::class!(NSCursor), arrowCursor]
            };
            let _: () = msg_send![cursor, set];
        })
        .map_err(|error| error.to_string())?;
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (app, pointing);

    Ok(())
}

#[tauri::command]
pub fn perform_titlebar_double_click(window: tauri::WebviewWindow) -> Result<(), String> {
    crate::titlebar::perform_titlebar_double_click(window)
}

#[tauri::command]
pub fn get_installation_diagnostics(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<InstallationDiagnostics, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let app_path = executable
        .ancestors()
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(PathBuf::from)
        .unwrap_or(executable);
    let data_path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(InstallationDiagnostics::collect_with_database(
        app_path,
        data_path,
        db.database_path(),
    ))
}

#[tauri::command]
pub fn get_third_party_licenses() -> ThirdPartyLicenseDocument {
    crate::third_party_licenses::document().clone()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMoveReport {
    location: LibraryLocationInfo,
    recovery_path: String,
}

#[tauri::command]
pub fn get_library_location(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<LibraryLocationInfo, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    Ok(library_storage::location_info(
        &app_data,
        &db.database_path(),
    ))
}

#[tauri::command]
pub async fn move_library(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<LibraryMoveReport>, String> {
    let Some(folder) = app
        .dialog()
        .file()
        .set_title("Choose Pasted Library Folder")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let directory = folder
        .into_path()
        .map_err(|error| format!("The selected library location is not a local folder: {error}"))?;
    let current_path = db.database_path();
    let target = library_storage::validate_destination_directory(&directory, &current_path)?;
    if target == current_path {
        let app_data = app
            .path()
            .app_data_dir()
            .map_err(|error| error.to_string())?;
        return Ok(Some(LibraryMoveReport {
            location: library_storage::location_info(&app_data, &current_path),
            recovery_path: current_path.to_string_lossy().into_owned(),
        }));
    }

    let db_for_move = Arc::clone(&db);
    let target_for_move = target.clone();
    let previous = tauri::async_runtime::spawn_blocking(move || {
        db_for_move.relocate_database(target_for_move)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| format!("Could not move the Pasted library: {error}"))?;

    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    if let Err(error) = library_storage::persist_location(&app_data, &target) {
        db.switch_to_database(previous.clone())
            .map_err(|rollback| {
                format!("{error} The previous library also could not be reopened: {rollback}")
            })?;
        return Err(error);
    }
    let _ = db.log_activity("library_moved", "Moved the Pasted library");
    Ok(Some(LibraryMoveReport {
        location: library_storage::location_info(&app_data, &target),
        recovery_path: previous.to_string_lossy().into_owned(),
    }))
}

#[tauri::command]
pub async fn restore_default_library_location(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<LibraryMoveReport, String> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    let target = library_storage::default_database_path(&app_data);
    let current = db.database_path();
    if current == target {
        return Ok(LibraryMoveReport {
            location: library_storage::location_info(&app_data, &current),
            recovery_path: current.to_string_lossy().into_owned(),
        });
    }

    let archived_default = library_storage::archive_existing_database(&target)?;
    let db_for_move = Arc::clone(&db);
    let target_for_move = target.clone();
    let move_result = tauri::async_runtime::spawn_blocking(move || {
        db_for_move.relocate_database(target_for_move)
    })
    .await
    .map_err(|error| error.to_string())?;
    let previous = match move_result {
        Ok(previous) => previous,
        Err(error) => {
            if let Some(archived) = archived_default.as_deref() {
                library_storage::restore_archived_database(archived, &target);
            }
            return Err(format!(
                "Could not restore the default library location: {error}"
            ));
        }
    };

    if let Err(error) = library_storage::persist_location(&app_data, &target) {
        db.switch_to_database(previous.clone())
            .map_err(|rollback| {
                format!("{error} The custom library also could not be reopened: {rollback}")
            })?;
        library_storage::remove_database_files(&target);
        if let Some(archived) = archived_default.as_deref() {
            library_storage::restore_archived_database(archived, &target);
        }
        return Err(error);
    }
    let _ = db.log_activity(
        "library_moved",
        "Restored the default Pasted library location",
    );
    Ok(LibraryMoveReport {
        location: library_storage::location_info(&app_data, &target),
        recovery_path: previous.to_string_lossy().into_owned(),
    })
}

fn apply_feature_policy_changes(app: &AppHandle, db: &Arc<DbState>, changed: &[Feature]) {
    for feature in changed {
        if features::is_enabled(db, *feature) {
            continue;
        }
        match feature {
            Feature::Hud => {
                if let Some(window) = app.get_webview_window("hud") {
                    let _ = window.hide();
                }
            }
            Feature::Queue => {
                if let Some(queue) = app.try_state::<Arc<SequentialQueueState>>() {
                    queue.stop_queue();
                    let _ = app.emit("sequential-updated", queue.get_status());
                }
            }
            Feature::Ocr => {
                if let Some(ocr) = app.try_state::<Arc<crate::ocr::OcrService>>() {
                    ocr.cancel();
                }
            }
            Feature::Notifications => {
                if let Some(window) = app.get_webview_window("capture-feedback") {
                    let _ = window.hide();
                }
            }
            _ => {}
        }
    }
    refresh_native_app_menu(app, db);
    crate::refresh_tray_menu(app, db);
    let _ = register_all_app_shortcuts(app);
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileClipPreview {
    index: usize,
    data_url: Option<String>,
    text_content: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
}

fn parse_file_clip_paths(value: &str) -> Vec<String> {
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

#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
fn looks_like_pdf(bytes: &[u8]) -> bool {
    bytes
        .windows(5)
        .take(1_024)
        .any(|window| window == b"%PDF-")
}

fn pdf_preview_cache_key(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pasted-pdf-first-page-v2-white-background\0");
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn clip_file_preview_cache_key(content_hash: &str, index: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pasted-file-preview-v1\0");
    hasher.update(content_hash.as_bytes());
    hasher.update([0]);
    hasher.update(index.to_le_bytes());
    format!("clip-{:x}", hasher.finalize())
}

fn flatten_image_on_white(image: image::DynamicImage) -> image::DynamicImage {
    let source = image.to_rgba8();
    let mut background = image::RgbaImage::from_pixel(
        source.width(),
        source.height(),
        image::Rgba([255, 255, 255, 255]),
    );
    image::imageops::overlay(&mut background, &source, 0, 0);
    image::DynamicImage::ImageRgba8(background)
}

fn read_pdf_preview_cache(cache_directory: &std::path::Path, key: &str) -> Option<Vec<u8>> {
    read_bounded_file(
        &cache_directory.join(format!("{key}.webp")),
        crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES as u64,
    )
}

fn prune_pdf_preview_cache(cache_directory: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(cache_directory) else {
        return;
    };
    let mut cached = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).ok()?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return None;
            }
            let modified = metadata.modified().unwrap_or(std::time::UNIX_EPOCH);
            Some((path, metadata.len(), modified))
        })
        .collect::<Vec<_>>();
    cached.sort_by_key(|(_, _, modified)| *modified);
    let mut total_bytes = cached
        .iter()
        .fold(0u64, |total, (_, size, _)| total.saturating_add(*size));
    let mut excess_items = cached
        .len()
        .saturating_sub(crate::resource_limits::MAX_FILE_PREVIEW_CACHE_ITEMS);
    for (path, size, _) in cached {
        if excess_items == 0 && total_bytes <= crate::resource_limits::MAX_FILE_PREVIEW_CACHE_BYTES
        {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total_bytes = total_bytes.saturating_sub(size);
            excess_items = excess_items.saturating_sub(1);
        }
    }
}

fn write_pdf_preview_cache(cache_directory: &std::path::Path, key: &str, bytes: &[u8]) {
    if bytes.len() > crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES
        || std::fs::create_dir_all(cache_directory).is_err()
    {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = std::fs::metadata(cache_directory) {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            let _ = std::fs::set_permissions(cache_directory, permissions);
        }
    }
    let path = cache_directory.join(format!("{key}.webp"));
    let written = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = file.metadata()?.permissions();
            permissions.set_mode(0o600);
            file.set_permissions(permissions)?;
        }
        Ok(())
    })();
    if written.is_ok() {
        prune_pdf_preview_cache(cache_directory);
    }
}

#[cfg(target_os = "macos")]
fn render_pdf_first_page(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    if !looks_like_pdf(bytes) {
        return None;
    }
    let working_directory = std::env::temp_dir().join(format!(
        "pasted_pdf_preview_{}_{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let input_path = working_directory.join("document.pdf");
    let output_path = working_directory.join("thumbnail.png");
    std::fs::create_dir_all(&working_directory).ok()?;

    let rendered = (|| {
        std::fs::write(&input_path, bytes).ok()?;
        let mut child = Command::new("/usr/bin/sips")
            .args(["-s", "format", "png", "-Z", "1600"])
            .arg(&input_path)
            .arg("--out")
            .arg(&output_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait().ok()? {
                if status.success() {
                    break;
                }
                return None;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            thread::sleep(Duration::from_millis(20));
        }
        read_bounded_file(
            &output_path,
            crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES as u64,
        )
    })();
    let _ = std::fs::remove_dir_all(&working_directory);
    rendered
}

#[cfg(not(target_os = "macos"))]
fn render_pdf_first_page(_bytes: &[u8]) -> Option<Vec<u8>> {
    None
}

fn read_bounded_file(path: &std::path::Path, max_bytes: u64) -> Option<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > max_bytes {
        return None;
    }
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::with_capacity(metadata.len().min(max_bytes) as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= max_bytes).then_some(bytes)
}

fn collect_file_clip_previews(
    paths: &[String],
    mode: &str,
    configured_max_bytes: u64,
    cache_directory: Option<&std::path::Path>,
    only_index: Option<usize>,
    clip_content_hash: Option<&str>,
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
            cache_directory.and_then(|directory| read_pdf_preview_cache(directory, key))
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
                });
                continue;
            }
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
            });
            continue;
        }
        let (preview_bytes, pdf_cache_key, was_cached) = if is_pdf_path(path) {
            let key = pdf_preview_cache_key(&bytes);
            if let Some(cached) =
                cache_directory.and_then(|directory| read_pdf_preview_cache(directory, &key))
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
                write_pdf_preview_cache(directory, key, &encoded);
            }
        }
        if let (Some(directory), Some(key)) = (cache_directory, clip_cache_key.as_deref()) {
            write_pdf_preview_cache(directory, key, &encoded);
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
        });
    }
    previews
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
pub async fn analyze_content(
    request: AnalyzeContentRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::analysis_execution::AnalyzerPreview, String> {
    let AnalyzeContentRequest {
        text,
        clip_id,
        source,
        policy,
        include_extractor,
        include_detectors,
        include_enricher,
    } = request;
    if text.is_some() == clip_id.is_some() {
        return Err("Provide exactly one of text or clipId".into());
    }
    let policy = policy
        .as_deref()
        .unwrap_or("interactive")
        .parse::<crate::analysis_contract::AnalysisPolicy>()?;
    let include_enricher = include_enricher.unwrap_or(true);
    if include_enricher
        && policy.includes(crate::analysis_contract::AnalysisPass::Enrich)
        && !features::is_enabled(&db, Feature::Transformations)
    {
        return Err("Transformations is disabled in Settings → Functionality".into());
    }
    let options = crate::analysis_execution::AnalyzerOptions {
        policy,
        include_extractor: include_extractor.unwrap_or(false),
        include_detectors: include_detectors.unwrap_or(true),
        include_enricher,
    };
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || match (text, clip_id) {
        (Some(text), None) => {
            crate::analysis_execution::analyze_text(&db, &text, source.as_deref(), options)
        }
        (None, Some(clip_id)) => crate::analysis_execution::analyze_clip(&db, clip_id, options),
        _ => unreachable!("input combination validated"),
    })
    .await
    .map_err(|error| error.to_string())?
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeContentRequest {
    text: Option<String>,
    clip_id: Option<i64>,
    source: Option<String>,
    policy: Option<String>,
    include_extractor: Option<bool>,
    include_detectors: Option<bool>,
    include_enricher: Option<bool>,
}

#[tauri::command]
pub async fn get_file_clip_previews(
    clip_id: i64,
    mode: String,
    max_size_mb: u64,
    only_index: Option<usize>,
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
        let configured_max_bytes = max_size_mb.saturating_mul(1024 * 1024);
        Ok(collect_file_clip_previews(
            &paths,
            &mode,
            configured_max_bytes,
            cache_directory.as_deref(),
            only_index,
            Some(&clip.content_hash),
        ))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn get_clips(
    search_query: Option<String>,
    bin_id: Option<i64>,
    only_pinned: bool,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipItem>, String> {
    db.get_clips_page(search_query.as_deref(), bin_id, only_pinned, limit, offset)
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureFeedbackClip {
    id: i64,
    content_type: String,
    preview_text: Option<String>,
    source: String,
    is_pinned: bool,
    is_protected: bool,
    is_trashed: bool,
}

fn bounded_preview_text(value: &str) -> String {
    value.chars().take(280).collect()
}

#[tauri::command]
pub fn get_capture_feedback_clip(
    id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<CaptureFeedbackClip, String> {
    features::require(&db, Feature::Notifications)?;
    let clip = db.get_clip_by_id(id).map_err(|error| error.to_string())?;
    let preview_text = if clip.content_type == "file" {
        clip.text_content
            .as_deref()
            .map(parse_file_clip_paths)
            .map(|paths| {
                bounded_preview_text(
                    &paths
                        .iter()
                        .filter_map(|path| std::path::Path::new(path).file_name())
                        .filter_map(|name| name.to_str())
                        .collect::<Vec<_>>()
                        .join(" · "),
                )
            })
    } else {
        clip.text_content
            .as_deref()
            .map(bounded_preview_text)
            .filter(|text| !text.is_empty())
    };

    Ok(CaptureFeedbackClip {
        id: clip.id,
        content_type: clip.content_type,
        preview_text,
        source: clip.source,
        is_pinned: clip.is_pinned,
        is_protected: clip.is_protected,
        is_trashed: clip.is_trashed,
    })
}

#[tauri::command]
pub fn get_clip_collection_summary(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::ClipCollectionSummary, String> {
    db.get_clip_collection_summary()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_image(db: State<'_, Arc<DbState>>, id: i64) -> Result<Option<String>, String> {
    db.get_clip_image(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_trashed_clips(
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<ClipItem>, String> {
    db.get_trashed_clips_page(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.restore_clip(id).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_all_trashed_clips(
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Trash)?;
    db.restore_all_trashed_clips()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn purge_clip_permanently(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_clip_permanently(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn empty_trash(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.empty_trash().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_activity_logs(
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::ActivityLog>, String> {
    db.get_activity_logs(limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_activity_logs(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.clear_activity_logs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_activity_json(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db
        .export_activity_json()
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Activity as JSON");
    Ok(exported)
}

#[tauri::command]
pub fn export_activity_csv(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db
        .export_activity_csv()
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Activity as CSV");
    Ok(exported)
}

#[tauri::command]
pub fn get_content_detectors(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_detection::Detector>, String> {
    db.get_content_detectors()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_content_extractors(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_extraction::Extractor>, String> {
    db.get_content_extractors()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_content_extractor(
    input: crate::content_extraction::ExtractorDefinitionInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_extraction::Extractor, String> {
    db.create_content_extractor(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_content_extractor_definition(
    id: i64,
    input: crate::content_extraction::ExtractorDefinitionInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_extraction::Extractor, String> {
    db.update_content_extractor_definition(id, &input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn duplicate_content_extractor(
    reference: String,
    name: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_extraction::Extractor, String> {
    db.duplicate_content_extractor(&reference, name.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_content_extractor(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_content_extractor(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_default_content_extractors(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_extraction::Extractor>, String> {
    db.restore_default_content_extractors()
        .map_err(|error| error.to_string())?;
    db.get_content_extractors()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_library_items(
    kind: Option<String>,
    include_archived: Option<bool>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::library_items::LibraryItemView>, String> {
    db.get_library_items(kind.as_deref(), include_archived.unwrap_or(false))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_library_item_enabled(
    kind: String,
    stable_ref: String,
    enabled: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.set_library_item_enabled(&kind, &stable_ref, enabled)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_content_types(
    include_archived: Option<bool>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_types::ContentTypeDefinition>, String> {
    db.get_content_types(include_archived.unwrap_or(false))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_content_type_groups(
    include_archived: Option<bool>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_types::ContentTypeGroupDefinition>, String> {
    db.get_content_type_groups(include_archived.unwrap_or(false))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_content_type_group(
    input: crate::content_types::ContentTypeGroupInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_types::ContentTypeGroupDefinition, String> {
    db.create_content_type_group(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_content_type_group(
    id: String,
    input: crate::content_types::ContentTypeGroupInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_types::ContentTypeGroupDefinition, String> {
    db.update_content_type_group(&id, &input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_content_type_group_archived(
    id: String,
    archived: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.set_content_type_group_archived(&id, archived)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_content_type_group(id: String, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_content_type_group(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_default_content_type_groups(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_types::ContentTypeGroupDefinition>, String> {
    db.restore_default_content_type_groups()
        .map_err(|error| error.to_string())?;
    db.get_content_type_groups(true)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_content_type(
    input: crate::content_types::ContentTypeInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_types::ContentTypeDefinition, String> {
    db.create_content_type(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_content_type(
    id: String,
    input: crate::content_types::ContentTypeInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_types::ContentTypeDefinition, String> {
    db.update_content_type(&id, &input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_content_type_archived(
    id: String,
    archived: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.set_content_type_archived(&id, archived)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_default_content_types(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_types::ContentTypeDefinition>, String> {
    db.restore_default_content_types()
        .map_err(|error| error.to_string())?;
    db.get_content_types(true)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_content_detector(
    input: crate::content_detection::DetectorInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_detection::Detector, String> {
    db.create_content_detector(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_content_detector(
    id: i64,
    input: crate::content_detection::DetectorInput,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_detection::Detector, String> {
    db.update_content_detector(id, &input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn duplicate_content_detector(
    reference: String,
    name: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::content_detection::Detector, String> {
    db.duplicate_content_detector(&reference, name.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_content_detector(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_content_detector(id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn restore_default_content_detectors(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::content_detection::Detector>, String> {
    db.restore_default_content_detectors()
        .map_err(|error| error.to_string())?;
    db.get_content_detectors()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rescan_content_detection_history(
    confirmed: bool,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<ContentDetectionRescanReport, String> {
    if !confirmed {
        return Err("History rescans require explicit confirmation.".to_string());
    }
    let db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || db.rescan_content_detection())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let _ = app.emit("app-menu-action", "refresh-data");
    Ok(report)
}

#[tauri::command]
pub fn test_content_detector(
    input: crate::content_detection::DetectorInput,
    sample: String,
) -> Result<crate::detection_execution::DetectionResult, String> {
    crate::content_detection::validate_detector_input(&input)?;
    let detector = crate::content_detection::Detector {
        id: 0,
        stable_ref: "preview".into(),
        name: input.name,
        content_type: input.content_type.clone(),
        description: input.description,
        patterns: input.patterns,
        validator: input.validator,
        enabled: true,
        priority: input.priority,
        is_builtin: false,
        defaults: None,
        is_deleted: false,
    };
    Ok(crate::detection_execution::analyze_detector(
        &sample, &detector,
    ))
}

#[tauri::command]
pub fn save_app_setting(
    key: String,
    value: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    let previous = db.get_setting(&key).map_err(|e| e.to_string())?;
    db.save_setting(&key, &value).map_err(|e| e.to_string())?;
    if let Some(activity) =
        crate::settings_activity::describe_setting_change(&key, previous.as_deref(), &value)
    {
        let _ = db.log_activity(activity.event_type, &activity.description);
    }
    if let Some(feature) = Feature::from_setting_key(&key) {
        apply_feature_policy_changes(&app, &db, &[feature]);
    }
    if key == "menubarIconStyle" {
        crate::refresh_tray_icon(&app, &value);
    }
    emit_window_appearance_change(&app, &key, &value);
    Ok(())
}

#[tauri::command]
pub fn save_app_settings(
    values: std::collections::HashMap<String, String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    let mut activities = values
        .iter()
        .filter_map(|(key, value)| {
            let previous = db.get_setting(key).ok().flatten();
            crate::settings_activity::describe_setting_change(key, previous.as_deref(), value)
        })
        .collect::<Vec<_>>();
    activities.sort_by(|left, right| left.description.cmp(&right.description));
    db.save_settings(&values)
        .map_err(|error| error.to_string())?;
    if activities.len() == 1 {
        let activity = &activities[0];
        let _ = db.log_activity(activity.event_type, &activity.description);
    } else if !activities.is_empty() {
        let description = activities
            .iter()
            .map(|activity| activity.description.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        let _ = db.log_activity("settings_changed", &description);
    }
    let changed = values
        .keys()
        .filter_map(|key| Feature::from_setting_key(key))
        .collect::<Vec<_>>();
    if !changed.is_empty() {
        apply_feature_policy_changes(&app, &db, &changed);
    }
    for (key, value) in values {
        if key == "menubarIconStyle" {
            crate::refresh_tray_icon(&app, &value);
        }
        emit_window_appearance_change(&app, &key, &value);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[tauri::command]
pub fn play_system_sound(sound_id: Option<u32>) {
    let id = sound_id.unwrap_or(1057);
    unsafe {
        #[link(name = "AudioToolbox", kind = "framework")]
        extern "C" {
            fn AudioServicesPlaySystemSound(sound_id: u32);
        }
        AudioServicesPlaySystemSound(id);
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn play_system_sound(_sound_id: Option<u32>) {}

#[tauri::command]
pub fn get_all_app_settings(
    db: State<'_, Arc<DbState>>,
) -> Result<std::collections::HashMap<String, String>, String> {
    db.get_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn enforce_clip_retention(
    keep_count: i64,
    keep_age_days: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_clip_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn enforce_trash_retention(
    keep_count: i64,
    keep_age_days: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_trash_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn enforce_activity_retention(
    keep_count: i64,
    keep_age_days: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_activity_retention(keep_count, keep_age_days)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn enforce_revision_retention(
    keep_count: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    db.enforce_revision_retention(keep_count)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_clip_note(
    clip_id: i64,
    note: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Notes)?;
    db.update_clip_note(clip_id, note.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.delete_clip(id).map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_pin_clip(id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    features::require(&db, Feature::Pinning)?;
    db.toggle_pin(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn assign_clip_bin(
    clip_id: i64,
    bin_id: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<ClipItem>, String> {
    features::require(&db, Feature::Bins)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::bin_assignment::assign_clips_to_bin(&db, vec![clip_id], bin_id)
            .map(|outcome| outcome.updated_clips.into_iter().next())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn remove_clip_bin(
    clip_id: i64,
    bin_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<BinAssignmentOutcome, String> {
    features::require(&db, Feature::Bins)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::bin_assignment::remove_clips_from_bin(&db, vec![clip_id], bin_id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn reorder_pinned_clips(ids: Vec<i64>, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    features::require(&db, Feature::Pinning)?;
    db.reorder_pinned_clips(ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_bin_clips(
    bin_id: i64,
    clip_ids: Vec<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    db.reorder_bin_clips(bin_id, clip_ids)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_clip_versions(
    clip_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::db::ClipVersion>, String> {
    features::require(&db, Feature::Revisions)?;
    db.get_clip_versions_page(
        clip_id,
        limit.unwrap_or(50).clamp(1, 100),
        offset.unwrap_or(0).max(0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_clip_version_count(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<i64, String> {
    features::require(&db, Feature::Revisions)?;
    db.get_clip_version_count(clip_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_clip_version(
    clip_id: i64,
    version_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipItem, String> {
    features::require(&db, Feature::Revisions)?;
    db.restore_clip_version(clip_id, version_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn batch_pin_clips(
    ids: Vec<i64>,
    pin_state: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Pinning)?;
    db.batch_pin_clips(ids, pin_state)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_trash_clips(
    ids: Vec<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    db.batch_trash_clips(ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn batch_assign_bin_clips(
    ids: Vec<i64>,
    bin_id: Option<i64>,
    db: State<'_, Arc<DbState>>,
) -> Result<BinAssignmentOutcome, String> {
    features::require(&db, Feature::Bins)?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::bin_assignment::assign_clips_to_bin(&db, ids, bin_id)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn export_backup_file(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    let suggested_name = format!(
        "Pasted_History_and_Organization_{}.json",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title("Export History and Organization")
        .set_file_name(suggested_name)
        .add_filter("Pasted JSON Export", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };

    let path = selected_file.into_path().map_err(|error| {
        format!("The selected export location is not a writable file path: {error}")
    })?;
    let json = db.export_backup_json().map_err(|error| error.to_string())?;
    std::fs::write(&path, json)
        .map_err(|error| format!("Could not save the history and organization export: {error}"))?;
    let _ = db.log_activity(
        "data_export_completed",
        "Exported History and Organization as JSON",
    );
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub async fn export_full_backup_file(
    client_state_json: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::FullBackupReport>, String> {
    let suggested_name = format!(
        "Pasted_Full_Backup_{}.pastedbackup",
        chrono::Local::now().format("%Y-%m-%d")
    );
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title("Create Full Pasted Backup")
        .set_file_name(suggested_name)
        .add_filter("Pasted Full Backup", &["pastedbackup"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = selected_file
        .into_path()
        .map_err(|error| format!("The selected backup location is not writable: {error}"))?;
    if let Some(state) = client_state_json.as_deref() {
        db.save_setting("backedUpClientState", state)
            .map_err(|error| error.to_string())?;
    }
    let window_flags =
        StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN;
    let _ = app.save_window_state(window_flags);
    let window_state_json = app
        .path()
        .app_config_dir()
        .ok()
        .and_then(|directory| std::fs::read_to_string(directory.join(app.filename())).ok());
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        db.create_full_backup(
            &path,
            client_state_json.as_deref(),
            window_state_json.as_deref(),
        )
        .inspect(|_| {
            let _ = db.log_activity("backup_created", "Created a complete recovery backup");
        })
        .map(Some)
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn restore_full_backup_file(
    current_client_state_json: Option<String>,
    backup_path: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::FullRestoreReport>, String> {
    let path = if let Some(path) = backup_path {
        PathBuf::from(path)
    } else {
        let Some(selected_file) = app
            .dialog()
            .file()
            .set_title("Restore Full Pasted Backup")
            .add_filter("Pasted Full Backup", &["pastedbackup"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        selected_file
            .into_path()
            .map_err(|error| format!("The selected backup is not accessible: {error}"))?
    };
    let window_flags =
        StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED | StateFlags::FULLSCREEN;
    let _ = app.save_window_state(window_flags);
    let current_window_state_json = app
        .path()
        .app_config_dir()
        .ok()
        .and_then(|directory| std::fs::read_to_string(directory.join(app.filename())).ok());
    let db = Arc::clone(&db);
    let restore_db = Arc::clone(&db);
    let (report, _client_state, restored_window_state) =
        tauri::async_runtime::spawn_blocking(move || {
            restore_db
                .restore_full_backup(
                    &path,
                    current_client_state_json.as_deref(),
                    current_window_state_json.as_deref(),
                )
                .map_err(|error| error.to_string())
        })
        .await
        .map_err(|error| error.to_string())??;

    if let Some(window_state) = restored_window_state {
        let parsed = serde_json::from_str::<serde_json::Value>(&window_state)
            .map_err(|error| format!("The backup contains invalid window state: {error}"))?;
        let directory = app
            .path()
            .app_config_dir()
            .map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
        std::fs::write(
            directory.join(app.filename()),
            serde_json::to_vec_pretty(&parsed).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("Could not restore the saved window state: {error}"))?;
    }
    if let Ok(cache_directory) = app.path().app_cache_dir() {
        let _ = std::fs::remove_dir_all(cache_directory);
    }
    let _ = db.log_activity(
        "backup_recovery_completed",
        "Recovered the complete state from a backup",
    );
    if !tauri::is_dev() {
        let restart_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            restart_handle.restart();
        });
    }
    Ok(Some(report))
}

#[tauri::command]
pub fn consume_pending_full_restore_client_state(
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    db.consume_pending_full_restore_client_state()
        .map_err(|error| error.to_string())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFileInspection {
    path: String,
    name: String,
    kind: String,
    format: String,
    size_bytes: u64,
    report: Option<serde_json::Value>,
    library: Option<LibraryArchiveInspection>,
    backup: Option<FullBackupInspection>,
}

#[tauri::command]
pub async fn choose_import_file(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<ImportFileInspection>, String> {
    let Some(selected_file) = app
        .dialog()
        .file()
        .set_title("Choose Data to Import or Recover")
        .add_filter("Pasted Data", &["json", "csv", "pastedbackup"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = selected_file
        .into_path()
        .map_err(|error| format!("The selected file is not accessible: {error}"))?;
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || inspect_import_file_path(path, &db))
        .await
        .map_err(|error| error.to_string())?
        .map(Some)
}

fn inspect_import_file_path(path: PathBuf, db: &DbState) -> Result<ImportFileInspection, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("The selected file is not accessible: {error}"))?;
    if !metadata.is_file() {
        return Err("The selected item is not a file.".to_string());
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Selected file")
        .to_string();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let base = |kind: &str, format: &str| ImportFileInspection {
        path: path.to_string_lossy().into_owned(),
        name: name.clone(),
        kind: kind.to_string(),
        format: format.to_string(),
        size_bytes: metadata.len(),
        report: None,
        library: None,
        backup: None,
    };

    if extension == "pastedbackup" {
        let inspection = db
            .inspect_full_backup(&path)
            .map_err(|error| format!("The backup is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            backup: Some(inspection),
            ..base("backup", "backup")
        });
    }
    if !matches!(extension.as_str(), "json" | "csv") {
        return Err("Choose a JSON, CSV, or Pasted Backup file.".to_string());
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|error| format!("The selected file could not be read: {error}"))?;
    if extension == "csv" {
        let header = contents.lines().next().unwrap_or_default();
        if header.starts_with("timestamp,observed_timestamp,event_name,") {
            let report = db
                .inspect_activity_csv(&contents)
                .map_err(|error| format!("The Activity CSV is not valid: {error}"))?;
            return Ok(ImportFileInspection {
                report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
                ..base("activity", "csv")
            });
        }
        if header.starts_with("id,content_type,source,") {
            let report = db
                .inspect_clips_csv(&contents)
                .map_err(|error| format!("The Clips CSV is not valid: {error}"))?;
            return Ok(ImportFileInspection {
                report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
                ..base("clips", "csv")
            });
        }
        return Err("The CSV does not match a supported Clips or Activity export.".to_string());
    }

    let parsed: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|error| format!("The selected file is not valid JSON: {error}"))?;
    if parsed.is_array() {
        let report = db
            .inspect_clips_json(&contents)
            .map_err(|error| format!("The Clips JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
            ..base("clips", "json")
        });
    }
    let object = parsed
        .as_object()
        .ok_or_else(|| "The JSON does not match a supported export.".to_string())?;
    if object
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .is_some()
        && object
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    {
        let report = db
            .inspect_activity_json(&contents)
            .map_err(|error| format!("The Activity JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            report: Some(serde_json::to_value(report).map_err(|error| error.to_string())?),
            ..base("activity", "json")
        });
    }
    if object
        .get("clips")
        .and_then(serde_json::Value::as_array)
        .is_some()
        && object
            .get("bins")
            .and_then(serde_json::Value::as_array)
            .is_some()
        && object
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .is_some()
    {
        let inspection = DbState::inspect_library_archive_json(&contents)
            .map_err(|error| format!("The History and Organization JSON is not valid: {error}"))?;
        return Ok(ImportFileInspection {
            library: Some(inspection),
            ..base("organization", "json")
        });
    }
    Err("The JSON does not match a supported export.".to_string())
}

#[tauri::command]
pub async fn import_inspected_file(
    path: String,
    kind: String,
    format: String,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<serde_json::Value, String> {
    let refresh_menu = kind == "organization";
    let db = Arc::clone(&db);
    let worker_db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || {
        let contents = std::fs::read_to_string(PathBuf::from(path))
            .map_err(|error| format!("The selected file could not be read: {error}"))?;
        let result: Result<serde_json::Value, String> = match (kind.as_str(), format.as_str()) {
            ("clips", "json") => serde_json::to_value(
                worker_db
                    .import_clips_json(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("clips", "csv") => serde_json::to_value(
                worker_db
                    .import_clips_csv(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("activity", "json") => serde_json::to_value(
                worker_db
                    .import_activity_json(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("activity", "csv") => serde_json::to_value(
                worker_db
                    .import_activity_csv(&contents)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string()),
            ("organization", "json") => {
                let imported = worker_db
                    .import_backup_json(&contents)
                    .map_err(|error| error.to_string())?;
                Ok(serde_json::json!({ "importedCount": imported }))
            }
            _ => Err("The selected import action is not supported.".to_string()),
        };
        result
    })
    .await
    .map_err(|error| error.to_string())??;
    if refresh_menu {
        refresh_native_app_menu(&app, &db);
    }
    Ok(report)
}

#[tauri::command]
pub fn get_external_import_sources() -> Vec<crate::external_import::ExternalImportSourceInfo> {
    crate::external_import::source_infos()
}

#[tauri::command]
pub async fn import_external_history(
    source: String,
    path: Option<String>,
    choose_file: Option<bool>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::external_import::ExternalImportReport>, String> {
    let source = source.parse::<crate::external_import::ExternalImportSource>()?;
    let selected_path =
        if choose_file.unwrap_or(false) {
            let mut picker = app
                .dialog()
                .file()
                .set_title(if source.prefers_folder_selection() {
                    format!("Choose the {} Data Folder", source.label())
                } else {
                    format!("Import {} History", source.label())
                });
            if let Some(directory) = source.suggested_directory() {
                picker = picker.set_directory(directory);
            }
            if source.prefers_folder_selection() {
                let Some(selected_folder) = picker.blocking_pick_folder() else {
                    return Ok(None);
                };
                Some(selected_folder.into_path().map_err(|error| {
                    format!("The selected history folder is not accessible: {error}")
                })?)
            } else {
                let Some(selected_file) = picker
                    .add_filter(
                        "Clipboard History",
                        &["sqlite", "db", "alfdb", "plist", "data"],
                    )
                    .blocking_pick_file()
                else {
                    return Ok(None);
                };
                Some(selected_file.into_path().map_err(|error| {
                    format!("The selected history file is not accessible: {error}")
                })?)
            }
        } else {
            path.map(PathBuf::from)
        };
    let db = Arc::clone(&db);
    let report = tauri::async_runtime::spawn_blocking(move || {
        crate::external_import::import_history(&db, source, selected_path).map(Some)
    })
    .await
    .map_err(|error| error.to_string())??;
    if let Some(capacity) = report
        .as_ref()
        .and_then(|report| report.history_capacity_adjusted_to)
    {
        emit_window_appearance_change(&app, "keepClipCount", &capacity.to_string());
    }
    Ok(report)
}

#[tauri::command]
pub fn factory_reset_app(
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<FactoryResetReport, String> {
    let report = db.factory_reset().map_err(|error| error.to_string())?;

    if let Some(queue) = app.try_state::<Arc<SequentialQueueState>>() {
        queue.clear_queue();
        let _ = app.emit("sequential-updated", queue.get_status());
    }

    // Cached previews are derived from library state and must not survive a reset.
    if let Ok(cache_directory) = app.path().app_cache_dir() {
        let _ = std::fs::remove_dir_all(cache_directory);
    }

    // A packaged app can restart its own executable. During `tauri dev`, that same
    // exit tears down the supervising CLI and Vite server, so the frontend reloads
    // its webview in place instead after it has cleared browser-side caches.
    if !tauri::is_dev() {
        let restart_handle = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(250));
            restart_handle.restart();
        });
    }

    Ok(report)
}

#[tauri::command]
pub fn copy_clip_to_system(
    text: Option<String>,
    image_base64: Option<String>,
    file_paths: Option<Vec<String>>,
) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;

    if let Some(paths) = file_paths {
        if paths.is_empty() || !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File list exceeds Pasted's safety limit".to_string());
        }
        clipboard
            .set()
            .file_list(&paths)
            .map_err(|error| error.to_string())?;
    } else if let Some(img_b64) = image_base64 {
        // Strip data:image/png;base64,
        let clean = img_b64.split(',').next_back().unwrap_or(&img_b64);
        if clean.len() > crate::resource_limits::MAX_STORED_IMAGE_BASE64_BYTES {
            return Err("Clip image exceeds Pasted's safety limit".to_string());
        }
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|e| e.to_string())?;

        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let img_data = arboard::ImageData {
            width: rgba.width() as usize,
            height: rgba.height() as usize,
            bytes: std::borrow::Cow::Owned(rgba.into_raw()),
        };
        clipboard.set_image(img_data).map_err(|e| e.to_string())?;
    } else if let Some(t) = text {
        if t.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
            return Err("Clip text exceeds Pasted's safety limit".to_string());
        }
        clipboard.set_text(t).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub(crate) fn write_clip_to_clipboard(
    clipboard: &mut Clipboard,
    clip: &ClipItem,
) -> Result<(), String> {
    if clip.content_type == "file" {
        let paths = clip
            .text_content
            .as_deref()
            .ok_or_else(|| "File clip has no path metadata".to_string())
            .and_then(|value| {
                serde_json::from_str::<Vec<String>>(value)
                    .map_err(|_| "File clip has invalid path metadata".to_string())
            })?;
        if paths.is_empty() || !crate::resource_limits::file_list_within_limit(&paths) {
            return Err("File list exceeds Pasted's safety limit".to_string());
        }
        return clipboard
            .set()
            .file_list(&paths)
            .map_err(|error| error.to_string());
    }
    if clip.content_type == "image" {
        let image_base64 = clip
            .image_base64
            .as_deref()
            .ok_or_else(|| "Image clip has no stored image data".to_string())?;
        let clean = image_base64.split(',').next_back().unwrap_or(image_base64);
        if clean.len() > crate::resource_limits::MAX_STORED_IMAGE_BASE64_BYTES {
            return Err("Clip image exceeds Pasted's safety limit".to_string());
        }
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|error| error.to_string())?;
        let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
        let rgba = image.to_rgba8();
        return clipboard
            .set_image(arboard::ImageData {
                width: rgba.width() as usize,
                height: rgba.height() as usize,
                bytes: std::borrow::Cow::Owned(rgba.into_raw()),
            })
            .map_err(|error| error.to_string());
    }
    if let Some(text) = clip.text_content.as_deref() {
        return clipboard.set_text(text).map_err(|error| error.to_string());
    }
    Err("Clip has no copyable content".to_string())
}

fn clip_internal_clipboard_fingerprint(clip: &ClipItem) -> Result<String, String> {
    if clip.content_type == "file" {
        let paths = clip
            .text_content
            .as_deref()
            .ok_or_else(|| "File clip has no path metadata".to_string())
            .and_then(|value| {
                serde_json::from_str::<Vec<String>>(value)
                    .map_err(|_| "File clip has invalid path metadata".to_string())
            })?;
        return Ok(crate::clipboard_fingerprint::file_list(&paths));
    }
    if clip.content_type == "image" {
        let image_base64 = clip
            .image_base64
            .as_deref()
            .ok_or_else(|| "Image clip has no stored image data".to_string())?;
        let clean = image_base64.split(',').next_back().unwrap_or(image_base64);
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, clean)
            .map_err(|error| error.to_string())?;
        let image = image::load_from_memory(&bytes).map_err(|error| error.to_string())?;
        return Ok(crate::clipboard_fingerprint::image_rgba(
            image.to_rgba8().as_raw(),
        ));
    }
    if let Some(text) = clip.text_content.as_deref() {
        return Ok(text.to_string());
    }
    Err("Clip has no copyable content".to_string())
}

#[tauri::command]
pub fn copy_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    sequential: State<'_, Arc<SequentialQueueState>>,
) -> Result<(), String> {
    copy_clip_by_id_shared(&db, &sequential, clip_id)
}

pub(crate) fn copy_clip_by_id_shared(
    db: &DbState,
    sequential: &SequentialQueueState,
    clip_id: i64,
) -> Result<(), String> {
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    let fingerprint = clip_internal_clipboard_fingerprint(&clip)?;
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    sequential.mark_internal_clipboard_write(&fingerprint);
    if let Err(error) = write_clip_to_clipboard(&mut clipboard, &clip) {
        sequential.clear_internal_clipboard_write();
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub fn paste_text_to_frontmost(text: String, app: AppHandle) -> Result<(), String> {
    if text.len() > crate::resource_limits::MAX_CLIP_TEXT_BYTES {
        return Err("Clip text exceeds Pasted's 8 MB safety limit".to_string());
    }
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    clipboard
        .set_text(text)
        .map_err(|error| error.to_string())?;

    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.hide();
    }

    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate_cmd_v_paste();
    });

    Ok(())
}

pub(crate) fn execute_clipboard_pipeline(
    db: &DbState,
    pipeline_ref: Option<&str>,
    paste_result: bool,
) -> Result<crate::transformation_service::ExecutionOutcome, String> {
    features::require(db, Feature::Transformations)?;
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    let input = clipboard.get_text().map_err(|error| error.to_string())?;
    let outcome = crate::transformation_service::execute_shortcut_pipeline(db, input, pipeline_ref)
        .map_err(|error| error.to_string())?;
    clipboard
        .set_text(&outcome.output)
        .map_err(|error| error.to_string())?;
    if paste_result {
        thread::spawn(|| {
            thread::sleep(Duration::from_millis(50));
            let _ = simulate_cmd_v_paste();
        });
    }
    Ok(outcome)
}

#[tauri::command]
pub fn get_bins(db: State<'_, Arc<DbState>>) -> Result<Vec<Bin>, String> {
    db.get_bins().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_bin(
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<Bin, String> {
    features::require(&db, Feature::Bins)?;
    let bin = db
        .create_bin(&name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())?;
    refresh_native_app_menu(&app, &db);
    Ok(bin)
}

#[tauri::command]
pub fn delete_bin(
    id: i64,
    disposition: Option<String>,
    destination_bin_id: Option<i64>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    db.delete_bin(
        id,
        disposition.as_deref().unwrap_or("keep"),
        destination_bin_id,
    )
    .map_err(|e| e.to_string())?;
    refresh_native_app_menu(&app, &db);
    Ok(())
}

#[tauri::command]
pub fn update_bin(
    id: i64,
    name: String,
    icon: String,
    color: String,
    smart_rule: Option<String>,
    app: AppHandle,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    db.update_bin(id, &name, &icon, &color, smart_rule.as_deref())
        .map_err(|e| e.to_string())?;
    refresh_native_app_menu(&app, &db);
    Ok(())
}

#[tauri::command]
pub fn get_pipelines(db: State<'_, Arc<DbState>>) -> Result<Vec<Pipeline>, String> {
    db.get_pipelines().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_pipeline(
    name: String,
    steps: Vec<PipelineStepInput>,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<Pipeline, String> {
    features::require(&db, Feature::Transformations)?;
    let pipeline = db
        .create_pipeline(&name, &steps, shortcut.as_deref())
        .map_err(|error| error.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(pipeline)
}

#[tauri::command]
pub fn update_pipeline(
    pipeline_ref: String,
    name: String,
    steps: Vec<PipelineStepInput>,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<Pipeline, String> {
    features::require(&db, Feature::Transformations)?;
    let pipeline = db
        .update_pipeline(&pipeline_ref, &name, &steps, shortcut.as_deref())
        .map_err(|error| error.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(pipeline)
}

#[tauri::command]
pub fn update_pipeline_shortcut(
    pipeline_ref: String,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.update_pipeline_shortcut(&pipeline_ref, shortcut.as_deref())
        .map_err(|error| error.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_pipeline(
    pipeline_ref: String,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.delete_pipeline(&pipeline_ref)
        .map_err(|error| error.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(())
}

#[tauri::command]
pub async fn preview_pipeline_steps(
    input: String,
    steps: Vec<PipelineStepInput>,
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<String, crate::transformation_service::ExecutionError> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::transformation_service::ExecutionError {
            code: "feature_disabled",
            message,
            step: None,
            operation_ref: None,
        });
    }
    let cancellation = client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::transformation_service::preview_pipeline_steps(
            &db,
            &input,
            &steps,
            client_request_id.as_deref(),
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        )
    })
    .await
    .map_err(|error| crate::transformation_service::ExecutionError {
        code: "executor_join_failed",
        message: error.to_string(),
        step: None,
        operation_ref: None,
    })?
}

#[tauri::command]
pub fn update_bin_shortcut(
    id: i64,
    shortcut: Option<String>,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    db.update_bin_shortcut(id, shortcut.as_deref())
        .map_err(|e| e.to_string())?;
    let _ = register_all_app_shortcuts(&app);
    Ok(())
}

#[tauri::command]
pub fn get_bin_transform_ref(
    bin_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<String>, String> {
    db.get_bin_transform_ref(bin_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_bin_transform_ref(
    bin_id: i64,
    transform_ref: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Bins)?;
    features::require(&db, Feature::Transformations)?;
    db.set_bin_transform_ref(bin_id, transform_ref.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_operations(db: State<'_, Arc<DbState>>) -> Result<Vec<crate::db::Operation>, String> {
    db.get_operations().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_intelligence_connections(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<IntelligenceConnection>, String> {
    db.get_intelligence_connections()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn detect_intelligence_connections(
    db: State<'_, Arc<DbState>>,
) -> Result<Vec<crate::intelligence_connections::DetectedIntelligenceConnection>, String> {
    features::require(&db, Feature::Transformations)?;
    let detected = tauri::async_runtime::spawn_blocking(
        crate::intelligence_connections::detect_intelligence_connections,
    )
    .await
    .map_err(|error| error.to_string())?;
    for candidate in &detected {
        let endpoint = if candidate.provider_kind == "cli" {
            candidate.executable_path.as_deref()
        } else {
            candidate.default_endpoint
        };
        db.ensure_intelligence_connection_candidate(
            candidate.name,
            candidate.provider_kind,
            endpoint,
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(detected)
}

#[tauri::command]
pub fn create_intelligence_connection(
    name: String,
    provider_kind: String,
    endpoint: Option<String>,
    model: Option<String>,
    credential_ref: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<IntelligenceConnection, String> {
    features::require(&db, Feature::Transformations)?;
    if name.trim().is_empty() {
        return Err("Connection name cannot be empty".to_string());
    }
    crate::intelligence_connections::validate_credential_reference(credential_ref.as_deref())?;
    db.create_intelligence_connection(
        &name,
        &provider_kind,
        endpoint.as_deref(),
        model.as_deref(),
        credential_ref.as_deref(),
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Preserve the established flat Tauri IPC contract.
pub fn update_intelligence_connection(
    id: String,
    name: String,
    provider_kind: String,
    endpoint: Option<String>,
    model: Option<String>,
    credential_ref: Option<String>,
    enabled: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    if name.trim().is_empty() {
        return Err("Connection name cannot be empty".to_string());
    }
    crate::intelligence_connections::validate_credential_reference(credential_ref.as_deref())?;
    db.update_intelligence_connection(IntelligenceConnectionUpdate {
        id: &id,
        name: &name,
        provider_kind: &provider_kind,
        endpoint: endpoint.as_deref(),
        model: model.as_deref(),
        credential_ref: credential_ref.as_deref(),
        enabled,
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_intelligence_connection(
    id: String,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.delete_intelligence_connection(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reorder_intelligence_connections(
    ids: Vec<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.reorder_intelligence_connections(&ids)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn plan_transformation_intent(
    request: crate::intelligence_executor::PlanIntentRequest,
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::PlanIntentOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::intelligence_executor::IntelligenceExecutionError {
            code: "feature_disabled",
            message,
        });
    }
    let cancellation = client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::intelligence_executor::plan_intent_with_cancellation(
            &db,
            request,
            client_request_id.as_deref(),
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        );
        match &result {
            Ok(outcome) => {
                let _ = db.log_activity(
                    "transform_drafted",
                    &format!(
                        "Drafted a {}-step Transform with {} in {} ms",
                        outcome.plan.steps.len(),
                        outcome.connection_name,
                        outcome.duration_ms
                    ),
                );
            }
            Err(error) => {
                if error.code == "execution_cancelled" {
                    let _ =
                        db.log_activity("transform_draft_cancelled", "Cancelled Transform draft");
                } else {
                    let _ = db.log_activity(
                        "transform_draft_failed",
                        &format!("Transform draft failed ({})", error.code),
                    );
                }
            }
        }
        result
    })
    .await
    .map_err(
        |error| crate::intelligence_executor::IntelligenceExecutionError {
            code: "executor_join_failed",
            message: error.to_string(),
        },
    )?
}

#[tauri::command]
pub async fn test_transformation_plan(
    request: crate::intelligence_executor::ExecutePlanRequest,
    client_request_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::intelligence_executor::ExecutePlanOutcome,
    crate::intelligence_executor::IntelligenceExecutionError,
> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::intelligence_executor::IntelligenceExecutionError {
            code: "feature_disabled",
            message,
        });
    }
    let cancellation = client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        let result = crate::intelligence_executor::execute_plan_with_cancellation(
            &db,
            request,
            client_request_id.as_deref(),
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        );
        match &result {
            Ok(outcome) => {
                let provider = outcome
                    .connection_name
                    .as_deref()
                    .unwrap_or("local Operations");
                let _ = db.log_activity(
                    "transform_tested",
                    &format!(
                        "Tested a Transform with {provider} in {} ms",
                        outcome.duration_ms
                    ),
                );
            }
            Err(error) => {
                if error.code == "execution_cancelled" {
                    let _ = db.log_activity("transform_test_cancelled", "Cancelled Transform test");
                } else {
                    let _ = db.log_activity(
                        "transform_test_failed",
                        &format!("Transform test failed ({})", error.code),
                    );
                }
            }
        }
        result
    })
    .await
    .map_err(
        |error| crate::intelligence_executor::IntelligenceExecutionError {
            code: "executor_join_failed",
            message: error.to_string(),
        },
    )?
}

#[tauri::command]
pub fn get_intent_transforms(db: State<'_, Arc<DbState>>) -> Result<Vec<SavedTransform>, String> {
    db.get_intent_transforms()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_transforms(db: State<'_, Arc<DbState>>) -> Result<Vec<TransformDefinition>, String> {
    features::require(&db, Feature::Transformations)?;
    db.get_transform_definitions()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_saved_transform(
    name: String,
    plan: crate::transformation_intent::TransformationPlan,
    connection_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<SavedTransform, String> {
    features::require(&db, Feature::Transformations)?;
    let transform_name = if name.trim().is_empty() {
        plan.summary.trim()
    } else {
        name.trim()
    };
    let transform = db
        .create_saved_transform(transform_name, &plan, connection_id.as_deref())
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity(
        "transform_saved",
        &format!("Saved Transform: {}", transform.name),
    );
    Ok(transform)
}

#[tauri::command]
pub fn update_saved_transform(
    transform_ref: String,
    name: String,
    plan: crate::transformation_intent::TransformationPlan,
    connection_id: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<SavedTransform, String> {
    features::require(&db, Feature::Transformations)?;
    let transform_name = if name.trim().is_empty() {
        plan.summary.trim()
    } else {
        name.trim()
    };
    let transform = db
        .update_saved_transform(
            &transform_ref,
            transform_name,
            &plan,
            connection_id.as_deref(),
        )
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity(
        "transform_updated",
        &format!("Updated Transform: {}", transform.name),
    );
    Ok(transform)
}

#[tauri::command]
pub fn delete_saved_transform(
    transform_ref: String,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.delete_saved_transform(&transform_ref)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn apply_transform_preview_to_clip(
    clip_id: i64,
    transform_ref: String,
    expected_input: String,
    output: String,
    connection_id: Option<String>,
    duration_ms: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::ClipTransformationProvenance, String> {
    features::require(&db, Feature::Transformations)?;
    let provenance = db
        .apply_transform_output_to_clip(TransformClipApplication {
            clip_id,
            transform_ref: &transform_ref,
            expected_input: &expected_input,
            output: &output,
            connection_id: connection_id.as_deref(),
            duration_ms,
            bin_move: None,
        })
        .map_err(|error| error.to_string())?;
    let _ = db.log_activity(
        "clip_transformed",
        &format!(
            "Applied Transform {} to clip #{}",
            provenance.transform_name, clip_id
        ),
    );
    Ok(provenance)
}

#[tauri::command]
pub fn get_clip_transformation_provenance(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<Option<crate::db::ClipTransformationProvenance>, String> {
    db.get_clip_transformation_provenance(clip_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_operation(
    name: String,
    op_type: String,
    config: Option<String>,
    category: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::Operation, String> {
    features::require(&db, Feature::Transformations)?;
    db.create_operation(&name, &op_type, config.as_deref(), category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_operation(
    id: i64,
    name: String,
    op_type: String,
    config: Option<String>,
    category: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.update_operation(id, &name, &op_type, config.as_deref(), category.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn duplicate_operation(
    reference: String,
    name: Option<String>,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::Operation, String> {
    features::require(&db, Feature::Transformations)?;
    db.duplicate_operation(&reference, name.as_deref())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn delete_operation(id: i64, db: State<'_, Arc<DbState>>) -> Result<(), String> {
    features::require(&db, Feature::Transformations)?;
    db.delete_operation(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn transform_text(
    input: String,
    filter_type: String,
    config: Option<String>,
) -> Result<String, String> {
    crate::transformation_service::execute_legacy_preview(&input, &filter_type, config.as_deref())
}

#[tauri::command]
pub async fn execute_transformation(
    request: crate::transformation_service::ExecutionRequest,
    db: State<'_, Arc<DbState>>,
) -> Result<
    crate::transformation_service::ExecutionOutcome,
    crate::transformation_service::ExecutionError,
> {
    if let Err(message) = features::require(&db, Feature::Transformations) {
        return Err(crate::transformation_service::ExecutionError {
            code: "feature_disabled",
            message,
            step: None,
            operation_ref: None,
        });
    }
    let cancellation = request
        .client_request_id
        .clone()
        .map(crate::transformation_service::CancellationRegistration::register);
    let db = Arc::clone(&db);
    tauri::async_runtime::spawn_blocking(move || {
        crate::transformation_service::execute_with_cancellation(
            &db,
            request,
            cancellation
                .as_ref()
                .map(|registration| registration.flag()),
        )
    })
    .await
    .map_err(|error| crate::transformation_service::ExecutionError {
        code: "executor_join_failed",
        message: error.to_string(),
        step: None,
        operation_ref: None,
    })?
}

#[tauri::command]
pub fn cancel_transformation_execution(client_request_id: String) -> bool {
    crate::transformation_service::cancel_execution(&client_request_id)
}

#[tauri::command]
pub fn get_intelligence_scheduler_snapshot() -> crate::intelligence_scheduler::SchedulerSnapshot {
    crate::intelligence_scheduler::snapshot()
}

// Sequential Paste Commands
#[tauri::command]
pub fn start_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    seq.start_queue();
    let _ = db.log_activity(
        "queue_recording_started",
        "Started recording copies into the Queue",
    );
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn push_sequential_item(
    item: String,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    if item.is_empty() {
        return Err("Only clips containing text can be added to the Copy Queue".to_string());
    }
    seq.push_item(item)?;
    let _ = db.log_activity("queue_item_added", "Added a text clip to the Queue");
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[cfg(target_os = "macos")]
pub fn simulate_cmd_v_paste() -> Result<(), String> {
    use std::process::Command;
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .output()
        .map_err(|error| format!("Could not start macOS paste automation: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(
            if detail.contains("not authorized") || detail.contains("-1743") {
                "macOS blocked Paste Next. Allow Accessibility access for Pasted (or the terminal/IDE running this development build), then try again.".to_string()
            } else if detail.is_empty() {
                "macOS rejected the simulated paste. Check Pasted's Accessibility permission."
                    .to_string()
            } else {
                format!("macOS rejected the simulated paste: {detail}")
            },
        )
    }
}

#[cfg(target_os = "windows")]
pub fn simulate_cmd_v_paste() -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("powershell")
        .arg("-Command")
        .arg("$wshell = New-Object -ComObject wscript.shell; $wshell.SendKeys('^v')")
        .status()
        .map_err(|error| format!("Could not start Windows paste automation: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "Windows rejected the simulated paste".to_string())
}

#[cfg(target_os = "linux")]
pub fn simulate_cmd_v_paste() -> Result<(), String> {
    use std::process::Command;
    let status = Command::new("xdotool")
        .arg("key")
        .arg("ctrl+v")
        .status()
        .map_err(|error| format!("Could not start Linux paste automation: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "Linux rejected the simulated paste".to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
pub fn simulate_cmd_v_paste() -> Result<(), String> {
    Err("Paste automation is unavailable on this platform".to_string())
}

fn ensure_paste_automation_available() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    if !check_accessibility_permission().is_trusted {
        return Err("Paste Next needs Accessibility access. Allow Pasted (or the terminal/IDE running this development build) in System Settings, then try again.".to_string());
    }
    Ok(())
}

fn restore_main_window_after_queue_failure(app: &AppHandle) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

fn restore_main_window_after_ui_paste(app: &AppHandle) {
    // Give the destination control time to process Command/Ctrl+V before
    // Pasted takes focus back for continued Queue management.
    thread::sleep(Duration::from_millis(220));
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
}

pub(crate) fn paste_queue_item(
    seq: &SequentialQueueState,
    db: &DbState,
    app: &AppHandle,
    index: usize,
    restore_after_success: bool,
) -> Result<Option<String>, String> {
    let Some((item_id, text)) = seq.peek_item(index) else {
        return Ok(None);
    };
    if let Err(error) = ensure_paste_automation_available() {
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }
    let paste_target = app.state::<Arc<crate::paste_target::PasteTargetState>>();
    let target = match paste_target.prepare_last_external() {
        Ok(target) => target,
        Err(error) => {
            let _ = db.log_activity("queue_paste_failed", &error);
            return Err(error);
        }
    };
    seq.mark_internal_clipboard_write(&text);
    let mut clipboard = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            seq.clear_internal_clipboard_write();
            let message = format!("Could not access the clipboard for Queue paste: {error}");
            let _ = db.log_activity("queue_paste_failed", &message);
            return Err(message);
        }
    };
    if let Err(error) = clipboard.set_text(&text) {
        seq.clear_internal_clipboard_write();
        let message = format!("Could not place the next Queue item on the clipboard: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }

    if let Err(error) = paste_target.paste_to(&target) {
        seq.clear_internal_clipboard_write();
        restore_main_window_after_queue_failure(app);
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }

    if let Err(error) = seq.consume_item(item_id) {
        seq.clear_internal_clipboard_write();
        restore_main_window_after_queue_failure(app);
        let message =
            format!("The Queue item was copied but could not be committed as pasted: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    let _ = db.log_activity(
        "queue_item_pasted",
        &format!(
            "Pasted the next Queue item ({} remaining)",
            status.total_count
        ),
    );
    if restore_after_success {
        restore_main_window_after_ui_paste(app);
    }
    Ok(Some(text))
}

pub(crate) fn paste_next_queue_item(
    seq: &SequentialQueueState,
    db: &DbState,
    app: &AppHandle,
) -> Result<Option<String>, String> {
    paste_queue_item(seq, db, app, 0, false)
}

#[tauri::command]
pub fn pop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    paste_queue_item(&seq, &db, &app, 0, true)
}

#[tauri::command]
pub fn paste_sequential_item_by_index(
    index: usize,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    paste_queue_item(&seq, &db, &app, index, true)
}

#[tauri::command]
pub fn remove_sequential_item_by_index(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
    index: usize,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    if seq.remove_item_by_index(index).is_some() {
        let _ = db.log_activity("queue_item_removed", "Removed an item from the Queue");
    }
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn reorder_sequential_items(
    item_ids: Vec<u64>,
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    seq.reorder_items(&item_ids)?;
    let _ = db.log_activity("queue_reordered", "Reordered the Queue");
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn stop_sequential_paste(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<SequentialStatus, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    seq.stop_queue();
    let _ = db.log_activity(
        "queue_recording_stopped",
        "Stopped recording copies into the Queue",
    );
    let status = seq.get_status();
    let _ = app.emit("sequential-updated", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn paste_all_sequential(
    seq: State<'_, Arc<SequentialQueueState>>,
    app: AppHandle,
) -> Result<Option<String>, String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Queue)?;
    paste_all_queue_items(&seq, &db, &app)
}

pub(crate) fn paste_all_queue_items(
    seq: &SequentialQueueState,
    db: &DbState,
    app: &AppHandle,
) -> Result<Option<String>, String> {
    let status = seq.get_status();
    if status.queue.is_empty() {
        return Ok(None);
    }
    if let Err(error) = ensure_paste_automation_available() {
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }
    let paste_target = app.state::<Arc<crate::paste_target::PasteTargetState>>();
    let target = match paste_target.prepare_last_external() {
        Ok(target) => target,
        Err(error) => {
            let _ = db.log_activity("queue_paste_failed", &error);
            return Err(error);
        }
    };
    let combined = status.queue.join("\n\n");
    seq.mark_internal_clipboard_write(&combined);
    let mut cb = match Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(error) => {
            seq.clear_internal_clipboard_write();
            let message = format!("Could not access the clipboard for Queue paste: {error}");
            let _ = db.log_activity("queue_paste_failed", &message);
            return Err(message);
        }
    };
    if let Err(error) = cb.set_text(&combined) {
        seq.clear_internal_clipboard_write();
        let message = format!("Could not place the Queue on the clipboard: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }
    if let Err(error) = paste_target.paste_to(&target) {
        seq.clear_internal_clipboard_write();
        restore_main_window_after_queue_failure(app);
        let _ = db.log_activity("queue_paste_failed", &error);
        return Err(error);
    }
    if let Err(error) = seq.consume_prefix(&status.item_ids) {
        seq.clear_internal_clipboard_write();
        restore_main_window_after_queue_failure(app);
        let message = format!("The Queue pasted but could not be cleared: {error}");
        let _ = db.log_activity("queue_paste_failed", &message);
        return Err(message);
    }
    let updated = seq.get_status();
    let _ = app.emit("sequential-updated", updated);
    let _ = db.log_activity(
        "queue_all_pasted",
        &format!("Pasted {} Queue items together", status.total_count),
    );
    restore_main_window_after_ui_paste(app);

    Ok(Some(combined))
}

#[tauri::command]
pub fn get_sequential_status(
    seq: State<'_, Arc<SequentialQueueState>>,
) -> Result<SequentialStatus, String> {
    Ok(seq.get_status())
}

#[tauri::command]
pub fn get_queue_paste_target(app: AppHandle) -> crate::paste_target::PasteTarget {
    app.state::<Arc<crate::paste_target::PasteTargetState>>()
        .snapshot()
}

// Window & Activation Policy Commands
#[tauri::command]
pub fn toggle_hud_window(app: AppHandle) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    features::require(&db, Feature::Hud)?;
    if let Some(window) = app.get_webview_window("hud") {
        let is_vis = window.is_visible().unwrap_or(false);
        if is_vis {
            let _ = window.hide();
        } else {
            #[cfg(target_os = "macos")]
            let mut pos_payload: Option<serde_json::Value> = None;

            #[cfg(target_os = "macos")]
            {
                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalPoint {
                    x: f64,
                    y: f64,
                }

                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalSize {
                    width: f64,
                    height: f64,
                }

                #[repr(C)]
                #[derive(Copy, Clone, Debug)]
                struct LocalRect {
                    origin: LocalPoint,
                    size: LocalSize,
                }

                use objc::runtime::{Class, Object};
                use objc::{msg_send, sel, sel_impl};

                unsafe {
                    if let Some(event_class) = Class::get("NSEvent") {
                        let loc: LocalPoint = msg_send![event_class, mouseLocation];

                        let screens_class = Class::get("NSScreen");
                        if let Some(screens_cls) = screens_class {
                            let screens_array: *mut Object = msg_send![screens_cls, screens];
                            let screen_count: usize = msg_send![screens_array, count];

                            let mut target_screen: Option<*mut Object> = None;
                            let mut primary_height = 1080.0;

                            if screen_count > 0 {
                                let first_screen: *mut Object =
                                    msg_send![screens_array, objectAtIndex: 0usize];
                                let first_frame: LocalRect = msg_send![first_screen, frame];
                                primary_height = first_frame.size.height;
                            }

                            for i in 0..screen_count {
                                let screen: *mut Object =
                                    msg_send![screens_array, objectAtIndex: i];
                                let frame: LocalRect = msg_send![screen, frame];
                                if loc.x >= frame.origin.x
                                    && loc.x <= frame.origin.x + frame.size.width
                                    && loc.y >= frame.origin.y
                                    && loc.y <= frame.origin.y + frame.size.height
                                {
                                    target_screen = Some(screen);
                                    break;
                                }
                            }

                            let active_screen =
                                target_screen.unwrap_or_else(|| msg_send![screens_cls, mainScreen]);

                            if !active_screen.is_null() {
                                let vis_frame: LocalRect = msg_send![active_screen, visibleFrame];

                                let mouse_top_y = primary_height - loc.y;
                                let vis_top =
                                    primary_height - (vis_frame.origin.y + vis_frame.size.height);
                                let vis_bottom = primary_height - vis_frame.origin.y;
                                let vis_left = vis_frame.origin.x;
                                let vis_right = vis_frame.origin.x + vis_frame.size.width;

                                let hud_width = 360.0;
                                let hud_height = 440.0;

                                // Horizontal positioning (centered on cursor) & clamping
                                let mut target_x = loc.x - (hud_width / 2.0);
                                target_x = target_x.clamp(
                                    vis_left + 8.0,
                                    (vis_right - hud_width - 8.0).max(vis_left + 8.0),
                                );

                                // Vertical positioning & dynamic flip if near bottom edge
                                let mut target_y = mouse_top_y + 8.0;
                                if target_y + hud_height > vis_bottom - 8.0 {
                                    target_y = mouse_top_y - hud_height - 8.0;
                                }
                                target_y = target_y.clamp(
                                    vis_top + 8.0,
                                    (vis_bottom - hud_height - 8.0).max(vis_top + 8.0),
                                );

                                let is_flipped = target_y < mouse_top_y;
                                let payload = serde_json::json!({
                                    "flipped": is_flipped,
                                    "cursorX": loc.x,
                                    "cursorY": mouse_top_y,
                                    "targetX": target_x,
                                    "targetY": target_y
                                });
                                let _ = window.emit("hud_position_updated", payload.clone());
                                pos_payload = Some(payload);

                                if let Ok(ns_win_ptr) = window.ns_window() {
                                    let ns_win = ns_win_ptr as *mut Object;
                                    let _: () = msg_send![ns_win, setHasShadow: 0i8];
                                    let _: () = msg_send![ns_win, setAlphaValue: 0.0f64];
                                    let cocoa_y = primary_height - target_y - hud_height;
                                    let origin = LocalPoint {
                                        x: target_x,
                                        y: cocoa_y,
                                    };
                                    let _: () = msg_send![ns_win, setFrameOrigin: origin];
                                }

                                let _ = window.set_position(tauri::Position::Logical(
                                    tauri::LogicalPosition {
                                        x: target_x,
                                        y: target_y,
                                    },
                                ));
                            }
                        }
                    }
                }
            }

            let _ = window.show();
            let _ = window.set_focus();
            #[cfg(target_os = "macos")]
            {
                if let Ok(ns_win_ptr) = window.ns_window() {
                    use objc::runtime::Object;
                    use objc::{msg_send, sel, sel_impl};
                    unsafe {
                        let ns_win = ns_win_ptr as *mut Object;
                        let _: () = msg_send![ns_win, setAlphaValue: 1.0f64];
                    }
                }
                if let Some(payload) = pos_payload {
                    let _ = window.emit("hud_position_updated", payload);
                }
            }
        }
    } else {
        return Err("HUD window is unavailable".to_string());
    }
    Ok(())
}

#[tauri::command]
pub fn paste_clip_by_id(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
    app: AppHandle,
) -> Result<(), String> {
    paste_clip_from_hud(&db, &app, clip_id)
}

pub(crate) fn paste_clip_from_hud(
    db: &DbState,
    app: &AppHandle,
    clip_id: i64,
) -> Result<(), String> {
    features::require(db, Feature::Hud)?;
    paste_clip_to_last_external(db, app, clip_id)
}

pub(crate) fn paste_clip_to_last_external(
    db: &DbState,
    app: &AppHandle,
    clip_id: i64,
) -> Result<(), String> {
    let clip = db
        .get_clip_by_id(clip_id)
        .map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    if !check_accessibility_permission().is_trusted {
        return Err("HUD paste needs Accessibility access. Allow Pasted (or the terminal/IDE running this development build) in System Settings, then try again.".to_string());
    }

    let paste_target = app.state::<Arc<crate::paste_target::PasteTargetState>>();
    let target = paste_target.prepare_last_external_for_hud()?;
    let internal_fingerprint = clip_internal_clipboard_fingerprint(&clip)?;
    let mut clipboard = Clipboard::new()
        .map_err(|_| "The system clipboard is unavailable right now.".to_string())?;
    let sequential = app.state::<Arc<SequentialQueueState>>();
    sequential.mark_internal_clipboard_write(&internal_fingerprint);
    if let Err(error) = write_clip_to_clipboard(&mut clipboard, &clip) {
        sequential.clear_internal_clipboard_write();
        let explanation = match clip.content_type.as_str() {
            "file" => "This clip contains unavailable files.",
            "image" => "This clip's image cannot be prepared for pasting.",
            _ => "This clip's text cannot be prepared for pasting.",
        };
        let _ = db.log_activity(
            "hud_paste_failed",
            &format!("{explanation} System detail: {error}"),
        );
        return Err(explanation.to_string());
    }

    if let Some(hud) = app.get_webview_window("hud") {
        let _ = hud.hide();
    }
    if let Err(error) = paste_target.paste_clip_to(&target) {
        if let Some(hud) = app.get_webview_window("hud") {
            let _ = hud.show();
            let _ = hud.set_focus();
        }
        let _ = db.log_activity("hud_paste_failed", &error);
        return Err(error);
    }

    let _ = db.log_activity(
        "hud_clip_pasted",
        &format!("Pasted clip {} into {} from HUD", clip.id, target.name),
    );
    Ok(())
}

#[tauri::command]
pub fn toggle_clip_protected(clip_id: i64, db: State<'_, Arc<DbState>>) -> Result<bool, String> {
    features::require(&db, Feature::Protection)?;
    db.toggle_protected(clip_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn batch_protect_clips(
    ids: Vec<i64>,
    protected_state: bool,
    db: State<'_, Arc<DbState>>,
) -> Result<ClipMutationSummary, String> {
    features::require(&db, Feature::Protection)?;
    db.batch_protect_clips(ids, protected_state)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trash_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.trash_unpinned_clips().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn purge_unpinned_clips(db: State<'_, Arc<DbState>>) -> Result<(), String> {
    db.purge_unpinned_clips().map_err(|e| e.to_string())
}

fn get_dvorak_code_for_char(ch: char) -> Option<tauri_plugin_global_shortcut::Code> {
    use tauri_plugin_global_shortcut::Code;

    match ch.to_ascii_uppercase() {
        'A' => Some(Code::KeyA),
        'B' => Some(Code::KeyN),
        'C' => Some(Code::KeyI),
        'D' => Some(Code::KeyH),
        'E' => Some(Code::KeyD),
        'F' => Some(Code::KeyW),
        'G' => Some(Code::KeyE),
        'H' => Some(Code::KeyJ),
        'I' => Some(Code::KeyG),
        'J' => Some(Code::KeyP),
        'K' => Some(Code::BracketLeft),
        'L' => Some(Code::KeyU),
        'M' => Some(Code::KeyM),
        'N' => Some(Code::KeyL),
        'O' => Some(Code::KeyS),
        'P' => Some(Code::KeyR),
        'Q' => Some(Code::KeyO),
        'R' => Some(Code::KeyY),
        'S' => Some(Code::Semicolon),
        'T' => Some(Code::KeyK),
        'U' => Some(Code::KeyF),
        'V' => Some(Code::Period),
        'W' => Some(Code::Comma),
        'X' => Some(Code::KeyQ),
        'Y' => Some(Code::KeyT),
        'Z' => Some(Code::Slash),
        '1' => Some(Code::Digit1),
        '2' => Some(Code::Digit2),
        '3' => Some(Code::Digit3),
        '4' => Some(Code::Digit4),
        '5' => Some(Code::Digit5),
        '6' => Some(Code::Digit6),
        '7' => Some(Code::Digit7),
        '8' => Some(Code::Digit8),
        '9' => Some(Code::Digit9),
        '0' => Some(Code::Digit0),
        '`' => Some(Code::Backquote),
        _ => None,
    }
}

fn normalize_shortcut_aliases(shortcut: &str) -> String {
    shortcut
        .replace("CmdOrCtrl", "Super")
        .replace("Command", "Super")
        .replace("Cmd", "Super")
        .replace("Option", "Alt")
        .replace("Control", "Ctrl")
        .replace(['ç', 'Ç'], "C")
        .replace(['√', '◊'], "V")
        .replace(['µ', 'Â'], "M")
        .replace('≈', "X")
        .replace('ß', "S")
        .replace('∂', "D")
        .replace('ƒ', "F")
        .replace('©', "G")
        .replace('®', "R")
        .replace('†', "T")
        .replace('¥', "Y")
        .replace(['ø', 'Ø'], "O")
        .replace(['π', '∏'], "P")
        .replace(['å', 'Å'], "A")
        .replace('∫', "B")
        .replace('∆', "J")
        .replace('˚', "K")
        .replace('¬', "L")
        .replace('Ω', "Z")
        .replace('œ', "Q")
        .replace('∑', "W")
}

pub fn parse_shortcut_str(sc_str: &str) -> Option<tauri_plugin_global_shortcut::Shortcut> {
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::Shortcut;

    let s = sc_str.trim();
    if s.is_empty() {
        return None;
    }

    if let Ok(sc) = Shortcut::from_str(s) {
        return Some(sc);
    }

    let clean = normalize_shortcut_aliases(s);

    if let Ok(sc) = Shortcut::from_str(&clean) {
        return Some(sc);
    }

    let parts: Vec<&str> = clean.split('+').collect();
    if let Some(last) = parts.last() {
        let last_trim = last.trim();
        if last_trim.len() == 1 && last_trim.chars().next().unwrap().is_ascii_alphabetic() {
            let key_str = format!("Key{}", last_trim.to_ascii_uppercase());
            let converted = format!("{}+{}", parts[..parts.len() - 1].join("+"), key_str);
            if let Ok(sc) = Shortcut::from_str(&converted) {
                return Some(sc);
            }
        }
        if last_trim.len() == 1 && last_trim.chars().next().unwrap().is_ascii_digit() {
            let key_str = format!("Digit{}", last_trim);
            let converted = format!("{}+{}", parts[..parts.len() - 1].join("+"), key_str);
            if let Ok(sc) = Shortcut::from_str(&converted) {
                return Some(sc);
            }
        }
    }

    None
}

pub fn parse_shortcut_str_all_layouts(
    sc_str: &str,
) -> Option<Vec<tauri_plugin_global_shortcut::Shortcut>> {
    use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

    let s = sc_str.trim();
    if s.is_empty() {
        return None;
    }

    let clean = normalize_shortcut_aliases(s);

    let mut shortcuts = Vec::new();

    if let Some(sc) = parse_shortcut_str(&clean) {
        shortcuts.push(sc);
    }

    let parts: Vec<&str> = clean.split('+').collect();
    if let Some(last) = parts.last() {
        let last_trim = last.trim();
        if last_trim.len() == 1 {
            let ch = last_trim.chars().next().unwrap();
            let mut mods = Modifiers::empty();
            for m in &parts[..parts.len() - 1] {
                match m.trim() {
                    "Super" => mods |= Modifiers::SUPER,
                    "Alt" => mods |= Modifiers::ALT,
                    "Ctrl" => mods |= Modifiers::CONTROL,
                    "Shift" => mods |= Modifiers::SHIFT,
                    _ => {}
                }
            }

            if let Some(dvorak_code) = get_dvorak_code_for_char(ch) {
                let dvorak_sc = Shortcut::new(Some(mods), dvorak_code);
                if !shortcuts.contains(&dvorak_sc) {
                    shortcuts.push(dvorak_sc);
                }
            }
        }
    }

    if shortcuts.is_empty() {
        None
    } else {
        Some(shortcuts)
    }
}

pub fn register_all_app_shortcuts(app: &AppHandle) -> Result<(), String> {
    if let Some(mgr) = app.try_state::<Arc<crate::hotkey_manager::HotkeyManager>>() {
        mgr.register_all(app)
    } else {
        Err("HotkeyManager state not initialized".to_string())
    }
}

#[derive(serde::Serialize, Clone)]
pub struct AccessibilityStatus {
    pub is_trusted: bool,
    pub is_dev_mode: bool,
}

pub fn check_accessibility_permission() -> AccessibilityStatus {
    let is_trusted = {
        #[cfg(target_os = "macos")]
        {
            use std::ptr;
            #[link(name = "ApplicationServices", kind = "framework")]
            extern "C" {
                fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
            }
            unsafe { AXIsProcessTrustedWithOptions(ptr::null()) }
        }
        #[cfg(not(target_os = "macos"))]
        true
    };

    let is_dev_mode = cfg!(debug_assertions);

    AccessibilityStatus {
        is_trusted,
        is_dev_mode,
    }
}

#[derive(serde::Serialize)]
pub struct HotkeyCapabilityStatus {
    pub platform: String,
    pub backend: String,
    pub state: String,
    pub is_trusted: bool,
    pub is_dev_mode: bool,
    pub configured_count: usize,
    pub registered_count: usize,
    pub issues: Vec<crate::hotkey_manager::HotkeyRegistrationIssue>,
}

#[tauri::command]
pub fn get_hotkey_capability_status(app: AppHandle) -> HotkeyCapabilityStatus {
    let accessibility = check_accessibility_permission();
    let registration = app
        .try_state::<Arc<crate::hotkey_manager::HotkeyManager>>()
        .map(|manager| manager.registration_status())
        .unwrap_or_default();
    let platform = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unsupported"
    };

    HotkeyCapabilityStatus {
        platform: platform.into(),
        backend: registration.backend,
        state: registration.state,
        is_trusted: accessibility.is_trusted,
        is_dev_mode: accessibility.is_dev_mode,
        configured_count: registration.configured_count,
        registered_count: registration.registered_count,
        issues: registration.issues,
    }
}

#[tauri::command]
pub fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_Accessibility")
            .spawn();

        let status = check_accessibility_permission();
        status.is_trusted
    }
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .arg("/c")
            .arg("start ms-settings:privacy-accessibility")
            .spawn();
        true
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        let _ = Command::new("gnome-control-center").spawn();
        true
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    true
}

#[tauri::command]
pub fn register_app_setting_hotkey(
    key: String,
    value: String,
    app: AppHandle,
) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    db.save_setting(&key, &value)
        .map_err(|error| error.to_string())?;
    register_all_app_shortcuts(&app)
}

#[tauri::command]
pub fn register_hud_shortcut(shortcut_str: String, app: AppHandle) -> Result<(), String> {
    let db = app.state::<Arc<DbState>>();
    db.save_setting("hudHotkey", &shortcut_str)
        .map_err(|error| error.to_string())?;
    register_all_app_shortcuts(&app)
}

#[tauri::command]
pub fn set_dock_visibility(show_dock: bool, app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use tauri::ActivationPolicy;
        if show_dock {
            let _ = app.set_activation_policy(ActivationPolicy::Regular);
        } else {
            let _ = app.set_activation_policy(ActivationPolicy::Accessory);
        }
    }
    let _ = show_dock;
    let _ = app;
    Ok(())
}

#[tauri::command]
pub fn open_emoji_picker() -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \" \" using {control down, command down}")
            .spawn()
            .is_ok()
    }

    #[cfg(not(target_os = "macos"))]
    false
}

#[cfg(target_os = "macos")]
fn macos_application_icon_data_url(application_name: &str) -> Option<String> {
    fn application_bundle_path(name: &str) -> Option<PathBuf> {
        let mut roots = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
            PathBuf::from("/System/Applications/Utilities"),
            PathBuf::from("/System/Library/CoreServices"),
        ];
        if let Some(home) = dirs::home_dir() {
            roots.insert(0, home.join("Applications"));
        }
        for root in &roots {
            let direct = root.join(format!("{name}.app"));
            if direct.is_dir() {
                return Some(direct);
            }
        }
        for root in roots {
            let Ok(entries) = std::fs::read_dir(root) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok).take(512) {
                let path = entry.path();
                if path.extension().is_some_and(|extension| extension == "app")
                    && path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .is_some_and(|stem| stem.eq_ignore_ascii_case(name))
                {
                    return Some(path);
                }
            }
        }
        None
    }

    fn bundle_icon_path(bundle: &std::path::Path) -> Option<PathBuf> {
        let resources = bundle.join("Contents/Resources");
        let info = plist::Value::from_file(bundle.join("Contents/Info.plist")).ok()?;
        if let Some(icon_name) = info
            .as_dictionary()
            .and_then(|dictionary| dictionary.get("CFBundleIconFile"))
            .and_then(plist::Value::as_string)
        {
            let mut icon = resources.join(icon_name);
            if icon.extension().is_none() {
                icon.set_extension("icns");
            }
            if icon.is_file() {
                return Some(icon);
            }
        }
        std::fs::read_dir(resources)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "icns")
            })
    }

    let candidates = match application_name {
        "VS Code" => vec!["Visual Studio Code", "VS Code"],
        "System Clipboard" | "Unknown" | "Unknown Source" => Vec::new(),
        name => vec![name],
    };
    static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    for icon_path in candidates
        .into_iter()
        .filter_map(application_bundle_path)
        .filter_map(|bundle| bundle_icon_path(&bundle))
    {
        let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let output_path = std::env::temp_dir().join(format!(
            "pasted-source-icon-{}-{sequence}.png",
            std::process::id(),
        ));
        let status = std::process::Command::new("/usr/bin/sips")
            .args(["-s", "format", "png", "--resampleWidth", "32"])
            .arg(&icon_path)
            .arg("--out")
            .arg(&output_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok();
        let png = status
            .filter(std::process::ExitStatus::success)
            .and_then(|_| std::fs::read(&output_path).ok());
        let _ = std::fs::remove_file(&output_path);
        if let Some(png) = png.filter(|bytes| !bytes.is_empty() && bytes.len() <= 512 * 1024) {
            return Some(format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(png),
            ));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn linux_application_icon_data_url(application_name: &str) -> Option<String> {
    use gtk::gdk_pixbuf::Pixbuf;

    fn desktop_files(root: &std::path::Path) -> Vec<PathBuf> {
        std::fs::read_dir(root)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "desktop")
            })
            .take(2048)
            .collect()
    }

    fn find_icon_file(icon: &str) -> Option<PathBuf> {
        let direct = PathBuf::from(icon);
        if direct.is_absolute() && direct.is_file() {
            return Some(direct);
        }
        static ICON_FILES: once_cell::sync::Lazy<std::collections::HashMap<String, PathBuf>> =
            once_cell::sync::Lazy::new(|| {
                let mut roots = vec![
                    PathBuf::from("/usr/share/icons"),
                    PathBuf::from("/usr/share/pixmaps"),
                ];
                if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
                    roots.insert(0, PathBuf::from(data_home).join("icons"));
                } else if let Some(home) = dirs::home_dir() {
                    roots.insert(0, home.join(".local/share/icons"));
                }
                let mut files = std::collections::HashMap::new();
                let mut pending = roots;
                let mut visited = 0usize;
                while let Some(directory) = pending.pop() {
                    let Ok(entries) = std::fs::read_dir(directory) else {
                        continue;
                    };
                    for entry in entries.filter_map(Result::ok) {
                        visited += 1;
                        if visited > 50_000 {
                            return files;
                        }
                        let path = entry.path();
                        if path.is_dir() {
                            pending.push(path);
                        } else if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                            if matches!(
                                path.extension().and_then(|extension| extension.to_str()),
                                Some("png" | "svg" | "xpm")
                            ) {
                                files.entry(name.to_string()).or_insert(path);
                            }
                        }
                    }
                }
                files
            });
        [
            format!("{icon}.png"),
            format!("{icon}.svg"),
            format!("{icon}.xpm"),
        ]
        .iter()
        .find_map(|candidate| ICON_FILES.get(candidate).cloned())
    }

    let mut roots = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        roots.insert(0, PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = dirs::home_dir() {
        roots.insert(0, home.join(".local/share/applications"));
    }
    let source = application_name.trim().to_lowercase();
    if source.is_empty()
        || matches!(
            source.as_str(),
            "system clipboard" | "unknown" | "unknown source"
        )
    {
        return None;
    }
    let mut partial_match = None;
    for path in roots.iter().flat_map(|root| desktop_files(root)) {
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > 256 * 1024 {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(path) else {
            continue;
        };
        let name = contents.lines().find_map(|line| line.strip_prefix("Name="));
        let icon = contents.lines().find_map(|line| line.strip_prefix("Icon="));
        let (Some(name), Some(icon)) = (name, icon) else {
            continue;
        };
        let normalized = name.trim().to_lowercase();
        if normalized == source {
            partial_match = Some(icon.to_string());
            break;
        }
        if partial_match.is_none() && (source.contains(&normalized) || normalized.contains(&source))
        {
            partial_match = Some(icon.to_string());
        }
    }
    let icon_path = find_icon_file(partial_match?.trim())?;
    let pixbuf = Pixbuf::from_file_at_scale(icon_path, 32, 32, true).ok()?;
    let png = pixbuf.save_to_bufferv("png", &[]).ok()?;
    (png.len() <= 512 * 1024).then(|| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    })
}

#[cfg(target_os = "windows")]
fn windows_application_icons(
    sources: &[String],
) -> Result<std::collections::HashMap<String, String>, String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.Drawing
$names = [Console]::In.ReadToEnd() | ConvertFrom-Json
$roots = @(
  "$env:APPDATA\Microsoft\Windows\Start Menu\Programs",
  "$env:ProgramData\Microsoft\Windows\Start Menu\Programs"
)
$links = @(Get-ChildItem -Path $roots -Filter '*.lnk' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 4096)
$shell = New-Object -ComObject WScript.Shell
$result = @{}
foreach ($name in $names) {
  $link = $links | Where-Object { $_.BaseName -ieq $name } | Select-Object -First 1
  if ($null -eq $link) {
    $link = $links | Where-Object { $name -like ('*' + $_.BaseName + '*') -or $_.BaseName -like ('*' + $name + '*') } | Select-Object -First 1
  }
  if ($null -eq $link) { continue }
  $target = $shell.CreateShortcut($link.FullName).TargetPath
  if ([string]::IsNullOrWhiteSpace($target) -or -not (Test-Path -LiteralPath $target)) { continue }
  $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($target)
  if ($null -eq $icon) { continue }
  $bitmap = New-Object System.Drawing.Bitmap($icon.ToBitmap(), 32, 32)
  $stream = New-Object System.IO.MemoryStream
  $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
  $result[$name] = 'data:image/png;base64,' + [Convert]::ToBase64String($stream.ToArray())
  $stream.Dispose(); $bitmap.Dispose(); $icon.Dispose()
}
$result | ConvertTo-Json -Compress
"#;
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start the Windows icon resolver: {error}"))?;
    let input = serde_json::to_vec(sources).map_err(|error| error.to_string())?;
    child
        .stdin
        .take()
        .ok_or("Windows icon resolver input was unavailable.")?
        .write_all(&input)
        .map_err(|error| error.to_string())?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() || output.stdout.len() > 8 * 1024 * 1024 {
        return Ok(std::collections::HashMap::new());
    }
    serde_json::from_slice(&output.stdout).or_else(|_| Ok(std::collections::HashMap::new()))
}

static SOURCE_ICON_CACHE: once_cell::sync::Lazy<
    parking_lot::Mutex<std::collections::HashMap<String, String>>,
> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(std::collections::HashMap::new()));

fn cache_resolved_source_icons(
    mut existing: std::collections::HashMap<String, String>,
    resolved: std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    let mut cache = SOURCE_ICON_CACHE.lock();
    for (name, icon) in resolved {
        if crate::resource_limits::validate_raster_data_url(&icon).is_err() {
            continue;
        }
        cache.insert(name.clone(), icon.clone());
        existing.insert(name, icon);
    }
    existing
}

#[tauri::command]
pub async fn get_source_icons(
    sources: Vec<String>,
    app: AppHandle,
) -> Result<std::collections::HashMap<String, String>, String> {
    if sources.len() > 128 || sources.iter().any(|name| name.len() > 256) {
        return Err("Source icon request exceeds the supported limit.".to_string());
    }
    let (cached_icons, uncached_sources) = {
        let cache = SOURCE_ICON_CACHE.lock();
        let cached = sources
            .iter()
            .filter_map(|name| cache.get(name).cloned().map(|icon| (name.clone(), icon)))
            .collect::<std::collections::HashMap<_, _>>();
        let uncached = sources
            .into_iter()
            .filter(|name| !cache.contains_key(name))
            .collect::<Vec<_>>();
        (cached, uncached)
    };
    if uncached_sources.is_empty() {
        return Ok(cached_icons);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = app;
        let resolved = tauri::async_runtime::spawn_blocking(move || {
            uncached_sources
                .into_iter()
                .filter_map(|name| macos_application_icon_data_url(&name).map(|icon| (name, icon)))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .await
        .map_err(|error| error.to_string())?;
        Ok(cache_resolved_source_icons(cached_icons, resolved))
    }

    #[cfg(target_os = "linux")]
    {
        let _ = app;
        let resolved = tauri::async_runtime::spawn_blocking(move || {
            uncached_sources
                .into_iter()
                .filter_map(|name| linux_application_icon_data_url(&name).map(|icon| (name, icon)))
                .collect()
        })
        .await
        .map_err(|error| error.to_string())?;
        Ok(cache_resolved_source_icons(cached_icons, resolved))
    }

    #[cfg(target_os = "windows")]
    {
        let _ = app;
        let resolved = tauri::async_runtime::spawn_blocking(move || {
            windows_application_icons(&uncached_sources)
        })
        .await
        .map_err(|error| error.to_string())??;
        Ok(cache_resolved_source_icons(cached_icons, resolved))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (uncached_sources, app);
        Ok(cached_icons)
    }
}

#[tauri::command]
pub fn get_installed_applications(db: State<'_, Arc<DbState>>) -> Result<Vec<String>, String> {
    let mut apps = std::collections::BTreeSet::new();

    if let Ok(history_apps) = db.get_distinct_sources() {
        for app in history_apps {
            if !app.trim().is_empty() {
                apps.insert(app);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let dirs = [
            "/Applications",
            "/System/Applications",
            "/System/Applications/Utilities",
        ];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "app") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            apps.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let dirs = ["/usr/share/applications", "/usr/local/share/applications"];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "desktop") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            let clean_name = name.trim_end_matches(".desktop");
                            apps.insert(clean_name.to_string());
                        }
                    }
                }
            }
        }
    }

    let common = [
        "1Password",
        "Bitwarden",
        "Safari",
        "Google Chrome",
        "Firefox",
        "Slack",
        "Signal",
        "Telegram",
        "VS Code",
        "Terminal",
        "Warp",
        "Xcode",
        "Discord",
        "Keychain Access",
        "Passwords",
    ];
    for c in &common {
        apps.insert(c.to_string());
    }

    Ok(apps.into_iter().collect())
}

#[tauri::command]
pub fn extract_ocr_from_clip(
    clip_id: i64,
    db: State<'_, Arc<DbState>>,
) -> Result<crate::extraction_execution::ExtractionApplicationResult, String> {
    features::require(&db, Feature::Ocr)?;
    let extractor = db
        .active_image_text_extractor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No available image text Extractor is enabled".to_string())?;
    let clip = db.get_clip_by_id(clip_id).map_err(|e| e.to_string())?;

    let image = clip
        .image_base64
        .as_deref()
        .ok_or_else(|| "Clip has no extractable image data".to_string())?;
    let bytes = crate::ocr::decode_stored_image(image)
        .ok_or_else(|| "Clip has no extractable image data".to_string())?;
    let detectors = features::is_enabled(&db, Feature::ContentDetection)
        .then(|| db.get_content_detectors().ok())
        .flatten();
    let analysis =
        crate::extraction_execution::analyze_image(bytes, &extractor, detectors.as_deref());
    crate::extraction_execution::apply_image_analysis(
        &db,
        clip_id,
        &clip.content_hash,
        &extractor,
        detectors.is_some(),
        analysis,
    )
    .map_err(|error| match error {
        rusqlite::Error::InvalidParameterName(message) => message,
        error => error.to_string(),
    })
}

#[tauri::command]
pub fn get_ocr_backfill_status(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::OcrBackfillStatus, String> {
    db.get_ocr_backfill_status()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn start_ocr_backfill(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<(), String> {
    features::require(&db, Feature::Ocr)?;
    if db
        .active_image_text_extractor()
        .map_err(|error| error.to_string())?
        .is_none()
    {
        return Err("No available image text Extractor is enabled".to_string());
    }
    ocr.start_backfill()
}

#[tauri::command]
pub fn cancel_ocr_backfill(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<(), String> {
    features::require(&db, Feature::Ocr)?;
    ocr.cancel();
    Ok(())
}

#[tauri::command]
pub fn retry_failed_ocr(
    db: State<'_, Arc<DbState>>,
    ocr: State<'_, Arc<crate::ocr::OcrService>>,
) -> Result<usize, String> {
    features::require(&db, Feature::Ocr)?;
    let count = db.reset_failed_ocr().map_err(|error| error.to_string())?;
    if count > 0 {
        ocr.start_backfill()?;
    }
    Ok(count)
}

#[tauri::command]
pub fn toggle_clipboard_pause(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
    db: State<'_, Arc<DbState>>,
) -> Result<bool, String> {
    let current = monitor_state
        .is_manually_paused
        .load(std::sync::atomic::Ordering::Relaxed);
    let new_val = !current;
    monitor_state
        .is_manually_paused
        .store(new_val, std::sync::atomic::Ordering::Relaxed);

    if new_val {
        let _ = db.log_activity(
            "recording_manually_paused",
            "Clipboard recording manually paused",
        );
    } else {
        let _ = db.log_activity(
            "recording_manually_resumed",
            "Clipboard recording manually resumed",
        );
    }

    Ok(monitor_state.is_paused())
}

#[tauri::command]
pub fn is_clipboard_paused(
    monitor_state: State<'_, Arc<crate::clipboard_monitor::ClipboardMonitorState>>,
) -> Result<bool, String> {
    Ok(monitor_state.is_paused())
}

#[tauri::command]
pub fn export_clips_json(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db.export_clips_json().map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Clips as JSON");
    Ok(exported)
}

#[cfg(test)]
fn csv_cell(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    let neutralized = if matches!(
        value.chars().next(),
        Some('=' | '+' | '-' | '@' | '\t' | '\r')
    ) {
        format!("'{escaped}")
    } else {
        escaped
    };
    format!("\"{neutralized}\"")
}

#[tauri::command]
pub fn export_clips_csv(db: State<'_, Arc<DbState>>) -> Result<String, String> {
    let exported = db.export_clips_csv().map_err(|error| error.to_string())?;
    let _ = db.log_activity("data_export_completed", "Exported Clips as CSV");
    Ok(exported)
}

#[tauri::command]
pub fn get_analytics_summary(
    db: State<'_, Arc<DbState>>,
) -> Result<crate::db::AnalyticsSummary, String> {
    db.get_analytics_summary().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_cli_to_path() -> Result<String, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    let bin_dir = exe_path.parent().ok_or("Cannot locate binary directory")?;
    let cli_exe = bin_dir.join("pasted");

    if !cli_exe.exists() {
        return Err(format!(
            "pasted binary not found at '{:?}'. Run 'cargo build --bin pasted' first.",
            cli_exe
        ));
    }

    #[cfg(unix)]
    {
        let target_dir = dirs::home_dir()
            .map(|home| home.join(".local/bin"))
            .ok_or("Cannot locate your home directory")?;
        let symlink_path = install_cli_symlink(&cli_exe, &target_dir)?;
        Ok(format!(
            "Successfully installed the pasted command at '{}'. Make sure that directory is in your PATH.",
            symlink_path.display()
        ))
    }

    #[cfg(not(unix))]
    {
        Err("Automatic CLI installation is not supported on this platform yet".to_string())
    }
}

#[cfg(unix)]
fn install_cli_symlink(
    cli_exe: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    use std::fs;
    use std::os::unix::fs::symlink;

    fs::create_dir_all(target_dir).map_err(|error| {
        format!(
            "Failed to create CLI directory '{}': {error}",
            target_dir.display()
        )
    })?;
    let symlink_path = target_dir.join("pasted");
    match fs::symlink_metadata(&symlink_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let existing_target = fs::read_link(&symlink_path).map_err(|error| {
                format!(
                    "Failed to inspect existing CLI link '{}': {error}",
                    symlink_path.display()
                )
            })?;
            if existing_target == cli_exe {
                return Ok(symlink_path);
            }
            return Err(format!(
                "Refusing to replace existing CLI link '{}' (currently points to '{}')",
                symlink_path.display(),
                existing_target.display()
            ));
        }
        Ok(_) => {
            return Err(format!(
                "Refusing to replace existing file '{}'",
                symlink_path.display()
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Failed to inspect CLI destination '{}': {error}",
                symlink_path.display()
            ));
        }
    }

    symlink(cli_exe, &symlink_path).map_err(|error| {
        format!(
            "Failed to create CLI link '{}': {error}",
            symlink_path.display()
        )
    })?;
    Ok(symlink_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocr_text_never_replaces_an_image_clips_copy_fingerprint() {
        let rgba = vec![12, 34, 56, 255];
        let image = image::RgbaImage::from_raw(1, 1, rgba.clone()).unwrap();
        let mut encoded = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let image_base64 = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(encoded.into_inner())
        );
        let clip = ClipItem {
            id: 1,
            content_type: "image".to_string(),
            text_content: Some("recognized OCR text".to_string()),
            html_content: None,
            image_base64: Some(image_base64),
            image_path: None,
            content_hash: "stored-image-hash".to_string(),
            source: "Screenshot".to_string(),
            is_pinned: false,
            is_protected: false,
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
            clip_internal_clipboard_fingerprint(&clip).unwrap(),
            crate::clipboard_fingerprint::image_rgba(&rgba)
        );
    }

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
        std::env::temp_dir().join(format!("pasted-{label}-{}-{nonce}", std::process::id()))
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_never_overwrites_an_existing_file() {
        let root = unique_test_directory("cli-preserve");
        let bin_dir = root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let destination = bin_dir.join("pasted");
        std::fs::write(&destination, "user-owned").unwrap();

        let error = install_cli_symlink(&root.join("source"), &bin_dir).unwrap_err();
        assert!(error.contains("Refusing to replace existing file"));
        assert_eq!(std::fs::read_to_string(&destination).unwrap(), "user-owned");

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn cli_install_is_idempotent_for_its_existing_link() {
        let root = unique_test_directory("cli-idempotent");
        let source = root.join("pasted-source");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&source, "binary").unwrap();
        let bin_dir = root.join("bin");

        let first = install_cli_symlink(&source, &bin_dir).unwrap();
        let second = install_cli_symlink(&source, &bin_dir).unwrap();
        assert_eq!(first, second);
        assert_eq!(std::fs::read_link(second).unwrap(), source);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_parse_shortcut_str_variations() {
        assert!(parse_shortcut_str("CmdOrCtrl+Shift+V").is_some());
        assert!(parse_shortcut_str("Control+Alt+C").is_some());
        assert!(parse_shortcut_str("Ctrl+Alt+KeyC").is_some());
        assert!(parse_shortcut_str("Alt+Super+KeyV").is_some());
        assert!(parse_shortcut_str("Option+Cmd+C").is_some());
        assert!(parse_shortcut_str("Command+Shift+V").is_some());
        assert!(parse_shortcut_str("Control+Option+C").is_some());
        assert!(parse_shortcut_str("Control+Option+V").is_some());
        assert!(parse_shortcut_str("Super+Alt+KeyC").is_some());
        assert!(parse_shortcut_str("").is_none());
        assert!(parse_shortcut_str("   ").is_none());

        // Equivalence checks for key representations
        let sc1 = parse_shortcut_str("Option+Command+C").unwrap();
        let sc2 = parse_shortcut_str("Alt+Super+KeyC").unwrap();
        assert_eq!(
            sc1, sc2,
            "Option+Command+C should resolve to identical Shortcut struct as Alt+Super+KeyC"
        );

        // Option unicode character resolution tests
        let sc_unicode_c = parse_shortcut_str("Alt+ç").unwrap();
        let sc_ascii_c = parse_shortcut_str("Alt+KeyC").unwrap();
        assert_eq!(sc_unicode_c, sc_ascii_c, "Alt+ç should map to Alt+KeyC");
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
            let parsed = parse_shortcut_str(s);
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
    fn csv_cells_escape_structure_and_neutralize_formulas() {
        assert_eq!(csv_cell("plain text"), "\"plain text\"");
        assert_eq!(
            csv_cell("commas, quotes \" and\nlines"),
            "\"commas, quotes \"\" and\nlines\""
        );
        assert_eq!(csv_cell("=2+2"), "\"'=2+2\"");
        assert_eq!(csv_cell("+SUM(A1:A2)"), "\"'+SUM(A1:A2)\"");
        assert_eq!(csv_cell("-1+2"), "\"'-1+2\"");
        assert_eq!(csv_cell("@SUM(A1:A2)"), "\"'@SUM(A1:A2)\"");
        assert_eq!(csv_cell("\t=2+2"), "\"'\t=2+2\"");
        assert_eq!(csv_cell("\r=2+2"), "\"'\r=2+2\"");
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

        let restored = collect_file_clip_previews(
            &paths,
            "safe",
            1024 * 1024,
            Some(&cache),
            None,
            Some("files:ephemeral"),
        );
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].width, Some(3));
        assert_eq!(restored[0].height, Some(2));
        assert_eq!(restored[0].data_url, initial[0].data_url);

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

        write_pdf_preview_cache(&root, &key, b"cached-thumbnail");
        assert_eq!(
            read_pdf_preview_cache(&root, &key).as_deref(),
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
