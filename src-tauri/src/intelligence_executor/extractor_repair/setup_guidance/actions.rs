pub(super) fn has_install_action(item: &str) -> bool {
    [
        "brew install ",
        "winget install ",
        "apt install ",
        "apt-get install ",
        "dnf install ",
        "pacman -s ",
        "zypper install ",
        "cargo install ",
        "pipx install ",
    ]
    .iter()
    .any(|command| item.contains(command))
        || has_direct_artifact_url(item)
}

pub(super) fn has_direct_artifact_url(item: &str) -> bool {
    item.split_whitespace()
        .filter_map(|token| token.strip_prefix("https://"))
        .map(|token| {
            token.trim_matches(|character: char| {
                !character.is_alphanumeric() && !"/._-".contains(character)
            })
        })
        .any(|url| {
            let filename = url.rsplit('/').next().unwrap_or_default();
            filename.contains('.') && !filename.contains('*')
        })
}
