use regex::RegexBuilder;

#[derive(Debug, Default)]
pub(crate) struct ParsedClipSearch {
    pub(crate) clip_ids: Vec<i64>,
    pub(crate) sources: Vec<String>,
    pub(crate) clip_types: Vec<String>,
    pub(crate) content_types: Vec<String>,
    pub(crate) file_formats: Vec<String>,
    pub(crate) terms: Vec<String>,
    pub(crate) requires_note: bool,
    pub(crate) requires_named: bool,
    pub(crate) requires_pinned: bool,
    pub(crate) requires_protected: bool,
    pub(crate) requires_trashed: bool,
    pub(crate) incomplete: bool,
    pub(crate) regex: Option<String>,
    pub(crate) regex_fallback: Option<String>,
}

fn tokenize(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    for character in query.chars() {
        if let Some(expected) = quote {
            if character == expected {
                quote = None;
            } else {
                token.push(character);
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character.is_whitespace() {
            if !token.is_empty() {
                tokens.push(std::mem::take(&mut token));
            }
        } else {
            token.push(character);
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

pub(crate) fn parse_clip_search(query: &str) -> ParsedClipSearch {
    let trimmed = query.trim();
    let mut parsed = ParsedClipSearch::default();
    if trimmed
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("regex:"))
    {
        let pattern = &trimmed[6..];
        if pattern.trim().is_empty() {
            parsed.incomplete = true;
        } else if RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .is_ok()
        {
            parsed.regex = Some(pattern.to_string());
        } else {
            parsed.regex_fallback = Some(pattern.to_lowercase());
        }
        return parsed;
    }
    for token in tokenize(trimmed) {
        let lower = token.to_lowercase();
        let push_filter = |values: &mut Vec<String>, value: &str, incomplete: &mut bool| {
            if value.is_empty() {
                *incomplete = true;
            } else {
                values.push(value.to_string());
            }
        };
        if let Some(value) = lower.strip_prefix("id:") {
            let values = value.split(',').collect::<Vec<_>>();
            if values.is_empty() || values.iter().any(|value| value.is_empty()) {
                parsed.incomplete = true;
            } else {
                for value in values {
                    match value.parse::<i64>() {
                        Ok(id) if id > 0 => parsed.clip_ids.push(id),
                        _ => parsed.incomplete = true,
                    }
                }
            }
        } else if let Some(value) = lower.strip_prefix("source:") {
            push_filter(&mut parsed.sources, value.trim(), &mut parsed.incomplete);
        } else if let Some(value) = lower.strip_prefix("clip:") {
            push_filter(&mut parsed.clip_types, value.trim(), &mut parsed.incomplete);
        } else if let Some(value) = lower.strip_prefix("content:") {
            push_filter(
                &mut parsed.content_types,
                value.trim(),
                &mut parsed.incomplete,
            );
        } else if let Some(value) = lower.strip_prefix("format:") {
            push_filter(
                &mut parsed.file_formats,
                value.trim(),
                &mut parsed.incomplete,
            );
        } else if lower == "has:note" {
            parsed.requires_note = true;
        } else if lower == "is:named" || lower == "has:name" {
            parsed.requires_named = true;
        } else if lower == "is:pinned" {
            parsed.requires_pinned = true;
        } else if lower == "is:protected" {
            parsed.requires_protected = true;
        } else if lower == "is:trashed" {
            parsed.requires_trashed = true;
        } else if !lower.is_empty() {
            parsed.terms.push(lower);
        }
    }
    parsed
}
