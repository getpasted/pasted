use super::support::*;

#[test]
fn extractor_recipes_have_cli_authoring_and_execution_parity() {
    let database = temporary_path("extractor-recipe", "db");
    let recipe_path = temporary_path("extractor-recipe", "json");
    let input_path = temporary_path("extractor-input", "txt");
    let recipe = serde_json::json!({
        "definitionVersion": 1,
        "accepts": ["file_references"],
        "output": "searchable_text",
        "steps": [{
            "id": "extract",
            "executable": {
                "path": env!("CARGO_BIN_EXE_pasted"),
                "discover": [],
                "versionArguments": ["--version"]
            },
            "arguments": ["licenses"],
            "mode": "once",
            "capture": "stdout_text",
            "timeoutSeconds": 30
        }],
        "resources": []
    });
    std::fs::write(
        &recipe_path,
        serde_json::to_vec_pretty(&recipe).expect("serialize recipe"),
    )
    .expect("write recipe");
    std::fs::write(&input_path, "input").expect("write input");

    let created = success_json(
        &database,
        &[
            "extractor",
            "create",
            "--name",
            "Portable Test Extractor",
            "--recipe",
            recipe_path.to_str().expect("recipe path"),
            "--json",
        ],
    );
    assert_eq!(created["engine"], "recipe-v1");
    assert_eq!(created["recipe"]["accepts"][0], "file_references");
    assert_eq!(
        created["recipe"]["acceptedFileFormats"],
        serde_json::json!(["*"])
    );

    let history = success_json(
        &database,
        &[
            "extractor",
            "history",
            created["stableRef"].as_str().expect("stable ref"),
            "--json",
        ],
    );
    assert_eq!(history[0]["source"], "manual");

    let run = success_json(
        &database,
        &[
            "extractor",
            "run",
            created["stableRef"].as_str().expect("stable ref"),
            "--file",
            input_path.to_str().expect("input path"),
            "--json",
        ],
    );
    assert_eq!(run["outcome"], "produced");
    assert!(run["output"]
        .as_str()
        .is_some_and(|output| !output.is_empty()));

    clean_database(&database);
    let _ = std::fs::remove_file(recipe_path);
    let _ = std::fs::remove_file(input_path);
}

