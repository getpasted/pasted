pub(crate) fn eligible_paths(
    accepted_formats: &[String],
    paths: &[String],
    detected_formats: &[Option<String>],
) -> Vec<String> {
    if accepted_formats.iter().any(|format| format == "*") {
        return paths.to_vec();
    }
    paths
        .iter()
        .zip(detected_formats)
        .filter(|(_, detected)| {
            detected
                .as_ref()
                .is_some_and(|format| accepted_formats.contains(format))
        })
        .map(|(path, _)| path.clone())
        .collect()
}
