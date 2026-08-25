use super::*;

fn ocr_acceptance_image() -> Vec<u8> {
    const SCALE: u32 = 12;
    const GLYPH_WIDTH: u32 = 5;
    const GLYPH_HEIGHT: u32 = 7;
    const TEXT: &str = "PASTED OCR";
    fn glyph(character: char) -> [&'static str; GLYPH_HEIGHT as usize] {
        match character {
            'P' => [
                "11110", "10001", "10001", "11110", "10000", "10000", "10000",
            ],
            'A' => [
                "01110", "10001", "10001", "11111", "10001", "10001", "10001",
            ],
            'S' => [
                "01111", "10000", "10000", "01110", "00001", "00001", "11110",
            ],
            'T' => [
                "11111", "00100", "00100", "00100", "00100", "00100", "00100",
            ],
            'E' => [
                "11111", "10000", "10000", "11110", "10000", "10000", "11111",
            ],
            'D' => [
                "11110", "10001", "10001", "10001", "10001", "10001", "11110",
            ],
            'O' => [
                "01110", "10001", "10001", "10001", "10001", "10001", "01110",
            ],
            'C' => [
                "01111", "10000", "10000", "10000", "10000", "10000", "01111",
            ],
            'R' => [
                "11110", "10001", "10001", "11110", "10100", "10010", "10001",
            ],
            _ => ["00000"; GLYPH_HEIGHT as usize],
        }
    }

    let margin = 24;
    let advance = (GLYPH_WIDTH + 2) * SCALE;
    let width = margin * 2 + advance * TEXT.chars().count() as u32;
    let height = margin * 2 + GLYPH_HEIGHT * SCALE;
    let mut image = image::GrayImage::from_pixel(width, height, image::Luma([255]));
    for (index, character) in TEXT.chars().enumerate() {
        for (row, pixels) in glyph(character).iter().enumerate() {
            for (column, pixel) in pixels.bytes().enumerate() {
                if pixel != b'1' {
                    continue;
                }
                for y in 0..SCALE {
                    for x in 0..SCALE {
                        image.put_pixel(
                            margin + index as u32 * advance + column as u32 * SCALE + x,
                            margin + row as u32 * SCALE + y,
                            image::Luma([0]),
                        );
                    }
                }
            }
        }
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageLuma8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

#[cfg(target_os = "macos")]
#[test]
fn advertised_apple_vision_engine_is_linked() {
    assert!(objc::runtime::Class::get("VNRecognizeTextRequest").is_some());
    assert!(objc::runtime::Class::get("VNClassifyImageRequest").is_some());
}

struct FixedEngine {
    outcome: ExtractionOutcome,
}

impl ExtractorEngine for FixedEngine {
    fn id(&self) -> &'static str {
        "test-v1"
    }

    fn availability(&self) -> EngineAvailability {
        EngineAvailability {
            is_available: true,
            unavailable_reason: None,
        }
    }

    fn extract(&self, _image_bytes: &[u8]) -> ExtractionOutcome {
        self.outcome.clone()
    }
}

fn extractor(engine: &str) -> Extractor {
    Extractor {
        id: 1,
        stable_ref: "extractor:test".into(),
        name: "Test Extractor".into(),
        description: String::new(),
        engine: engine.into(),
        executable_path: None,
        model_path: None,
        input_contract: "image".into(),
        output_contract: "searchable_text".into(),
        enabled: true,
        priority: 10,
        revision: 1,
        is_builtin: false,
        is_available: true,
        unavailable_reason: None,
        runtime: runtime_status_for(engine, None),
        recipe: test_recipe("image"),
        recipe_hash: "test".into(),
        default_recipe: None,
        defaults: None,
    }
}

fn produced(text: impl Into<String>) -> ExtractionOutcome {
    ExtractionOutcome::Produced {
        text: text.into(),
        labels: Vec::new(),
    }
}

#[test]
fn registry_dispatches_typed_engine_outcomes() {
    let engine = FixedEngine {
        outcome: produced("recognized"),
    };
    let engines: [&dyn ExtractorEngine; 1] = [&engine];
    let registry = ExtractorEngineRegistry::new(&engines);

    assert_eq!(
        registry.execute(&extractor("test-v1"), b"image"),
        produced("recognized")
    );
}

#[test]
fn registry_rejects_unknown_contracts_before_engine_dispatch() {
    let engine = FixedEngine {
        outcome: produced("should not run"),
    };
    let engines: [&dyn ExtractorEngine; 1] = [&engine];
    let registry = ExtractorEngineRegistry::new(&engines);
    let mut invalid = extractor("test-v1");
    invalid.recipe.accepts = vec![ExtractorInputKind::FileReferences];

    assert_eq!(
        registry.execute(&invalid, b"image"),
        ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "invalid_contract".into(),
                message: "This extraction contract is not supported.".into(),
            }
        }
    );
}

