pub(super) fn matching_items<'a>(
    items: &'a [String],
    label: &str,
    aliases: &[impl AsRef<str>],
) -> Vec<&'a str> {
    let label = label.to_lowercase();
    items
        .iter()
        .filter(|item| {
            item.contains(&label)
                || aliases
                    .iter()
                    .any(|alias| item.contains(&alias.as_ref().to_lowercase()))
        })
        .map(String::as_str)
        .collect()
}
