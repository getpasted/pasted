use super::*;

#[test]
fn test_hotkey_manager_maps_actions() {
    let mgr = HotkeyManager::new();
    let sc = crate::keyboard_shortcuts::parse("CmdOrCtrl+Shift+V").unwrap();
    mgr.action_map
        .write()
        .insert(sc, AppHotkeyAction::ToggleHud);
    assert_eq!(
        mgr.action_map.read().get(&sc),
        Some(&AppHotkeyAction::ToggleHud)
    );
    mgr.action_map.write().clear();
    assert_eq!(mgr.action_map.read().get(&sc), None);
}

#[test]
fn clip_hotkeys_keep_their_stable_clip_id() {
    let action = AppHotkeyAction::PasteClipById(42);
    assert_eq!(action, AppHotkeyAction::PasteClipById(42));
    assert_ne!(action, AppHotkeyAction::PasteClip(1));
}

#[test]
fn converts_pasted_hotkeys_to_xdg_triggers() {
    assert_eq!(
        shortcut_to_xdg_trigger("CmdOrCtrl+Shift+V"),
        Some("CTRL+SHIFT+v".into())
    );
    assert_eq!(
        shortcut_to_xdg_trigger("Alt+Shift+Space"),
        Some("ALT+SHIFT+space".into())
    );
    assert_eq!(shortcut_to_xdg_trigger("Super+F8"), Some("LOGO+F8".into()));
    assert_eq!(shortcut_to_xdg_trigger("Ctrl+NoSuchKey"), None);
}

#[test]
fn xdg_preflight_rejects_invalid_and_duplicate_hotkeys() {
    let specs = vec![
        HotkeySpec {
            id: "first".into(),
            description: "First".into(),
            hotkey: "Alt+Shift+V".into(),
            action: AppHotkeyAction::ToggleHud,
        },
        HotkeySpec {
            id: "duplicate".into(),
            description: "Duplicate".into(),
            hotkey: "Option+Shift+V".into(),
            action: AppHotkeyAction::ToggleMainWindow,
        },
        HotkeySpec {
            id: "invalid".into(),
            description: "Invalid".into(),
            hotkey: "Alt+NoSuchKey".into(),
            action: AppHotkeyAction::OpenTransformations,
        },
        HotkeySpec {
            id: "valid".into(),
            description: "Valid".into(),
            hotkey: "Control+F8".into(),
            action: AppHotkeyAction::ToggleCopyQueue,
        },
    ];

    let (prepared, issues) = prepare_xdg_hotkeys(specs);
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].0.id, "valid");
    assert_eq!(prepared[0].1, "CTRL+F8");
    assert_eq!(issues.len(), 3);
    assert!(issues
        .iter()
        .any(|issue| issue.message.contains("more than one action")));
    assert!(issues
        .iter()
        .any(|issue| issue.message.contains("could not understand")));
}

#[test]
fn clipboard_hotkey_actions_do_not_overlap() {
    let manager = HotkeyManager::new();
    let first = manager.clipboard_action_guard.try_lock();
    assert!(first.is_some());
    assert!(manager.clipboard_action_guard.try_lock().is_none());
    drop(first);
    assert!(manager.clipboard_action_guard.try_lock().is_some());
}
