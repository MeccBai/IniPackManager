use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    collections::HashMap,
    collections::HashSet,
    fs,
    fs::File,
    hash::{Hash, Hasher},
    io::{self, Read},
    path::{Path, PathBuf},
};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
#[cfg(not(windows))]
use tauri_plugin_opener::OpenerExt;
use zip::ZipArchive;

include!("app_types.rs");
include!("app_storage.rs");
include!("app_pack_config.rs");
include!("app_pack_values.rs");
include!("app_pack_apply.rs");
include!("app_preset.rs");
include!("app_commands.rs");
include!("app_remote.rs");

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            list_instances,
            list_presets,
            launch_instance_game,
            finish_startup,
            list_instance_components,
            preview_add_instance,
            pick_instance_folder,
            pick_pack_folder,
            import_pack_zip,
            export_instance_configuration,
            import_instance_configuration,
            import_remote_package,
            list_remote_packages,
            load_pack_definition,
            get_app_settings,
            save_app_settings_command,
            add_instance,
            update_instance,
            delete_instance,
            apply_pack,
            save_instance_component_state,
            set_instance_component_enabled,
            delete_instance_component
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
