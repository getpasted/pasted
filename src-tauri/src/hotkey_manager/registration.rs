use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

#[cfg(target_os = "linux")]
use super::wayland_backend::is_wayland_session;
use super::{
    native_backend_name, AppHotkeyAction, HotkeyManager, HotkeyRegistrationStatus, HotkeySpec,
};
use crate::db::DbState;
use crate::features::{self, Feature};

impl HotkeyManager {
    pub fn register_all(self: &Arc<Self>, app: &AppHandle) -> Result<(), String> {
        let _registration = self.registration_guard.lock();
        let db_opt = app.try_state::<Arc<DbState>>();
        let Some(db) = db_opt else {
            return Err("Database state not initialized".to_string());
        };
        let settings = db.get_all_settings().map_err(|error| error.to_string())?;
        let feature_enabled = |feature: Feature| {
            features::setting_value_is_enabled(
                settings.get(feature.setting_key()).map(String::as_str),
            )
        };

        if !feature_enabled(Feature::Hotkeys) {
            self.clear_registrations(app);
            *self.registration_status.write() = HotkeyRegistrationStatus {
                backend: native_backend_name().to_string(),
                state: "disabled".to_string(),
                configured_count: 0,
                registered_count: 0,
                issues: Vec::new(),
                bindings: Vec::new(),
            };
            let _ = app.emit("hotkey-registration-changed", ());
            return Ok(());
        }

        let get_setting = |key: &str, default_val: &str| -> Option<String> {
            match settings.get(key) {
                Some(s) => {
                    let trimmed = s.trim().to_string();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed)
                    }
                }
                None => {
                    if default_val.trim().is_empty() {
                        None
                    } else {
                        Some(default_val.to_string())
                    }
                }
            }
        };

        let mut specs = Vec::new();
        let mut add_hotkey = |id: String,
                              description: String,
                              setting_str_opt: Option<String>,
                              action: AppHotkeyAction| {
            let Some(setting_str) = setting_str_opt else {
                return;
            };
            specs.push(HotkeySpec {
                id,
                description,
                hotkey: setting_str,
                action,
            });
        };

        if feature_enabled(Feature::Hud) {
            // HUD hotkey (default Option+Shift+V)
            let hud_sc = get_setting("hudHotkey", "Alt+Shift+V");
            add_hotkey(
                "hud".into(),
                "Show or hide the HUD".into(),
                hud_sc,
                AppHotkeyAction::ToggleHud,
            );
        }

        // Main window hotkey
        let main_sc = get_setting("openMainWindowHotkey", "");
        add_hotkey(
            "main-window".into(),
            "Show or hide Pasted".into(),
            main_sc,
            AppHotkeyAction::ToggleMainWindow,
        );

        if feature_enabled(Feature::AppLock) {
            add_hotkey(
                "app-lock".into(),
                "Lock Pasted".into(),
                get_setting("lockAppHotkey", "Alt+Shift+L"),
                AppHotkeyAction::LockApp,
            );
        }

        if feature_enabled(Feature::Transformations) {
            let transformations_sc = get_setting("openTransformationsHotkey", "");
            add_hotkey(
                "transformations".into(),
                "Open Transformations".into(),
                transformations_sc,
                AppHotkeyAction::OpenTransformations,
            );
        }

        if feature_enabled(Feature::Queue) {
            // Sequential Stack toggle (default Option+Shift+C)
            let seq_toggle_sc = get_setting("seqToggleHotkey", "Alt+Shift+C");
            add_hotkey(
                "queue-toggle".into(),
                "Enable or disable the Queue".into(),
                seq_toggle_sc,
                AppHotkeyAction::ToggleCopyQueue,
            );

            // Sequential Stack pop (default Option+Shift+X)
            let seq_pop_sc = get_setting("seqPopHotkey", "Alt+Shift+X");
            add_hotkey(
                "queue-paste-next".into(),
                "Paste the next Queue item".into(),
                seq_pop_sc,
                AppHotkeyAction::PopCopyQueue,
            );
        }

        // Recent clip hotkeys
        for i in 1..=9 {
            let key = format!("pasteClip{}Hotkey", i);
            let sc = get_setting(&key, "");
            add_hotkey(
                format!("paste-clip-{i}"),
                format!("Paste clip {i}"),
                sc,
                AppHotkeyAction::PasteClip(i),
            );
        }

        for (clip_id, shortcut) in db.get_clip_hotkeys().map_err(|error| error.to_string())? {
            add_hotkey(
                format!("clip-{clip_id}"),
                format!("Paste assigned clip #{clip_id}"),
                Some(shortcut),
                AppHotkeyAction::PasteClipById(clip_id),
            );
        }

        if feature_enabled(Feature::Transformations) {
            // Last-Transform hotkeys
            let copy_last_manual_transform_sc = get_setting("copyLastPipelineHotkey", "");
            add_hotkey(
                "copy-last-transform".into(),
                "Copy with the last Advanced Transform".into(),
                copy_last_manual_transform_sc,
                AppHotkeyAction::CopyWithLastManualTransform,
            );
            let paste_last_manual_transform_sc = get_setting("pasteLastPipelineHotkey", "");
            add_hotkey(
                "paste-last-transform".into(),
                "Paste with the last Advanced Transform".into(),
                paste_last_manual_transform_sc,
                AppHotkeyAction::PasteWithLastManualTransform,
            );

            // Per-Transform hotkeys
            for (id, name, hotkey) in
                crate::manual_transform_service::hotkeys(&db).map_err(|error| error.to_string())?
            {
                add_hotkey(
                    format!("transform-{id}"),
                    format!("Run {name}"),
                    Some(hotkey),
                    AppHotkeyAction::PasteWithManualTransform(format!("transform:{id}")),
                );
            }
        }

        if feature_enabled(Feature::Bins) {
            // Bin hotkeys
            for (id, name, hotkey) in db.get_bin_hotkeys().map_err(|error| error.to_string())? {
                add_hotkey(
                    format!("bin-{id}"),
                    format!("Open {name}"),
                    Some(hotkey),
                    AppHotkeyAction::OpenBin(id),
                );
            }
        }

        // Keep the currently working registrations active until the complete
        // replacement snapshot has been read successfully.
        self.clear_registrations(app);

        #[cfg(target_os = "linux")]
        if is_wayland_session() {
            return self.register_wayland_portal(app.clone(), specs);
        }

        #[cfg(target_os = "linux")]
        return self.register_x11(app.clone(), specs);

        #[cfg(not(target_os = "linux"))]
        self.register_native(app, specs)
    }
}
