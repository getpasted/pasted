use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

use crate::operation_registry::is_builtin_operation;

static RE_HTML: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]*>").unwrap());
static RE_EMOJI: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]").unwrap()
});
static RE_MD_BOLD: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*\*([^*]+)\*\*|__([^_]+)__").unwrap());
static RE_MD_ITALIC: Lazy<Regex> = Lazy::new(|| Regex::new(r"\*([^*]+)\*|_([^_]+)_").unwrap());
static RE_MD_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`{1,3}([^`]+)`{1,3}").unwrap());
static RE_MD_HEADER: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^#+\s+").unwrap());
static RE_URL_TRACKING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[?&](?:utm_source|utm_medium|utm_campaign|utm_term|utm_content|fbclid|gclid|msclkid|mc_eid|_hsenc|ref)=[^&\s]+").unwrap()
});

pub fn apply_filter(
    input: &str,
    filter_type: &str,
    config: Option<&str>,
) -> Result<String, String> {
    let is_executor = matches!(filter_type, "pipeline" | "regex");
    if !is_executor && !is_builtin_operation(filter_type) {
        return Err(format!("Unknown operation type: {}", filter_type));
    }

    match filter_type {
        "pipeline" => {
            let cfg_str = config.ok_or_else(|| "Pipeline configuration is required".to_string())?;
            let steps = serde_json::from_str::<Vec<Value>>(cfg_str)
                .map_err(|error| format!("Invalid pipeline configuration: {}", error))?;
            let mut current = input.to_string();
            for (index, step) in steps.into_iter().enumerate() {
                let f_type = step["filter_type"]
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| format!("Pipeline step {} has no operation type", index + 1))?;
                let f_cfg = if step["config"].is_string() {
                    step["config"].as_str().map(|s| s.to_string())
                } else if !step["config"].is_null() {
                    Some(step["config"].to_string())
                } else {
                    None
                };
                current = apply_filter(&current, f_type, f_cfg.as_deref()).map_err(|error| {
                    format!("Pipeline step {} ({}) failed: {}", index + 1, f_type, error)
                })?;
            }
            Ok(current)
        }
        "lowercase" => Ok(input.to_lowercase()),
        "uppercase" => Ok(input.to_uppercase()),
        "titlecase" => Ok(to_title_case(input)),
        "camelcase" => Ok(to_camel_case(input)),
        "snakecase" => Ok(to_snake_case(input)),
        "kebabcase" => Ok(to_kebab_case(input)),
        "strip_html" => Ok(strip_html_tags(input)),
        "trim" => Ok(input
            .lines()
            .map(|l| l.trim())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()),
        "strip_newlines" => Ok(input
            .replace("\r\n", " ")
            .replace('\n', " ")
            .trim()
            .to_string()),
        "collapse_whitespace" => Ok(input.split_whitespace().collect::<Vec<_>>().join(" ")),
        "strip_diacritics" => Ok(strip_diacritics(input)),
        "strip_non_alphanumeric" => Ok(input
            .chars()
            .filter(|character| character.is_alphanumeric())
            .collect()),
        "url_encode" => Ok(urlencoding::encode(input).into_owned()),
        "url_decode" => urlencoding::decode(input)
            .map(|s| s.into_owned())
            .map_err(|e| e.to_string()),
        "base64_encode" => Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            input.as_bytes(),
        )),
        "base64_decode" => {
            let bytes =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, input.trim())
                    .map_err(|e| e.to_string())?;
            String::from_utf8(bytes).map_err(|e| e.to_string())
        }
        "json_format" => {
            let parsed: Value =
                serde_json::from_str(input).map_err(|e| format!("Invalid JSON: {}", e))?;
            serde_json::to_string_pretty(&parsed).map_err(|e| e.to_string())
        }
        "json_minify" => {
            let parsed: Value =
                serde_json::from_str(input).map_err(|e| format!("Invalid JSON: {}", e))?;
            serde_json::to_string(&parsed).map_err(|e| e.to_string())
        }
        "json_stringify" => serde_json::to_string(input).map_err(|e| e.to_string()),
        "strip_emojis" => Ok(strip_emojis(input)),
        "smileys_to_emoji" => Ok(convert_smileys_to_emoji(input)),
        "sentence_case" => Ok(to_sentence_case(input)),
        "constant_case" => Ok(to_snake_case(input).to_uppercase()),
        "alternating_case" => Ok(to_alternating_case(input)),
        "smart_punctuation" => Ok(apply_smart_punctuation(input)),
        "straighten_punctuation" => Ok(straighten_punctuation(input)),
        "strip_markdown" => Ok(strip_markdown_tags(input)),
        "strip_empty_lines" => Ok(strip_empty_lines(input)),
        "reverse_lines" => Ok(reverse_lines(input)),
        "reverse_text" => Ok(input.chars().rev().collect()),
        "sort_lines_asc" => Ok(sort_lines(input, false)),
        "sort_lines_desc" => Ok(sort_lines(input, true)),
        "sort_by_length" => Ok(sort_by_length(input)),
        "dedupe_lines" => Ok(dedupe_lines(input)),
        "number_lines" => Ok(number_lines(input)),
        "quote_text" => Ok(quote_text(input, config)),
        "clean_url_tracking" => Ok(clean_url_tracking_params(input)),
        "extract_urls" => Ok(extract_by_regex(input, r"https?://[^\s\)]+")),
        "extract_emails" => Ok(extract_by_regex(
            input,
            r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
        )),
        "extract_phones" => Ok(extract_by_regex(
            input,
            r"\b(?:\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b",
        )),
        "extract_ips" => Ok(extract_by_regex(input, r"\b(?:\d{1,3}\.){3}\d{1,3}\b")),
        "extract_numbers" => Ok(extract_by_regex(input, r"\b\d+(?:\.\d+)?\b")),
        "html_encode" => Ok(encode_html(input)),
        "html_decode" => Ok(decode_html(input)),
        "hex_encode" => Ok(encode_hex(input)),
        "hex_decode" => decode_hex(input),
        "wrap_tags" => {
            let tag = config.unwrap_or("div");
            Ok(format!("<{}>{}</{}>", tag, input, tag))
        }
        "html_paragraphs" => Ok(html_paragraphs(input)),
        "html_unordered_list" => Ok(html_unordered_list(input)),
        "regex" => {
            if let Some(cfg_str) = config {
                if let Ok(json) = serde_json::from_str::<Value>(cfg_str) {
                    let pattern = json["pattern"].as_str().unwrap_or("");
                    let replacement = json["replacement"].as_str().unwrap_or("");
                    if !pattern.is_empty() {
                        let match_mode = json["matchMode"].as_str().unwrap_or("regex");
                        let case_sensitive = json["caseSensitive"].as_bool().unwrap_or(false);
                        return find_and_replace(
                            input,
                            pattern,
                            replacement,
                            match_mode,
                            case_sensitive,
                        );
                    }
                }
            }
            Ok(input.to_string())
        }
        _ => Err(format!(
            "Registered operation has no executor implementation: {}",
            filter_type
        )),
    }
}

fn to_sentence_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in s.chars() {
        if capitalize_next && c.is_alphabetic() {
            result.push_str(&c.to_uppercase().to_string());
            capitalize_next = false;
        } else {
            result.push(c);
            if c == '.' || c == '!' || c == '?' {
                capitalize_next = true;
            }
        }
    }
    result
}

fn to_alternating_case(s: &str) -> String {
    let mut result = String::new();
    let mut upper = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if upper {
                result.push_str(&c.to_uppercase().to_string());
            } else {
                result.push_str(&c.to_lowercase().to_string());
            }
            upper = !upper;
        } else {
            result.push(c);
        }
    }
    result
}

fn straighten_punctuation(s: &str) -> String {
    s.replace(['“', '”'], "\"")
        .replace(['‘', '’'], "'")
        .replace('—', "--")
        .replace('…', "...")
}

fn strip_markdown_tags(s: &str) -> String {
    let step1 = RE_MD_BOLD.replace_all(s, "$1$2");
    let step2 = RE_MD_ITALIC.replace_all(&step1, "$1$2");
    let step3 = RE_MD_CODE.replace_all(&step2, "$1");
    let step4 = RE_MD_HEADER.replace_all(&step3, "");
    step4.to_string()
}

fn strip_empty_lines(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_emojis(s: &str) -> String {
    RE_EMOJI.replace_all(s, "").to_string()
}

fn convert_smileys_to_emoji(s: &str) -> String {
    s.replace(":-D", "😃")
        .replace(":D", "😃")
        .replace(":-)", "🙂")
        .replace(":)", "🙂")
        .replace(":^)", "🙂")
        .replace(";--)", "😉")
        .replace(";-)", "😉")
        .replace(";)", "😉")
        .replace(":-(", "🙁")
        .replace(":(", "🙁")
        .replace(":-P", "😛")
        .replace(":P", "😛")
        .replace(":-p", "😛")
        .replace(":p", "😛")
        .replace(":-O", "😮")
        .replace(":O", "😮")
        .replace(":-o", "😮")
        .replace(":o", "😮")
        .replace("<3", "❤️")
}

fn reverse_lines(s: &str) -> String {
    let mut lines: Vec<&str> = s.lines().collect();
    lines.reverse();
    lines.join("\n")
}

fn strip_diacritics(s: &str) -> String {
    s.nfd()
        .filter(|character| !is_combining_mark(*character))
        .collect()
}

fn html_paragraphs(s: &str) -> String {
    s.split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(|paragraph| format!("<p>{}</p>", encode_html(paragraph)))
        .collect::<Vec<_>>()
        .join("\n")
}

fn html_unordered_list(s: &str) -> String {
    let items = s
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| format!("  <li>{}</li>", encode_html(line)))
        .collect::<Vec<_>>();
    if items.is_empty() {
        String::new()
    } else {
        format!("<ul>\n{}\n</ul>", items.join("\n"))
    }
}

fn sort_by_length(s: &str) -> String {
    let mut lines: Vec<&str> = s.lines().collect();
    lines.sort_by_key(|l| l.len());
    lines.join("\n")
}

fn clean_url_tracking_params(s: &str) -> String {
    let mut result = RE_URL_TRACKING.replace_all(s, "").to_string();
    result = result.replace("?&", "?");
    if result.ends_with('?') {
        result.pop();
    }
    result
}

fn apply_smart_punctuation(s: &str) -> String {
    s.replace("...", "…")
        .replace("--", "—")
        .replace("\"\"", "\"")
        .replace(" \"", " “")
        .replace("\"", "”")
        .replace(" '", " ‘")
        .replace("'", "’")
}

fn extract_by_regex(s: &str, pattern: &str) -> String {
    if let Ok(re) = Regex::new(pattern) {
        let matches: Vec<&str> = re.find_iter(s).map(|m| m.as_str()).collect();
        if matches.is_empty() {
            s.to_string()
        } else {
            matches.join("\n")
        }
    } else {
        s.to_string()
    }
}

fn sort_lines(s: &str, reverse: bool) -> String {
    let mut lines: Vec<&str> = s.lines().collect();
    lines.sort_unstable();
    if reverse {
        lines.reverse();
    }
    lines.join("\n")
}

fn dedupe_lines(s: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for line in s.lines() {
        if seen.insert(line) {
            result.push(line);
        }
    }
    result.join("\n")
}

fn number_lines(s: &str) -> String {
    s.lines()
        .enumerate()
        .map(|(i, l)| format!("{}. {}", i + 1, l))
        .collect::<Vec<_>>()
        .join("\n")
}

fn quote_text(s: &str, config: Option<&str>) -> String {
    let parsed = config.and_then(|value| serde_json::from_str::<Value>(value).ok());
    let before = parsed
        .as_ref()
        .and_then(|value| value["before"].as_str())
        .unwrap_or("> ");
    let after = parsed
        .as_ref()
        .and_then(|value| value["after"].as_str())
        .unwrap_or("");
    let each_line = parsed
        .as_ref()
        .and_then(|value| value["applyToEachLine"].as_bool())
        .unwrap_or(true);

    if each_line {
        s.lines()
            .map(|line| format!("{before}{line}{after}"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        format!("{before}{s}{after}")
    }
}

fn find_and_replace(
    input: &str,
    pattern: &str,
    replacement: &str,
    match_mode: &str,
    case_sensitive: bool,
) -> Result<String, String> {
    let (regex_pattern, literal_replacement) = match match_mode {
        "literal" => (regex::escape(pattern), true),
        "wildcard" => {
            let escaped = regex::escape(pattern)
                .replace(r"\*", ".*?")
                .replace(r"\?", ".");
            (escaped, true)
        }
        "regex" => (pattern.to_string(), false),
        other => return Err(format!("Unknown find mode: {other}")),
    };
    let final_pattern = if case_sensitive {
        regex_pattern
    } else {
        format!("(?i:{regex_pattern})")
    };
    let regex = Regex::new(&final_pattern).map_err(|error| format!("Invalid Regex: {error}"))?;
    if literal_replacement {
        Ok(regex
            .replace_all(input, regex::NoExpand(replacement))
            .to_string())
    } else {
        Ok(regex.replace_all(input, replacement).to_string())
    }
}

fn encode_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn decode_html(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn encode_hex(s: &str) -> String {
    s.bytes()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_hex(s: &str) -> Result<String, String> {
    let clean = s.replace(' ', "");
    if !clean.len().is_multiple_of(2) {
        return Err("Invalid hex string length".to_string());
    }
    let mut bytes = Vec::new();
    for i in (0..clean.len()).step_by(2) {
        let byte = u8::from_str_radix(&clean[i..i + 2], 16).map_err(|e| e.to_string())?;
        bytes.push(byte);
    }
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn to_title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    for character in s.chars() {
        if character.is_alphabetic() {
            if capitalize_next {
                result.extend(character.to_uppercase());
            } else {
                result.extend(character.to_lowercase());
            }
            capitalize_next = false;
        } else {
            result.push(character);
            if character.is_whitespace() || matches!(character, '-' | '_' | '/') {
                capitalize_next = true;
            }
        }
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let words: Vec<&str> = s
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return String::new();
    }
    let mut res = words[0].to_lowercase();
    for w in &words[1..] {
        let mut chars = w.chars();
        if let Some(first) = chars.next() {
            res.push_str(&first.to_uppercase().to_string());
            res.push_str(&chars.as_str().to_lowercase());
        }
    }
    res
}

fn to_snake_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect::<Vec<String>>()
        .join("_")
}

fn to_kebab_case(s: &str) -> String {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect::<Vec<String>>()
        .join("-")
}

fn strip_html_tags(s: &str) -> String {
    RE_HTML.replace_all(s, "").to_string()
}

mod urlencoding {
    pub fn encode(data: &str) -> std::borrow::Cow<'_, str> {
        let mut encoded = String::with_capacity(data.len());
        for byte in data.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", byte));
                }
            }
        }
        std::borrow::Cow::Owned(encoded)
    }

    pub fn decode(data: &str) -> Result<std::borrow::Cow<'_, str>, String> {
        let mut decoded = Vec::with_capacity(data.len());
        let bytes = data.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                if i + 2 < bytes.len() {
                    let hex =
                        std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|e| e.to_string())?;
                    let byte = u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
                    decoded.push(byte);
                    i += 3;
                } else {
                    return Err("Incomplete hex sequence".to_string());
                }
            } else if bytes[i] == b'+' {
                decoded.push(b' ');
                i += 1;
            } else {
                decoded.push(bytes[i]);
                i += 1;
            }
        }
        String::from_utf8(decoded)
            .map(std::borrow::Cow::Owned)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_case_transformations() {
        assert_eq!(
            apply_filter("hello world", "uppercase", None).unwrap(),
            "HELLO WORLD"
        );
        assert_eq!(
            apply_filter("HELLO WORLD", "lowercase", None).unwrap(),
            "hello world"
        );
        assert_eq!(
            apply_filter("hello world", "titlecase", None).unwrap(),
            "Hello World"
        );
        assert_eq!(
            apply_filter("hello world", "camelcase", None).unwrap(),
            "helloWorld"
        );
        assert_eq!(
            apply_filter("hello world", "snakecase", None).unwrap(),
            "hello_world"
        );
        assert_eq!(
            apply_filter("hello world", "kebabcase", None).unwrap(),
            "hello-world"
        );
        assert_eq!(
            apply_filter("hello world", "constant_case", None).unwrap(),
            "HELLO_WORLD"
        );
    }

    #[test]
    fn test_cleaners_and_sanitizers() {
        assert_eq!(apply_filter("   hello   ", "trim", None).unwrap(), "hello");
        assert_eq!(
            apply_filter("hello\nworld", "strip_newlines", None).unwrap(),
            "hello world"
        );
        assert_eq!(
            apply_filter("<p>Hello <b>World</b></p>", "strip_html", None).unwrap(),
            "Hello World"
        );
        assert_eq!(
            apply_filter("  hello\n\twide   world  ", "collapse_whitespace", None).unwrap(),
            "hello wide world"
        );
        assert_eq!(
            apply_filter("Crème brûlée déjà vu", "strip_diacritics", None).unwrap(),
            "Creme brulee deja vu"
        );
        assert_eq!(
            apply_filter("hello, world! #42", "strip_non_alphanumeric", None).unwrap(),
            "helloworld42"
        );
    }

    #[test]
    fn test_competitor_utility_operations() {
        assert_eq!(
            apply_filter("Pasted 🚀", "reverse_text", None).unwrap(),
            "🚀 detsaP"
        );
        assert_eq!(
            apply_filter("hello \"Pasted\"", "json_stringify", None).unwrap(),
            "\"hello \\\"Pasted\\\"\""
        );
        assert_eq!(
            apply_filter("First & best\n\nSecond", "html_paragraphs", None).unwrap(),
            "<p>First &amp; best</p>\n<p>Second</p>"
        );
        assert_eq!(
            apply_filter("Alpha\nBeta & Gamma", "html_unordered_list", None).unwrap(),
            "<ul>\n  <li>Alpha</li>\n  <li>Beta &amp; Gamma</li>\n</ul>"
        );
        let list_item_config = serde_json::json!({
            "before": "<li>",
            "after": "</li>",
            "applyToEachLine": true
        });
        assert_eq!(
            apply_filter(
                "Alpha\nBeta",
                "quote_text",
                Some(&list_item_config.to_string())
            )
            .unwrap(),
            "<li>Alpha</li>\n<li>Beta</li>"
        );
        let list_config = serde_json::json!({
            "before": "<ul>\n",
            "after": "\n</ul>",
            "applyToEachLine": false
        });
        assert_eq!(
            apply_filter(
                "<li>Alpha</li>",
                "quote_text",
                Some(&list_config.to_string())
            )
            .unwrap(),
            "<ul>\n<li>Alpha</li>\n</ul>"
        );
    }

    #[test]
    fn find_and_replace_honors_editor_modes() {
        let literal = serde_json::json!({
            "pattern": "pasted.app",
            "replacement": "Pasted",
            "matchMode": "literal",
            "caseSensitive": false
        });
        assert_eq!(
            apply_filter(
                "PASTED.APP and pastedXapp",
                "regex",
                Some(&literal.to_string())
            )
            .unwrap(),
            "Pasted and pastedXapp"
        );

        let wildcard = serde_json::json!({
            "pattern": "hello *!",
            "replacement": "hello!",
            "matchMode": "wildcard",
            "caseSensitive": true
        });
        assert_eq!(
            apply_filter("hello Pasted!", "regex", Some(&wildcard.to_string())).unwrap(),
            "hello!"
        );

        let regex = serde_json::json!({
            "pattern": "(Pasted) (App)",
            "replacement": "$2: $1",
            "matchMode": "regex",
            "caseSensitive": true
        });
        assert_eq!(
            apply_filter("Pasted App", "regex", Some(&regex.to_string())).unwrap(),
            "App: Pasted"
        );
    }

    #[test]
    fn test_encodings() {
        let b64_encoded = apply_filter("Pasted App", "base64_encode", None).unwrap();
        let b64_decoded = apply_filter(&b64_encoded, "base64_decode", None).unwrap();
        assert_eq!(b64_decoded, "Pasted App");

        let url_encoded = apply_filter("hello world!", "url_encode", None).unwrap();
        let url_decoded = apply_filter(&url_encoded, "url_decode", None).unwrap();
        assert_eq!(url_decoded, "hello world!");

        let hex_encoded = apply_filter("Pasted", "hex_encode", None).unwrap();
        let hex_decoded = apply_filter(&hex_encoded, "hex_decode", None).unwrap();
        assert_eq!(hex_decoded, "Pasted");
    }

    #[test]
    fn test_smart_punctuation() {
        let text = "--- test...";
        let smart = apply_filter(text, "smart_punctuation", None).unwrap();
        assert!(smart.contains("—"));

        let straight = apply_filter("“Hello”", "straighten_punctuation", None).unwrap();
        assert_eq!(straight, "\"Hello\"");
    }

    #[test]
    fn direct_operations_do_not_require_a_pipeline() {
        assert_eq!(
            apply_filter("  one\n two  ", "trim", None).unwrap(),
            "one\ntwo"
        );
    }

    #[test]
    fn pipelines_execute_operations_in_order() {
        let pipeline = serde_json::json!([
            { "filter_type": "trim" },
            { "filter_type": "uppercase" },
            { "filter_type": "wrap_tags", "config": "strong" }
        ]);
        assert_eq!(
            apply_filter("  hello  ", "pipeline", Some(&pipeline.to_string())).unwrap(),
            "<strong>HELLO</strong>"
        );
    }

    #[test]
    fn malformed_pipelines_report_an_error() {
        let error = apply_filter("hello", "pipeline", Some("not json")).unwrap_err();
        assert!(error.contains("Invalid pipeline configuration"));

        let missing_type = serde_json::json!([{ "config": "ignored" }]);
        let error = apply_filter("hello", "pipeline", Some(&missing_type.to_string())).unwrap_err();
        assert!(error.contains("step 1 has no operation type"));
    }

    #[test]
    fn unknown_operations_never_fall_through_to_shell_execution() {
        let error = apply_filter(
            "sensitive input",
            "unregistered_custom_operation",
            Some("printf 'this must never execute'"),
        )
        .unwrap_err();
        assert_eq!(
            error,
            "Unknown operation type: unregistered_custom_operation"
        );
    }

    #[test]
    fn shell_scripts_fail_closed_even_inside_legacy_pipelines() {
        let direct = apply_filter("sensitive input", "shell_script", Some("cat")).unwrap_err();
        assert_eq!(direct, "Unknown operation type: shell_script");

        let pipeline = serde_json::json!([
            { "filter_type": "trim" },
            { "filter_type": "pipeline", "config": [
                { "filter_type": "shell_script", "config": "cat" }
            ] }
        ]);
        let error =
            apply_filter(" sensitive input ", "pipeline", Some(&pipeline.to_string())).unwrap_err();
        assert!(error.contains("Unknown operation type: shell_script"));
    }

    #[test]
    fn pipeline_errors_identify_the_failing_step() {
        let pipeline = serde_json::json!([
            { "filter_type": "trim" },
            { "filter_type": "missing_operation" }
        ]);
        let error = apply_filter(" hello ", "pipeline", Some(&pipeline.to_string())).unwrap_err();
        assert!(error.contains("Pipeline step 2 (missing_operation) failed"));
    }
}
