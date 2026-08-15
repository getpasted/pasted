use once_cell::sync::Lazy;
use parking_lot::Mutex;
use regex::Regex;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;

// Definitions recur across clips, so retain both successful and failed compilations without
// allowing an arbitrarily large custom registry to create an unbounded process cache.
const CLASSIFIER_REGEX_CACHE_CAPACITY: usize = 1_024;

#[derive(Default)]
struct ClassifierRegexCache {
    entries: HashMap<String, Option<Regex>>,
    insertion_order: VecDeque<String>,
}

impl ClassifierRegexCache {
    fn get(&self, pattern: &str) -> Option<Option<Regex>> {
        self.entries.get(pattern).cloned()
    }

    fn insert(&mut self, pattern: String, regex: Option<Regex>) -> Option<Regex> {
        if let Some(existing) = self.entries.get(&pattern) {
            return existing.clone();
        }
        if self.entries.len() >= CLASSIFIER_REGEX_CACHE_CAPACITY {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.insertion_order.push_back(pattern.clone());
        self.entries.insert(pattern, regex.clone());
        regex
    }
}

static CLASSIFIER_REGEX_CACHE: Lazy<Mutex<ClassifierRegexCache>> =
    Lazy::new(|| Mutex::new(ClassifierRegexCache::default()));

fn compiled_classifier_pattern(pattern: &str) -> Option<Regex> {
    if let Some(regex) = CLASSIFIER_REGEX_CACHE.lock().get(pattern) {
        return regex;
    }
    let compiled = Regex::new(pattern).ok();
    CLASSIFIER_REGEX_CACHE
        .lock()
        .insert(pattern.to_owned(), compiled)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Classifier {
    pub id: i64,
    pub stable_ref: String,
    pub name: String,
    pub content_type: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub validator: Option<String>,
    pub enabled: bool,
    pub priority: i64,
    pub is_builtin: bool,
    #[serde(default)]
    pub defaults: Option<ClassifierDefaults>,
    #[serde(default)]
    pub is_deleted: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ClassifierDefaults {
    pub name: String,
    pub content_type: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub validator: Option<String>,
    pub enabled: bool,
    pub priority: i64,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ClassifierInput {
    pub name: String,
    pub content_type: String,
    pub description: String,
    pub patterns: Vec<String>,
    pub validator: Option<String>,
    pub enabled: bool,
    pub priority: i64,
}

#[derive(Clone, Copy)]
pub struct ClassifierPreset {
    pub stable_ref: &'static str,
    pub name: &'static str,
    pub content_type: &'static str,
    pub description: &'static str,
    pub patterns: &'static [&'static str],
    pub validator: Option<&'static str>,
    pub priority: i64,
}

pub const CLASSIFIER_PRESETS: &[ClassifierPreset] = &[
    ClassifierPreset {
        stable_ref: "color",
        name: "Colors",
        content_type: "color",
        description: "Hex, RGB, and HSL color values",
        patterns: &[
            r"(?i)^#(?:[0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$",
            r"(?i)^(?:rgb|rgba|hsl|hsla)\(.+\)$",
        ],
        validator: None,
        priority: 10,
    },
    ClassifierPreset {
        stable_ref: "url",
        name: "Web Links",
        content_type: "link",
        description: "Web, file, and email URLs",
        patterns: &[r"(?i)^(?:(?:https?|file)://|mailto:).+$"],
        validator: None,
        priority: 20,
    },
    ClassifierPreset {
        stable_ref: "email",
        name: "Email Addresses",
        content_type: "email",
        description: "Individual email addresses",
        patterns: &[
            r"(?i)^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+$",
        ],
        validator: None,
        priority: 30,
    },
    ClassifierPreset {
        stable_ref: "env_block",
        name: "Environment Blocks",
        content_type: "env_block",
        description: "Two or more environment assignments",
        patterns: &[r"(?m)^(?:(?:export\s+)?[A-Za-z_][A-Za-z0-9_]*\s*=.*\s*){2,}$"],
        validator: Some("env_block"),
        priority: 40,
    },
    ClassifierPreset {
        stable_ref: "jwt",
        name: "JSON Web Tokens",
        content_type: "jwt",
        description: "Three-part JSON Web Tokens",
        patterns: &[r"^eyJ[A-Za-z0-9_-]{2,}\.[A-Za-z0-9_-]{2,}\.[A-Za-z0-9_-]*$"],
        validator: None,
        priority: 50,
    },
    ClassifierPreset {
        stable_ref: "credential",
        name: "Credentials",
        content_type: "credential",
        description: "Known API-key formats and secret assignments",
        patterns: &[
            r"(?i)^(?:(?:sk_(?:live|test|proj)_|gh[opusr]_|github_pat_|xox[baprs]-|sk-ant-)[A-Za-z0-9_.=-]+|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{20,})$",
            r"(?i)^(?:export\s+)?(?:api[_-]?key|access[_-]?token|auth[_-]?token|client[_-]?secret|secret[_-]?key|password|passwd)\s*[:=]\s*\S+$",
        ],
        validator: None,
        priority: 60,
    },
    ClassifierPreset {
        stable_ref: "payment_card",
        name: "Payment Cards",
        content_type: "payment_card",
        description: "Card-number candidates with checksum validation",
        patterns: &[r"^[0-9][0-9 -]{11,21}[0-9]$"],
        validator: Some("luhn"),
        priority: 70,
    },
    ClassifierPreset {
        stable_ref: "iban",
        name: "IBANs",
        content_type: "iban",
        description: "International bank account numbers",
        patterns: &[r"(?i)^[A-Z]{2}[0-9]{2}(?:[ ]?[A-Z0-9]){11,30}$"],
        validator: Some("iban"),
        priority: 80,
    },
    ClassifierPreset {
        stable_ref: "ip_address",
        name: "IP Addresses",
        content_type: "ip_address",
        description: "IPv4 and IPv6 addresses",
        patterns: &[r"^[0-9A-Fa-f:.]+$"],
        validator: Some("ip"),
        priority: 90,
    },
    ClassifierPreset {
        stable_ref: "mac_address",
        name: "MAC Addresses",
        content_type: "mac_address",
        description: "Colon, dash, or dotted hardware addresses",
        patterns: &[
            r"(?i)^(?:[0-9a-f]{2}[:-]){5}[0-9a-f]{2}$",
            r"(?i)^(?:[0-9a-f]{4}\.){2}[0-9a-f]{4}$",
        ],
        validator: None,
        priority: 100,
    },
    ClassifierPreset {
        stable_ref: "uuid",
        name: "UUIDs",
        content_type: "uuid",
        description: "Standard versioned UUID values",
        patterns: &[
            r"(?i)^(?:urn:uuid:)?\{?[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\}?$",
        ],
        validator: None,
        priority: 110,
    },
    ClassifierPreset {
        stable_ref: "hash",
        name: "Hashes",
        content_type: "hash",
        description: "Common hexadecimal digest lengths",
        patterns: &[r"(?i)^(?:[0-9a-f]{32}|[0-9a-f]{40}|[0-9a-f]{64}|[0-9a-f]{96}|[0-9a-f]{128})$"],
        validator: None,
        priority: 120,
    },
    ClassifierPreset {
        stable_ref: "file_path",
        name: "File Paths",
        content_type: "file_path",
        description: "Unix, home-relative, UNC, and drive-letter paths",
        patterns: &[r"^(?:/|~/|\./|\.\./|\\\\).+$", r"(?i)^[a-z]:[\\/].+$"],
        validator: None,
        priority: 130,
    },
    ClassifierPreset {
        stable_ref: "env_variable",
        name: "Environment Variables",
        content_type: "env_variable",
        description: "Single shell-style environment assignments",
        patterns: &[r"^(?:export\s+)?[A-Za-z_][A-Za-z0-9_]*\s*=.*$"],
        validator: None,
        priority: 140,
    },
    ClassifierPreset {
        stable_ref: "shell_command",
        name: "Shell Commands",
        content_type: "shell_command",
        description: "Common terminal commands",
        patterns: &[
            r"^(?:\$\s*)?(?:sudo\s+)?(?:cd|ls|pwd|mkdir|touch|cp|mv|rm|cat|grep|rg|find|curl|wget|ssh|scp|git|docker|podman|kubectl|npm|pnpm|yarn|cargo|python3?|node|brew|apt|dnf|systemctl|chmod|chown)(?:\s|$)",
        ],
        validator: None,
        priority: 150,
    },
    ClassifierPreset {
        stable_ref: "phone",
        name: "Phone Numbers",
        content_type: "phone",
        description: "Formatted international and local phone numbers",
        patterns: &[r"^\+?[0-9][0-9 ().-]{5,}[0-9](?:\s*(?:x|ext\.?)[ ]?\d{1,6})?$"],
        validator: Some("phone"),
        priority: 160,
    },
    ClassifierPreset {
        stable_ref: "code",
        name: "Code",
        content_type: "code",
        description: "Common programming-language syntax",
        patterns: &[
            r"(?s)(?:\bfunction\s|\bconst\s|\blet\s|\bvar\s|\bimport\s|\bpub fn\s|\bclass\s|\bdef\s|\bSELECT\s|\{.*\}.*;)",
        ],
        validator: None,
        priority: 170,
    },
    ClassifierPreset {
        stable_ref: "prose",
        name: "Prose",
        content_type: "prose",
        description: "Sentence-like natural-language text",
        patterns: &[r"(?s)^.{24,}$"],
        validator: Some("prose"),
        priority: 180,
    },
];

pub fn classifier_defaults(stable_ref: &str) -> Option<ClassifierDefaults> {
    CLASSIFIER_PRESETS
        .iter()
        .find(|preset| preset.stable_ref == stable_ref)
        .map(|preset| ClassifierDefaults {
            name: preset.name.into(),
            content_type: preset.content_type.into(),
            description: preset.description.into(),
            patterns: preset
                .patterns
                .iter()
                .map(|pattern| (*pattern).into())
                .collect(),
            validator: preset.validator.map(str::to_string),
            enabled: true,
            priority: preset.priority,
        })
}

pub fn validate_classifier_input(input: &ClassifierInput) -> Result<(), String> {
    if input.name.trim().is_empty() || input.name.chars().count() > 80 {
        return Err("Classifier names must contain 1–80 characters".into());
    }
    if input.content_type.trim().is_empty()
        || !input.content_type.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
    {
        return Err("Content types must use lowercase letters, numbers, and underscores".into());
    }
    if input.description.chars().count() > 500 {
        return Err("Classifier descriptions are limited to 500 characters".into());
    }
    if !(-10_000..=10_000).contains(&input.priority) {
        return Err("Classifier priority must be between -10,000 and 10,000".into());
    }
    if input.validator.as_deref().is_some_and(|validator| {
        !matches!(
            validator,
            "env_block" | "luhn" | "iban" | "ip" | "phone" | "prose"
        )
    }) {
        return Err("Unknown classifier validator".into());
    }
    if input.patterns.is_empty() || input.patterns.len() > 16 {
        return Err("Classifiers require 1–16 regex patterns".into());
    }
    for pattern in &input.patterns {
        if pattern.len() > 2_048 {
            return Err("Classifier regex patterns are limited to 2,048 characters".into());
        }
        Regex::new(pattern).map_err(|error| format!("Invalid regex: {error}"))?;
    }
    Ok(())
}

pub fn classify_text(text: &str, classifiers: &[Classifier]) -> String {
    classify_with_classifiers(text, classifiers)
        .map(|matched| matched.content_type)
        .unwrap_or_else(|| "text".into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassificationMatch {
    pub classifier_ref: String,
    pub content_type: String,
}

pub fn classify_with_classifiers(
    text: &str,
    classifiers: &[Classifier],
) -> Option<ClassificationMatch> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    classifiers
        .iter()
        .filter(|classifier| classifier.enabled)
        .find_map(|classifier| {
            let candidate = classifier.patterns.iter().any(|pattern| {
                compiled_classifier_pattern(pattern).is_some_and(|regex| regex.is_match(trimmed))
            });
            (candidate && passes_validator(trimmed, classifier.validator.as_deref())).then(|| {
                ClassificationMatch {
                    classifier_ref: classifier.stable_ref.clone(),
                    content_type: classifier.content_type.clone(),
                }
            })
        })
}

fn passes_validator(value: &str, validator: Option<&str>) -> bool {
    match validator {
        None => true,
        Some("env_block") => is_env_block(value),
        Some("luhn") => is_payment_card(value),
        Some("iban") => is_iban(value),
        Some("ip") => value.parse::<IpAddr>().is_ok(),
        Some("phone") => is_phone(value),
        Some("prose") => is_prose(value),
        Some(_) => false,
    }
}

static ENV_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:export\s+)?[A-Za-z_][A-Za-z0-9_]*\s*=.*$").expect("environment regex")
});
static PHONE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\+?[0-9][0-9 ().-]{5,}[0-9](?:\s*(?:x|ext\.?)[ ]?\d{1,6})?$")
        .expect("phone regex")
});
static IBAN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Z]{2}[0-9]{2}[A-Z0-9]{11,30}$").expect("IBAN regex"));

