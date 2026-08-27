use rusqlite::Result;

use pasted_lib::update_manifest::check_for_cli_update;

use super::json_error;

pub(crate) fn run(args: &[String]) -> Result<()> {
    if args.get(2).map(String::as_str) != Some("check") {
        eprintln!("Usage: pasted update check [--json]");
        return Ok(());
    }
    let report = check_for_cli_update(env!("CARGO_PKG_VERSION")).map_err(|error| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(error)))
    })?;
    if args.iter().any(|argument| argument == "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(json_error)?
        );
    } else if let Some(version) = report.version.as_deref() {
        println!("Pasted {version} is available.");
        println!("Open Settings → About in Pasted to install and restart.");
    } else {
        println!("Pasted {} is up to date.", report.current_version);
    }
    Ok(())
}
