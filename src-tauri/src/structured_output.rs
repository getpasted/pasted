use serde_json::{Map, Value};
use std::collections::HashSet;

const MAX_SCHEMA_DEPTH: usize = 32;
const COMMON_KEYWORDS: &[&str] = &["$schema", "title", "description", "type", "enum"];

pub fn validate_schema(schema: &Value) -> Result<(), String> {
    validate_schema_at(schema, "$", 0)
}

pub fn validate_output(schema: &Value, output: &str) -> Result<(), String> {
    validate_schema(schema)?;
    let value = serde_json::from_str::<Value>(output.trim())
        .map_err(|error| format!("Structured output is not valid JSON: {error}"))?;
    validate_value(schema, &value, "$", 0)
}

fn validate_schema_at(schema: &Value, path: &str, depth: usize) -> Result<(), String> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(format!(
            "Output schema exceeds the {MAX_SCHEMA_DEPTH}-level nesting limit"
        ));
    }
    let object = schema
        .as_object()
        .ok_or_else(|| format!("Output schema at {path} must be an object"))?;
    let (concrete_type, nullable) = declared_type(object, path)?;
    let allowed = allowed_keywords(concrete_type);
    if let Some(keyword) = object.keys().find(|keyword| {
        !COMMON_KEYWORDS.contains(&keyword.as_str()) && !allowed.contains(&keyword.as_str())
    }) {
        return Err(format!(
            "Output schema keyword {keyword} at {path} is not supported"
        ));
    }
    if let Some(dialect) = object.get("$schema") {
        let dialect = dialect
            .as_str()
            .ok_or_else(|| format!("Output schema dialect at {path} must be a string"))?;
        if dialect != "https://json-schema.org/draft/2020-12/schema" {
            return Err(format!("Output schema dialect {dialect} is not supported"));
        }
    }
    for annotation in ["title", "description"] {
        if object
            .get(annotation)
            .is_some_and(|value| !value.is_string())
        {
            return Err(format!(
                "Output schema {annotation} at {path} must be a string"
            ));
        }
    }
    if let Some(values) = object.get("enum") {
        validate_enum(values, concrete_type, nullable, path)?;
    }

    match concrete_type {
        "object" => validate_object_schema(object, path, depth),
        "array" => validate_array_schema(object, path, depth),
        "string" => validate_string_schema(object, path),
        "boolean" => Ok(()),
        "integer" | "number" => validate_numeric_schema(object, path),
        _ => Err(format!(
            "Output schema type {concrete_type} at {path} is not supported"
        )),
    }
}

fn declared_type<'a>(
    object: &'a Map<String, Value>,
    path: &str,
) -> Result<(&'a str, bool), String> {
    let inferred;
    let values = match object.get("type") {
        Some(Value::String(value)) => vec![value.as_str()],
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| format!("Output schema type at {path} must contain strings"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(format!("Output schema type at {path} is invalid")),
        None if object.contains_key("properties") => {
            inferred = "object";
            vec![inferred]
        }
        None => {
            return Err(format!(
                "Output schema at {path} is missing a supported type"
            ))
        }
    };
    let nullable = values.contains(&"null");
    let concrete = values
        .iter()
        .copied()
        .filter(|value| *value != "null")
        .collect::<Vec<_>>();
    if concrete.len() != 1 || values.len() != concrete.len() + usize::from(nullable) {
        return Err(format!(
            "Output schema type union at {path} is not supported"
        ));
    }
    Ok((concrete[0], nullable))
}

fn allowed_keywords(concrete_type: &str) -> &'static [&'static str] {
    match concrete_type {
        "object" => &["properties", "required", "additionalProperties"],
        "array" => &["items", "minItems", "maxItems"],
        "integer" | "number" => &["minimum", "maximum"],
        "string" => &["minLength", "maxLength", "pattern"],
        _ => &[],
    }
}

fn validate_object_schema(
    object: &Map<String, Value>,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    let properties = object
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Output object schema at {path} must define properties"))?;
    let required = match object.get("required") {
        None => Vec::new(),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| format!("Required properties at {path} must be strings"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err(format!("Required properties at {path} must be an array")),
    };
    let mut seen = HashSet::new();
    for name in required {
        if !seen.insert(name) {
            return Err(format!("Required property {name} at {path} is duplicated"));
        }
        if !properties.contains_key(name) {
            return Err(format!("Required property {name} at {path} is not defined"));
        }
    }
    if let Some(additional) = object.get("additionalProperties") {
        if !additional.is_boolean() {
            return Err(format!(
                "additionalProperties at {path} must be true or false"
            ));
        }
    }
    for (name, property) in properties {
        validate_schema_at(property, &format!("{path}.{name}"), depth + 1)?;
    }
    Ok(())
}

fn validate_array_schema(
    object: &Map<String, Value>,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    let items = object
        .get("items")
        .ok_or_else(|| format!("Output array schema at {path} must define items"))?;
    validate_schema_at(items, &format!("{path}[]"), depth + 1)?;
    let minimum = optional_nonnegative_integer(object, "minItems", path)?;
    let maximum = optional_nonnegative_integer(object, "maxItems", path)?;
    if minimum
        .zip(maximum)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(format!("minItems exceeds maxItems at {path}"));
    }
    Ok(())
}

