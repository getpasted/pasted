use rusqlite::ToSql;

pub(super) fn base(fts_like: &str) -> Vec<String> {
    vec![
        format!(
            "clips.id IN (SELECT rowid FROM clips_fts
                               WHERE text_content {fts_like})"
        ),
        format!(
            "(clips.id IN (SELECT rowid FROM clip_searchable_text_fts
                                WHERE searchable_text {fts_like})
          AND EXISTS (SELECT 1 FROM clip_searchable_text AS extracted
                      WHERE extracted.clip_id = clips.id
                        AND extracted.input_hash = clips.content_hash))"
        ),
        super::super::clip_visual_labels::search_condition().into(),
    ]
}

pub(super) fn push_visual_label_parameters(parameters: &mut Vec<Box<dyn ToSql>>, pattern: &str) {
    parameters.push(Box::new(pattern.to_string()));
    parameters.push(Box::new(pattern.to_string()));
}