fn is_env_block(value: &str) -> bool {
    let lines: Vec<&str> = value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    lines.len() >= 2 && lines.iter().all(|line| ENV_LINE.is_match(line))
}

fn is_phone(value: &str) -> bool {
    if !PHONE.is_match(value) {
        return false;
    }
    let digit_count = value.chars().filter(char::is_ascii_digit).count();
    let has_phone_punctuation = value.starts_with('+')
        || value.contains(' ')
        || value.contains('-')
        || value.contains('(')
        || value.contains('.');
    (7..=15).contains(&digit_count) && has_phone_punctuation
}

fn is_payment_card(value: &str) -> bool {
    if !value
        .chars()
        .all(|character| character.is_ascii_digit() || character == ' ' || character == '-')
    {
        return false;
    }
    let digits: Vec<u32> = value
        .chars()
        .filter_map(|character| character.to_digit(10))
        .collect();
    if !(13..=19).contains(&digits.len()) {
        return false;
    }
    let checksum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(index, digit)| {
            if index % 2 == 1 {
                let doubled = digit * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                *digit
            }
        })
        .sum();
    checksum.is_multiple_of(10)
}

fn is_iban(value: &str) -> bool {
    let compact: String = value
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase();
    if !IBAN.is_match(&compact) {
        return false;
    }
    let rearranged = format!("{}{}", &compact[4..], &compact[..4]);
    let mut remainder = 0_u32;
    for character in rearranged.chars() {
        if let Some(digit) = character.to_digit(10) {
            remainder = (remainder * 10 + digit) % 97;
        } else {
            let value = character as u32 - 'A' as u32 + 10;
            remainder = (remainder * 100 + value) % 97;
        }
    }
    remainder == 1
}

