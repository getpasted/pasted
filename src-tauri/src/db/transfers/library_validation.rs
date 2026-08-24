use std::collections::HashSet;

use rusqlite::Result;

use super::super::{
    ensure_resource_size, ensure_safe_raster_data_url, normalize_imported_clip_types,
    normalize_library_archive_timestamps, BackupPayload, DbState, LibraryArchiveInspection,
    PipelineStep, BACKUP_SCHEMA_VERSION,
};

impl DbState {
    pub(in crate::db) fn parse_library_archive(
        json_str: &str,
    ) -> Result<(BackupPayload, LibraryArchiveInspection)> {
        ensure_resource_size(
            json_str,
            crate::resource_limits::MAX_BACKUP_IMPORT_BYTES,
            "Transfer file",
        )?;
        let mut payload: BackupPayload = serde_json::from_str(json_str).map_err(|error| {
            rusqlite::Error::InvalidParameterName(format!("invalid transfer JSON: {error}"))
        })?;
        normalize_library_archive_timestamps(&mut payload)?;
        payload.content_types.retain(|content_type| {
            !crate::content_types::is_structural_clip_type_id(&content_type.id)
        });
        for clip in &mut payload.clips {
            normalize_imported_clip_types(clip)?;
            clip.shortcut = clip
                .shortcut
                .take()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            if clip
                .shortcut
                .as_ref()
                .is_some_and(|value| value.len() > 256)
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transfer clip contains an invalid shortcut".into(),
                ));
            }
            if clip.shortcut.is_some() {
                clip.is_protected = true;
                clip.is_explicitly_protected = Some(true);
            }
        }
        let inspection = Self::preflight_library_archive(&payload)?;
        Ok((payload, inspection))
    }

    fn preflight_library_archive(payload: &BackupPayload) -> Result<LibraryArchiveInspection> {
        use crate::resource_limits::{
            MAX_CLIP_NOTE_BYTES, MAX_CLIP_TEXT_BYTES, MAX_LIBRARY_ARCHIVE_ROWS,
        };

        if !(1..=BACKUP_SCHEMA_VERSION).contains(&payload.version) {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "unsupported transfer schema version {} (supported: 1-{BACKUP_SCHEMA_VERSION})",
                payload.version
            )));
        }
        let total_rows = [
            payload.clips.len(),
            payload.bins.len(),
            payload.pipelines.len(),
            payload.operations.len(),
            payload.saved_transforms.len(),
            payload.bin_transforms.len(),
            payload.ocr_metadata.len(),
            payload.content_classifiers.len(),
            payload.content_types.len(),
            payload.content_type_groups.len(),
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)
        .ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                "Transfer row count exceeds supported limits".to_string(),
            )
        })?;
        if total_rows > MAX_LIBRARY_ARCHIVE_ROWS {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "Transfer file contains more than {MAX_LIBRARY_ARCHIVE_ROWS} records"
            )));
        }
        if payload.content_type_groups.len() > 64 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Transfer file contains more than 64 content type groups".to_string(),
            ));
        }
        if payload.content_types.len() > 256 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Transfer file contains more than 256 content types".to_string(),
            ));
        }
        if payload.content_classifiers.len() > 128 {
            return Err(rusqlite::Error::InvalidParameterName(
                "Transfer file contains more than 128 content classifiers".to_string(),
            ));
        }

        let unique = |values: Vec<String>, label: &str| -> Result<HashSet<String>> {
            let mut seen = HashSet::with_capacity(values.len());
            for value in values {
                if value.trim().is_empty() || !seen.insert(value.clone()) {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer file contains an empty or duplicate {label}: {value}"
                    )));
                }
            }
            Ok(seen)
        };
        let unique_ids = |values: Vec<i64>, label: &str| -> Result<HashSet<i64>> {
            let mut seen = HashSet::with_capacity(values.len());
            for value in values {
                if value <= 0 || !seen.insert(value) {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer file contains an invalid or duplicate {label}: {value}"
                    )));
                }
            }
            Ok(seen)
        };

        let _group_ids = unique(
            payload
                .content_type_groups
                .iter()
                .map(|group| group.id.clone())
                .collect(),
            "content type group ID",
        )?;
        for group in &payload.content_type_groups {
            crate::content_types::validate_content_type_group_input(
                &crate::content_types::ContentTypeGroupInput {
                    id: group.id.clone(),
                    label: group.label.clone(),
                    sort_order: group.sort_order,
                },
            )
            .map_err(rusqlite::Error::InvalidParameterName)?;
        }
        let available_group_ids = payload
            .content_type_groups
            .iter()
            .filter(|group| !group.is_archived)
            .map(|group| group.id.clone())
            .chain(
                crate::content_types::CONTENT_TYPE_GROUP_PRESETS
                    .iter()
                    .map(|preset| preset.id.to_string()),
            )
            .collect::<HashSet<_>>();
        unique(
            payload
                .content_types
                .iter()
                .map(|content_type| content_type.id.clone())
                .collect(),
            "content type ID",
        )?;
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
            if !available_group_ids.contains(&content_type.group) {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer content type {} references a missing Group",
                    content_type.id
                )));
            }
        }

        unique(
            payload
                .content_classifiers
                .iter()
                .map(|classifier| classifier.stable_ref.clone())
                .collect(),
            "classifier reference",
        )?;
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
        }

        let bin_ids = unique_ids(payload.bins.iter().map(|bin| bin.id).collect(), "Bin ID")?;
        for bin in &payload.bins {
            if !matches!(bin.bin_type.as_str(), "category" | "tag") {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer Bin {} has an invalid type",
                    bin.id
                )));
            }
            if let Some(rule) = bin.smart_rule.as_deref() {
                crate::smart_bins::parse_rule_json(rule).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "Transfer Bin {} has an invalid smart rule: {error}",
                        bin.id
                    ))
                })?;
                if bin.protect_clips {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer Smart Bin {} cannot apply inherited protection",
                        bin.id
                    )));
                }
                if bin.conceal_clips {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "Transfer Smart Bin {} cannot apply inherited concealment",
                        bin.id
                    )));
                }
            }
        }

        let clip_ids = unique_ids(
            payload.clips.iter().map(|clip| clip.id).collect(),
            "clip ID",
        )?;
        unique(
            payload
                .clips
                .iter()
                .map(|clip| clip.content_hash.clone())
                .collect(),
            "clip content hash",
        )?;
        let image_hashes = payload
            .clips
            .iter()
            .filter(|clip| clip.content_type == "image")
            .map(|clip| clip.content_hash.as_str())
            .collect::<HashSet<_>>();
        for clip in &payload.clips {
            if let Some(text) = clip.text_content.as_deref() {
                ensure_resource_size(text, MAX_CLIP_TEXT_BYTES, "Imported clip text")?;
            }
            if let Some(html) = clip.html_content.as_deref() {
                ensure_resource_size(html, MAX_CLIP_TEXT_BYTES, "Imported clip HTML")?;
            }
            if let Some(image) = clip.image_base64.as_deref() {
                ensure_safe_raster_data_url(image, "Imported clip image")?;
            }
            if let Some(note) = clip.note.as_deref() {
                ensure_resource_size(note, MAX_CLIP_NOTE_BYTES, "Imported clip note")?;
            }
            if clip.bin_id.is_some_and(|id| !bin_ids.contains(&id))
                || clip
                    .bin_ids
                    .as_ref()
                    .is_some_and(|ids| ids.iter().any(|id| !bin_ids.contains(id)))
            {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer clip {} references a missing Bin",
                    clip.id
                )));
            }
        }
        for bin in &payload.bins {
            let mut ordered = HashSet::new();
            if bin
                .clip_order
                .iter()
                .any(|id| !clip_ids.contains(id) || !ordered.insert(*id))
            {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer Bin {} contains an invalid clip order",
                    bin.id
                )));
            }
        }

        unique(
            payload
                .ocr_metadata
                .iter()
                .map(|entry| entry.content_hash.clone())
                .collect(),
            "OCR content hash",
        )?;
        for metadata in &payload.ocr_metadata {
            if !image_hashes.contains(metadata.content_hash.as_str())
                || !matches!(
                    metadata.status.as_str(),
                    "complete" | "no_text" | "failed" | "never" | "queued" | "running"
                )
                || metadata
                    .engine_version
                    .as_ref()
                    .is_some_and(|value| value.is_empty() || value.len() > 80)
                || metadata
                    .extractor_ref
                    .as_ref()
                    .is_some_and(|value| value.is_empty() || value.len() > 160)
                || metadata
                    .extractor_name
                    .as_ref()
                    .is_some_and(|value| value.is_empty() || value.len() > 80)
            {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "Transfer file has invalid OCR metadata for {}",
                    metadata.content_hash
                )));
            }
        }

        let custom_operation_refs = payload
            .operations
            .iter()
            .filter(|operation| operation.id >= 0)
            .map(|operation| {
                operation
                    .stable_id
                    .strip_prefix("custom:")
                    .filter(|id| !id.is_empty())
                    .map(|_| operation.stable_id.clone())
                    .ok_or_else(|| {
                        rusqlite::Error::InvalidParameterName(
                            "custom operation in transfer file is missing a stable reference"
                                .to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let custom_operation_refs = unique(custom_operation_refs, "custom operation reference")?;
        let validate_step = |step: &PipelineStep| -> Result<()> {
            if !matches!(step.failure_policy.as_str(), "stop" | "skip") {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "invalid failure policy: {}",
                    step.failure_policy
                )));
            }
            if let Some(config) = step.config_json.as_deref() {
                serde_json::from_str::<serde_json::Value>(config).map_err(|error| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "invalid step config JSON: {error}"
                    ))
                })?;
            }
            let valid = step
                .operation_ref
                .strip_prefix("builtin:")
                .is_some_and(crate::operation_registry::is_builtin_operation)
                || custom_operation_refs.contains(&step.operation_ref);
            if !valid {
                return Err(rusqlite::Error::InvalidParameterName(format!(
                    "unknown operation reference: {}",
                    step.operation_ref
                )));
            }
            Ok(())
        };
        unique(
            payload
                .pipelines
                .iter()
                .map(|pipeline| pipeline.stable_ref.clone())
                .collect(),
            "legacy pipeline reference",
        )?;
        for pipeline in &payload.pipelines {
            if pipeline
                .stable_ref
                .strip_prefix("pipeline:")
                .is_none_or(str::is_empty)
                || pipeline.steps.is_empty()
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "legacy pipeline in transfer file is missing a stable reference or steps"
                        .to_string(),
                ));
            }
            for step in &pipeline.steps {
                validate_step(step)?;
            }
        }
        let transform_refs = unique(
            payload
                .saved_transforms
                .iter()
                .map(|transform| transform.stable_ref.clone())
                .collect(),
            "Transform reference",
        )?;
        for transform in &payload.saved_transforms {
            if transform
                .stable_ref
                .strip_prefix("transform:")
                .is_none_or(str::is_empty)
                || !matches!(transform.authoring_kind.as_str(), "manual" | "intent")
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transform in transfer file has invalid identity metadata".to_string(),
                ));
            }
            transform
                .plan
                .validate()
                .map_err(rusqlite::Error::InvalidParameterName)?;
        }
        for binding in &payload.bin_transforms {
            if !bin_ids.contains(&binding.bin_id)
                || (!transform_refs.contains(&binding.transform_ref)
                    && !payload.pipelines.iter().any(|pipeline| {
                        binding.transform_ref.strip_prefix("transform:")
                            == pipeline.stable_ref.strip_prefix("pipeline:")
                    }))
            {
                return Err(rusqlite::Error::InvalidParameterName(
                    "Transfer file contains an invalid Bin Transform binding".to_string(),
                ));
            }
        }

        Ok(LibraryArchiveInspection {
            schema_version: payload.version,
            clip_count: payload.clips.len(),
            bin_count: payload.bins.len(),
            operation_count: payload
                .operations
                .iter()
                .filter(|item| item.id >= 0)
                .count(),
            transform_count: payload.saved_transforms.len() + payload.pipelines.len(),
            classifier_count: payload.content_classifiers.len(),
            content_type_count: payload.content_types.len(),
        })
    }

    pub fn inspect_library_archive_json(json_str: &str) -> Result<LibraryArchiveInspection> {
        Self::parse_library_archive(json_str).map(|(_, inspection)| inspection)
    }
}
