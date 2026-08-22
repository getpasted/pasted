use std::sync::Arc;

use tauri::{AppHandle, Emitter};

use super::{
    AppHotkeyAction, HotkeyManager, HotkeyRegistrationIssue, HotkeyRegistrationStatus, HotkeySpec,
    X11ShortcutTask,
};

impl HotkeyManager {
    #[cfg(target_os = "linux")]
    pub(super) fn register_x11(
        self: &Arc<Self>,
        app: AppHandle,
        specs: Vec<HotkeySpec>,
    ) -> Result<(), String> {
        let (stop_sender, stop_receiver) = std::sync::mpsc::channel();
        let (ready_sender, ready_receiver) = std::sync::mpsc::sync_channel(1);
        let manager = Arc::clone(self);
        let thread = std::thread::spawn(move || {
            let result = run_x11_shortcuts(&manager, &app, &specs, &stop_receiver, &ready_sender);
            if let Err(error) = result {
                let _ = ready_sender.try_send(Err(error.clone()));
                eprintln!("[Pasted Hotkeys] X11 hotkey backend stopped: {error}");
            }
        });
        let result = match ready_receiver.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(result) => result,
            Err(_) => {
                let _ = stop_sender.send(());
                let _ = thread.join();
                return Err("Timed out while registering X11 hotkeys.".to_string());
            }
        };
        if result.is_ok() {
            *self.x11_task.lock() = Some(X11ShortcutTask {
                stop: stop_sender,
                thread,
            });
        } else {
            let _ = stop_sender.send(());
            let _ = thread.join();
        }
        result
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone)]
struct X11RegisteredShortcut {
    keycode: u8,
    modifiers: x11rb::protocol::xproto::ModMask,
    action: AppHotkeyAction,
    pressed: bool,
}