fn is_prose(value: &str) -> bool {
    let words = value.split_whitespace().count();
    words >= 8
        && value
            .chars()
            .any(|character| matches!(character, '.' | '!' | '?'))
        && value.chars().any(char::is_alphabetic)
}

#[cfg(test)]
mod tests {
    use super::{
        classify_text, compiled_classifier_pattern, Classifier, CLASSIFIER_PRESETS,
        CLASSIFIER_REGEX_CACHE_CAPACITY,
    };

    fn classify(value: &str) -> String {
        let classifiers = CLASSIFIER_PRESETS
            .iter()
            .enumerate()
            .map(|(index, preset)| Classifier {
                id: index as i64,
                stable_ref: preset.stable_ref.into(),
                name: preset.name.into(),
                content_type: preset.content_type.into(),
                description: preset.description.into(),
                patterns: preset
                    .patterns
                    .iter()
                    .map(|pattern| (*pattern).into())
                    .collect(),
                validator: preset.validator.map(str::to_string),
                enabled: true,
                priority: preset.priority,
                is_builtin: true,
                defaults: super::classifier_defaults(preset.stable_ref),
                is_deleted: false,
            })
            .collect::<Vec<_>>();
        classify_text(value, &classifiers)
    }

    #[test]
    fn recognizes_structured_text_without_revealing_it() {
        for (value, expected) in [
            ("hello@example.com", "email"),
            ("/Users/pasted/Downloads/report.pdf", "file_path"),
            ("sk_test_abcdefghijklmnopqrstuvwxyz", "credential"),
            ("AKIAIOSFODNN7EXAMPLE", "credential"),
            ("4242 4242 4242 4242", "payment_card"),
            ("DATABASE_URL=postgres://localhost/pasted", "env_variable"),
            ("d41d8cd98f00b204e9800998ecf8427e", "hash"),
            ("GB82 WEST 1234 5698 7654 32", "iban"),
            ("gb82 west 1234 5698 7654 32", "iban"),
            ("2001:db8::1", "ip_address"),
            ("00:1A:2B:3C:4D:5E", "mac_address"),
            ("+1 (312) 555-0187", "phone"),
            ("git status --short", "shell_command"),
            ("550e8400-e29b-41d4-a716-446655440000", "uuid"),
        ] {
            assert_eq!(classify(value), expected, "failed to classify {value}");
        }
    }

