use super::*;

#[test]
fn test_parse_shortcut_str_variations() {
    assert!(crate::keyboard_shortcuts::parse("CmdOrCtrl+Shift+V").is_some());
    assert!(crate::keyboard_shortcuts::parse("Control+Alt+C").is_some());
    assert!(crate::keyboard_shortcuts::parse("Ctrl+Alt+KeyC").is_some());
    assert!(crate::keyboard_shortcuts::parse("Alt+Super+KeyV").is_some());
    assert!(crate::keyboard_shortcuts::parse("Option+Cmd+C").is_some());
    assert!(crate::keyboard_shortcuts::parse("Command+Shift+V").is_some());
    assert!(crate::keyboard_shortcuts::parse("Control+Option+C").is_some());
    assert!(crate::keyboard_shortcuts::parse("Control+Option+V").is_some());
    assert!(crate::keyboard_shortcuts::parse("Super+Alt+KeyC").is_some());
    assert!(crate::keyboard_shortcuts::parse("").is_none());
    assert!(crate::keyboard_shortcuts::parse("   ").is_none());

    let sc1 = crate::keyboard_shortcuts::parse("Option+Command+C").unwrap();
    let sc2 = crate::keyboard_shortcuts::parse("Alt+Super+KeyC").unwrap();
    assert_eq!(
        sc1, sc2,
        "Option+Command+C should resolve to identical Shortcut struct as Alt+Super+KeyC"
    );
}

#[test]
fn app_setting_hotkey_keys_are_narrowly_scoped() {
    assert!(is_app_setting_hotkey_key("hudHotkey"));
    assert!(is_app_setting_hotkey_key("lockAppHotkey"));
    assert!(is_app_setting_hotkey_key("smartPasteHotkey"));
    assert!(is_app_setting_hotkey_key("pasteClip1Hotkey"));
    assert!(is_app_setting_hotkey_key("pasteClip9Hotkey"));
    assert!(!is_app_setting_hotkey_key("unlockAppHotkey"));
    assert!(!is_app_setting_hotkey_key("pasteClip0Hotkey"));
    assert!(!is_app_setting_hotkey_key("pasteClip10Hotkey"));
    assert!(!is_app_setting_hotkey_key("enableAppLock"));
}

#[test]
fn unrelated_hotkey_conflicts_do_not_reject_a_change() {
    let issues = vec![crate::hotkey_manager::HotkeyRegistrationIssue {
        hotkey: "Alt+Shift+V".into(),
        description: "HUD".into(),
        message: "Unavailable".into(),
    }];
    assert!(!changed_hotkeys_have_registration_issue(
        &["Alt+Shift+L".into()],
        &issues
    ));
    assert!(changed_hotkeys_have_registration_issue(
        &[" Alt+Shift+V ".into()],
        &issues
    ));
    assert!(!changed_hotkeys_have_registration_issue(
        &[String::new()],
        &issues
    ));
}
