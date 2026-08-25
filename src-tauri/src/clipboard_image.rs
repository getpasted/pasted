use image::ImageDecoder as _;
use std::io::Read as _;
use std::path::Path;

fn read_bounded_image(path: &Path) -> Option<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > crate::resource_limits::MAX_FILE_PREVIEW_INPUT_BYTES
    {
        return None;
    }
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(crate::resource_limits::MAX_FILE_PREVIEW_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= crate::resource_limits::MAX_FILE_PREVIEW_INPUT_BYTES).then_some(bytes)
}

fn decode_oriented_image(bytes: Vec<u8>) -> Option<image::DynamicImage> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let dimensions = decoder.dimensions();
    if !crate::resource_limits::image_dimensions_within_limit(dimensions.0, dimensions.1) {
        return None;
    }
    let orientation = decoder.orientation().ok()?;
    let mut image = image::DynamicImage::from_decoder(decoder).ok()?;
    image.apply_orientation(orientation);
    Some(image)
}

pub(crate) fn image_file_rgba_fingerprint(path: &Path) -> Option<String> {
    let image = decode_oriented_image(read_bounded_image(path)?).map(|image| image.to_rgba8())?;
    Some(crate::clipboard_fingerprint::image_rgba(image.as_raw()))
}

pub(crate) fn image_file_clipboard_payload(path: &Path) -> Option<arboard::ImageData<'static>> {
    let image = decode_oriented_image(read_bounded_image(path)?).map(|image| image.to_rgba8())?;
    Some(arboard::ImageData {
        width: usize::try_from(image.width()).ok()?,
        height: usize::try_from(image.height()).ok()?,
        bytes: std::borrow::Cow::Owned(image.into_raw()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_path(extension: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "pasted-oriented-image-{}-{}.{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            extension
        ))
    }

    #[test]
    fn copied_image_files_share_the_clipboard_rgba_fingerprint() {
        let path = temporary_path("png");
        let rgba = vec![10, 20, 30, 255, 40, 50, 60, 128];
        image::RgbaImage::from_raw(2, 1, rgba.clone())
            .unwrap()
            .save(&path)
            .unwrap();

        assert_eq!(
            image_file_rgba_fingerprint(&path),
            Some(crate::clipboard_fingerprint::image_rgba(&rgba))
        );
        let payload = image_file_clipboard_payload(&path).unwrap();
        assert_eq!((payload.width, payload.height), (2, 1));
        assert_eq!(payload.bytes.as_ref(), rgba);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn exif_orientation_is_applied_before_clipboard_capture() {
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .encode(
                &[255, 0, 0, 0, 0, 255],
                2,
                1,
                image::ExtendedColorType::Rgb8,
            )
            .unwrap();
        let exif_orientation_six = [
            0xff, 0xe1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 0x2a, 0, 8, 0, 0, 0,
            1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
        ];
        jpeg.splice(2..2, exif_orientation_six);
        let path = temporary_path("jpg");
        std::fs::write(&path, jpeg).unwrap();

        let payload = image_file_clipboard_payload(&path).unwrap();
        assert_eq!((payload.width, payload.height), (1, 2));
        let _ = std::fs::remove_file(path);
    }
}
