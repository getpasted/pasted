use super::super::*;
use super::*;

pub(crate) fn run_registry(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    let kind = argument_value(&args, "--kind");
    if matches!(subcommand, "enable" | "disable") {
        let kind = kind.ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                "registry enable/disable requires --kind".to_string(),
            )
        })?;
        let stable_ref = argument_value(&args, "--ref").ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                "registry enable/disable requires --ref".to_string(),
            )
        })?;
        db.set_library_item_enabled(&kind, &stable_ref, subcommand == "enable")?;
        if args.iter().any(|argument| argument == "--json") {
            println!(
                "{}",
                serde_json::json!({
                    "kind": kind,
                    "stableRef": stable_ref,
                    "enabled": subcommand == "enable",
                })
            );
        } else {
            println!(
                "{} {} {}.",
                if subcommand == "enable" {
                    "Enabled"
                } else {
                    "Disabled"
                },
                kind,
                stable_ref
            );
        }
        return Ok(());
    }
    if subcommand != "list" && !subcommand.starts_with('-') {
        return Err(rusqlite::Error::InvalidParameterName(format!(
            "Unknown registry command: {subcommand}"
        )));
    }
    let items = db.get_library_items(
        kind.as_deref(),
        args.iter().any(|argument| argument == "--all"),
    )?;
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&items).map_err(json_error)?
        );
    } else {
        for view in items {
            println!(
                "{}\t{}\t{}",
                view.item.kind, view.item.stable_ref, view.item.name
            );
        }
    }
    Ok(())
}

