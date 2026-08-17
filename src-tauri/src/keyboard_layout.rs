use tauri_plugin_global_shortcut::Code;

#[cfg(test)]
fn find_code_for_character<T: Copy>(
    character: char,
    candidates: &[(Code, T)],
    mut character_for_candidate: impl FnMut(T) -> Option<char>,
) -> Option<Code> {
    let target = character.to_ascii_lowercase();
    if !target.is_ascii_alphabetic() {
        return None;
    }
    candidates.iter().find_map(|(code, candidate)| {
        character_for_candidate(*candidate)
            .filter(|translated| translated.to_ascii_lowercase() == target)
            .map(|_| *code)
    })
}

#[cfg(target_os = "macos")]
const ANSI_PRINTABLE_CODES: &[(Code, u16)] = &[
    (Code::KeyA, 0x00),
    (Code::KeyB, 0x0b),
    (Code::KeyC, 0x08),
    (Code::KeyD, 0x02),
    (Code::KeyE, 0x0e),
    (Code::KeyF, 0x03),
    (Code::KeyG, 0x05),
    (Code::KeyH, 0x04),
    (Code::KeyI, 0x22),
    (Code::KeyJ, 0x26),
    (Code::KeyK, 0x28),
    (Code::KeyL, 0x25),
    (Code::KeyM, 0x2e),
    (Code::KeyN, 0x2d),
    (Code::KeyO, 0x1f),
    (Code::KeyP, 0x23),
    (Code::KeyQ, 0x0c),
    (Code::KeyR, 0x0f),
    (Code::KeyS, 0x01),
    (Code::KeyT, 0x11),
    (Code::KeyU, 0x20),
    (Code::KeyV, 0x09),
    (Code::KeyW, 0x0d),
    (Code::KeyX, 0x07),
    (Code::KeyY, 0x10),
    (Code::KeyZ, 0x06),
    (Code::Digit1, 0x12),
    (Code::Digit2, 0x13),
    (Code::Digit3, 0x14),
    (Code::Digit4, 0x15),
    (Code::Digit5, 0x17),
    (Code::Digit6, 0x16),
    (Code::Digit7, 0x1a),
    (Code::Digit8, 0x1c),
    (Code::Digit9, 0x19),
    (Code::Digit0, 0x1d),
    (Code::Equal, 0x18),
    (Code::Minus, 0x1b),
    (Code::BracketRight, 0x1e),
    (Code::BracketLeft, 0x21),
    (Code::Quote, 0x27),
    (Code::Semicolon, 0x29),
    (Code::Backslash, 0x2a),
    (Code::Comma, 0x2b),
    (Code::Slash, 0x2c),
    (Code::Period, 0x2f),
    (Code::Backquote, 0x32),
];

#[cfg(target_os = "macos")]
mod macos {
    use super::{Code, ANSI_PRINTABLE_CODES};
    use parking_lot::RwLock;
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::sync::OnceLock;

    #[derive(Default)]
    struct LayoutSnapshot {
        initialized: bool,
        signature: Option<String>,
        codes: HashMap<char, Code>,
        command_codes: HashMap<char, Code>,
        characters: HashMap<Code, char>,
    }

    static ACTIVE_LAYOUT: OnceLock<RwLock<LayoutSnapshot>> = OnceLock::new();
    static LAYOUT_CHANGE_SENDER: OnceLock<std::sync::mpsc::SyncSender<()>> = OnceLock::new();

