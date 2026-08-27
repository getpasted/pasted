use super::extractor_recipe_schema;

#[test]
fn proposal_schema_uses_the_structured_outputs_subset() {
    let schema = extractor_recipe_schema();
    crate::structured_output::validate_schema(&schema).unwrap();
    assert!(schema
        .pointer("/properties/recipe/properties/accepts/uniqueItems")
        .is_none());
    assert_eq!(
        schema.pointer("/properties/recipe/properties/accepts/items/enum"),
        Some(&serde_json::json!(["image", "file_references"]))
    );
    assert_eq!(
        schema.pointer(
            "/properties/recipe/properties/steps/items/properties/noOutputExitCodes/items/minimum"
        ),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/properties/recipe/properties/postProcessing/items/properties/kind/enum"),
        Some(&serde_json::json!(["filter_labels_by_confidence"]))
    );
    assert!(schema
        .pointer("/properties/recipe/properties/minimumVisualLabelConfidence")
        .is_none());
}
