use super::*;

pub(super) fn build(contains: bool, value: &str, parameters: &mut Vec<Box<dyn ToSql>>) -> String {
    let pattern = if contains {
        format!("%{}%", escape_like_literal(&value.to_lowercase()))
    } else {
        value.to_lowercase()
    };
    parameters.push(Box::new(pattern.clone()));
    parameters.push(Box::new(pattern));
    let comparison = if contains {
        "LIKE ? ESCAPE '\\'"
    } else {
        "= ?"
    };
    format!(
        "(EXISTS (
        SELECT 1 FROM clip_visual_label_overrides AS manual
        WHERE manual.clip_id = clips.id AND manual.operation = 'add'
          AND LOWER(manual.label) {comparison}
    ) OR EXISTS (
        SELECT 1
        FROM clip_analysis_results AS extracted,
             json_each(extracted.result_json, '$.labels') AS label
        WHERE extracted.clip_id = clips.id
          AND extracted.content_hash = clips.content_hash
          AND extracted.input_hash = clips.content_hash
          AND json_extract(extracted.result_json, '$.outcome') = 'produced'
          AND LOWER(json_extract(label.value, '$.value')) {comparison}
          AND NOT EXISTS (
              SELECT 1 FROM clip_visual_label_overrides AS suppressed
              WHERE suppressed.clip_id = clips.id AND suppressed.operation = 'suppress'
                AND LOWER(suppressed.label) = LOWER(json_extract(label.value, '$.value'))
          )
    ))"
    )
}