#[cfg(target_os = "linux")]
pub(super) fn run_x11_shortcuts(
    manager: &Arc<HotkeyManager>,
    app: &AppHandle,
    specs: &[HotkeySpec],
    stop: &std::sync::mpsc::Receiver<()>,
    ready: &std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xkb::ConnectionExt as _;
    use x11rb::protocol::{xkb, Event};

    let (connection, screen) = x11rb::connect(None).map_err(|error| error.to_string())?;
    connection
        .xkb_use_extension(1, 0)
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?;
    connection
        .xkb_select_events(
            xkb::ID::USE_CORE_KBD.into(),
            xkb::EventType::default(),
            xkb::EventType::MAP_NOTIFY | xkb::EventType::STATE_NOTIFY,
            xkb::MapPart::KEY_SYMS,
            xkb::MapPart::KEY_SYMS,
            &xkb::SelectEventsAux::new(),
        )
        .map_err(|error| error.to_string())?
        .check()
        .map_err(|error| error.to_string())?;
    let root = connection.setup().roots[screen].root;
    let mut registered = Vec::new();
    let initial = rebuild_x11_shortcuts(manager, app, &connection, root, specs, &mut registered);
    let _ = ready.send(initial.clone());
    initial?;

    let full_mask = x11rb::protocol::xproto::KeyButMask::CONTROL
        | x11rb::protocol::xproto::KeyButMask::SHIFT
        | x11rb::protocol::xproto::KeyButMask::MOD4
        | x11rb::protocol::xproto::KeyButMask::MOD1;
    loop {
        if stop.try_recv().is_ok() {
            unregister_x11_shortcuts(&connection, root, &registered);
            return Ok(());
        }
        while let Some(event) = connection
            .poll_for_event()
            .map_err(|error| error.to_string())?
        {
            match event {
                Event::KeyPress(event) => {
                    let modifiers =
                        x11rb::protocol::xproto::ModMask::from((event.state & full_mask).bits());
                    for shortcut in registered.iter_mut().filter(|shortcut| {
                        shortcut.keycode == event.detail && shortcut.modifiers == modifiers
                    }) {
                        if !shortcut.pressed {
                            shortcut.pressed = true;
                            manager.dispatch_action(app, shortcut.action.clone());
                        }
                    }
                }
                Event::KeyRelease(event) => {
                    for shortcut in registered
                        .iter_mut()
                        .filter(|shortcut| shortcut.keycode == event.detail)
                    {
                        shortcut.pressed = false;
                    }
                }
                Event::XkbMapNotify(_) | Event::XkbNewKeyboardNotify(_) => {
                    if let Err(error) = rebuild_x11_shortcuts(
                        manager,
                        app,
                        &connection,
                        root,
                        specs,
                        &mut registered,
                    ) {
                        eprintln!(
                            "[Pasted Hotkeys] Could not refresh hotkeys after an X11 map change: {error}"
                        );
                    }
                }
                Event::XkbStateNotify(event)
                    if event.changed.contains(
                        xkb::StatePart::GROUP_STATE
                            | xkb::StatePart::GROUP_BASE
                            | xkb::StatePart::GROUP_LATCH
                            | xkb::StatePart::GROUP_LOCK,
                    ) =>
                {
                    if let Err(error) = rebuild_x11_shortcuts(
                        manager,
                        app,
                        &connection,
                        root,
                        specs,
                        &mut registered,
                    ) {
                        eprintln!(
                            "[Pasted Hotkeys] Could not refresh hotkeys after an X11 group change: {error}"
                        );
                    }
                }
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn rebuild_x11_shortcuts<C: x11rb::connection::Connection>(
    manager: &HotkeyManager,
    app: &AppHandle,
    connection: &C,
    root: u32,
    specs: &[HotkeySpec],
    registered: &mut Vec<X11RegisteredShortcut>,
) -> Result<(), String> {
    use x11rb::protocol::xproto::ConnectionExt as _;

    unregister_x11_shortcuts(connection, root, registered);
    registered.clear();
    let mut issues = Vec::new();
    let mut parsed_specs = Vec::new();
    let mut hotkey_counts = HashMap::<String, usize>::new();
    for spec in specs {
        let parsed = parse_x11_shortcut(&spec.hotkey).and_then(|(modifiers, keysym)| {
            resolve_x11_keycode(connection, keysym).map(|keycode| (modifiers, keycode))
        });
        let (modifiers, keycode) = match parsed {
            Ok(parsed) => parsed,
            Err(message) => {
                issues.push(HotkeyRegistrationIssue {
                    hotkey: spec.hotkey.clone(),
                    description: spec.description.clone(),
                    message,
                });
                continue;
            }
        };
        let identity = format!("{:?}:{keycode}", modifiers);
        *hotkey_counts.entry(identity.clone()).or_default() += 1;
        parsed_specs.push((spec, modifiers, keycode, identity));
    }

    for (spec, modifiers, keycode, identity) in parsed_specs {
        if hotkey_counts.get(&identity).copied().unwrap_or_default() > 1 {
            issues.push(HotkeyRegistrationIssue {
                hotkey: spec.hotkey.clone(),
                description: spec.description.clone(),
                message: "This hotkey is assigned to more than one action.".into(),
            });
            continue;
        }
        let mut failure = None;
        for ignored in x11_ignored_modifiers() {
            match connection.grab_key(
                false,
                root,
                modifiers | ignored,
                keycode,
                x11rb::protocol::xproto::GrabMode::ASYNC,
                x11rb::protocol::xproto::GrabMode::ASYNC,
            ) {
                Ok(cookie) => {
                    if let Err(error) = cookie.check() {
                        failure = Some(error.to_string());
                        break;
                    }
                }
                Err(error) => {
                    failure = Some(error.to_string());
                    break;
                }
            }
        }
        if let Some(message) = failure {
            for ignored in x11_ignored_modifiers() {
                let _ = connection.ungrab_key(keycode, root, modifiers | ignored);
            }
            issues.push(HotkeyRegistrationIssue {
                hotkey: spec.hotkey.clone(),
                description: spec.description.clone(),
                message,
            });
        } else {
            registered.push(X11RegisteredShortcut {
                keycode,
                modifiers,
                action: spec.action.clone(),
                pressed: false,
            });
        }
    }
    connection.flush().map_err(|error| error.to_string())?;
    *manager.registration_status.write() = HotkeyRegistrationStatus {
        backend: "x11".into(),
        state: if issues.is_empty() {
            "ready"
        } else {
            "conflict"
        }
        .into(),
        configured_count: specs.len(),
        registered_count: registered.len(),
        issues: issues.clone(),
        bindings: Vec::new(),
    };
    let _ = app.emit("hotkey-registration-changed", ());
    if issues.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} hotkey{} could not be registered",
            issues.len(),
            if issues.len() == 1 { "" } else { "s" }
        ))
    }
}