#[test]
fn registry_normalizes_blank_and_oversized_engine_output() {
    let blank_engine = FixedEngine {
        outcome: produced("  "),
    };
    let blank_engines: [&dyn ExtractorEngine; 1] = [&blank_engine];
    let blank_registry = ExtractorEngineRegistry::new(&blank_engines);
    assert_eq!(
        blank_registry.execute(&extractor("test-v1"), b"image"),
        ExtractionOutcome::NoOutput
    );

    let oversized_engine = FixedEngine {
        outcome: produced("x".repeat(crate::resource_limits::MAX_OCR_TEXT_BYTES + 1)),
    };
    let oversized_engines: [&dyn ExtractorEngine; 1] = [&oversized_engine];
    let oversized_registry = ExtractorEngineRegistry::new(&oversized_engines);
    assert!(matches!(
        oversized_registry.execute(&extractor("test-v1"), b"image"),
        ExtractionOutcome::Failed {
            failure: ExtractionFailure { ref code, .. }
        } if code == "output_too_large"
    ));
}

#[cfg(unix)]
#[test]
fn custom_command_executes_the_bounded_v1_protocol() {
    use std::os::unix::fs::PermissionsExt;

    let workspace = crate::external_tools::PrivateWorkspace::create("custom-engine-test").unwrap();
    let executable = workspace.join("extractor");
    fs::write(
            &executable,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'Example Extractor 1.2.3'; exit 0; fi\nif [ \"$1\" = \"--pasted-extract-v1\" ] && [ -f \"$2\" ]; then printf '{\"text\":\"custom searchable text\"}'; exit 0; fi\nexit 2\n",
        )
        .unwrap();
    let mut permissions = executable.metadata().unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions).unwrap();

    let mut custom = extractor(CUSTOM_COMMAND_ENGINE);
    custom.executable_path = Some(executable.to_string_lossy().into_owned());
    custom.runtime = runtime_status_for(CUSTOM_COMMAND_ENGINE, custom.executable_path.as_deref());
    assert_eq!(
        custom.runtime.version.as_deref(),
        Some("Example Extractor 1.2.3")
    );
    assert!(
        system_engine_registry()
            .availability_for(CUSTOM_COMMAND_ENGINE, Some(&executable), None,)
            .is_available
    );
    assert_eq!(
        system_engine_registry().execute(&custom, b"image"),
        produced("custom searchable text")
    );
}

#[test]
fn shipped_definition_upgrades_preserve_only_user_overrides() {
    let previous = ExtractorDefinitionInput {
        name: "Shipped".into(),
        description: "Old description".into(),
        engine: TESSERACT_ENGINE.into(),
        executable_path: None,
        model_path: None,
        input_contract: "image".into(),
        output_contract: "searchable_text".into(),
        enabled: true,
        priority: 20,
    };
    let current = ExtractorDefinitionInput {
        name: "My OCR".into(),
        executable_path: Some("/custom/tesseract".into()),
        ..previous.clone()
    };
    let next = ExtractorDefinitionInput {
        description: "New shipped description".into(),
        priority: 15,
        ..previous.clone()
    };
    let merged = merge_shipped_definition(&current, &previous, &next);
    assert_eq!(merged.name, "My OCR");
    assert_eq!(merged.executable_path.as_deref(), Some("/custom/tesseract"));
    assert_eq!(merged.description, "New shipped description");
    assert_eq!(merged.priority, 15);
}

#[test]
fn bundled_recipe_migration_repairs_the_interim_apple_locator() {
    let mut recipe = EXTRACTOR_PRESETS
        .iter()
        .find(|preset| preset.stable_ref == APPLE_VISION_OCR_REF)
        .unwrap()
        .recipe();
    recipe.steps[0].executable.discover = vec!["pasted".into()];
    recipe.steps[0].executable.version_arguments = vec!["--version".into()];

    let migrated = migrate_builtin_recipe_compatibility(APPLE_VISION_OCR_REF, &recipe, None);

    assert_eq!(
        migrated.steps[0].executable.discover,
        [BUNDLED_EXTRACTOR_EXECUTABLE]
    );
    assert!(migrated.steps[0].executable.version_arguments.is_empty());
}

#[test]
fn bundled_recipe_migration_preserves_the_configured_whisper_model() {
    let mut recipe = EXTRACTOR_PRESETS
        .iter()
        .find(|preset| preset.stable_ref == WHISPER_TRANSCRIPTION_REF)
        .unwrap()
        .recipe();
    recipe.steps = vec![ExtractorCommandStep {
        id: "extract".into(),
        executable: ExtractorExecutable {
            path: Some("/custom/whisper-cli".into()),
            discover: vec!["whisper-cli".into()],
            version_arguments: vec!["--version".into()],
        },
        arguments: vec![
            "--model".into(),
            "{resource.model.path}".into(),
            "--file".into(),
            "{input.path}".into(),
            "--no-timestamps".into(),
        ],
        mode: ExtractorStepMode::EachInput,
        capture: ExtractorCapture::StdoutText,
        output_extension: None,
        timeout_seconds: 300,
    }];

    let migrated = migrate_builtin_recipe_compatibility(
        WHISPER_TRANSCRIPTION_REF,
        &recipe,
        Some("/models/ggml-base.bin"),
    );

    assert_eq!(migrated.steps.len(), 2);
    assert_eq!(
        migrated.steps[1].executable.path.as_deref(),
        Some("/custom/whisper-cli")
    );
    assert_eq!(
        migrated.resources[0].path.as_deref(),
        Some("/models/ggml-base.bin")
    );
}

