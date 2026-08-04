use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationConfigKind {
    None,
    HtmlTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OperationDefinition {
    pub key: &'static str,
    pub name: &'static str,
    pub category_id: &'static str,
    pub category_label: &'static str,
    pub config_kind: OperationConfigKind,
}

const fn operation(
    key: &'static str,
    name: &'static str,
    category_id: &'static str,
    category_label: &'static str,
) -> OperationDefinition {
    OperationDefinition {
        key,
        name,
        category_id,
        category_label,
        config_kind: OperationConfigKind::None,
    }
}

const fn configured_operation(
    key: &'static str,
    name: &'static str,
    category_id: &'static str,
    category_label: &'static str,
    config_kind: OperationConfigKind,
) -> OperationDefinition {
    OperationDefinition {
        key,
        name,
        category_id,
        category_label,
        config_kind,
    }
}

pub static BUILTIN_OPERATIONS: &[OperationDefinition] = &[
    operation(
        "clean_url_tracking",
        "Clean URL Tracking",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "strip_html",
        "Plain Text / Strip HTML",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "strip_markdown",
        "Strip Markdown Formatting",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "strip_emojis",
        "Remove Emoji",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "smileys_to_emoji",
        "Convert Smileys to Emoji",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "trim",
        "Trim Whitespace",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "strip_newlines",
        "Strip Newlines",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "collapse_whitespace",
        "Collapse Whitespace",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "strip_diacritics",
        "Strip Diacritics",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "strip_non_alphanumeric",
        "Strip Non-Alphanumeric",
        "cleaning",
        "Cleaners & Sanitizers",
    ),
    operation(
        "smart_punctuation",
        "Smart Punctuation",
        "typography",
        "Smart Formatting",
    ),
    operation(
        "straighten_punctuation",
        "Straighten Punctuation",
        "typography",
        "Smart Formatting",
    ),
    operation("uppercase", "UPPERCASE", "case", "Case Transformations"),
    operation("lowercase", "lowercase", "case", "Case Transformations"),
    operation("titlecase", "Title Case", "case", "Case Transformations"),
    operation(
        "sentence_case",
        "Sentence case",
        "case",
        "Case Transformations",
    ),
    operation("camelcase", "camelCase", "case", "Case Transformations"),
    operation("snakecase", "snake_case", "case", "Case Transformations"),
    operation("kebabcase", "kebab-case", "case", "Case Transformations"),
    operation(
        "constant_case",
        "CONSTANT_CASE",
        "case",
        "Case Transformations",
    ),
    operation(
        "alternating_case",
        "aLtErNaTiNg cAsE",
        "case",
        "Case Transformations",
    ),
    operation(
        "json_format",
        "Format JSON",
        "structure",
        "Structure & Formatting",
    ),
    operation(
        "json_minify",
        "Minify JSON",
        "structure",
        "Structure & Formatting",
    ),
    operation(
        "json_stringify",
        "Quote as JSON String",
        "structure",
        "Structure & Formatting",
    ),
    configured_operation(
        "wrap_tags",
        "Wrap in HTML Tags",
        "structure",
        "Structure & Formatting",
        OperationConfigKind::HtmlTag,
    ),
    operation(
        "html_paragraphs",
        "Create HTML Paragraphs",
        "structure",
        "Structure & Formatting",
    ),
    operation(
        "html_unordered_list",
        "Create HTML List",
        "structure",
        "Structure & Formatting",
    ),
    operation(
        "extract_urls",
        "Extract URLs",
        "extraction",
        "Data Extraction",
    ),
    operation(
        "extract_emails",
        "Extract Emails",
        "extraction",
        "Data Extraction",
    ),
    operation(
        "extract_phones",
        "Extract Phone Numbers",
        "extraction",
        "Data Extraction",
    ),
    operation(
        "extract_ips",
        "Extract IP Addresses",
        "extraction",
        "Data Extraction",
    ),
    operation(
        "extract_numbers",
        "Extract Numbers",
        "extraction",
        "Data Extraction",
    ),
    operation(
        "sort_lines_asc",
        "Sort Lines (A–Z)",
        "lines",
        "Line Operations",
    ),
    operation(
        "sort_lines_desc",
        "Sort Lines (Z–A)",
        "lines",
        "Line Operations",
    ),
    operation(
        "sort_by_length",
        "Sort Lines by Length",
        "lines",
        "Line Operations",
    ),
    operation(
        "dedupe_lines",
        "Deduplicate Lines",
        "lines",
        "Line Operations",
    ),
    operation("reverse_lines", "Reverse Lines", "lines", "Line Operations"),
    operation("reverse_text", "Reverse Text", "lines", "Line Operations"),
    operation(
        "strip_empty_lines",
        "Strip Empty Lines",
        "lines",
        "Line Operations",
    ),
    operation("number_lines", "Number Lines", "lines", "Line Operations"),
    operation("quote_text", "Quote Text", "lines", "Line Operations"),
    operation(
        "html_encode",
        "HTML Entity Encode",
        "encoding",
        "Encodings & Decodings",
    ),
    operation(
        "html_decode",
        "HTML Entity Decode",
        "encoding",
        "Encodings & Decodings",
    ),
    operation(
        "hex_encode",
        "Hex Encode",
        "encoding",
        "Encodings & Decodings",
    ),
    operation(
        "hex_decode",
        "Hex Decode",
        "encoding",
        "Encodings & Decodings",
    ),
    operation(
        "url_encode",
        "URL Encode",
        "encoding",
        "Encodings & Decodings",
    ),
    operation(
        "url_decode",
        "URL Decode",
        "encoding",
        "Encodings & Decodings",
    ),
    operation(
        "base64_encode",
        "Base64 Encode",
        "encoding",
        "Encodings & Decodings",
    ),
    operation(
        "base64_decode",
        "Base64 Decode",
        "encoding",
        "Encodings & Decodings",
    ),
];

pub fn is_builtin_operation(key: &str) -> bool {
    BUILTIN_OPERATIONS
        .iter()
        .any(|operation| operation.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn built_in_operation_keys_and_names_are_unique() {
        let mut keys = HashSet::new();
        let mut names = HashSet::new();
        for operation in BUILTIN_OPERATIONS {
            assert!(
                keys.insert(operation.key),
                "duplicate key: {}",
                operation.key
            );
            assert!(
                names.insert(operation.name),
                "duplicate name: {}",
                operation.name
            );
            assert!(!operation.category_id.is_empty());
            assert!(!operation.category_label.is_empty());
        }
    }

    #[test]
    fn executor_only_kinds_are_not_presented_as_built_ins() {
        assert!(!is_builtin_operation("pipeline"));
        assert!(!is_builtin_operation("regex"));
        assert!(!is_builtin_operation("shell_script"));
    }
}