#[cfg(target_os = "linux")]
fn unregister_x11_shortcuts<C: x11rb::connection::Connection>(
    connection: &C,
    root: u32,
    registered: &[X11RegisteredShortcut],
) {
    use x11rb::protocol::xproto::ConnectionExt as _;
    for shortcut in registered {
        for ignored in x11_ignored_modifiers() {
            let _ = connection.ungrab_key(shortcut.keycode, root, shortcut.modifiers | ignored);
        }
    }
    let _ = connection.flush();
}

#[cfg(target_os = "linux")]
fn x11_ignored_modifiers() -> [x11rb::protocol::xproto::ModMask; 4] {
    use x11rb::protocol::xproto::ModMask;
    [
        ModMask::default(),
        ModMask::LOCK,
        ModMask::M2,
        ModMask::LOCK | ModMask::M2,
    ]
}

#[cfg(target_os = "linux")]
fn parse_x11_shortcut(shortcut: &str) -> Result<(x11rb::protocol::xproto::ModMask, u32), String> {
    use x11rb::protocol::xproto::ModMask;

    let mut parts: Vec<&str> = shortcut.split('+').map(str::trim).collect();
    let key = parts
        .pop()
        .filter(|key| !key.is_empty())
        .ok_or_else(|| "The hotkey has no key.".to_string())?;
    let mut modifiers = ModMask::default();
    for modifier in parts {
        match modifier.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "cmdorctrl" | "commandorcontrol" => {
                modifiers |= ModMask::CONTROL;
            }
            "alt" | "option" => modifiers |= ModMask::M1,
            "shift" => modifiers |= ModMask::SHIFT,
            "cmd" | "command" | "meta" | "super" | "logo" => modifiers |= ModMask::M4,
            _ => return Err(format!("Unknown hotkey modifier: {modifier}")),
        }
    }
    let normalized = key.to_ascii_lowercase();
    let keysym = match normalized.as_str() {
        value if value.chars().count() == 1 => value.chars().next().unwrap() as u32,
        "space" | "spacebar" => 0x20,
        "tab" => 0xff09,
        "enter" | "return" => 0xff0d,
        "escape" | "esc" => 0xff1b,
        "backspace" => 0xff08,
        "delete" => 0xffff,
        "home" => 0xff50,
        "arrowleft" | "left" => 0xff51,
        "arrowup" | "up" => 0xff52,
        "arrowright" | "right" => 0xff53,
        "arrowdown" | "down" => 0xff54,
        "pageup" => 0xff55,
        "pagedown" => 0xff56,
        "end" => 0xff57,
        value if value.starts_with('f') => {
            let number = value[1..]
                .parse::<u32>()
                .map_err(|_| format!("Unknown hotkey key: {key}"))?;
            if !(1..=35).contains(&number) {
                return Err(format!("Unknown hotkey key: {key}"));
            }
            0xffbd + number
        }
        _ => return Err(format!("Unknown hotkey key: {key}")),
    };
    Ok((modifiers, keysym))
}

#[cfg(target_os = "linux")]
fn resolve_x11_keycode<C: x11rb::connection::Connection>(
    connection: &C,
    target_keysym: u32,
) -> Result<u8, String> {
    use x11rb::protocol::xkb;
    use x11rb::protocol::xkb::ConnectionExt as _;

    let device: xkb::DeviceSpec = xkb::ID::USE_CORE_KBD.into();
    let state = connection
        .xkb_get_state(device)
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?;
    let map = connection
        .xkb_get_map(
            device,
            xkb::MapPart::KEY_SYMS,
            xkb::MapPart::default(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            xkb::VMod::default(),
            0,
            0,
            0,
            0,
            0,
            0,
        )
        .map_err(|error| error.to_string())?
        .reply()
        .map_err(|error| error.to_string())?;
    let group = u8::from(state.group) as usize;
    let keymaps = map
        .map
        .syms_rtrn
        .ok_or_else(|| "X11 did not return a keyboard symbol map.".to_string())?;
    for (offset, keymap) in keymaps.iter().enumerate() {
        let width = keymap.width as usize;
        if width == 0 {
            continue;
        }
        let group_count = keymap.syms.len() / width;
        if group_count == 0 {
            continue;
        }
        let active_group = group.min(group_count - 1);
        if keymap.syms.get(active_group * width).copied() == Some(target_keysym) {
            return map
                .first_key_sym
                .checked_add(offset as u8)
                .ok_or_else(|| "The X11 keycode is out of range.".to_string());
        }
    }
    Err(format!(
        "The active X11 layout does not provide keysym 0x{target_keysym:x}."
    ))
}
