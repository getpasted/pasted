use super::super::*;
use super::*;

pub(crate) fn run_connections(
    args: Vec<String>,
    db_path: PathBuf,
    _conn: Connection,
) -> Result<()> {
    let db = DbState::new(db_path.clone())?;
    require_feature(&db, Feature::Transformations);
    let subcommand = args.get(2).map(String::as_str).unwrap_or("list");
    let json = args.iter().any(|argument| argument == "--json");
    match subcommand {
        "list" | "ls" => {
            let connections = db.get_intelligence_connections()?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&connections).map_err(json_error)?
                );
            } else {
                for connection in connections {
                    print_connection(&connection, false)?;
                }
            }
        }
        "get" => {
            let id = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted connection get <id> [--json]");
                std::process::exit(2);
            });
            print_connection(&db.get_intelligence_connection(id)?, json)?;
        }
        "detect" | "discover" => {
            let detected = pasted_lib::intelligence_connections::detect_intelligence_connections();
            for candidate in &detected {
                let endpoint = if candidate.provider_kind == "cli" {
                    candidate.executable_path.as_deref()
                } else {
                    candidate.default_endpoint
                };
                db.ensure_intelligence_connection_candidate(
                    candidate.name,
                    candidate.provider_kind,
                    endpoint,
                )?;
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&detected).map_err(json_error)?
                );
            } else if detected.is_empty() {
                println!("No local intelligence connections detected.");
            } else {
                for candidate in detected {
                    println!(
                        "{}\t{}\t{}",
                        candidate.adapter_id, candidate.provider_kind, candidate.name
                    );
                }
            }
        }
        "create" | "new" => {
            let name = argument_value(&args, "--name").unwrap_or_else(|| {
            eprintln!("Usage: pasted connection create --name NAME --provider KIND [--endpoint VALUE] [--model MODEL] [--credential-ref REF] [--json]");
            std::process::exit(2);
        });
            let provider = argument_value(&args, "--provider").unwrap_or_else(|| {
                eprintln!("Connection creation requires --provider.");
                std::process::exit(2);
            });
            let credential_ref = argument_value(&args, "--credential-ref");
            pasted_lib::intelligence_connections::validate_credential_reference(
                credential_ref.as_deref(),
            )
            .unwrap_or_else(|error| {
                eprintln!("{error}");
                std::process::exit(2);
            });
            let connection = db.create_intelligence_connection(
                &name,
                &provider,
                argument_value(&args, "--endpoint").as_deref(),
                argument_value(&args, "--model").as_deref(),
                credential_ref.as_deref(),
            )?;
            print_connection(&connection, json)?;
        }
        "update" | "edit" => {
            let id = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted connection update <id> [options] [--json]");
                std::process::exit(2);
            });
            let current = db.get_intelligence_connection(id)?;
            let credential_ref = optional_argument_update(
                &args,
                "--credential-ref",
                "--clear-credential-ref",
                current.credential_ref.clone(),
            );
            pasted_lib::intelligence_connections::validate_credential_reference(
                credential_ref.as_deref(),
            )
            .unwrap_or_else(|error| {
                eprintln!("{error}");
                std::process::exit(2);
            });
            let enabled = if args.iter().any(|argument| argument == "--disabled") {
                false
            } else if args.iter().any(|argument| argument == "--enabled") {
                true
            } else {
                current.enabled
            };
            let name = argument_value(&args, "--name").unwrap_or(current.name);
            let provider = argument_value(&args, "--provider").unwrap_or(current.provider_kind);
            let endpoint =
                optional_argument_update(&args, "--endpoint", "--clear-endpoint", current.endpoint);
            let model = optional_argument_update(&args, "--model", "--clear-model", current.model);
            db.update_intelligence_connection(IntelligenceConnectionUpdate {
                id,
                name: &name,
                provider_kind: &provider,
                endpoint: endpoint.as_deref(),
                model: model.as_deref(),
                credential_ref: credential_ref.as_deref(),
                enabled,
            })?;
            print_connection(&db.get_intelligence_connection(id)?, json)?;
        }
        "delete" | "remove" => {
            let id = args.get(3).unwrap_or_else(|| {
                eprintln!("Usage: pasted connection delete <id> [--json]");
                std::process::exit(2);
            });
            let connection = db.get_intelligence_connection(id)?;
            db.delete_intelligence_connection(id)?;
            if json {
                println!("{}", serde_json::json!({ "deleted": true, "id": id }));
            } else {
                println!("Deleted Connection {}.", connection.name);
            }
        }
        "order" => {
            let ids = args
                .iter()
                .skip(3)
                .filter(|argument| argument.as_str() != "--json")
                .cloned()
                .collect::<Vec<_>>();
            if ids.is_empty() {
                eprintln!("Usage: pasted connection order <id>... [--json]");
                std::process::exit(2);
            }
            db.reorder_intelligence_connections(&ids)?;
            if json {
                println!("{}", serde_json::json!({ "connectionIds": ids }));
            } else {
                println!("Reordered {} Connections.", ids.len());
            }
        }
        _ => {
            eprintln!("Usage: pasted connection list|get|detect|create|update|delete|order [options] [--json]");
            std::process::exit(2);
        }
    }
    Ok(())
}
