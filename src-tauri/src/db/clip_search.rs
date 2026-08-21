use regex::RegexBuilder;
use rusqlite::{Connection, Result};

#[derive(Debug, Default)]
pub(super) struct ParsedClipSearch {
    pub sources: Vec<String>,
    pub clip_types: Vec<String>,
    pub content_types: Vec<String>,
    pub file_formats: Vec<String>,
    pub terms: Vec<String>,
    pub requires_note: bool,
    pub requires_named: bool,
    pub requires_pinned: bool,
    pub requires_protected: bool,
    pub requires_trashed: bool,
    pub incomplete: bool,
    pub regex: Option<String>,
    pub regex_fallback: Option<String>,
}

#[derive(Clone, Copy)]
pub(super) struct ClipSearchFeaturePolicy {
    pub clip_types: bool,
    pub content_types: bool,
    pub file_formats: bool,
    pub sources: bool,
    pub notes: bool,
    pub naming: bool,
    pub pinning: bool,
    pub protection: bool,
    pub trash: bool,
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

pub(super) fn parse_clip_search(query: &str) -> ParsedClipSearch {
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
        if let Some(value) = lower.strip_prefix("source:") {
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

pub(super) fn clip_search_feature_policy(conn: &Connection) -> Result<ClipSearchFeaturePolicy> {
    conn.query_row(
        "SELECT
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableClipTypes' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableTypes' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableFileFormats' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableSources' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableNotes' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableNaming' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enablePinning' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableProtection' AND LOWER(TRIM(value)) IN ('false', '0')),
            NOT EXISTS(SELECT 1 FROM settings WHERE key = 'enableTrash' AND LOWER(TRIM(value)) IN ('false', '0'))",
        [],
        |row| Ok(ClipSearchFeaturePolicy {
            clip_types: row.get(0)?, content_types: row.get(1)?, file_formats: row.get(2)?,
            sources: row.get(3)?, notes: row.get(4)?, naming: row.get(5)?,
            pinning: row.get(6)?, protection: row.get(7)?, trash: row.get(8)?,
        }),
    )
}
