use rusqlite::{params, OptionalExtension, Result};

use super::{invalid_extractor_input, DbState};

mod runtime;

fn insert_extractor_authoring_session(
    transaction: &rusqlite::Transaction<'_>,
    extractor_id: i64,
    manifest: Option<&crate::extractor_recipe::ExtractorAuthoringManifest>,
) -> Result<Option<i64>> {
    let Some(manifest) = manifest else {
        return Ok(None);
    };
    transaction.execute(
        "INSERT INTO extractor_authoring_sessions
            (extractor_id, source, provider, model, original_prompt, manifest_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            extractor_id,
            manifest.source.stable_name(),
            manifest.provider,
            manifest.model,
            manifest.original_prompt,
            manifest.manifest_version,
        ],
    )?;
    let session_id = transaction.last_insert_rowid();
    for (sequence, message) in manifest.messages.iter().enumerate() {
        let structured_content_json = message
            .structured_content
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        transaction.execute(
            "INSERT INTO extractor_authoring_messages
                (session_id, sequence, role, content, structured_content_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session_id,
                sequence as i64,
                message.role.stable_name(),
                message.content,
                structured_content_json,
                message.created_at,
            ],
        )?;
    }
    Ok(Some(session_id))
}

impl DbState {
    pub(super) fn normalize_json_config(config: Option<&str>) -> String {
        match config {
            Some(value) if serde_json::from_str::<serde_json::Value>(value).is_ok() => {
                value.to_string()
            }
            Some(value) => serde_json::Value::String(value.to_string()).to_string(),
            None => "{}".to_string(),
        }
    }

    pub fn get_content_extractors(&self) -> Result<Vec<crate::content_extraction::Extractor>> {
        runtime::load_content_extractors(self)
    }