#[test]
fn unknown_and_unavailable_engines_fail_with_stable_codes() {
    let registry = ExtractorEngineRegistry::new(&[]);
    assert_eq!(
        registry.execute(&extractor("missing-v1"), b"image"),
        ExtractionOutcome::Failed {
            failure: ExtractionFailure {
                code: "engine_not_installed".into(),
                message: "This extraction engine is not installed.".into(),
            }
        }
    );

    let apple = system_engine_registry().availability(APPLE_VISION_ENGINE);
    assert_eq!(apple.is_available, cfg!(target_os = "macos"));
    assert_eq!(
        apple.unavailable_reason.is_none(),
        cfg!(target_os = "macos")
    );

    let tesseract = system_engine_registry().availability(TESSERACT_ENGINE);
    assert_eq!(
        tesseract.is_available,
        find_tesseract_executable().is_some()
    );
    assert_eq!(
        tesseract.unavailable_reason.is_none(),
        tesseract.is_available
    );
}

#[test]
fn apple_vision_adapter_rejects_empty_and_invalid_input_without_output() {
    assert_eq!(perform_apple_vision_ocr(&[]), None);
    assert_eq!(perform_apple_vision_ocr(&[0, 1, 2, 3, 4]), None);
}

#[test]
fn whisper_classifies_native_container_and_unsupported_audio() {
    assert_eq!(
        whisper_audio_preparation(Path::new("recording.WAV")),
        Some(WhisperAudioPreparation::Native)
    );
    assert_eq!(
        whisper_audio_preparation(Path::new("recording.m4a")),
        Some(WhisperAudioPreparation::FfmpegWav)
    );
    assert_eq!(
        whisper_audio_preparation(Path::new("recording.AAC")),
        Some(WhisperAudioPreparation::FfmpegWav)
    );
    assert_eq!(whisper_audio_preparation(Path::new("notes.txt")), None);
}

#[test]
fn whisper_reports_unsupported_files_instead_of_no_speech() {
    let workspace =
        crate::external_tools::PrivateWorkspace::create("unsupported-audio-test").unwrap();
    let input = workspace.join("notes.txt");
    fs::write(&input, b"not audio").unwrap();
    let outcome = perform_whisper_cpp_transcription(
        Path::new("unused-whisper"),
        Path::new("unused-model"),
        &[input.to_string_lossy().into_owned()],
        Duration::from_secs(1),
    );
    assert!(matches!(
        outcome,
        ExtractionOutcome::Failed {
            failure: ExtractionFailure { ref code, .. }
        } if code == "unsupported_input"
    ));
}

#[test]
fn ffmpeg_prepares_m4a_for_whisper_when_installed() {
    let Some(ffmpeg) = find_ffmpeg_executable() else {
        return;
    };
    let workspace = crate::external_tools::PrivateWorkspace::create("m4a-test").unwrap();
    let input = workspace.join("tone.m4a");
    let status = Command::new(ffmpeg)
        .args(["-nostdin", "-v", "error", "-y", "-f", "lavfi", "-i"])
        .arg("sine=frequency=440:duration=0.2")
        .args(["-c:a", "aac"])
        .arg(&input)
        .status()
        .unwrap();
    assert!(status.success());

    let prepared = prepare_whisper_audio(
        &input,
        WhisperAudioPreparation::FfmpegWav,
        &workspace,
        0,
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(
        prepared.extension().and_then(|value| value.to_str()),
        Some("wav")
    );
    assert!(prepared.metadata().unwrap().len() > 44);
}

#[test]
fn tesseract_adapter_recognizes_text_when_installed() {
    let Some(executable) = find_tesseract_executable() else {
        return;
    };
    let outcome = perform_tesseract_ocr(
        &executable,
        &ocr_acceptance_image(),
        Duration::from_secs(15),
    );
    assert!(
        matches!(outcome, ExtractionOutcome::Produced { ref text, .. }
                if text.to_ascii_uppercase().contains("PASTE")),
        "unexpected Tesseract result: {outcome:?}"
    );
}

#[test]
fn shipped_tesseract_recipe_uses_the_universal_runner() {
    if find_tesseract_executable().is_none() {
        return;
    }
    let recipe = EXTRACTOR_PRESETS
        .iter()
        .find(|preset| preset.stable_ref == TESSERACT_OCR_REF)
        .unwrap()
        .recipe();
    let outcome = crate::extractor_recipe::execute_image(&recipe, &ocr_acceptance_image());
    assert!(
        matches!(outcome, ExtractionOutcome::Produced { ref text, .. }
                if text.to_ascii_uppercase().contains("PASTE")),
        "unexpected recipe result: {outcome:?}"
    );
}
