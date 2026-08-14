use sha2::{Digest, Sha256};

pub fn text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn file_list(paths: &[String]) -> String {
    let mut hasher = Sha256::new();
    for path in paths {
        hasher.update(path.as_bytes());
        hasher.update([0]);
    }
    format!("files:{:x}", hasher.finalize())
}

pub fn image_rgba(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_fingerprints_preserve_selection_order() {
        let first = vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()];
        let reversed = vec!["/tmp/b.txt".to_string(), "/tmp/a.txt".to_string()];
        assert_ne!(file_list(&first), file_list(&reversed));
        assert_eq!(file_list(&first), file_list(&first));
    }

    #[test]
    fn image_fingerprints_match_exact_rgba_content() {
        assert_eq!(image_rgba(&[1, 2, 3, 4]), image_rgba(&[1, 2, 3, 4]));
        assert_ne!(image_rgba(&[1, 2, 3, 4]), image_rgba(&[1, 2, 3, 5]));
    }

    #[test]
    fn text_fingerprints_are_stable_and_content_sensitive() {
        assert_eq!(text("hello"), text("hello"));
        assert_ne!(text("hello"), text("Hello"));
    }
}
