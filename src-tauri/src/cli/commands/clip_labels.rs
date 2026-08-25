use super::super::*;
use super::*;

pub(super) fn run(args: &[String], db: &DbState, json: bool) -> Result<()> {
    let action = args.get(3).map(String::as_str).unwrap_or("list");
    let clip_id = parse_i64_argument(
        args,
        4,
        "Usage: pasted clip labels list|add|remove|reset <clip-id> [label] [--yes] [--json]",
    );
    let labels = match action {
        "list" => db.get_effective_visual_labels(clip_id)?,
        "add" | "remove" => {
            let Some(label) = args.get(5).filter(|value| !value.starts_with("--")) else {
                eprintln!("Usage: pasted clip labels {action} <clip-id> <label> [--json]");
                std::process::exit(2);
            };
            if action == "add" {
                db.add_visual_label(clip_id, label)?
            } else {
                db.remove_visual_label(clip_id, label)?
            }
        }
        "reset" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("Resetting Visual Labels removes every manual addition and suppression. Re-run with --yes.");
                std::process::exit(2);
            }
            db.reset_visual_labels(clip_id)?
        }
        _ => {
            eprintln!("Usage: pasted clip labels list|add|remove|reset <clip-id> [label] [--yes] [--json]");
            std::process::exit(2);
        }
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&labels).map_err(json_error)?
        );
    } else if labels.labels.is_empty() {
        println!("Clip #{clip_id} has no Visual Labels.");
    } else {
        for label in labels.labels {
            let source = match label.source {
                pasted_lib::db::clip_visual_labels::VisualLabelSource::Detected => "detected",
                pasted_lib::db::clip_visual_labels::VisualLabelSource::Manual => "manual",
            };
            let confidence = label
                .confidence_basis_points
                .map(|value| format!("\t{:.2}%", f64::from(value) / 100.0))
                .unwrap_or_default();
            println!("{}\t{}{}", label.value, source, confidence);
        }
    }
    Ok(())
}
