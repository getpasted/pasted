pub(super) fn extractor_recipe_schema() -> serde_json::Value {
    let mut schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["name", "description", "recipe", "setupGuidance"],
        "properties": {
            "name": { "type": "string", "minLength": 1, "maxLength": 80 },
            "description": { "type": "string", "maxLength": 240 },
            "setupGuidance": {
                "type": "array",
                "maxItems": 16,
                "items": { "type": "string", "maxLength": 500 }
            },
            "recipe": {
                "type": "object",
                "additionalProperties": false,
                "required": ["definitionVersion", "accepts", "acceptedFileFormats", "postProcessing", "output", "steps", "resources"],
                "properties": {
                    "definitionVersion": { "type": "integer", "enum": [1] },
                    "accepts": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 2,
                        "items": { "type": "string", "enum": ["image", "file_references"] }
                    },
                    "acceptedFileFormats": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 64,
                        "items": { "type": "string", "pattern": "^(?:\\*|[a-z0-9]{1,16})$" }
                    },
                    "output": { "type": "string", "enum": ["searchable_text"] },
                    "steps": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 16,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["id", "executable", "arguments", "mode", "capture", "outputExtension", "noOutputExitCodes", "timeoutSeconds"],
                            "properties": {
                                "id": { "type": "string", "pattern": "^[A-Za-z0-9_-]{1,64}$" },
                                "executable": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "required": ["path", "discover", "versionArguments"],
                                    "properties": {
                                        "path": { "type": ["string", "null"] },
                                        "discover": { "type": "array", "maxItems": 16, "items": { "type": "string" } },
                                        "versionArguments": { "type": "array", "maxItems": 16, "items": { "type": "string" } }
                                    }
                                },
                                "arguments": { "type": "array", "maxItems": 128, "items": { "type": "string" } },
                                "mode": { "type": "string", "enum": ["once", "each_input"] },
                                "capture": { "type": "string", "enum": ["ignore", "stdout_text", "file_text", "pasted_json_v1"] },
                                "outputExtension": { "type": ["string", "null"], "maxLength": 16 },
                                "noOutputExitCodes": { "type": "array", "maxItems": 16, "items": { "type": "integer", "minimum": 1, "maximum": 2147483647 } },
                                "timeoutSeconds": { "type": "integer", "minimum": 1, "maximum": 600 }
                            }
                        }
                    },
                    "resources": {
                        "type": "array",
                        "maxItems": 32,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["id", "label", "kind", "required", "path"],
                            "properties": {
                                "id": { "type": "string", "pattern": "^[A-Za-z0-9_-]{1,64}$" },
                                "label": { "type": "string", "minLength": 1, "maxLength": 80 },
                                "kind": { "type": "string", "enum": ["file", "directory"] },
                                "required": { "type": "boolean" },
                                "path": { "type": ["string", "null"] }
                            }
                        }
                    }
                }
            }
        }
    });
    let recipe = schema
        .pointer_mut("/properties/recipe")
        .and_then(serde_json::Value::as_object_mut)
        .expect("recipe schema object");
    recipe
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .expect("recipe properties")
        .insert(
            "postProcessing".into(),
            serde_json::json!({
                "type": "array",
                "maxItems": 1,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["kind", "minimumPercent"],
                    "properties": {
                        "kind": { "type": "string", "enum": ["filter_labels_by_confidence"] },
                        "minimumPercent": { "type": "integer", "minimum": 0, "maximum": 100 }
                    }
                }
            }),
        );
    schema
}
