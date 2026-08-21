use super::super::*;
use super::*;

pub(crate) fn run_clips(args: Vec<String>, db_path: PathBuf, _conn: Connection) -> Result<()> {
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("help");
    let json = args.iter().any(|argument| argument == "--json");
    match subcommand {
        "export" => {
            let path = args
                .get(3)
                .filter(|argument| !argument.starts_with("--"))
                .map(PathBuf::from);
            let format = argument_value(&args, "--format").unwrap_or_else(|| {
                path.as_ref()
                    .and_then(|value| value.extension())
                    .and_then(|value| value.to_str())
                    .unwrap_or("json")
                    .to_ascii_lowercase()
            });
            let contents = match format.as_str() {
                "json" => db.export_clips_json()?,
                "csv" => db.export_clips_csv()?,
                _ => {
                    eprintln!("Clip export format must be json or csv.");
                    std::process::exit(2);
                }
            };
            if let Some(path) = path {
                fs::write(&path, contents)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
                if json {
                    println!("{}", serde_json::json!({ "format": format, "path": path }));
                } else {
                    println!("Exported clips in History to {}.", path.display());
                }
            } else {
                print!("{contents}");
            }
        }
        "import" => {
            let Some(path) = args.get(3).filter(|argument| !argument.starts_with("--")) else {
                eprintln!("Usage: pasted clip import <path> [--format json|csv] [--json]");
                std::process::exit(2);
            };
            let format = argument_value(&args, "--format").unwrap_or_else(|| {
                Path::new(path)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("json")
                    .to_ascii_lowercase()
            });
            let contents = read_library_archive(Path::new(path))?;
            let report = match format.as_str() {
                "json" => db.import_clips_json(&contents)?,
                "csv" => db.import_clips_csv(&contents)?,
                _ => {
                    eprintln!("Clip import format must be json or csv.");
                    std::process::exit(2);
                }
            };
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(json_error)?
                );
            } else {
                println!(
                    "Imported {} clips; skipped {} duplicates.",
                    report.imported_count, report.duplicate_count
                );
            }
        }
        "get" | "show" => {
            let Some(clip_id) = args.get(3).and_then(|value| value.parse::<i64>().ok()) else {
                eprintln!("Usage: pasted clip get <clip-id> [--json]");
                std::process::exit(2);
            };
            let clip = db.get_clip_by_id(clip_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&clip).map_err(json_error)?
                );
            } else {
                println!(
                    "#{}\t{}\t{}\t{}",
                    clip.id,
                    clip.content_type,
                    clip.source,
                    clip.text_content.as_deref().unwrap_or("")
                );
            }
        }
        "note" => {
            let clip_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted clip note <clip-id> [--text TEXT | --clear | --stdin] [--json]",
            );
            let note = if args.iter().any(|argument| argument == "--clear") {
                None
            } else {
                Some(match argument_value(&args, "--text") {
                    Some(note) => note,
                    None => read_stdin_bounded(pasted_lib::resource_limits::MAX_CLIP_NOTE_BYTES)?,
                })
            };
            db.update_clip_note(clip_id, note.as_deref())?;
            if json {
                println!("{}", serde_json::json!({ "clipId": clip_id, "note": note }));
            } else {
                println!("Updated note for clip #{clip_id}.");
            }
        }
        "revisions" | "versions" => {
            let clip_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted clip revisions <clip-id> [--limit N] [--offset N] [--json]",
            );
            let limit = argument_value(&args, "--limit")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(50)
                .clamp(1, 1_000);
            let offset = argument_value(&args, "--offset")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(0)
                .max(0);
            let revisions = db.get_clip_versions_page(clip_id, limit, offset)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&revisions).map_err(json_error)?
                );
            } else if revisions.is_empty() {
                println!("No revisions for clip #{clip_id}.");
            } else {
                for revision in revisions {
                    println!(
                        "{}\t{}\t{}",
                        revision.id,
                        revision.created_at,
                        revision.action_label.as_deref().unwrap_or("Revision")
                    );
                }
            }
        }
        "restore-revision" | "restore-version" => {
            let clip_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted clip restore-revision <clip-id> <revision-id> [--json]",
            );
            let revision_id = parse_i64_argument(
                &args,
                4,
                "Usage: pasted clip restore-revision <clip-id> <revision-id> [--json]",
            );
            let clip = db.restore_clip_version(clip_id, revision_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&clip).map_err(json_error)?
                );
            } else {
                println!("Restored revision #{revision_id} for clip #{clip_id}.");
            }
        }
        "provenance" => {
            let clip_id =
                parse_i64_argument(&args, 3, "Usage: pasted clip provenance <clip-id> [--json]");
            let provenance = db.get_clip_transformation_provenance(clip_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&provenance).map_err(json_error)?
                );
            } else if let Some(provenance) = provenance {
                println!(
                    "{}\trevision {}\t{} ms\t{}",
                    provenance.transform_ref,
                    provenance.transform_revision,
                    provenance.duration_ms,
                    provenance.transform_name
                );
            } else {
                println!("Clip #{clip_id} has no Transform provenance.");
            }
        }
        "copy" | "paste" => {
            let clip_id =
                parse_i64_argument(&args, 3, "Usage: pasted clip copy|paste <clip-id> [--json]");
            let action = if subcommand == "copy" {
                pasted_lib::live_app::LiveAppAction::CopyClip { clip_id }
            } else {
                pasted_lib::live_app::LiveAppAction::PasteClip { clip_id }
            };
            let result = send_live_or_exit(action);
            print_live_result(&result, json)?;
        }
        "hotkey" => {
            require_feature(&db, Feature::Protection);
            require_feature(&db, Feature::Hotkeys);
            let clip_id = parse_i64_argument(
                &args,
                3,
                "Usage: pasted clip hotkey <clip-id> <hotkey|none> [--json]",
            );
            let value = args.get(4).unwrap_or_else(|| {
                eprintln!("Usage: pasted clip hotkey <clip-id> <hotkey|none> [--json]");
                std::process::exit(2);
            });
            let hotkey =
                (!matches!(value.as_str(), "none" | "null" | "-")).then_some(value.as_str());
            db.update_clip_hotkey(clip_id, hotkey)?;
            let description = if hotkey.is_some() {
                format!("Assigned a hotkey to clip #{clip_id}")
            } else {
                format!("Removed the hotkey from clip #{clip_id}")
            };
            db.log_activity("clip_hotkey_changed", &description)?;
            let updated = db.get_clip_by_id(clip_id)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "clipId": clip_id,
                        "hotkey": hotkey,
                        "protected": updated.is_protected
                    })
                );
            } else if hotkey.is_some() {
                println!("Assigned a hotkey to protected clip #{clip_id}.");
            } else {
                println!(
                    "Removed the hotkey from clip #{clip_id}; existing protection was unchanged."
                );
            }
        }
        "pin" | "unpin" => {
            require_feature(&db, Feature::Pinning);
            let ids = parse_clip_ids(&args, 3);
            let summary = db.batch_pin_clips(ids, subcommand == "pin")?;
            print_mutation_summary(&summary, json)?;
        }
        "order-pinned" => {
            require_feature(&db, Feature::Pinning);
            let ids = parse_clip_ids(&args, 3);
            db.reorder_pinned_clips(ids.clone())?;
            if json {
                println!("{}", serde_json::json!({ "clipIds": ids }));
            } else {
                println!("Saved the order of {} pinned clips.", ids.len());
            }
        }
        "protect" | "unprotect" => {
            require_feature(&db, Feature::Protection);
            let ids = parse_clip_ids(&args, 3);
            let summary = db.batch_protect_clips(ids, subcommand == "protect")?;
            print_mutation_summary(&summary, json)?;
        }
        "conceal" | "unconceal" => {
            require_feature(&db, Feature::Concealment);
            let ids = parse_clip_ids(&args, 3);
            let summary = db.batch_conceal_clips(ids, subcommand == "conceal")?;
            print_mutation_summary(&summary, json)?;
        }
        "trash" => {
            require_feature(&db, Feature::Trash);
            let summary = db.batch_trash_clips(parse_clip_ids(&args, 3))?;
            print_mutation_summary(&summary, json)?;
        }
        "restore" => {
            require_feature(&db, Feature::Trash);
            let ids = parse_clip_ids(&args, 3);
            let requested_count = ids.len();
            let mut changed_ids = Vec::new();
            for id in ids {
                changed_ids.extend(db.restore_clip(id)?.clip_ids);
            }
            let summary = ClipMutationSummary {
                action: "restore".to_string(),
                requested_count,
                changed_count: changed_ids.len(),
                skipped_count: requested_count.saturating_sub(changed_ids.len()),
                clip_ids: changed_ids,
            };
            print_mutation_summary(&summary, json)?;
        }
        "restore-all" => {
            require_feature(&db, Feature::Trash);
            let summary = db.restore_all_trashed_clips()?;
            print_mutation_summary(&summary, json)?;
        }
        "purge" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("Permanent deletion cannot be undone. Re-run with --yes.");
                std::process::exit(2);
            }
            let ids = parse_clip_ids(&args, 3);
            for id in &ids {
                db.purge_clip_permanently(*id)?;
            }
            if json {
                println!("{}", serde_json::json!({ "purgedClipIds": ids }));
            } else {
                println!(
                    "Permanently deleted {} requested clips; protected clips were preserved.",
                    ids.len()
                );
            }
        }
        "empty-trash" => {
            if !args.iter().any(|argument| argument == "--yes") {
                eprintln!("Emptying Trash is permanent. Re-run with --yes.");
                std::process::exit(2);
            }
            db.empty_trash()?;
            if json {
                println!("{}", serde_json::json!({ "emptied": true }));
            } else {
                println!("Emptied Trash; protected clips were preserved.");
            }
        }
        "assign" => {
            require_feature(&db, Feature::Bins);
            let Some(destination) = args.get(3) else {
                eprintln!("Usage: pasted clip assign <bin-id|none> <clip-id>... [--json]");
                std::process::exit(2);
            };
            let bin_id = if matches!(destination.as_str(), "none" | "null" | "-") {
                None
            } else {
                destination.parse::<i64>().ok().or_else(|| {
                    eprintln!("Bin ID must be an integer or 'none'.");
                    std::process::exit(2);
                })
            };
            let outcome =
                assign_clips_to_bin(&db, parse_clip_ids(&args, 4), bin_id).map_err(|error| {
                    rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error)))
                })?;
            print_mutation_summary(&outcome.mutation, json)?;
        }
        "remove-bin" => {
            require_feature(&db, Feature::Bins);
            let Some(bin_id) = args.get(3).and_then(|value| value.parse::<i64>().ok()) else {
                eprintln!("Usage: pasted clip remove-bin <bin-id> <clip-id>... [--json]");
                std::process::exit(2);
            };
            let outcome = pasted_lib::bin_assignment::remove_clips_from_bin(
                &db,
                parse_clip_ids(&args, 4),
                bin_id,
            )
            .map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error)))
            })?;
            print_mutation_summary(&outcome.mutation, json)?;
        }
        _ => {
            eprintln!("Usage: pasted clip get|note|revisions|restore-revision|provenance|copy|paste|hotkey|pin|unpin|order-pinned|protect|unprotect|conceal|unconceal|trash|restore|restore-all|purge|empty-trash|assign|remove-bin|export|import [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
