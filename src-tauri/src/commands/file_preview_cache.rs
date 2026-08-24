use sha2::{Digest, Sha256};
use std::io::Read;

#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
pub(super) fn looks_like_pdf(bytes: &[u8]) -> bool {
    bytes
        .windows(5)
        .take(1_024)
        .any(|window| window == b"%PDF-")
}

pub(super) fn pdf_preview_cache_key(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pasted-pdf-first-page-v2-white-background\0");
    hasher.update(bytes);
    crate::hashing::finalize_sha256_hex(hasher)
}

pub(super) fn clip_file_preview_cache_key(content_hash: &str, index: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"pasted-file-preview-v1\0");
    hasher.update(content_hash.as_bytes());
    hasher.update([0]);
    hasher.update(index.to_le_bytes());
    format!("clip-{}", crate::hashing::finalize_sha256_hex(hasher))
}

pub(super) fn flatten_image_on_white(image: image::DynamicImage) -> image::DynamicImage {
    let source = image.to_rgba8();
    let mut background = image::RgbaImage::from_pixel(
        source.width(),
        source.height(),
        image::Rgba([255, 255, 255, 255]),
    );
    image::imageops::overlay(&mut background, &source, 0, 0);
    image::DynamicImage::ImageRgba8(background)
}

pub(super) fn read_preview_cache(cache_directory: &std::path::Path, key: &str) -> Option<Vec<u8>> {
    read_bounded_file(
        &cache_directory.join(format!("{key}.webp")),
        crate::resource_limits::MAX_FILE_PREVIEW_OUTPUT_BYTES as u64,
    )
}

fn prune_preview_cache(cache_directory: &std::path::Path) {
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

pub(super) fn write_preview_cache(cache_directory: &std::path::Path, key: &str, bytes: &[u8]) {
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
        prune_preview_cache(cache_directory);
    }
}

#[cfg(target_os = "macos")]
pub(super) fn render_pdf_first_page(bytes: &[u8]) -> Option<Vec<u8>> {
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
            std::thread::sleep(Duration::from_millis(20));
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
pub(super) fn render_pdf_first_page(_bytes: &[u8]) -> Option<Vec<u8>> {
    None
}

pub(super) fn read_bounded_file(path: &std::path::Path, max_bytes: u64) -> Option<Vec<u8>> {
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
