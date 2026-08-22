use std::{borrow::Cow, path::Path};

use super::WHISPER_TRANSCRIPTION_REF;

pub(super) fn eligible_paths<'a>(stable_ref: &str, paths: &'a [String]) -> Cow<'a, [String]> {
    if stable_ref != WHISPER_TRANSCRIPTION_REF {
        return Cow::Borrowed(paths);
    }
    Cow::Owned(
        paths
            .iter()
            .filter(|path| {
                super::engine_runtime::whisper_audio_preparation(Path::new(path)).is_some()
            })
            .cloned()
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_receives_only_audio_file_references() {
        let paths = vec!["document.pdf".into(), "recording.m4a".into()];
        let eligible = eligible_paths(WHISPER_TRANSCRIPTION_REF, &paths);
        assert_eq!(eligible.as_ref(), ["recording.m4a"]);
    }
}