#[test]
fn extractor_lifecycle_and_registry_capabilities_run_end_to_end() {
    let database = temporary_path("extractors", "db");
    let shipped = success_json(&database, &["extractor", "list", "--json"]);
    let tesseract = shipped
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["stableRef"] == "extractor:tesseract-ocr")
        })
        .expect("shipped Tesseract Extractor");
    assert_eq!(tesseract["engine"], "recipe-v1");
    assert_eq!(
        tesseract["recipe"]["steps"][0]["executable"]["discover"],
        serde_json::json!(["tesseract"])
    );
    assert_eq!(tesseract["inputContract"], "image");
    assert_eq!(
        tesseract["recipe"]["accepts"],
        serde_json::json!(["image", "file_references"])
    );
    assert_eq!(
        tesseract["recipe"]["acceptedFileFormats"],
        serde_json::json!(["bmp", "gif", "jpg", "png", "tif", "webp"])
    );
    assert_eq!(tesseract["outputContract"], "searchable_text");
    assert!(tesseract["isAvailable"].is_boolean());
    assert_eq!(tesseract["runtime"]["method"], "recipe");
    assert!(tesseract["runtime"]["usesAutomaticDiscovery"].is_boolean());
    let whisper = shipped
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["stableRef"] == "extractor:whisper-transcription")
        })
        .expect("shipped Whisper Extractor");
    assert_eq!(whisper["engine"], "recipe-v1");
    assert_eq!(
        whisper["recipe"]["steps"][0]["executable"]["discover"],
        serde_json::json!(["ffmpeg"])
    );
    assert_eq!(
        whisper["recipe"]["steps"][1]["executable"]["discover"],
        serde_json::json!(["whisper-cli"])
    );
    assert_eq!(whisper["inputContract"], "file_references");
    assert_eq!(whisper["outputContract"], "searchable_text");
    assert_eq!(
        whisper["recipe"]["acceptedFileFormats"],
        serde_json::json!(["aac", "flac", "m4a", "mp3", "ogg", "wav"])
    );
    assert_eq!(whisper["modelPath"], Value::Null);
    let configured_whisper = success_json(
        &database,
        &[
            "extractor",
            "update",
            "extractor:whisper-transcription",
            "--model",
            "/tmp/pasted-cli-missing-whisper-model.bin",
            "--json",
        ],
    );
    assert_eq!(
        configured_whisper["modelPath"],
        "/tmp/pasted-cli-missing-whisper-model.bin"
    );
    assert_eq!(configured_whisper["isAvailable"], false);
    let narrowed_whisper = success_json(
        &database,
        &[
            "extractor",
            "update",
            "extractor:whisper-transcription",
            "--format",
            "mp3",
            "--format",
            "wav",
            "--json",
        ],
    );
    assert_eq!(
        narrowed_whisper["recipe"]["acceptedFileFormats"],
        serde_json::json!(["mp3", "wav"])
    );

    let missing_executable = temporary_path("missing-custom-extractor", "bin");
    let executable = missing_executable.to_str().expect("custom executable path");
    let created = success_json(
        &database,
        &[
            "extractor",
            "create",
            "--name",
            "CLI Extractor",
            "--method",
            "custom-command",
            "--executable",
            executable,
            "--enabled",
            "--json",
        ],
    );
    let stable_ref = created["stableRef"].as_str().expect("Extractor stable ref");
    assert_eq!(created["engine"], "recipe-v1");
    assert_eq!(
        created["recipe"]["steps"][0]["executable"]["path"],
        executable
    );
    assert_eq!(created["executablePath"], executable);
    assert_eq!(created["isAvailable"], false);
    assert_eq!(created["enabled"], true);
    assert_eq!(created["revision"], 1);

    let fetched = success_json(&database, &["extractor", "get", stable_ref, "--json"]);
    assert_eq!(fetched["name"], "CLI Extractor");

    let registry = success_json(
        &database,
        &["registry", "list", "--kind", "extractor", "--json"],
    );
    let registry_item = registry
        .as_array()
        .and_then(|items| items.iter().find(|item| item["stableRef"] == stable_ref))
        .expect("Extractor registry item");
    assert_eq!(registry_item["analysisPass"], "extract");
    assert_eq!(
        registry_item["participantContract"]["provides"],
        serde_json::json!(["searchable_text", "analyzable_text"])
    );
    assert_eq!(registry_item["typeRelations"][0]["typeId"], "image");
    assert_eq!(registry_item["capabilities"]["canDuplicate"], true);
    assert_eq!(registry_item["capabilities"]["canDelete"], true);

    let duplicate = success_json(
        &database,
        &[
            "extractor",
            "duplicate",
            stable_ref,
            "--name",
            "CLI Extractor Copy",
            "--json",
        ],
    );
    assert_eq!(duplicate["name"], "CLI Extractor Copy");

    let image = temporary_path("extractor-input", "png");
    std::fs::write(&image, b"private image bytes").expect("write Extractor input");
    let preview_output = run(
        &database,
        &[
            "extractor",
            "run",
            stable_ref,
            "--file",
            image.to_str().expect("image path"),
            "--json",
        ],
    );
    assert_eq!(preview_output.status.code(), Some(1));
    let preview: Value = serde_json::from_slice(&preview_output.stdout).expect("Extractor JSON");
    assert_eq!(preview["formatVersion"], 1);
    assert_eq!(preview["policy"], "interactive");
    assert_eq!(preview["through"], "suggest");
    assert_eq!(preview["targetKind"], "extractor");
    assert_eq!(preview["targetRef"], stable_ref);
    assert_eq!(preview["outcome"], "failed");
    assert_eq!(preview["failure"]["code"], "engine_unavailable");
    assert_eq!(preview["appliedClipId"], Value::Null);
    assert_eq!(preview["ocrUpdated"], false);
    assert_eq!(preview["searchableTextUpdated"], false);
    assert_eq!(preview["classificationUpdated"], false);
    assert_eq!(preview["participants"][0]["pass"], "extract");
    assert_eq!(preview["participants"][0]["stableRef"], stable_ref);
    assert!(!preview.to_string().contains("private image bytes"));
    assert_eq!(
        analysis_fixture("extractor-interactive-unavailable")["failure"]["code"],
        "engine_not_installed"
    );
    let _ = std::fs::remove_file(image);

    success_json(
        &database,
        &[
            "registry",
            "disable",
            "--kind",
            "extractor",
            "--ref",
            stable_ref,
            "--json",
        ],
    );
    let disabled = success_json(&database, &["extractor", "get", stable_ref, "--json"]);
    assert_eq!(disabled["enabled"], false);
    let activity = success_json(&database, &["activity", "list", "--all", "--json"]);
    assert!(activity
        .as_array()
        .is_some_and(|logs| logs.iter().any(|log| {
            log["event_type"] == "content_extractor_disabled"
                && log["description"] == "Disabled Extractor \"CLI Extractor\""
        })));

    let deleted = success_json(&database, &["extractor", "delete", stable_ref, "--json"]);
    assert_eq!(deleted["deleted"], true);
    clean_database(&database);
}
