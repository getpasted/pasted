use sha2::{Digest, Sha256};
use std::fmt::Write;

pub(crate) fn finalize_sha256_hex(hasher: Sha256) -> String {
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_preserves_the_stable_lowercase_contract() {
        let mut hasher = Sha256::new();
        hasher.update(b"abc");

        let encoded = finalize_sha256_hex(hasher);
        assert_eq!(encoded.len(), 64);
        assert_eq!(
            encoded,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
