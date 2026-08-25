pub mod analysis_attempt_policy;
pub mod analysis_contract;
pub mod analysis_execution;
pub mod app_event_names;
#[cfg(feature = "gui")]
pub mod app_events;
#[cfg(feature = "gui")]
mod app_exclusions;
pub mod app_lock;
#[cfg(feature = "gui")]
mod app_menu;
#[cfg(feature = "gui")]
mod app_runtime;
#[cfg(feature = "gui")]
mod app_tray;
#[cfg(feature = "gui")]
mod app_windows;
pub mod application_error;
pub mod bin_assignment;
pub mod classification_execution;
#[cfg(feature = "gui")]
pub mod clipboard_actions;
#[cfg(feature = "gui")]
mod clipboard_capture_policy;
mod clipboard_fingerprint;
#[cfg(feature = "gui")]
mod clipboard_image;
#[cfg(feature = "gui")]
mod clipboard_ingestion;
#[cfg(feature = "gui")]
mod clipboard_monitor;
#[cfg(feature = "gui")]
mod commands;
pub mod content_analysis;
pub mod content_classification;
pub mod content_extraction;
pub mod content_inspection;
pub mod content_suggestions;
pub mod content_types;
pub mod db;
pub mod external_import;
mod external_tools;
pub mod extraction_execution;
#[cfg(any(feature = "gui", test))]
mod extraction_reuse;
pub mod extractor_recipe;
pub mod features;
pub mod file_reference_health;
mod filter_engine;
mod hashing;
#[cfg(feature = "gui")]
mod hotkey_manager;
#[cfg(feature = "gui")]
pub mod hud_window;
pub mod inspection_execution;
pub mod installation_diagnostics;
pub mod intelligence_connections;
pub mod intelligence_executor;
mod intelligence_provider;
mod intelligence_scheduler;
#[cfg(feature = "gui")]
mod keyboard_layout;
#[cfg(feature = "gui")]
pub mod keyboard_shortcuts;
pub mod library_items;
pub mod library_storage;
#[cfg(all(feature = "gui", target_os = "linux"))]
mod linux_native_theme;
pub mod live_app;
pub mod localization;
pub mod manual_transform_service;
pub mod ocr;
#[cfg(test)]
mod operation_plugins;
mod operation_registry;
#[cfg(feature = "gui")]
pub mod paste_automation;
#[cfg(feature = "gui")]
mod paste_target;
pub mod platform_capabilities;
pub mod private_browsing;
#[cfg(feature = "gui")]
pub mod queue_actions;
pub mod resource_limits;
#[cfg(feature = "gui")]
mod sequential_paste;
pub mod settings_activity;
pub mod settings_contract;
pub mod settings_service;
pub mod smart_bins;
pub mod storage_protection;
pub mod suggestion_execution;
pub mod third_party_licenses;
#[cfg(feature = "gui")]
mod titlebar;
pub mod transformation_intent;
pub mod transformation_service;

#[cfg(feature = "gui")]
use std::sync::Arc;
#[cfg(feature = "gui")]
use tauri::Manager;