pub(crate) fn run_types(args: Vec<String>, db_path: PathBuf, conn: Connection) -> Result<()> {
    drop(conn);
    let db = DbState::new(db_path.clone())?;
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    let json = args.iter().any(|argument| argument == "--json");
    match subcommand {
        "group-list" => {
            let groups =
                db.get_content_type_groups(args.iter().any(|argument| argument == "--all"))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&groups).map_err(json_error)?
                );
            } else {
                for group in groups {
                    println!(
                        "{}\t{}\t{}\t{}",
                        group.id,
                        group.sort_order,
                        if group.is_archived {
                            "archived"
                        } else {
                            "active"
                        },
                        group.label
                    );
                }
            }
        }
        "group-create" => {
            let id = argument_value(&args, "--id").unwrap_or_else(|| {
                eprintln!(
                    "Usage: pasted type group-create --id ID --name NAME [--order NUMBER] [--json]"
                );
                std::process::exit(2);
            });
            let label = argument_value(&args, "--name").unwrap_or_else(|| {
                eprintln!("Group creation requires --name.");
                std::process::exit(2);
            });
            let created = db.create_content_type_group(&ContentTypeGroupInput {
                id,
                label,
                sort_order: argument_value(&args, "--order")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(100),
            })?;
            println!(
                "{}",
                if json {
                    serde_json::to_string_pretty(&created).map_err(json_error)?
                } else {
                    format!("Saved content type group {}: {}", created.id, created.label)
                }
            );
        }
        "group-update" => {
            let id = args.get(3).cloned().unwrap_or_else(|| {
                eprintln!(
                    "Usage: pasted type group-update <id> [--name NAME] [--order NUMBER] [--json]"
                );
                std::process::exit(2);
            });
            let current = db
                .get_content_type_groups(true)?
                .into_iter()
                .find(|item| item.id == id)
                .unwrap_or_else(|| {
                    eprintln!("Content type group {id} was not found.");
                    std::process::exit(1);
                });
            let updated = db.update_content_type_group(
                &id,
                &ContentTypeGroupInput {
                    id: id.clone(),
                    label: argument_value(&args, "--name").unwrap_or(current.label),
                    sort_order: argument_value(&args, "--order")
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(current.sort_order),
                },
            )?;
            println!(
                "{}",
                if json {
                    serde_json::to_string_pretty(&updated).map_err(json_error)?
                } else {
                    format!("Saved content type group {}: {}", updated.id, updated.label)
                }
            );
        }
        "group-archive" | "group-restore" => {
            let id = args.get(3).cloned().unwrap_or_else(|| {
                eprintln!("Usage: pasted type {subcommand} <id>");
                std::process::exit(2);
            });
            db.set_content_type_group_archived(&id, subcommand == "group-archive")?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "id": id, "archived": subcommand == "group-archive" })
                );
            } else {
                println!(
                    "{} content type group {id}.",
                    if subcommand == "group-archive" {
                        "Archived"
                    } else {
                        "Restored"
                    }
                );
            }
        }
        "group-delete" => {
            let id = args.get(3).cloned().unwrap_or_else(|| {
                eprintln!("Usage: pasted type group-delete <id>");
                std::process::exit(2);
            });
            db.delete_content_type_group(&id)?;
            if json {
                println!("{}", serde_json::json!({ "id": id, "deleted": true }));
            } else {
                println!("Deleted content type group {id}.");
            }
        }
        "group-restore-defaults" => {
            db.restore_default_content_type_groups()?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "restoredDefaults": true, "kind": "contentTypeGroups" })
                );
            } else {
                println!("Restored built-in content type groups.");
            }
        }
        "list" | "ls" => {
            let types = db.get_content_types(args.iter().any(|argument| argument == "--all"))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&types).map_err(json_error)?
                );
            } else {
                for item in types {
                    println!(
                        "{}\t{}\t{}\t{}",
                        item.id,
                        item.icon,
                        if item.is_archived {
                            "archived"
                        } else {
                            "active"
                        },
                        item.label
                    );
                }
            }
        }
        "create" => {
            let id = argument_value(&args, "--id").unwrap_or_else(|| {
            eprintln!("Usage: pasted type create --id ID --name NAME [--icon ICON] [--group GROUP] [--json]");
            std::process::exit(2);
        });
            let label = argument_value(&args, "--name").unwrap_or_else(|| {
                eprintln!("Type creation requires --name.");
                std::process::exit(2);
            });
            let created = db.create_content_type(&ContentTypeInput {
                id,
                label,
                icon: argument_value(&args, "--icon").unwrap_or_else(|| "FileText".into()),
                group: argument_value(&args, "--group").unwrap_or_else(|| "custom".into()),
            })?;
            print_content_type(&created, json)?;
        }
        "update" => {
            let id = args.get(3).cloned().unwrap_or_else(|| {
            eprintln!("Usage: pasted type update <id> [--name NAME] [--icon ICON] [--group GROUP] [--json]");
            std::process::exit(2);
        });
            let current = db
                .get_content_types(true)?
                .into_iter()
                .find(|item| item.id == id)
                .unwrap_or_else(|| {
                    eprintln!("Content type {id} was not found.");
                    std::process::exit(1);
                });
            let updated = db.update_content_type(
                &id,
                &ContentTypeInput {
                    id: id.clone(),
                    label: argument_value(&args, "--name").unwrap_or(current.label),
                    icon: argument_value(&args, "--icon").unwrap_or(current.icon),
                    group: argument_value(&args, "--group").unwrap_or(current.group),
                },
            )?;
            print_content_type(&updated, json)?;
        }
        "archive" | "restore" => {
            let id = args.get(3).cloned().unwrap_or_else(|| {
                eprintln!("Usage: pasted type {subcommand} <id>");
                std::process::exit(2);
            });
            db.set_content_type_archived(&id, subcommand == "archive")?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "id": id, "archived": subcommand == "archive" })
                );
            } else {
                println!(
                    "{} content type {id}.",
                    if subcommand == "archive" {
                        "Archived"
                    } else {
                        "Restored"
                    }
                );
            }
        }
        "restore-defaults" => {
            db.restore_default_content_types()?;
            db.restore_default_content_type_groups()?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "restoredDefaults": true, "kind": "contentTypes" })
                );
            } else {
                println!("Restored built-in content type names, icons, and groups.");
            }
        }
        _ => {
            eprintln!(
                "Usage: pasted type list|create|update|archive|restore|restore-defaults [--json]"
            );
            std::process::exit(2);
        }
    }
    Ok(())
}
