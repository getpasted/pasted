use rusqlite::Result;

use super::super::DbState;

struct StoredExtractor {
    id: i64,
    stable_ref: String,
    name: String,
    description: String,
    engine: String,
    executable_path: Option<String>,
    model_path: Option<String>,
    input_contract: String,
    output_contract: String,
    enabled: bool,
    priority: i64,
    revision: i64,
    is_builtin: bool,
    recipe_json: String,
}

pub(super) fn load_content_extractors(
    db: &DbState,
) -> Result<Vec<crate::content_extraction::Extractor>> {
    let stored = {
        let conn = db.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, stable_ref, name, description, engine, executable_path, model_path,
                    input_contract, output_contract, enabled, priority, revision, is_builtin,
                    recipe_json
             FROM content_extractors WHERE is_deleted = 0 ORDER BY priority, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(StoredExtractor {
                id: row.get(0)?,
                stable_ref: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                engine: row.get(4)?,
                executable_path: row.get(5)?,
                model_path: row.get(6)?,
                input_contract: row.get(7)?,
                output_contract: row.get(8)?,
                enabled: row.get(9)?,
                priority: row.get(10)?,
                revision: row.get(11)?,
                is_builtin: row.get(12)?,
                recipe_json: row.get(13)?,
            })
        })?;
        let stored = rows.collect::<Result<Vec<_>>>()?;
        drop(statement);
        drop(conn);
        stored
    };

    stored.into_iter().map(decorate_runtime).collect()
}

fn decorate_runtime(stored: StoredExtractor) -> Result<crate::content_extraction::Extractor> {
    let preset = crate::content_extraction::EXTRACTOR_PRESETS
        .iter()
        .find(|preset| preset.stable_ref == stored.stable_ref);
    let recipe =
        serde_json::from_str::<crate::extractor_recipe::ExtractorRecipe>(&stored.recipe_json)
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    13,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
    let recipe_hash = recipe.hash().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            13,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other(error)),
        )
    })?;
    let (availability, runtime) = if stored.engine == crate::content_extraction::RECIPE_ENGINE {
        (
            crate::extractor_recipe::availability(&recipe),
            crate::extractor_recipe::runtime_status_summary(&recipe),
        )
    } else {
        (
            crate::content_extraction::engine_availability_for(
                &stored.engine,
                stored.executable_path.as_deref(),
                stored.model_path.as_deref(),
            ),
            crate::content_extraction::runtime_status_summary_for(
                &stored.engine,
                stored.executable_path.as_deref(),
            ),
        )
    };
    Ok(crate::content_extraction::Extractor {
        id: stored.id,
        stable_ref: stored.stable_ref,
        name: stored.name,
        description: stored.description,
        engine: stored.engine,
        executable_path: stored.executable_path,
        model_path: stored.model_path,
        input_contract: stored.input_contract,
        output_contract: stored.output_contract,
        enabled: stored.enabled,
        priority: stored.priority,
        revision: stored.revision,
        is_builtin: stored.is_builtin,
        is_available: availability.is_available,
        unavailable_reason: availability.unavailable_reason,
        runtime,
        recipe,
        recipe_hash,
        default_recipe: preset.map(crate::content_extraction::ExtractorPreset::recipe),
        defaults: preset.map(crate::content_extraction::ExtractorPreset::definition),
    })
}
