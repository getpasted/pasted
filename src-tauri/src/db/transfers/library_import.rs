use rusqlite::{params, Result};

use super::super::{
    clip_names, ensure_resource_size, ensure_safe_raster_data_url, replace_imported_content_types,
    retire_structural_content_type_entries, DbState, PipelineStepInput,
};

impl DbState {
    pub fn import_backup_json(&self, json_str: &str) -> Result<usize> {
        let (payload, _) = Self::parse_library_archive(json_str)?;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let mut bin_id_map = std::collections::HashMap::new();
        if payload.content_type_groups.len() > 64 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Backup contains more than 64 content type groups".to_string(),
            ));
        }
        for group in &payload.content_type_groups {
            crate::content_types::validate_content_type_group_input(
                &crate::content_types::ContentTypeGroupInput {
                    id: group.id.clone(),
                    label: group.label.clone(),
                    sort_order: group.sort_order,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            let is_builtin = crate::content_types::CONTENT_TYPE_GROUP_PRESETS
                .iter()
                .any(|preset| preset.id == group.id);
            tx.execute(
                "INSERT INTO content_type_groups
                    (id, label, sort_order, is_builtin, is_archived)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label, sort_order = excluded.sort_order,
                    is_archived = CASE WHEN content_type_groups.is_builtin = 1 THEN 0 ELSE excluded.is_archived END,
                    updated_at = CURRENT_TIMESTAMP",
                params![group.id, group.label, group.sort_order, is_builtin, group.is_archived],
            )?;
        }
        if payload.content_types.len() > 256 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Backup contains more than 256 content types".to_string(),
            ));
        }
        for content_type in &payload.content_types {
            crate::content_types::validate_content_type_input(
                &crate::content_types::ContentTypeInput {
                    id: content_type.id.clone(),
                    label: content_type.label.clone(),
                    icon: content_type.icon.clone(),
                    group: content_type.group.clone(),
                    conceal_clips: content_type.conceal_clips.unwrap_or_else(|| {
                        crate::content_types::CONTENT_TYPE_PRESETS
                            .iter()
                            .find(|preset| preset.id == content_type.id)
                            .is_some_and(|preset| preset.conceal_clips())
                    }),
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            let group_exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM content_type_groups WHERE id = ?1 AND is_archived = 0)",
                params![content_type.group],
                |row| row.get(0),
            )?;
            if !group_exists {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Backup content type {} references a missing or archived Group",
                    content_type.id
                )));
            }
            let is_builtin = crate::content_types::CONTENT_TYPE_PRESETS
                .iter()
                .any(|preset| preset.id == content_type.id);
            tx.execute(
                "INSERT INTO content_types
                    (id, label, icon, group_name, is_builtin, is_archived, conceal_clips)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label, icon = excluded.icon,
                    group_name = excluded.group_name,
                    conceal_clips = excluded.conceal_clips,
                    is_archived = CASE WHEN content_types.is_builtin = 1 THEN 0 ELSE excluded.is_archived END,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    content_type.id,
                    content_type.label,
                    content_type.icon,
                    content_type.group,
                    is_builtin,
                    content_type.is_archived,
                    content_type.conceal_clips.unwrap_or_else(|| {
                        crate::content_types::CONTENT_TYPE_PRESETS
                            .iter()
                            .find(|preset| preset.id == content_type.id)
                            .is_some_and(|preset| preset.conceal_clips())
                    }),
                ],
            )?;
        }
        if payload.content_classifiers.len() > 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Backup contains more than 128 content classifiers".to_string(),
            ));
        }
        for classifier in &payload.content_classifiers {
            crate::content_classification::validate_classifier_input(
                &crate::content_classification::ClassifierInput {
                    name: classifier.name.clone(),
                    content_type: classifier.content_type.clone(),
                    description: classifier.description.clone(),
                    patterns: classifier.patterns.clone(),
                    validator: classifier.validator.clone(),
                    enabled: classifier.enabled,
                    priority: classifier.priority,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
            let patterns_json = serde_json::to_string(&classifier.patterns)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            tx.execute(
                "INSERT INTO content_classifiers
                    (stable_ref, name, content_type, description, patterns_json, validator,
                     enabled, priority, is_builtin, is_deleted)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(stable_ref) DO UPDATE SET
                    name = excluded.name, content_type = excluded.content_type,
                    description = excluded.description, patterns_json = excluded.patterns_json,
                    validator = excluded.validator, enabled = excluded.enabled,
                    priority = excluded.priority, is_builtin = excluded.is_builtin,
                    is_deleted = excluded.is_deleted, updated_at = CURRENT_TIMESTAMP",
                params![
                    classifier.stable_ref,
                    classifier.name,
                    classifier.content_type,
                    classifier.description,
                    patterns_json,
                    classifier.validator,
                    classifier.enabled,
                    classifier.priority,
                    classifier.is_builtin,
                    classifier.is_deleted
                ],
            )?;
        }
        let bin_clip_orders = payload
            .bins
            .iter()
            .map(|bin| (bin.id, bin.clip_order.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        let ocr_metadata = payload
            .ocr_metadata
            .iter()
            .map(|entry| (entry.content_hash.clone(), entry.clone()))
            .collect::<std::collections::HashMap<_, _>>();

        for mut bin in payload.bins {
            if let Some(rule) = bin.smart_rule.as_mut() {
                *rule = rule.replace("\"source_app\"", "\"source\"");
                *rule = crate::smart_bins::normalize_rule_json(rule)
                    .map_err(rusqlite::Error::InvalidParameterName)?;
            }
            let existing_id = tx.query_row(
                "SELECT id FROM bins WHERE name = ?1 AND COALESCE(bin_type, 'category') = ?2 LIMIT 1",
                params![bin.name, bin.bin_type],
                |row| row.get::<_, i64>(0),
            ).ok();
            let new_id = if let Some(id) = existing_id {
                tx.execute(
                    "UPDATE bins SET icon = ?1, color = ?2, smart_rule = ?3, shortcut = ?4,
                                     protect_clips = ?5, conceal_clips = ?6 WHERE id = ?7",
                    params![
                        bin.icon,
                        bin.color,
                        bin.smart_rule,
                        bin.shortcut,
                        bin.protect_clips,
                        bin.conceal_clips,
                        id
                    ],
                )?;
                id
            } else {
                tx.execute(
                    "INSERT INTO bins (name, icon, color, smart_rule, bin_type, shortcut, protect_clips, conceal_clips, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![bin.name, bin.icon, bin.color, bin.smart_rule, bin.bin_type, bin.shortcut, bin.protect_clips, bin.conceal_clips, bin.created_at],
                )?;
                tx.last_insert_rowid()
            };
            bin_id_map.insert(bin.id, new_id);
        }

        for operation in payload.operations {
            // Registry built-ins are definitions, not persisted records.
            if operation.id < 0 {
                continue;
            }
            let operation_id = operation.stable_id.strip_prefix("custom:").ok_or_else(|| {
                rusqlite::Error::InvalidParameterName(
                    "custom operation backup is missing a stable reference".to_string(),
                )
            })?;
            let (executor_kind, config_json) =
                Self::operation_storage_fields(&operation.op_type, operation.config.as_deref());
            tx.execute(
                "INSERT INTO custom_operations
                    (id, name, executor_kind, config_json, category, trusted, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    executor_kind = excluded.executor_kind,
                    config_json = excluded.config_json,
                    category = excluded.category,
                    trusted = 0,
                    updated_at = CURRENT_TIMESTAMP",
                params![
                    operation_id,
                    operation.name,
                    executor_kind,
                    config_json,
                    operation.category,
                    operation.created_at
                ],
            )?;
        }

        for pipeline in payload.pipelines {
            let pipeline_id = pipeline
                .stable_ref
                .strip_prefix("pipeline:")
                .ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(
                        "pipeline backup is missing a stable reference".to_string(),
                    )
                })?;
            let steps = pipeline
                .steps
                .iter()
                .map(|step| PipelineStepInput {
                    operation_ref: step.operation_ref.clone(),
                    config_json: step.config_json.clone(),
                    failure_policy: step.failure_policy.clone(),
                })
                .collect::<Vec<_>>();
            Self::validate_pipeline_steps(&tx, &steps)?;
            let plan_json =
                serde_json::to_string(&Self::manual_transform_plan(&pipeline.name, &steps)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let collision: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM saved_transforms WHERE id = ?1)",
                params![pipeline_id],
                |row| row.get(0),
            )?;
            let transform_id = if collision {
                tx.query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))?
            } else {
                pipeline_id.to_string()
            };
            tx.execute(
                "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, shortcut, authoring_kind,
                     revision, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, 'manual', ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    plan_json = excluded.plan_json,
                    shortcut = excluded.shortcut,
                    authoring_kind = 'manual',
                    revision = excluded.revision,
                    updated_at = excluded.updated_at",
                params![
                    transform_id,
                    pipeline.name,
                    plan_json,
                    pipeline.shortcut,
                    pipeline.revision,
                    pipeline.created_at,
                    pipeline.updated_at
                ],
            )?;
        }

        for transform in payload.saved_transforms {
            let transform_id =
                transform
                    .stable_ref
                    .strip_prefix("transform:")
                    .ok_or_else(|| {
                        rusqlite::Error::InvalidParameterName(
                            "saved Transform backup is missing a stable reference".to_string(),
                        )
                    })?;
            transform
                .plan
                .validate()
                .map_err(rusqlite::Error::InvalidParameterName)?;
            let plan_json = serde_json::to_string(&transform.plan)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            tx.execute(
                "INSERT INTO saved_transforms
                    (id, name, plan_json, connection_id, shortcut, authoring_kind,
                     revision, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    plan_json = excluded.plan_json,
                    connection_id = NULL,
                    shortcut = excluded.shortcut,
                    authoring_kind = excluded.authoring_kind,
                    revision = excluded.revision,
                    updated_at = excluded.updated_at",
                params![
                    transform_id,
                    transform.name,
                    plan_json,
                    transform.shortcut,
                    transform.authoring_kind,
                    transform.revision,
                    transform.created_at,
                    transform.updated_at
                ],
            )?;
        }

        for binding in payload.bin_transforms {
            let Some(mapped_bin_id) = bin_id_map.get(&binding.bin_id) else {
                continue;
            };
            let transform_id = binding
                .transform_ref
                .strip_prefix("transform:")
                .unwrap_or(&binding.transform_ref);
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM saved_transforms WHERE id = ?1)",
                params![transform_id],
                |row| row.get(0),
            )?;
            if exists {
                tx.execute(
                    "UPDATE bins SET default_transform_id = ?1 WHERE id = ?2",
                    params![transform_id, mapped_bin_id],
                )?;
            }
        }

        let mut imported = 0;
        let mut clip_id_map = std::collections::HashMap::new();
        for clip in payload.clips {
            let old_clip_id = clip.id;
            if let Some(text) = clip.text_content.as_deref() {
                ensure_resource_size(
                    text,
                    crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                    "Imported clip text",
                )?;
            }
            if let Some(html) = clip.html_content.as_deref() {
                ensure_resource_size(
                    html,
                    crate::resource_limits::MAX_CLIP_TEXT_BYTES,
                    "Imported clip HTML",
                )?;
            }
            if let Some(image) = clip.image_base64.as_deref() {
                ensure_safe_raster_data_url(image, "Imported clip image")?;
            }
            if let Some(note) = clip.note.as_deref() {
                ensure_resource_size(
                    note,
                    crate::resource_limits::MAX_CLIP_NOTE_BYTES,
                    "Imported clip note",
                )?;
            }
            let clip_name = clip_names::normalize_clip_name(clip.name.as_deref())?;
            let mapped_primary_bin = clip.bin_id.and_then(|id| bin_id_map.get(&id).copied());
            tx.execute(
                "INSERT INTO clips (
                    content_type, text_content, html_content, image_base64, image_path, content_hash,
                    source, is_pinned, is_protected, is_concealed, is_revealed, pin_order, bin_id, note,
                    name, is_trashed, trashed_at, created_at, shortcut
                 ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
                 ON CONFLICT(content_hash) DO UPDATE SET
                    content_type = excluded.content_type,
                    text_content = excluded.text_content,
                    html_content = excluded.html_content,
                    image_base64 = excluded.image_base64,
                    source = excluded.source,
                    is_pinned = excluded.is_pinned,
                    is_protected = excluded.is_protected,
                    is_concealed = excluded.is_concealed,
                    is_revealed = excluded.is_revealed,
                    pin_order = excluded.pin_order,
                    bin_id = excluded.bin_id,
                    note = excluded.note,
                    name = excluded.name,
                    is_trashed = excluded.is_trashed,
                    trashed_at = excluded.trashed_at,
                    created_at = excluded.created_at,
                    shortcut = excluded.shortcut",
                params![
                    clip.content_type, clip.text_content, clip.html_content, clip.image_base64,
                    clip.content_hash, clip.source, clip.is_pinned,
                    clip.is_explicitly_protected.unwrap_or(clip.is_protected),
                    clip.is_explicitly_concealed.unwrap_or(clip.is_concealed),
                    clip.is_explicitly_revealed, clip.pin_order, mapped_primary_bin, clip.note,
                    clip_name, clip.is_trashed, clip.trashed_at, clip.created_at, clip.shortcut,
                ],
            )?;
            let new_clip_id = tx.query_row(
                "SELECT id FROM clips WHERE content_hash = ?1",
                params![clip.content_hash],
                |row| row.get::<_, i64>(0),
            )?;
            clip_id_map.insert(old_clip_id, new_clip_id);
            if !clip.content_types.is_empty() {
                replace_imported_content_types(
                    &tx,
                    new_clip_id,
                    &clip.content_hash,
                    &clip.content_type,
                    &clip.content_types,
                )?;
            }
            if clip.content_type == "image" {
                if let Some(metadata) = ocr_metadata.get(&clip.content_hash) {
                    let status = match metadata.status.as_str() {
                        "complete" | "no_text" | "failed" | "never" => metadata.status.as_str(),
                        _ => "never",
                    };
                    tx.execute(
                        "UPDATE clips
                         SET ocr_status = ?1, ocr_input_hash = ?2,
                             ocr_engine_version = ?3, ocr_extractor_ref = ?4,
                             ocr_extractor_name = ?5, ocr_attempted_at = ?6,
                             ocr_error = NULL
                         WHERE id = ?7",
                        params![
                            status,
                            metadata.input_hash.as_deref().unwrap_or(&clip.content_hash),
                            metadata.engine_version.as_deref(),
                            metadata.extractor_ref.as_deref(),
                            metadata.extractor_name.as_deref(),
                            metadata.attempted_at.as_deref(),
                            new_clip_id
                        ],
                    )?;
                }
            }
            tx.execute(
                "DELETE FROM clip_bins WHERE clip_id = ?1",
                params![new_clip_id],
            )?;
            for old_bin_id in clip.bin_ids.unwrap_or_default() {
                if let Some(new_bin_id) = bin_id_map.get(&old_bin_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                        params![new_clip_id, new_bin_id],
                    )?;
                }
            }
            if let Some(new_bin_id) = mapped_primary_bin {
                tx.execute(
                    "INSERT OR IGNORE INTO clip_bins (clip_id, bin_id) VALUES (?1, ?2)",
                    params![new_clip_id, new_bin_id],
                )?;
            }
            imported += 1;
        }

        for visual_label in payload.visual_label_overrides {
            let Some(new_clip_id) = clip_id_map.get(&visual_label.clip_id) else {
                continue;
            };
            tx.execute(
                "INSERT OR REPLACE INTO clip_visual_label_overrides
                    (clip_id, label, operation, updated_at)
                 VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))",
                params![new_clip_id, visual_label.label, visual_label.operation],
            )?;
        }

        for (old_bin_id, ordered_clip_ids) in bin_clip_orders {
            let Some(new_bin_id) = bin_id_map.get(&old_bin_id) else {
                continue;
            };
            tx.execute(
                "DELETE FROM bin_clip_order WHERE bin_id = ?1",
                params![new_bin_id],
            )?;
            for (position, old_clip_id) in ordered_clip_ids.into_iter().enumerate() {
                let Some(new_clip_id) = clip_id_map.get(&old_clip_id) else {
                    continue;
                };
                tx.execute(
                    "INSERT OR REPLACE INTO bin_clip_order (bin_id, clip_id, position)
                     VALUES (?1, ?2, ?3)",
                    params![new_bin_id, new_clip_id, position as i64],
                )?;
            }
        }

        retire_structural_content_type_entries(&tx)?;
        tx.commit()?;
        Ok(imported)
    }
}
