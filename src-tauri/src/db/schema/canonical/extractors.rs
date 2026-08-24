use super::*;

pub(super) fn initialize_extractor_registry(conn: &Connection) -> Result<()> {
    for preset in crate::content_extraction::EXTRACTOR_PRESETS {
        conn.execute(
            "INSERT OR IGNORE INTO content_extractors
                (stable_ref, name, description, engine, executable_path, model_path,
                 input_contract, output_contract, enabled, priority, revision,
                 shipped_revision, shipped_definition_json, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, 1, ?10, ?11, 1)",
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
                })?
            ],
        )?;
        conn.execute(
            "UPDATE content_extractors
             SET shipped_revision = COALESCE(shipped_revision, ?1),
                 shipped_definition_json = COALESCE(shipped_definition_json, ?2)
             WHERE stable_ref = ?3 AND is_builtin = 1",
            params![
                preset.revision,
                serde_json::to_string(&preset.definition()).map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                })?,
                preset.stable_ref,
            ],
        )?;
        let shipped = conn.query_row(
            "SELECT shipped_revision, shipped_definition_json,
                    name, description, engine, executable_path, model_path,
                    input_contract, output_contract, enabled, priority
             FROM content_extractors WHERE stable_ref = ?1 AND is_builtin = 1",
            params![preset.stable_ref],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    crate::content_extraction::ExtractorDefinitionInput {
                        name: row.get(2)?,
                        description: row.get(3)?,
                        engine: row.get(4)?,
                        executable_path: row.get(5)?,
                        model_path: row.get(6)?,
                        input_contract: row.get(7)?,
                        output_contract: row.get(8)?,
                        enabled: row.get(9)?,
                        priority: row.get(10)?,
                    },
                ))
            },
        )?;
        if shipped.0 < preset.revision {
            let previous = serde_json::from_str::<
                crate::content_extraction::ExtractorDefinitionInput,
            >(&shipped.1)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let next = preset.definition();
            let effective =
                crate::content_extraction::merge_shipped_definition(&shipped.2, &previous, &next);
            conn.execute(
                "UPDATE content_extractors
                 SET name = ?1, description = ?2, engine = ?3, executable_path = ?4,
                     model_path = ?5, input_contract = ?6, output_contract = ?7,
                     enabled = ?8, priority = ?9, revision = revision + 1,
                     shipped_revision = ?10, shipped_definition_json = ?11,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE stable_ref = ?12 AND is_builtin = 1",
                params![
                    effective.name,
                    effective.description,
                    effective.engine,
                    effective.executable_path,
                    effective.model_path,
                    effective.input_contract,
                    effective.output_contract,
                    effective.enabled,
                    effective.priority,
                    preset.revision,
                    serde_json::to_string(&next).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?,
                    preset.stable_ref,
                ],
            )?;
        }
        let recipe = preset.recipe();
        crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error,
            )))
        })?;
        let recipe_json = serde_json::to_string(&recipe)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let (current_recipe, previous_shipped_recipe) = conn.query_row(
            "SELECT recipe_json, shipped_recipe_json
             FROM content_extractors WHERE stable_ref = ?1 AND is_builtin = 1",
            params![preset.stable_ref],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )?;
        let effective_recipe = match (current_recipe, previous_shipped_recipe) {
            (Some(current), Some(previous)) => {
                let matches_previous = match (
                    serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&current),
                    serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&previous),
                ) {
                    (Ok(current), Ok(previous)) => current == previous,
                    _ => current == previous,
                };
                if matches_previous {
                    recipe_json.clone()
                } else {
                    current
                }
            }
            (Some(current), None) => current,
            _ => recipe_json.clone(),
        };
        let effective_recipe =
            serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&effective_recipe)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let effective_recipe = crate::content_extraction::migrate_builtin_recipe_compatibility(
            preset.stable_ref,
            &effective_recipe,
            shipped.2.model_path.as_deref(),
        );
        crate::extractor_recipe::validate_recipe(&effective_recipe).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error,
            )))
        })?;
        let effective_recipe = serde_json::to_string(&effective_recipe)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        conn.execute(
            "UPDATE content_extractors
             SET recipe_json = ?1, shipped_recipe_json = ?2
             WHERE stable_ref = ?3 AND is_builtin = 1",
            params![effective_recipe, recipe_json, preset.stable_ref],
        )?;
    }
    {
        let legacy = {
            let mut statement = conn.prepare(
                "SELECT id, name, description, engine, executable_path, model_path,
                        input_contract, output_contract, enabled, priority, revision
                 FROM content_extractors WHERE recipe_json IS NULL",
            )?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(10)?,
                        crate::content_extraction::ExtractorDefinitionInput {
                            name: row.get(1)?,
                            description: row.get(2)?,
                            engine: row.get(3)?,
                            executable_path: row.get(4)?,
                            model_path: row.get(5)?,
                            input_contract: row.get(6)?,
                            output_contract: row.get(7)?,
                            enabled: row.get(8)?,
                            priority: row.get(9)?,
                        },
                    ))
                })?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        for (id, revision, definition) in legacy {
            let recipe = crate::content_extraction::recipe_for_legacy_definition(&definition);
            crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error,
                )))
            })?;
            let recipe_json = serde_json::to_string(&recipe)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let recipe_hash = recipe.hash().map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
            })?;
            conn.execute(
                "UPDATE content_extractors SET recipe_json = ?1 WHERE id = ?2",
                params![recipe_json, id],
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO extractor_recipe_revisions
                    (extractor_id, revision, recipe_json, recipe_hash)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, revision, recipe_json, recipe_hash],
            )?;
        }
    }
    {
        let recipes = {
            let mut statement = conn.prepare(
                "SELECT id, revision, recipe_json FROM content_extractors
                 WHERE recipe_json IS NOT NULL",
            )?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>>>()?;
            rows
        };
        for (id, revision, recipe_json) in recipes {
            let recipe =
                serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&recipe_json)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let recipe_hash = recipe.hash().map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
            })?;
            conn.execute(
                "INSERT OR IGNORE INTO extractor_recipe_revisions
                    (extractor_id, revision, recipe_json, recipe_hash)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, revision, recipe_json, recipe_hash],
            )?;
        }
    }
    Ok(())
}