fn validate_numeric_schema(object: &Map<String, Value>, path: &str) -> Result<(), String> {
    let minimum = optional_number(object, "minimum", path)?;
    let maximum = optional_number(object, "maximum", path)?;
    if minimum
        .zip(maximum)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(format!("minimum exceeds maximum at {path}"));
    }
    Ok(())
}

fn validate_string_schema(object: &Map<String, Value>, path: &str) -> Result<(), String> {
    let minimum = optional_nonnegative_integer(object, "minLength", path)?;
    let maximum = optional_nonnegative_integer(object, "maxLength", path)?;
    if minimum
        .zip(maximum)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(format!("minLength exceeds maxLength at {path}"));
    }
    if let Some(pattern) = object.get("pattern") {
        let pattern = pattern
            .as_str()
            .ok_or_else(|| format!("pattern at {path} must be a string"))?;
        regex::Regex::new(pattern)
            .map_err(|error| format!("pattern at {path} is invalid: {error}"))?;
    }
    Ok(())
}

fn validate_enum(
    value: &Value,
    concrete_type: &str,
    nullable: bool,
    path: &str,
) -> Result<(), String> {
    let values = value
        .as_array()
        .filter(|values| !values.is_empty())
        .ok_or_else(|| format!("Output schema enum at {path} must be a nonempty array"))?;
    let mut seen = HashSet::new();
    for value in values {
        if !value_matches_type(value, concrete_type, nullable) {
            return Err(format!(
                "Output schema enum value at {path} has the wrong type"
            ));
        }
        let encoded = serde_json::to_string(value).map_err(|error| error.to_string())?;
        if !seen.insert(encoded) {
            return Err(format!("Output schema enum at {path} contains duplicates"));
        }
    }
    if concrete_type == "boolean" && values.len() == 1 {
        return Err(format!(
            "A single-value boolean enum at {path} is not supported"
        ));
    }
    if matches!(concrete_type, "object" | "array") {
        return Err(format!(
            "Output schema enum at {path} is not supported for {concrete_type}"
        ));
    }
    Ok(())
}

fn value_matches_type(value: &Value, concrete_type: &str, nullable: bool) -> bool {
    if value.is_null() {
        return nullable;
    }
    match concrete_type {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        _ => false,
    }
}

fn optional_nonnegative_integer(
    object: &Map<String, Value>,
    keyword: &str,
    path: &str,
) -> Result<Option<u64>, String> {
    object
        .get(keyword)
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("{keyword} at {path} must be a nonnegative integer"))
        })
        .transpose()
}

fn optional_number(
    object: &Map<String, Value>,
    keyword: &str,
    path: &str,
) -> Result<Option<f64>, String> {
    object
        .get(keyword)
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("{keyword} at {path} must be a finite number"))
        })
        .transpose()
}

fn validate_value(schema: &Value, value: &Value, path: &str, depth: usize) -> Result<(), String> {
    if depth > MAX_SCHEMA_DEPTH {
        return Err(format!(
            "Structured output exceeds the {MAX_SCHEMA_DEPTH}-level nesting limit"
        ));
    }
    let object = schema.as_object().expect("schema validated before output");
    let (concrete_type, nullable) = declared_type(object, path)?;
    if value.is_null() && nullable {
        return validate_enum_value(object, value, path);
    }
    if !value_matches_type(value, concrete_type, false) {
        return Err(format!(
            "Structured output at {path} must be {concrete_type}"
        ));
    }
    validate_enum_value(object, value, path)?;
    match concrete_type {
        "object" => validate_object_value(object, value.as_object().unwrap(), path, depth),
        "array" => validate_array_value(object, value.as_array().unwrap(), path, depth),
        "string" => validate_string_value(object, value.as_str().unwrap(), path),
        "integer" | "number" => validate_numeric_value(object, value, path),
        _ => Ok(()),
    }
}

