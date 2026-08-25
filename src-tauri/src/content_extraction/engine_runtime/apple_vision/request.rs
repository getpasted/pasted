use std::{fs, path::Path};

pub(super) fn read_images(request_path: &Path) -> Option<Vec<Vec<u8>>> {
    let request = fs::metadata(request_path)
        .ok()
        .filter(|metadata| metadata.is_file() && metadata.len() <= 1024 * 1024)
        .and_then(|_| fs::read(request_path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())?;
    let paths = request
        .pointer("/input/path")
        .and_then(serde_json::Value::as_str)
        .map(|path| vec![path])
        .or_else(|| {
            request
                .pointer("/input/paths")
                .and_then(serde_json::Value::as_array)
                .map(|paths| {
                    paths
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .take(crate::resource_limits::MAX_MEDIA_PROBE_FILES)
                        .collect()
                })
        })?;
    let images = paths
        .into_iter()
        .filter_map(|path| read_bounded_image(Path::new(path)))
        .collect::<Vec<_>>();
    (!images.is_empty()).then_some(images)
}

fn read_bounded_image(path: &Path) -> Option<Vec<u8>> {
    fs::metadata(path)
        .ok()
        .filter(|metadata| {
            metadata.is_file()
                && metadata.len() <= crate::resource_limits::MAX_ENCODED_IMAGE_BYTES as u64
        })
        .and_then(|_| fs::read(path).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_single_and_multiple_bounded_input_shapes() {
        let workspace = crate::external_tools::PrivateWorkspace::create("vision-request").unwrap();
        let first = workspace.join("first.png");
        let second = workspace.join("second.png");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();
        let request = workspace.join("request.json");
        fs::write(
            &request,
            serde_json::json!({ "input": { "path": first } }).to_string(),
        )
        .unwrap();
        assert_eq!(read_images(&request).unwrap(), [b"first".to_vec()]);
        fs::write(
            &request,
            serde_json::json!({ "input": { "paths": [first, second] } }).to_string(),
        )
        .unwrap();
        assert_eq!(
            read_images(&request).unwrap(),
            [b"first".to_vec(), b"second".to_vec()]
        );
    }
}