    pub fn get_content_extractor(
        &self,
        reference: &str,
    ) -> Result<crate::content_extraction::Extractor> {
        let numeric_id = reference.parse::<i64>().ok();
        self.get_content_extractors()?
            .into_iter()
            .find(|extractor| numeric_id == Some(extractor.id) || extractor.stable_ref == reference)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn create_content_extractor(
        &self,
        input: &crate::content_extraction::ExtractorDefinitionInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::content_extraction::validate_extractor_definition(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let recipe = crate::content_extraction::recipe_for_legacy_definition(input);
        crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let recipe_json = serde_json::to_string(&recipe)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let recipe_hash = recipe.hash().map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
        })?;
        let conn = self.conn.lock();
        let extractor_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM content_extractors WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;
        if extractor_count >= 64 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content Extractors are limited to 64 entries".into(),
            ));
        }
        conn.execute(
            "INSERT INTO content_extractors
                (stable_ref, name, description, engine, executable_path, model_path,
                 input_contract, output_contract, enabled, priority, revision, recipe_json,
                 is_builtin)
             VALUES ('pending', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, 0)",
            params![
                input.name.trim(),
                input.description.trim(),
                input.engine.trim(),
                input.executable_path.as_deref().map(str::trim),
                input.model_path.as_deref().map(str::trim),
                input.input_contract,
                input.output_contract,
                input.enabled,
                input.priority,
                recipe_json,
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE content_extractors SET stable_ref = ?1 WHERE id = ?2",
            params![format!("extractor:custom:{id}"), id],
        )?;
        conn.execute(
            "INSERT INTO extractor_recipe_revisions
                (extractor_id, revision, recipe_json, recipe_hash)
             VALUES (?1, 1, ?2, ?3)",
            params![id, recipe_json, recipe_hash],
        )?;
        drop(conn);
        let created = self.get_content_extractor(&id.to_string())?;
        let _ = self.log_activity(
            "content_extractor_created",
            &format!("Created Extractor \"{}\"", created.name),
        );
        Ok(created)
    }

    pub fn update_content_extractor_definition(
        &self,
        id: i64,
        input: &crate::content_extraction::ExtractorDefinitionInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::content_extraction::validate_extractor_definition(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let current = self.get_content_extractor(&id.to_string())?;
        if current.is_builtin
            && (current.engine != input.engine
                || current.input_contract != input.input_contract
                || current.output_contract != input.output_contract)
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "Built-in Extractor engine and contracts cannot be changed".into(),
            ));
        }
        let recipe = if input.engine == crate::content_extraction::RECIPE_ENGINE {
            current.recipe.clone()
        } else {
            crate::content_extraction::recipe_for_legacy_definition(input)
        };
        crate::extractor_recipe::validate_recipe(&recipe).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let recipe_json = serde_json::to_string(&recipe)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let recipe_hash = recipe.hash().map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
        })?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_extractors SET name = ?1, description = ?2, engine = ?3,
                    executable_path = ?4, model_path = ?5, input_contract = ?6,
                    output_contract = ?7, enabled = ?8, priority = ?9, recipe_json = ?10,
                    revision = revision + 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?11 AND is_deleted = 0",
            params![
                input.name.trim(),
                input.description.trim(),
                input.engine.trim(),
                input.executable_path.as_deref().map(str::trim),
                input.model_path.as_deref().map(str::trim),
                input.input_contract,
                input.output_contract,
                input.enabled,
                input.priority,
                recipe_json,
                id
            ],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self.get_content_extractor(&id.to_string())?;
        {
            let conn = self.conn.lock();
            conn.execute(
                "INSERT INTO extractor_recipe_revisions
                    (extractor_id, revision, recipe_json, recipe_hash)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, updated.revision, recipe_json, recipe_hash],
            )?;
        }
        self.log_analysis_participant_update(
            "extractor",
            &updated.stable_ref,
            &updated.name,
            current.enabled,
            updated.enabled,
        );
        Ok(updated)
    }

    pub fn create_content_extractor_recipe(
        &self,
        input: &crate::extractor_recipe::ExtractorRecipeDefinitionInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::extractor_recipe::validate_definition(input).map_err(invalid_extractor_input)?;
        let authoring = input
            .authoring
            .as_ref()
            .map(crate::extractor_recipe::canonicalize_authoring_manifest)
            .transpose()
            .map_err(invalid_extractor_input)?;
        let recipe_json = serde_json::to_string(&input.recipe)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let recipe_hash = input.recipe.hash().map_err(invalid_extractor_input)?;
        let input_contract = input
            .recipe
            .accepts
            .first()
            .map(crate::extractor_recipe::ExtractorInputKind::stable_name)
            .ok_or_else(|| invalid_extractor_input("Extractor recipes require an input"))?;
        let executable_path = input
            .recipe
            .steps
            .first()
            .and_then(|step| step.executable.path.as_deref());
        let model_path = input
            .recipe
            .resources
            .iter()
            .find(|resource| resource.id == "model")
            .and_then(|resource| resource.path.as_deref());
        let conn = self.conn.lock();
        let transaction = conn.unchecked_transaction()?;
        let extractor_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM content_extractors WHERE is_deleted = 0",
            [],
            |row| row.get(0),
        )?;
        if extractor_count >= 64 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Content Extractors are limited to 64 entries".into(),
            ));
        }
        transaction.execute(
            "INSERT INTO content_extractors
                (stable_ref, name, description, engine, executable_path, model_path,
                 input_contract, output_contract, enabled, priority, revision, recipe_json,
                 is_builtin)
             VALUES ('pending', ?1, ?2, ?3, ?4, ?5, ?6, 'searchable_text', ?7, ?8, 1, ?9, 0)",
            params![
                input.name.trim(),
                input.description.trim(),
                crate::content_extraction::RECIPE_ENGINE,
                executable_path,
                model_path,
                input_contract,
                input.enabled,
                input.priority,
                recipe_json,
            ],
        )?;
        let id = transaction.last_insert_rowid();
        transaction.execute(
            "UPDATE content_extractors SET stable_ref = ?1 WHERE id = ?2",
            params![format!("extractor:custom:{id}"), id],
        )?;
        let authoring_session_id =
            insert_extractor_authoring_session(&transaction, id, authoring.as_ref())?;
        transaction.execute(
            "INSERT INTO extractor_recipe_revisions
                (extractor_id, revision, recipe_json, recipe_hash, authoring_session_id)
             VALUES (?1, 1, ?2, ?3, ?4)",
            params![id, recipe_json, recipe_hash, authoring_session_id],
        )?;
        transaction.commit()?;
        drop(conn);
        let created = self.get_content_extractor(&id.to_string())?;
        let _ = self.log_activity(
            "content_extractor_created",
            &format!("Created Extractor \"{}\"", created.name),
        );
        Ok(created)
    }

    pub fn update_content_extractor_recipe(
        &self,
        id: i64,
        input: &crate::extractor_recipe::ExtractorRecipeDefinitionInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::extractor_recipe::validate_definition(input).map_err(invalid_extractor_input)?;
        let authoring = input
            .authoring
            .as_ref()
            .map(crate::extractor_recipe::canonicalize_authoring_manifest)
            .transpose()
            .map_err(invalid_extractor_input)?;
        let current = self.get_content_extractor(&id.to_string())?;
        let recipe_json = serde_json::to_string(&input.recipe)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let recipe_hash = input.recipe.hash().map_err(invalid_extractor_input)?;
        let input_contract = input
            .recipe
            .accepts
            .first()
            .map(crate::extractor_recipe::ExtractorInputKind::stable_name)
            .ok_or_else(|| invalid_extractor_input("Extractor recipes require an input"))?;
        let executable_path = input
            .recipe
            .steps
            .first()
            .and_then(|step| step.executable.path.as_deref());
        let model_path = input
            .recipe
            .resources
            .iter()
            .find(|resource| resource.id == "model")
            .and_then(|resource| resource.path.as_deref());
        let next_revision = current.revision.saturating_add(1);
        let conn = self.conn.lock();
        let transaction = conn.unchecked_transaction()?;
        let changed = transaction.execute(
            "UPDATE content_extractors
             SET name = ?1, description = ?2, engine = ?3, executable_path = ?4,
                 model_path = ?5, input_contract = ?6, output_contract = 'searchable_text',
                 enabled = ?7, priority = ?8, revision = ?9, recipe_json = ?10,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ?11 AND is_deleted = 0",
            params![
                input.name.trim(),
                input.description.trim(),
                crate::content_extraction::RECIPE_ENGINE,
                executable_path,
                model_path,
                input_contract,
                input.enabled,
                input.priority,
                next_revision,
                recipe_json,
                id,
            ],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let authoring_session_id =
            insert_extractor_authoring_session(&transaction, id, authoring.as_ref())?;
        transaction.execute(
            "INSERT INTO extractor_recipe_revisions
                (extractor_id, revision, recipe_json, recipe_hash, authoring_session_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                next_revision,
                recipe_json,
                recipe_hash,
                authoring_session_id
            ],
        )?;
        transaction.commit()?;
        drop(conn);
        let updated = self.get_content_extractor(&id.to_string())?;
        self.log_analysis_participant_update(
            "extractor",
            &updated.stable_ref,
            &updated.name,
            current.enabled,
            updated.enabled,
        );
        Ok(updated)
    }

    pub fn get_extractor_authoring_sessions(
        &self,
        reference: &str,
    ) -> Result<Vec<crate::extractor_recipe::ExtractorAuthoringSession>> {
        let extractor = self.get_content_extractor(reference)?;
        let conn = self.conn.lock();
        let mut statement = conn.prepare(
            "SELECT id, source, provider, model, original_prompt, created_at
             FROM extractor_authoring_sessions
             WHERE extractor_id = ?1 ORDER BY created_at, id",
        )?;
        let sessions = statement
            .query_map(params![extractor.id], |row| {
                let id = row.get::<_, i64>(0)?;
                let source = crate::extractor_recipe::ExtractorAuthoringSource::parse(
                    &row.get::<_, String>(1)?,
                )
                .ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::other("Invalid Extractor authoring source")),
                    )
                })?;
                Ok(crate::extractor_recipe::ExtractorAuthoringSession {
                    id,
                    extractor_id: extractor.id,
                    source,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    original_prompt: row.get(4)?,
                    created_at: row.get(5)?,
                    messages: Vec::new(),
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        let mut sessions = sessions;
        for session in &mut sessions {
            let mut messages = conn.prepare(
                "SELECT role, content, created_at, structured_content_json
                 FROM extractor_authoring_messages
                 WHERE session_id = ?1 ORDER BY sequence",
            )?;
            session.messages = messages
                .query_map(params![session.id], |row| {
                    let role = crate::extractor_recipe::ExtractorAuthoringRole::parse(
                        &row.get::<_, String>(0)?,
                    )
                    .ok_or_else(|| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::other("Invalid Extractor authoring role")),
                        )
                    })?;
                    let structured = row
                        .get::<_, Option<String>>(3)?
                        .map(|value| serde_json::from_str(&value))
                        .transpose()
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                3,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?;
                    Ok(crate::extractor_recipe::ExtractorAuthoringMessage {
                        role,
                        content: row.get(1)?,
                        created_at: row.get(2)?,
                        structured_content: structured,
                    })
                })?
                .collect::<Result<Vec<_>>>()?;
        }
        Ok(sessions)
    }

    pub fn duplicate_content_extractor(
        &self,
        reference: &str,
        name: Option<&str>,
    ) -> Result<crate::content_extraction::Extractor> {
        let source = self.get_content_extractor(reference)?;
        self.create_content_extractor_recipe(
            &crate::extractor_recipe::ExtractorRecipeDefinitionInput {
                name: name
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{} Copy", source.name)),
                description: source.description,
                enabled: source.enabled,
                priority: source.priority.saturating_add(1).min(10_000),
                recipe: source.recipe,
                authoring: None,
            },
        )
    }

    pub fn delete_content_extractor(&self, id: i64) -> Result<()> {
        let extractor = self.get_content_extractor(&id.to_string())?;
        let conn = self.conn.lock();
        let changed = conn.execute(
            "UPDATE content_extractors SET is_deleted = 1, enabled = 0,
                    updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND is_deleted = 0",
            params![id],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let _ = self.log_activity(
            "content_extractor_deleted",
            &format!("Deleted Extractor \"{}\"", extractor.name),
        );
        Ok(())
    }

    pub fn update_content_extractor(
        &self,
        id: i64,
        input: &crate::content_extraction::ExtractorInput,
    ) -> Result<crate::content_extraction::Extractor> {
        crate::content_extraction::validate_extractor_input(input).map_err(|error| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                error,
            )))
        })?;
        let conn = self.conn.lock();
        let previous_enabled = conn
            .query_row(
                "SELECT enabled FROM content_extractors WHERE id = ?1 AND is_deleted = 0",
                params![id],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let changed = conn.execute(
            "UPDATE content_extractors SET name = ?1, description = ?2, enabled = ?3,
                    priority = ?4, revision = revision + 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?5 AND is_deleted = 0",
            params![
                input.name.trim(),
                input.description.trim(),
                input.enabled,
                input.priority,
                id
            ],
        )?;
        drop(conn);
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let updated = self
            .get_content_extractors()?
            .into_iter()
            .find(|extractor| extractor.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        self.log_analysis_participant_update(
            "extractor",
            &updated.stable_ref,
            &updated.name,
            previous_enabled,
            updated.enabled,
        );
        Ok(updated)
    }

    pub fn restore_default_content_extractors(&self) -> Result<()> {
        let mut conn = self.conn.lock();
        let transaction = conn.transaction()?;
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
                     shipped_recipe_json, is_builtin, is_deleted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, 1, ?10, ?11, ?12, ?12, 1, 0)
                 ON CONFLICT(stable_ref) DO UPDATE SET
                    name = excluded.name, description = excluded.description,
                    engine = excluded.engine, executable_path = excluded.executable_path,
                    model_path = excluded.model_path, input_contract = excluded.input_contract,
                    output_contract = excluded.output_contract, enabled = 1,
                    priority = excluded.priority, revision = content_extractors.revision + 1,
                    shipped_revision = excluded.shipped_revision,
                    shipped_definition_json = excluded.shipped_definition_json,
                    recipe_json = excluded.recipe_json,
                    shipped_recipe_json = excluded.shipped_recipe_json, is_deleted = 0,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
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
                    recipe_json
                ],
            )?;
            let (id, revision) = transaction.query_row(
                "SELECT id, revision FROM content_extractors WHERE stable_ref = ?1",
                params![preset.stable_ref],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )?;
            let authoring = crate::extractor_recipe::ExtractorAuthoringManifest {
                manifest_version: crate::extractor_recipe::EXTRACTOR_AUTHORING_VERSION,
                source: crate::extractor_recipe::ExtractorAuthoringSource::Shipped,
                original_prompt: None,
                provider: None,
                model: None,
                messages: Vec::new(),
            };
            let authoring_session_id =
                insert_extractor_authoring_session(&transaction, id, Some(&authoring))?;
            transaction.execute(
                "INSERT INTO extractor_recipe_revisions
                    (extractor_id, revision, recipe_json, recipe_hash, authoring_session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, revision, recipe_json, recipe_hash, authoring_session_id],
            )?;
        }
        transaction.commit()?;
        drop(conn);
        let _ = self.log_activity(
            "content_extractors_restored",
            "Restored shipped extractor defaults",
        );
        Ok(())
    }

    pub fn active_image_text_extractor(
        &self,
    ) -> Result<Option<crate::content_extraction::Extractor>> {
        Ok(self
            .active_image_text_extractors_for_features(true)?
            .into_iter()
            .next())
    }

    pub fn active_image_text_extractor_for_features(
        &self,
        ocr_enabled: bool,
    ) -> Result<Option<crate::content_extraction::Extractor>> {
        Ok(self
            .active_image_text_extractors_for_features(ocr_enabled)?
            .into_iter()
            .next())
    }

    pub fn active_image_text_extractors_for_features(
        &self,
        ocr_enabled: bool,
    ) -> Result<Vec<crate::content_extraction::Extractor>> {
        Ok(self
            .get_content_extractors()?
            .into_iter()
            .filter(|extractor| {
                extractor.enabled
                    && extractor.is_available
                    && (ocr_enabled
                        || !matches!(
                            extractor.stable_ref.as_str(),
                            crate::content_extraction::APPLE_VISION_OCR_REF
                                | crate::content_extraction::TESSERACT_OCR_REF
                        ))
                    && extractor.supports_contract(
                        crate::analysis_contract::RepresentationKind::ImageBytes,
                        crate::analysis_contract::RepresentationKind::SearchableText,
                    )
            })
            .take(crate::content_extraction::MAX_ACTIVE_EXTRACTORS_PER_INPUT)
            .collect())
    }

    pub fn active_file_text_extractor(
        &self,
    ) -> Result<Option<crate::content_extraction::Extractor>> {
        Ok(self
            .active_file_text_extractors_for_features(true)?
            .into_iter()
            .next())
    }

    pub fn active_file_text_extractor_for_features(
        &self,
        transcriptions_enabled: bool,
    ) -> Result<Option<crate::content_extraction::Extractor>> {
        Ok(self
            .active_file_text_extractors_for_features(transcriptions_enabled)?
            .into_iter()
            .next())
    }

    pub fn active_file_text_extractors_for_features(
        &self,
        transcriptions_enabled: bool,
    ) -> Result<Vec<crate::content_extraction::Extractor>> {
        Ok(self
            .get_content_extractors()?
            .into_iter()
            .filter(|extractor| {
                extractor.enabled
                    && extractor.is_available
                    && (transcriptions_enabled
                        || extractor.stable_ref
                            != crate::content_extraction::WHISPER_TRANSCRIPTION_REF)
                    && extractor.supports_contract(
                        crate::analysis_contract::RepresentationKind::FileReferences,
                        crate::analysis_contract::RepresentationKind::SearchableText,
                    )
            })
            .take(crate::content_extraction::MAX_ACTIVE_EXTRACTORS_PER_INPUT)
            .collect())
    }
}