fn validate_string_value(
    schema: &Map<String, Value>,
    value: &str,
    path: &str,
) -> Result<(), String> {
    let length = value.chars().count();
    if schema
        .get("minLength")
        .and_then(Value::as_u64)
        .is_some_and(|minimum| length < minimum as usize)
    {
        return Err(format!("Structured output string at {path} is too short"));
    }
    if schema
        .get("maxLength")
        .and_then(Value::as_u64)
        .is_some_and(|maximum| length > maximum as usize)
    {
        return Err(format!("Structured output string at {path} is too long"));
    }
    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
        let regex = regex::Regex::new(pattern).expect("schema pattern validated");
        if !regex.is_match(value) {
            return Err(format!(
                "Structured output string at {path} does not match pattern"
            ));
        }
    }
    Ok(())
}

fn validate_enum_value(
    schema: &Map<String, Value>,
    value: &Value,
    path: &str,
) -> Result<(), String> {
    if schema
        .get("enum")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.contains(value))
    {
        return Err(format!(
            "Structured output at {path} is not an allowed enum value"
        ));
    }
    Ok(())
}

fn validate_object_value(
    schema: &Map<String, Value>,
    value: &Map<String, Value>,
    path: &str,
    depth: usize,
) -> Result<(), String> {
    let properties = schema["properties"].as_object().unwrap();
    for name in schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !value.contains_key(name) {
            return Err(format!("Structured output at {path} is missing {name}"));
        }
    }
    if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
        if let Some(name) = value.keys().find(|name| !properties.contains_key(*name)) {
            return Err(format!(
                "Structured output at {path} contains unexpected property {name}"
            ));
        }
    }
    for (name, property_schema) in properties {
        if let Some(property_value) = value.get(name) {
            validate_value(
                property_schema,
                property_value,
                &format!("{path}.{name}"),
                depth + 1,
            )?;
        }
    }
    Ok(())
}

fn validate_array_value(
    schema: &Map<String, Value>,
    value: &[Value],
    path: &str,
    depth: usize,
) -> Result<(), String> {
    if schema
        .get("minItems")
        .and_then(Value::as_u64)
        .is_some_and(|minimum| value.len() < minimum as usize)
    {
        return Err(format!("Structured output array at {path} is too short"));
    }
    if schema
        .get("maxItems")
        .and_then(Value::as_u64)
        .is_some_and(|maximum| value.len() > maximum as usize)
    {
        return Err(format!("Structured output array at {path} is too long"));
    }
    for (index, item) in value.iter().enumerate() {
        validate_value(
            &schema["items"],
            item,
            &format!("{path}[{index}]"),
            depth + 1,
        )?;
    }
    Ok(())
}

fn validate_numeric_value(
    schema: &Map<String, Value>,
    value: &Value,
    path: &str,
) -> Result<(), String> {
    let number = value.as_f64().expect("numeric output validated");
    if schema
        .get("minimum")
        .and_then(Value::as_f64)
        .is_some_and(|minimum| number < minimum)
    {
        return Err(format!(
            "Structured output number at {path} is below minimum"
        ));
    }
    if schema
        .get("maximum")
        .and_then(Value::as_f64)
        .is_some_and(|maximum| number > maximum)
    {
        return Err(format!(
            "Structured output number at {path} is above maximum"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "sentiment": { "type": "string", "enum": ["positive", "neutral", "negative"] },
                "scores": {
                    "type": "array",
                    "items": { "type": "number", "minimum": 0, "maximum": 1 },
                    "minItems": 1,
                    "maxItems": 2
                },
                "note": { "type": ["string", "null"] }
            },
            "required": ["sentiment", "scores", "note"],
            "additionalProperties": false
        })
    }

    #[test]
    fn validates_the_supported_schema_subset_and_output() {
        validate_schema(&schema()).unwrap();
        validate_output(
            &schema(),
            r#"{"sentiment":"positive","scores":[0.25,1],"note":null}"#,
        )
        .unwrap();
    }

    #[test]
    fn rejects_unsupported_constraints_instead_of_ignoring_them() {
        let unsupported = serde_json::json!({ "type": "string", "format": "email" });
        assert!(validate_schema(&unsupported)
            .unwrap_err()
            .contains("format"));
    }

    #[test]
    fn rejects_output_that_does_not_match_the_schema() {
        let error = validate_output(
            &schema(),
            r#"{"sentiment":"surprised","scores":[0.5],"note":null}"#,
        )
        .unwrap_err();
        assert!(error.contains("enum"));
    }

    #[test]
    fn rejects_unknown_required_properties_and_invalid_ranges() {
        assert!(validate_schema(&serde_json::json!({
            "type": "object",
            "properties": {},
            "required": ["missing"]
        }))
        .is_err());
        assert!(validate_schema(&serde_json::json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 2,
            "maxItems": 1
        }))
        .is_err());
    }
}
