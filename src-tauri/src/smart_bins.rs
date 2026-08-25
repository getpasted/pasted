use serde::{Deserialize, Serialize};

pub const SMART_BIN_RULE_VERSION: u32 = 1;
pub const MAX_SMART_BIN_CONDITIONS: usize = 32;
pub const MAX_SMART_BIN_VALUE_CHARS: usize = 2_048;
pub const CURRENT_TARGETS: [&str; 5] = [
    "clip_type",
    "content_type",
    "file_format",
    "source",
    "visual_label",
];
const LEGACY_TARGETS: [&str; 4] = ["origin_kind", "contains", "file_extension", "file_path"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmartBinCondition {
    #[serde(rename = "type")]
    pub target: String,
    #[serde(default)]
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmartBinRule {
    #[serde(default = "rule_version")]
    pub version: u32,
    pub conditions: Vec<SmartBinCondition>,
    #[serde(rename = "match", default = "default_match_mode")]
    pub match_mode: String,
}

fn rule_version() -> u32 {
    SMART_BIN_RULE_VERSION
}

fn default_match_mode() -> String {
    "any".into()
}

fn default_operator(target: &str) -> &'static str {
    if matches!(target, "source" | "contains" | "file_path") {
        "contains"
    } else {
        "is"
    }
}

fn validate_condition(mut condition: SmartBinCondition) -> Result<SmartBinCondition, String> {
    condition.target = condition.target.trim().to_string();
    if condition.target == "source_app" {
        condition.target = "source".into();
    }
    condition.value = condition.value.trim().to_string();
    condition.operator = condition.operator.trim().to_string();
    if condition.operator.is_empty() {
        condition.operator = default_operator(&condition.target).into();
    }
    if !CURRENT_TARGETS.contains(&condition.target.as_str())
        && !LEGACY_TARGETS.contains(&condition.target.as_str())
    {
        return Err(format!(
            "Smart Bin condition type '{}' is not supported",
            condition.target
        ));
    }
    if !matches!(condition.operator.as_str(), "is" | "contains") {
        return Err(format!(
            "Smart Bin condition operator '{}' must be 'is' or 'contains'",
            condition.operator
        ));
    }
    if condition.value.is_empty() {
        return Err("Smart Bin condition values cannot be empty".into());
    }
    if condition.value.chars().count() > MAX_SMART_BIN_VALUE_CHARS {
        return Err(format!(
            "Smart Bin condition values cannot exceed {MAX_SMART_BIN_VALUE_CHARS} characters"
        ));
    }
    Ok(condition)
}

pub fn parse_rule_json(input: &str) -> Result<SmartBinRule, String> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|error| format!("Smart Bin rule must be valid JSON: {error}"))?;
    let mut rule = if value.get("conditions").is_some() {
        serde_json::from_value::<SmartBinRule>(value)
            .map_err(|error| format!("Smart Bin rule is invalid: {error}"))?
    } else {
        let condition = serde_json::from_value::<SmartBinCondition>(value)
            .map_err(|error| format!("Smart Bin rule is invalid: {error}"))?;
        SmartBinRule {
            version: SMART_BIN_RULE_VERSION,
            conditions: vec![condition],
            match_mode: default_match_mode(),
        }
    };
    if rule.version != SMART_BIN_RULE_VERSION {
        return Err(format!(
            "Smart Bin rule version {} is not supported",
            rule.version
        ));
    }
    if rule.conditions.is_empty() {
        return Err("Smart Bin rules require at least one condition".into());
    }
    if rule.conditions.len() > MAX_SMART_BIN_CONDITIONS {
        return Err(format!(
            "Smart Bin rules cannot exceed {MAX_SMART_BIN_CONDITIONS} conditions"
        ));
    }
    if !matches!(rule.match_mode.as_str(), "any" | "all") {
        return Err("Smart Bin match mode must be 'any' or 'all'".into());
    }
    rule.conditions = rule
        .conditions
        .into_iter()
        .map(validate_condition)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rule)
}

pub fn normalize_rule_json(input: &str) -> Result<String, String> {
    serde_json::to_string(&parse_rule_json(input)?)
        .map_err(|error| format!("Smart Bin rule could not be encoded: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_legacy_single_conditions_without_changing_their_semantics() {
        let normalized = normalize_rule_json(r#"{"type":"source","value":"Safari"}"#).unwrap();
        assert_eq!(
            parse_rule_json(&normalized).unwrap(),
            SmartBinRule {
                version: 1,
                conditions: vec![SmartBinCondition {
                    target: "source".into(),
                    operator: "contains".into(),
                    value: "Safari".into(),
                }],
                match_mode: "any".into(),
            }
        );
        let source_app = normalize_rule_json(r#"{"type":"source_app","value":"Safari"}"#).unwrap();
        assert_eq!(
            parse_rule_json(&source_app).unwrap().conditions[0].target,
            "source"
        );
    }

    #[test]
    fn validates_the_public_rule_shape_and_bounds() {
        for invalid in [
            r#"{}"#,
            r#"{"conditions":[],"match":"any"}"#,
            r#"{"conditions":[{"type":"unknown","operator":"is","value":"x"}],"match":"any"}"#,
            r#"{"conditions":[{"type":"source","operator":"equals","value":"x"}],"match":"any"}"#,
            r#"{"conditions":[{"type":"source","operator":"is","value":""}],"match":"any"}"#,
            r#"{"conditions":[{"type":"source","operator":"is","value":"x"}],"match":"none"}"#,
            r#"{"version":2,"conditions":[{"type":"source","operator":"is","value":"x"}],"match":"any"}"#,
        ] {
            assert!(parse_rule_json(invalid).is_err(), "accepted {invalid}");
        }
    }
}
