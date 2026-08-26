use super::*;

pub(super) fn restore_factory_defaults(transaction: &Connection) -> Result<()> {
    insert_default_bins(transaction)?;
    for preset in crate::content_types::CONTENT_TYPE_GROUP_PRESETS {
        transaction.execute(
            "INSERT INTO content_type_groups
                (id, label, sort_order, is_builtin, is_archived)
             VALUES (?1, ?2, ?3, 1, 0)",
            params![preset.id, preset.label, preset.sort_order],
        )?;
    }
    for preset in crate::content_types::CONTENT_TYPE_PRESETS {
        transaction.execute(
            "INSERT INTO content_types
                (id, label, icon, group_name, is_builtin, is_archived, conceal_clips)
             VALUES (?1, ?2, ?3, ?4, 1, 0, ?5)",
            params![
                preset.id,
                preset.label,
                preset.icon,
                preset.group,
                preset.conceal_clips()
            ],
        )?;
    }
    for preset in crate::content_classification::CLASSIFIER_PRESETS {
        let patterns_json = serde_json::to_string(&preset.patterns)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        transaction.execute(
            "INSERT INTO content_classifiers
                (stable_ref, name, content_type, description, patterns_json, validator, enabled, priority, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, 1)",
            params![preset.stable_ref, preset.name, preset.content_type, preset.description, patterns_json, preset.validator, preset.priority],
        )?;
    }
    for preset in crate::content_extraction::EXTRACTOR_PRESETS {
        let recipe = preset.recipe();
        let recipe_json = serde_json::to_string(&recipe)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let recipe_hash = recipe.hash().map_err(invalid_extractor_input)?;
        transaction.execute(
            "INSERT INTO content_extractors
                (stable_ref, name, description, engine, executable_path, model_path,
                 input_contract, output_contract, enabled, priority, revision,
                 shipped_revision, shipped_definition_json, recipe_json,
                 shipped_recipe_json, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, 1, ?10, ?11, ?12, ?12, 1)",
            params![
                preset.stable_ref,
                preset.name,
                preset.description,
                preset.engine,
                preset.executable_path,
                preset.model_path,
                preset.input_contract,
                preset.output_contract,
                preset.priority,
                preset.revision,
                serde_json::to_string(&preset.definition()).map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                })?,
                recipe_json,
            ],
        )?;
        let extractor_id = transaction.last_insert_rowid();
        transaction.execute(
            "INSERT INTO extractor_recipe_revisions
                (extractor_id, revision, recipe_json, recipe_hash)
             VALUES (?1, 1, ?2, ?3)",
            params![extractor_id, recipe_json, recipe_hash],
        )?;
    }
    let _ = transaction.execute("INSERT INTO clips_fts(clips_fts) VALUES('rebuild')", []);
    Ok(())
}