    fn active_layout() -> &'static RwLock<LayoutSnapshot> {
        ACTIVE_LAYOUT.get_or_init(|| RwLock::new(LayoutSnapshot::default()))
    }

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        static kTISNotifySelectedKeyboardInputSourceChanged: *const c_void;
        static kTISPropertyInputSourceID: *const c_void;
        static kTISPropertyUnicodeKeyLayoutData: *const c_void;
        fn TISCopyCurrentKeyboardLayoutInputSource() -> *const c_void;
        fn TISGetInputSourceProperty(
            input_source: *const c_void,
            property_key: *const c_void,
        ) -> *const c_void;
        fn UCKeyTranslate(
            key_layout_ptr: *const c_void,
            virtual_key_code: u16,
            key_action: u16,
            modifier_key_state: u32,
            keyboard_type: u32,
            key_translate_options: u32,
            dead_key_state: *mut u32,
            max_string_length: u32,
            actual_string_length: *mut u32,
            unicode_string: *mut u16,
        ) -> i32;
        fn LMGetKbdType() -> u8;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFNotificationCenterGetDistributedCenter() -> *const c_void;
        fn CFNotificationCenterAddObserver(
            center: *const c_void,
            observer: *const c_void,
            callback: unsafe extern "C" fn(
                *const c_void,
                *mut c_void,
                *const c_void,
                *const c_void,
                *const c_void,
            ),
            name: *const c_void,
            object: *const c_void,
            suspension_behavior: isize,
        );
        fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
        fn CFStringGetCString(
            string: *const c_void,
            buffer: *mut i8,
            buffer_size: isize,
            encoding: u32,
        ) -> bool;
        fn CFRelease(value: *const c_void);
    }

    unsafe extern "C" fn layout_changed(
        _center: *const c_void,
        _observer: *mut c_void,
        _name: *const c_void,
        _object: *const c_void,
        _user_info: *const c_void,
    ) {
        if let Some(sender) = LAYOUT_CHANGE_SENDER.get() {
            let _ = sender.try_send(());
        }
    }

    pub(super) fn subscribe_to_layout_changes(sender: std::sync::mpsc::SyncSender<()>) -> bool {
        if LAYOUT_CHANGE_SENDER.set(sender).is_err() {
            return false;
        }
        unsafe {
            CFNotificationCenterAddObserver(
                CFNotificationCenterGetDistributedCenter(),
                std::ptr::null(),
                layout_changed,
                kTISNotifySelectedKeyboardInputSourceChanged,
                std::ptr::null(),
                4, // CFNotificationSuspensionBehaviorDeliverImmediately
            );
        }
        true
    }

    fn with_current_layout<T>(read: impl FnOnce(*const c_void, u32) -> T) -> Option<T> {
        unsafe {
            let input_source = TISCopyCurrentKeyboardLayoutInputSource();
            if input_source.is_null() {
                return None;
            }
            let data = TISGetInputSourceProperty(input_source, kTISPropertyUnicodeKeyLayoutData);
            if data.is_null() {
                CFRelease(input_source);
                return None;
            }
            let layout = CFDataGetBytePtr(data).cast::<c_void>();
            if layout.is_null() {
                CFRelease(input_source);
                return None;
            }
            let value = read(layout, LMGetKbdType() as u32);
            CFRelease(input_source);
            Some(value)
        }
    }

    fn character_for_scancode(
        layout: *const c_void,
        keyboard_type: u32,
        scancode: u16,
        modifier_state: u32,
    ) -> Option<char> {
        let mut dead_key_state = 0;
        let mut length = 0;
        let mut output = [0_u16; 4];
        let status = unsafe {
            UCKeyTranslate(
                layout,
                scancode,
                3, // kUCKeyActionDisplay
                modifier_state,
                keyboard_type,
                1, // kUCKeyTranslateNoDeadKeysMask
                &mut dead_key_state,
                output.len() as u32,
                &mut length,
                output.as_mut_ptr(),
            )
        };
        if status != 0 || length == 0 {
            return None;
        }
        char::decode_utf16(output[..length as usize].iter().copied())
            .next()
            .and_then(Result::ok)
    }

    pub(super) fn code_for_character(character: char, command_modifier: bool) -> Option<Code> {
        let layout = active_layout().read();
        let codes = if command_modifier {
            &layout.command_codes
        } else {
            &layout.codes
        };
        codes.get(&character.to_ascii_lowercase()).copied()
    }

    pub(super) fn character_for_code(code: Code) -> Option<char> {
        active_layout().read().characters.get(&code).copied()
    }

    pub(super) fn layout_signature() -> Option<String> {
        unsafe {
            let input_source = TISCopyCurrentKeyboardLayoutInputSource();
            if input_source.is_null() {
                return None;
            }
            let identifier = TISGetInputSourceProperty(input_source, kTISPropertyInputSourceID);
            if identifier.is_null() {
                CFRelease(input_source);
                return None;
            }
            let mut buffer = [0_i8; 256];
            let copied = CFStringGetCString(
                identifier,
                buffer.as_mut_ptr(),
                buffer.len() as isize,
                0x0800_0100, // kCFStringEncodingUTF8
            );
            CFRelease(input_source);
            if !copied {
                return None;
            }
            std::ffi::CStr::from_ptr(buffer.as_ptr())
                .to_str()
                .ok()
                .map(str::to_string)
        }
    }

    /// Refreshes the immutable layout data consumed by hotkey parsing and
    /// recording. Carbon's TIS APIs require the main dispatch queue, so this
    /// function must only be called from Tauri's main thread.
    pub(super) fn refresh_layout_cache() -> bool {
        let signature = layout_signature();
        let characters = with_current_layout(|layout, keyboard_type| {
            ANSI_PRINTABLE_CODES
                .iter()
                .filter_map(|(code, scancode)| {
                    character_for_scancode(layout, keyboard_type, *scancode, 0)
                        .map(|character| (*code, character))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
        let codes: HashMap<char, Code> = characters
            .iter()
            .filter_map(|(code, character)| {
                character
                    .is_ascii_graphic()
                    .then_some((character.to_ascii_lowercase(), *code))
            })
            .collect();
        let command_codes = with_current_layout(|layout, keyboard_type| {
            ANSI_PRINTABLE_CODES
                .iter()
                .filter_map(|(code, scancode)| {
                    // Carbon's modifier state is the high-byte form of cmdKey.
                    character_for_scancode(layout, keyboard_type, *scancode, 1).and_then(
                        |character| {
                            character
                                .is_ascii_graphic()
                                .then_some((character.to_ascii_lowercase(), *code))
                        },
                    )
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_else(|| codes.clone());

        let mut current = active_layout().write();
        let changed = current.initialized
            && (current.signature != signature
                || current.characters != characters
                || current.command_codes != command_codes);
        *current = LayoutSnapshot {
            initialized: true,
            signature,
            codes,
            command_codes,
            characters,
        };
        changed
    }
}

#[cfg_attr(target_os = "linux", allow(dead_code))]
pub fn code_for_character(character: char, command_modifier: bool) -> Option<Code> {
    #[cfg(target_os = "macos")]
    return macos::code_for_character(character, command_modifier);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (character, command_modifier);
        None
    }
}

pub fn logical_key_for_code(code: Code) -> Option<String> {
    #[cfg(target_os = "macos")]
    return macos::character_for_code(code)
        .filter(|character| character.is_ascii_graphic())
        .map(|character| {
            if character.is_ascii_alphabetic() {
                character.to_ascii_uppercase().to_string()
            } else {
                character.to_string()
            }
        });
    #[cfg(not(target_os = "macos"))]
    {
        let _ = code;
        None
    }
}

pub fn start_layout_monitor(app: tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        macos::refresh_layout_cache();
        let (change_sender, change_receiver) = std::sync::mpsc::sync_channel(1);
        if !macos::subscribe_to_layout_changes(change_sender) {
            return;
        }
        std::thread::spawn(move || loop {
            if change_receiver.recv().is_err() {
                return;
            }
            let (refresh_sender, refresh_receiver) = std::sync::mpsc::sync_channel(1);
            if app
                .run_on_main_thread(move || {
                    let _ = refresh_sender.send(macos::refresh_layout_cache());
                })
                .is_err()
            {
                return;
            }
            let Ok(changed) = refresh_receiver.recv_timeout(std::time::Duration::from_secs(2))
            else {
                continue;
            };
            if !changed {
                continue;
            }
            if let Err(error) = crate::commands::register_all_app_shortcuts(&app) {
                eprintln!(
                    "[Pasted Hotkeys] Could not refresh shortcuts after a layout change: {error}"
                );
            }
        });
    }

    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_lookup_uses_the_supplied_layout_translation() {
        let candidates = [(Code::KeyL, 1_u8), (Code::KeyP, 2_u8)];
        let code = find_code_for_character('L', &candidates, |candidate| match candidate {
            1 => Some('n'),
            2 => Some('l'),
            _ => None,
        });
        assert_eq!(code, Some(Code::KeyP));
    }
}