#[cfg(feature = "gui")]
pub fn run() {
    let hotkey_manager = Arc::new(hotkey_manager::HotkeyManager::new());

    tauri::Builder::default()
        .manage(hotkey_manager.clone())
        .on_menu_event(app_menu::handle_menu_event)
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_filter(|label| label == "main")
                .skip_initial_state("main")
                .with_state_flags(app_windows::main_window_state_flags())
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            app_runtime::handle_single_instance(app, &args);
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(mgr) = app.try_state::<Arc<hotkey_manager::HotkeyManager>>() {
                            mgr.dispatch(app, shortcut);
                        } else {
                            eprintln!("HotkeyManager state not found while dispatching a shortcut");
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::AppleScript,
            Some(vec!["--autostart"]),
        ))
        .on_page_load(|webview, payload| {
            if webview.label() == "main"
                && payload.event() == tauri::webview::PageLoadEvent::Finished
            {
                app_windows::mark_main_page_loaded(webview.app_handle());
            }
        })
        .setup(app_runtime::setup)
        .on_window_event(app_windows::handle_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::clips::get_clips,
            commands::clips::get_capture_feedback_clip,
            commands::clips::get_clip_image,
            commands::analysis::analyze_content,
            commands::file_previews::get_file_clip_previews,
            commands::clips::get_trashed_clips,
            commands::clips::restore_clip,
            commands::clips::restore_all_trashed_clips,
            commands::clips::purge_clip_permanently,
            commands::clips::empty_trash,
            commands::activity::get_activity_logs,
            commands::activity::clear_activity_logs,
            commands::activity::export_activity_json,
            commands::activity::export_activity_csv,
            commands::content_registry::get_content_classifiers,
            commands::extractors::get_content_extractors,
            commands::extractors::runtime::get_content_extractor_runtime,
            commands::analysis::get_content_inspectors,
            commands::extractors::choose_extractor_executable,
            commands::extractors::choose_extractor_resource_file,
            commands::extractors::diagnose_content_extractor_recipe,
            commands::extractors::test_content_extractor_recipe,
            commands::extractors::create_content_extractor_recipe,
            commands::extractors::update_content_extractor_recipe,
            commands::extractors::get_extractor_authoring_sessions,
            commands::extractors::duplicate_content_extractor,
            commands::extractors::delete_content_extractor,
            commands::extractors::restore_default_content_extractors,
            commands::content_registry::get_library_items,
            commands::content_registry::set_library_item_enabled,
            commands::content_registry::get_content_types,
            commands::content_registry::get_content_type_groups,
            commands::content_registry::create_content_type_group,
            commands::content_registry::update_content_type_group,
            commands::content_registry::set_content_type_group_archived,
            commands::content_registry::delete_content_type_group,
            commands::content_registry::restore_default_content_type_groups,
            commands::content_registry::content_type_policy::create_content_type,
            commands::content_registry::content_type_policy::update_content_type,
            commands::content_registry::content_type_policy::set_content_type_archived,
            commands::content_registry::content_type_policy::restore_default_content_types,
            commands::content_registry::create_content_classifier,
            commands::content_registry::update_content_classifier,
            commands::content_registry::duplicate_content_classifier,
            commands::content_registry::delete_content_classifier,
            commands::content_registry::restore_default_content_classifiers,
            commands::content_registry::get_clip_content_matches,
            commands::content_registry::rescan_content_classification_history,
            commands::content_registry::rescan_file_format_history,
            commands::content_registry::test_content_classifier,
            commands::platform::play_system_sound,
            commands::platform::quit_app,
            commands::clips::get_clip_collection_summary,
            commands::settings::save_app_setting,
            commands::settings::save_app_settings,
            commands::settings::get_all_app_settings,
            commands::app_lock::get_app_lock_status,
            commands::app_lock::configure_app_lock,
            commands::app_lock::disable_app_lock,
            commands::app_lock::lock_app,
            commands::app_lock::unlock_app,
            commands::app_lock::set_app_lock_system_auth,
            commands::app_lock::set_app_lock_apple_watch,
            commands::app_lock::set_app_lock_idle_minutes,
            commands::app_lock::set_app_lock_lock_on_sleep,
            commands::app_lock::set_app_lock_lock_on_restart,
            commands::app_lock::set_app_lock_capture_while_locked,
            commands::app_lock::reset_app_lock_policy,
            commands::platform::set_linux_native_menu_theme,
            commands::platform::set_overlay_cursor,
            commands::retention::enforce_clip_retention,
            commands::retention::enforce_trash_retention,
            commands::retention::enforce_activity_retention,
            commands::retention::revisions::enforce_revision_retention,
            commands::retention::analysis::enforce_analysis_attempt_retention,
            commands::clips::update_clip_note,
            commands::clip_metadata::update_clip_name,
            commands::search_indexes::get_search_index_status,
            commands::search_indexes::rebuild_search_index,
            commands::clips::delete_clip,
            commands::clips::toggle_pin_clip,
            commands::clips::assign_clip_bin,
            commands::clips::remove_clip_bin,
            commands::clips::reorder_pinned_clips,
            commands::clips::reorder_bin_clips,
            commands::clips::versions::get_clip_versions,
            commands::clips::versions::get_clip_version_count,
            commands::clips::versions::restore_clip_version,
            commands::clips::versions::delete_clip_version,
            commands::get_ocr_backfill_status,
            commands::start_ocr_backfill,
            commands::cancel_ocr_backfill,
            commands::retry_failed_ocr,
            commands::clips::batch_pin_clips,
            commands::clip_policies::batch_protect_clips,
            commands::clip_policies::toggle_clip_concealed,
            commands::clip_policies::batch_conceal_clips,
            commands::clips::batch_trash_clips,
            commands::clips::batch_assign_bin_clips,
            commands::copy_clip_to_system,
            commands::copy_clip_by_id,
            commands::paste_text_to_frontmost,
            commands::get_bins,
            commands::create_bin,
            commands::update_bin,
            commands::delete_bin,
            commands::get_manual_transforms,
            commands::create_manual_transform,
            commands::update_manual_transform,
            commands::update_manual_transform_hotkey,
            commands::delete_manual_transform,
            commands::preview_manual_transform_steps,
            commands::get_operations,
            commands::get_intelligence_connections,
            commands::detect_intelligence_connections,
            commands::create_intelligence_connection,
            commands::update_intelligence_connection,
            commands::delete_intelligence_connection,
            commands::reorder_intelligence_connections,
            commands::reset_intelligence_connections,
            commands::propose_extractor_recipe,
            commands::repair_extractor_recipe,
            commands::plan_transformation_intent,
            commands::test_transformation_plan,
            commands::get_intent_transforms,
            commands::get_transforms,
            commands::save_saved_transform,
            commands::update_saved_transform,
            commands::delete_saved_transform,
            commands::apply_transform_preview_to_clip,
            commands::get_clip_transformation_provenance,
            commands::create_operation,
            commands::update_operation,
            commands::duplicate_operation,
            commands::delete_operation,
            commands::transform_text,
            commands::execute_transformation,
            commands::cancel_transformation_execution,
            commands::get_intelligence_scheduler_snapshot,
            commands::platform::get_installation_diagnostics,
            commands::platform::get_third_party_licenses,
            commands::storage::get_library_location,
            commands::storage::get_storage_protection,
            commands::storage::move_library,
            commands::storage::restore_default_library_location,
            commands::clip_policies::toggle_clip_protected,
            commands::retention::trash_unpinned_clips,
            commands::retention::purge_unpinned_clips,
            commands::queue::start_sequential_paste,
            commands::queue::push_sequential_item,
            commands::queue::pop_sequential_paste,
            commands::queue::paste_sequential_item_by_index,
            commands::queue::remove_sequential_item_by_index,
            commands::queue::reorder_sequential_items,
            commands::queue::stop_sequential_paste,
            commands::queue::paste_all_sequential,
            commands::queue::get_sequential_status,
            commands::queue::get_queue_paste_target,
            commands::toggle_hud_window,
            commands::paste_clip_by_id,
            commands::platform::set_dock_visibility,
            commands::get_source_icons,
            commands::get_installed_applications,
            commands::platform::open_emoji_picker,
            commands::extract_ocr_from_clip,
            commands::get_clip_searchable_text,
            commands::get_clip_extraction_results,
            commands::get_clip_visual_labels,
            commands::add_clip_visual_label,
            commands::remove_clip_visual_label,
            commands::reset_clip_visual_labels,
            commands::get_clip_extraction_history,
            commands::library_access::search_clips,
            commands::extract_text_from_file_clip,
            commands::register_hud_hotkey,
            commands::update_bin_hotkey,
            commands::clip_policies::update_bin_protection,
            commands::clip_policies::update_bin_concealment,
            commands::get_clip_hotkey_assignments,
            commands::update_clip_hotkey,
            commands::get_bin_transform_ref,
            commands::set_bin_transform_ref,
            commands::register_app_setting_hotkey,
            commands::register_app_setting_hotkeys,
            commands::resolve_logical_shortcut_key,
            commands::capture::toggle_clipboard_pause,
            commands::capture::is_clipboard_paused,
            commands::library_access::export_clips_json,
            commands::library_access::export_clips_csv,
            commands::export_backup_file,
            commands::choose_import_file,
            commands::import_inspected_file,
            commands::export_full_backup_file,
            commands::restore_full_backup_file,
            commands::consume_pending_full_restore_client_state,
            commands::get_external_import_sources,
            commands::import_external_history,
            commands::factory_reset_app,
            commands::library_access::get_analytics_summary,
            commands::cli_installation::install_cli_to_path,
            commands::get_hotkey_capability_status,
            commands::request_accessibility_permission,
            commands::platform::open_backing_page,
            commands::platform::perform_titlebar_double_click,
            commands::platform::set_titlebar_direction
        ])
        .build(tauri::generate_context!())
        .expect("error while building Pasted application")
        .run(|app, event| app_runtime::handle_run_event(app, &event));
}
