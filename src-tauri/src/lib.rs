mod accounts;
mod launchprocess;
mod libraryManagement;

use accounts::{accountcreate, accountdelete, accountget, accountgetcurrent, accountsetcurrent};
use launchprocess::launchprocess;
use libraryManagement::{
    create_profile, delete_profile, delete_version, fetch_available_versions, get_all_profiles,
    get_installed_versions, get_installed_versions_info, get_java_path,
    install_java_component, install_version, is_version_installed, update_profile_last_played,
    update_profile_ram, get_fabric_versions, get_forge_versions, install_fabric_version, install_forge_version, cancel_download,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            accountcreate,
            accountget,
            accountgetcurrent,
            accountsetcurrent,
            accountdelete,
            launchprocess,
            fetch_available_versions,
            get_installed_versions,
            get_installed_versions_info,
            is_version_installed,
            delete_version,
            install_version,
            get_java_path,
            install_java_component,
            get_all_profiles,
            create_profile,
            delete_profile,
            update_profile_last_played,
            update_profile_ram,
            get_fabric_versions,
            get_forge_versions,
            install_fabric_version,
            install_forge_version,
            cancel_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}