use super::super::*;
use super::*;

pub(crate) fn run_bins(args: Vec<String>, db_path: PathBuf, _conn: Connection) -> Result<()> {
    let db = DbState::new(db_path.clone())?;
    let bins_setting = db.get_setting(Feature::Bins.setting_key())?;
    if !setting_value_is_enabled(bins_setting.as_deref()) {
        eprintln!("Bins are disabled in Settings → Functionality.");
        std::process::exit(1);
    }
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    match subcommand {
        "list" | "ls" => {
            let bins = db.get_bins()?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&bins).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?
                );
            } else {
                for bin in bins {
                    println!(
                        "{}\t{}\t{} clips",
                        bin.id,
                        bin.name,
                        bin.clip_count.unwrap_or(0)
                    );
                }
            }
        }
        "get" => {
            let bin_id = parse_i64_argument(&args, 3, "Usage: pasted bin get <bin-id> [--json]");
            let bin = db.get_bin(bin_id)?;
            let transform_ref = db.get_bin_transform_ref(bin_id)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "bin": bin, "transformRef": transform_ref })
                );
            } else {
                print_bin(&bin, false)?;
            }
        }
        "create" | "new" => {
            let name = argument_value(&args, "--name").unwrap_or_else(|| {
            eprintln!("Usage: pasted bin create --name NAME [--icon ICON] [--color COLOR] [--smart-rule-json JSON] [--transform REF] [--json]");
            std::process::exit(2);
        });
            let smart_rule = argument_value(&args, "--smart-rule-json");
            validate_smart_bin_rule_or_exit(smart_rule.as_deref());
            let bin = db.create_bin(
                &name,
                argument_value(&args, "--icon").as_deref().unwrap_or("📂"),
                argument_value(&args, "--color")
                    .as_deref()
                    .unwrap_or("default"),
                smart_rule.as_deref(),
            )?;
            if let Some(transform_ref) = argument_value(&args, "--transform") {
                db.set_bin_transform_ref(bin.id, Some(&transform_ref))?;
            }
            print_bin(
                &db.get_bin(bin.id)?,
                args.iter().any(|argument| argument == "--json"),
            )?;
        }
        "update" | "edit" => {
            let bin_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted bin update <bin-id> [options] [--json]",
            );
            let current = db.get_bin(bin_id)?;
            let smart_rule = optional_argument_update(
                &args,
                "--smart-rule-json",
                "--clear-smart-rule",
                current.smart_rule,
            );
            validate_smart_bin_rule_or_exit(smart_rule.as_deref());
            db.update_bin(
                bin_id,
                argument_value(&args, "--name")
                    .as_deref()
                    .unwrap_or(&current.name),
                argument_value(&args, "--icon")
                    .as_deref()
                    .unwrap_or(&current.icon),
                argument_value(&args, "--color")
                    .as_deref()
                    .unwrap_or(&current.color),
                smart_rule.as_deref(),
            )?;
            print_bin(
                &db.get_bin(bin_id)?,
                args.iter().any(|argument| argument == "--json"),
            )?;
        }
        "duplicate" | "copy" => {
            let bin_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted bin duplicate <bin-id> [--name NAME] [--json]",
            );
            let source = db.get_bin(bin_id)?;
            let duplicate_name =
                argument_value(&args, "--name").unwrap_or_else(|| format!("{} Copy", source.name));
            let duplicate = db.create_bin(
                &duplicate_name,
                &source.icon,
                &source.color,
                source.smart_rule.as_deref(),
            )?;
            if let Some(transform_ref) = db.get_bin_transform_ref(source.id)? {
                db.set_bin_transform_ref(duplicate.id, Some(&transform_ref))?;
            }
            if source.protect_clips {
                db.update_bin_protection(duplicate.id, true)?;
            }
            print_bin(
                &db.get_bin(duplicate.id)?,
                args.iter().any(|argument| argument == "--json"),
            )?;
        }
        "delete" | "remove" => {
            let bin_id = parse_i64_argument(&args, 3, "Usage: pasted bin delete <bin-id> [--disposition keep|trash|move --move-to BIN] [--json]");
            let bin = db.get_bin(bin_id)?;
            let disposition =
                argument_value(&args, "--disposition").unwrap_or_else(|| "keep".into());
            let destination =
                argument_value(&args, "--move-to").and_then(|value| value.parse::<i64>().ok());
            db.delete_bin(bin_id, &disposition, destination)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": true, "binId": bin_id, "disposition": disposition, "destinationBinId": destination })
                );
            } else {
                println!("Deleted Bin {}.", bin.name);
            }
        }
        "clips" => {
            let Some(bin_id) = args.get(3).and_then(|value| value.parse::<i64>().ok()) else {
                eprintln!("Usage: pasted bin clips <bin-id> [--json]");
                std::process::exit(2);
            };
            let clips = db.get_clips(Some(bin_id), false)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&clips).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?
                );
            } else {
                for (position, clip) in clips.iter().enumerate() {
                    println!(
                        "{}\t{}\t{}",
                        position + 1,
                        clip.id,
                        clip.text_content.as_deref().unwrap_or("")
                    );
                }
            }
        }
        "order" => {
            let Some(bin_id) = args.get(3).and_then(|value| value.parse::<i64>().ok()) else {
                eprintln!("Usage: pasted bin order <bin-id> <clip-id>... [--json]");
                std::process::exit(2);
            };
            let clip_ids = args
                .iter()
                .skip(4)
                .filter(|argument| argument.as_str() != "--json")
                .map(|value| value.parse::<i64>())
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_else(|_| {
                    eprintln!("Every clip ID must be an integer.");
                    std::process::exit(2);
                });
            db.reorder_bin_clips(bin_id, clip_ids.clone())?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "binId": bin_id, "clipIds": clip_ids })
                );
            } else {
                println!("Reordered {} clips in Bin #{bin_id}.", clip_ids.len());
            }
        }
        "transform" => {
            let bin_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted bin transform <bin-id> <transform-ref|none> [--json]",
            );
            let value = args.get(4).unwrap_or_else(|| {
                eprintln!("Usage: pasted bin transform <bin-id> <transform-ref|none> [--json]");
                std::process::exit(2);
            });
            let transform_ref =
                (!matches!(value.as_str(), "none" | "null" | "-")).then_some(value.as_str());
            db.set_bin_transform_ref(bin_id, transform_ref)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "binId": bin_id, "transformRef": transform_ref })
                );
            } else {
                println!("Updated the default Transform for Bin #{bin_id}.");
            }
        }
        "hotkey" => {
            require_feature(&db, Feature::Hotkeys);
            let bin_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted bin hotkey <bin-id> <hotkey|none> [--json]",
            );
            let value = args.get(4).unwrap_or_else(|| {
                eprintln!("Usage: pasted bin hotkey <bin-id> <hotkey|none> [--json]");
                std::process::exit(2);
            });
            let hotkey =
                (!matches!(value.as_str(), "none" | "null" | "-")).then_some(value.as_str());
            db.update_bin_hotkey(bin_id, hotkey)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({ "binId": bin_id, "hotkey": hotkey })
                );
            } else {
                println!("Updated the hotkey for Bin #{bin_id}.");
            }
        }
        "protect" | "protection" => {
            require_feature(&db, Feature::Protection);
            let bin_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted bin protect <bin-id> <on|off> [--json]",
            );
            let value = args.get(4).map(String::as_str).unwrap_or_else(|| {
                eprintln!("Usage: pasted bin protect <bin-id> <on|off> [--json]");
                std::process::exit(2);
            });
            let protect_clips = match value {
                "on" | "true" | "yes" => true,
                "off" | "false" | "no" => false,
                _ => {
                    eprintln!("Bin protection must be on or off.");
                    std::process::exit(2);
                }
            };
            db.update_bin_protection(bin_id, protect_clips)?;
            if args.iter().any(|argument| argument == "--json") {
                println!(
                    "{}",
                    serde_json::json!({
                        "binId": bin_id,
                        "protectClips": protect_clips
                    })
                );
            } else if protect_clips {
                println!("Enabled inherited protection for Bin #{bin_id}.");
            } else {
                println!("Disabled inherited protection for Bin #{bin_id}.");
            }
        }
        _ => {
            eprintln!("Usage: pasted bin list|get|create|update|duplicate|delete|clips|order|transform|hotkey|protect [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
