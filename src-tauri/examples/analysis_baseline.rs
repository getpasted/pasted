use pasted_lib::analysis_execution::{analyze_text, AnalyzerOptions};
use pasted_lib::classification_execution::analyze_classifiers;
use pasted_lib::content_classification::Classifier;
use pasted_lib::content_suggestions::suggest_smart_actions;
use pasted_lib::db::{DbState, PipelineStep, TransformAuthoringKind, TransformDefinition};
use pasted_lib::inspection_execution::inspect_text;
use serde::Serialize;
use std::hint::black_box;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const SAMPLE_COUNT: usize = 9;
const DEFAULT_ITERATIONS: usize = 100;
const STRESS_PARTICIPANTS: usize = 256;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Measurement {
    name: &'static str,
    iterations_per_sample: usize,
    samples: usize,
    median_ns_per_iteration: u128,
    p95_ns_per_iteration: u128,
    median_iterations_per_second: u128,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    format_version: u32,
    profile: &'static str,
    architecture: &'static str,
    measurements: Vec<Measurement>,
}

fn iterations() -> usize {
    let arguments = std::env::args().collect::<Vec<_>>();
    arguments
        .windows(2)
        .find(|pair| pair[0] == "--iterations")
        .and_then(|pair| pair[1].parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_ITERATIONS)
}

fn measure(mut operation: impl FnMut(), name: &'static str, iterations: usize) -> Measurement {
    for _ in 0..iterations.min(10) {
        operation();
    }
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let started = Instant::now();
        for _ in 0..iterations {
            operation();
        }
        samples.push((started.elapsed().as_nanos() / iterations as u128).max(1));
    }
    samples.sort_unstable();
    let median = samples[SAMPLE_COUNT / 2];
    let p95 = samples[SAMPLE_COUNT - 1];
    Measurement {
        name,
        iterations_per_sample: iterations,
        samples: SAMPLE_COUNT,
        median_ns_per_iteration: median,
        p95_ns_per_iteration: p95,
        median_iterations_per_second: 1_000_000_000 / median,
    }
}

fn stress_classifiers() -> Vec<Classifier> {
    (0..STRESS_PARTICIPANTS)
        .map(|index| Classifier {
            id: index as i64,
            stable_ref: format!("classifier:baseline:{index}"),
            name: format!("Baseline Classifier {index}"),
            content_type: "baseline".into(),
            description: String::new(),
            patterns: vec![format!(r"^unmatched-{index}$")],
            validator: None,
            enabled: true,
            priority: index as i64,
            is_builtin: false,
            defaults: None,
            is_deleted: false,
        })
        .collect()
}

fn stress_transforms() -> Vec<TransformDefinition> {
    (0..STRESS_PARTICIPANTS)
        .map(|index| TransformDefinition {
            id: index as i64,
            stable_ref: format!("transform:baseline:{index}"),
            name: if index + 1 == STRESS_PARTICIPANTS {
                "Clean URL".into()
            } else {
                format!("Unrelated Transform {index}")
            },
            authoring_kind: TransformAuthoringKind::Manual,
            execution_character: "replayable".into(),
            connection_id: None,
            shortcut: None,
            revision: 1,
            created_at: String::new(),
            updated_at: String::new(),
            plan: None,
            steps: vec![PipelineStep {
                position: 0,
                operation_ref: if index + 1 == STRESS_PARTICIPANTS {
                    "builtin:clean_url_tracking".into()
                } else {
                    "builtin:uppercase".into()
                },
                config_json: None,
                failure_policy: "stop".into(),
            }],
        })
        .collect()
}

fn temporary_database() -> (DbState, std::path::PathBuf) {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "pasted-analysis-baseline-{}-{stamp}.db",
        std::process::id()
    ));
    (
        DbState::new(path.clone()).expect("benchmark database"),
        path,
    )
}

fn main() {
    let iterations = iterations();
    let text = format!(
        "https://example.test/item?utm_source=baseline\n{}",
        "baseline words ".repeat(4_096)
    );
    let inspection = inspect_text(&text, Some("Pasted CLI"))
        .expect("baseline inspection")
        .result;
    let classifiers = stress_classifiers();
    let transforms = stress_transforms();
    let (database, database_path) = temporary_database();

    assert!((60_000..=66_000).contains(&text.len()));
    assert!(!analyze_classifiers("ordinary benchmark text", &classifiers).matched);
    let suggestion = suggest_smart_actions(&text, Some("link"), &inspection, &transforms);
    assert_eq!(suggestion.actions.len(), 1);
    assert_eq!(
        suggestion.actions[0].transform_ref,
        format!("transform:baseline:{}", STRESS_PARTICIPANTS - 1)
    );
    assert_eq!(
        analyze_text(
            &database,
            &text,
            Some("Pasted CLI"),
            AnalyzerOptions::default(),
        )
        .expect("baseline Analyzer")
        .analysis
        .participants
        .len(),
        3
    );

    let measurements = vec![
        measure(
            || {
                black_box(inspect_text(black_box(&text), Some("Pasted CLI")).unwrap());
            },
            "inspector_text_64k",
            iterations.saturating_mul(10),
        ),
        measure(
            || {
                black_box(analyze_classifiers(
                    black_box("ordinary benchmark text"),
                    black_box(&classifiers),
                ));
            },
            "classifier_256_no_match",
            iterations,
        ),
        measure(
            || {
                black_box(suggest_smart_actions(
                    black_box(&text),
                    Some("link"),
                    black_box(&inspection),
                    black_box(&transforms),
                ));
            },
            "suggestion_256_candidates_last_match",
            iterations,
        ),
        measure(
            || {
                black_box(
                    analyze_text(
                        black_box(&database),
                        black_box(&text),
                        Some("Pasted CLI"),
                        AnalyzerOptions::default(),
                    )
                    .unwrap(),
                );
            },
            "analyzer_interactive_text",
            iterations,
        ),
    ];

    println!(
        "{}",
        serde_json::to_string_pretty(&Report {
            format_version: 1,
            profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            },
            architecture: std::env::consts::ARCH,
            measurements,
        })
        .expect("serialize benchmark report")
    );

    drop(database);
    let _ = std::fs::remove_file(&database_path);
    let _ = std::fs::remove_file(format!("{}-wal", database_path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", database_path.display()));
}
