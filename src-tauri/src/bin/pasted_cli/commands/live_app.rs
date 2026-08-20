use super::{parse_clip_ids, print_live_result, send_live_or_exit};
use rusqlite::Result;

pub(crate) fn run_recording(args: &[String]) -> Result<()> {
    let subcommand = args.get(2).map(String::as_str).unwrap_or("status");
    let action = match subcommand {
        "status" => pasted_lib::live_app::LiveAppAction::ClipboardStatus,
        "pause" => pasted_lib::live_app::LiveAppAction::ClipboardSetPaused { paused: true },
        "resume" => pasted_lib::live_app::LiveAppAction::ClipboardSetPaused { paused: false },
        _ => {
            eprintln!("Usage: pasted recording status|pause|resume [--json]");
            std::process::exit(2);
        }
    };
    let result = send_live_or_exit(action);
    print_live_result(&result, args.iter().any(|argument| argument == "--json"))?;
    Ok(())
}

pub(crate) fn run_queue(args: &[String]) -> Result<()> {
    let subcommand = args.get(2).map(String::as_str).unwrap_or("status");
    let action = match subcommand {
        "status" => pasted_lib::live_app::LiveAppAction::QueueStatus,
        "start" => pasted_lib::live_app::LiveAppAction::QueueStart,
        "stop" => pasted_lib::live_app::LiveAppAction::QueueStop,
        "add" => pasted_lib::live_app::LiveAppAction::QueueAddClips {
            clip_ids: parse_clip_ids(args, 3),
        },
        "remove" => pasted_lib::live_app::LiveAppAction::QueueRemove {
            index: args
                .get(3)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or_else(|| {
                    eprintln!("Usage: pasted queue remove <zero-based-index> [--json]");
                    std::process::exit(2);
                }),
        },
        "order" => pasted_lib::live_app::LiveAppAction::QueueReorder {
            item_ids: args
                .iter()
                .skip(3)
                .filter(|argument| argument.as_str() != "--json")
                .map(|value| value.parse::<u64>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap_or_else(|_| {
                    eprintln!("Every Queue item ID must be an integer.");
                    std::process::exit(2);
                }),
        },
        "paste" => pasted_lib::live_app::LiveAppAction::QueuePaste {
            index: args
                .get(3)
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
        },
        "paste-all" => pasted_lib::live_app::LiveAppAction::QueuePasteAll,
        _ => {
            eprintln!("Usage: pasted queue status|start|stop|add|remove|order|paste|paste-all [arguments] [--json]");
            std::process::exit(2);
        }
    };
    let result = send_live_or_exit(action);
    print_live_result(&result, args.iter().any(|argument| argument == "--json"))?;
    Ok(())
}