    #[test]
    fn recognizes_blocks_tokens_and_prose() {
        assert_eq!(classify("HOST=localhost\nPORT=5432"), "env_block");
        assert_eq!(
            classify("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.signature"),
            "jwt"
        );
        assert_eq!(
            classify("This is a complete sentence with enough words to be recognized as prose."),
            "prose"
        );
    }

    #[test]
    fn avoids_ambiguous_numeric_and_identifier_false_positives() {
        assert_eq!(classify("313041"), "text");
        assert_eq!(classify("1234567890123456"), "text");
        assert_eq!(classify("not-an-email@example"), "text");
    }

    #[test]
    fn compiled_pattern_cache_preserves_valid_and_invalid_regex_behavior() {
        for _ in 0..2 {
            assert!(compiled_classifier_pattern(r"^cached-[0-9]+$")
                .is_some_and(|regex| regex.is_match("cached-42")));
            assert!(compiled_classifier_pattern("[").is_none());
        }
    }

    #[test]
    fn compiled_pattern_cache_is_bounded() {
        for index in 0..=CLASSIFIER_REGEX_CACHE_CAPACITY {
            assert!(compiled_classifier_pattern(&format!(r"^cache-bound-{index}$")).is_some());
        }
        assert!(
            super::CLASSIFIER_REGEX_CACHE.lock().entries.len() <= CLASSIFIER_REGEX_CACHE_CAPACITY
        );
    }
}
