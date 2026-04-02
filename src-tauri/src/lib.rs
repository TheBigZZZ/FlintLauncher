mod accounts;
mod launchprocess;
mod libraryManagement;
mod window_manager;
mod updater;

use accounts::{accountcreate, accountdelete, accountget, accountgetcurrent, accountsetcurrent};
use launchprocess::launchprocess;
use updater::check_for_updates;
use libraryManagement::{
    create_profile, delete_profile, delete_version, fetch_available_versions, get_all_profiles,
    get_installed_versions, get_installed_versions_info, get_java_path,
    install_java_component, install_version, is_version_installed, update_profile_last_played,
    update_profile_ram, get_fabric_versions, get_forge_versions, install_fabric_version, install_forge_version, cancel_download,
    load_game_settings, save_game_settings, reset_game_settings,
};
use std::fs;
use std::path::PathBuf;
use std::io::Write;
use tauri::Manager;

/// Get the lock file path for single instance checking
fn get_lock_file_path() -> PathBuf {
    let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(&app_data).join(".flint").join("app.lock")
}

/// Get the signal file path to tell the main instance to show itself
fn get_signal_file_path() -> PathBuf {
    let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(&app_data).join(".flint").join("app.signal")
}

/// Check if a process with the given PID is still running
fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        
        // Use tasklist command to check if process is running
        if let Ok(output) = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid)])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // If the PID appears in tasklist output, the process is running
            return stdout.contains(&pid.to_string());
        }
        false
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // On Unix-like systems, check if /proc/PID exists
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
}

/// Try to acquire the instance lock - returns true if we're the first instance
fn acquire_instance_lock() -> bool {
    let lock_path = get_lock_file_path();
    
    // Ensure the directory exists
    if let Some(parent) = lock_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    
    // Check if lock file exists
    if lock_path.exists() {
        // Try to read the PID from the lock file
        if let Ok(content) = fs::read_to_string(&lock_path) {
            if let Ok(old_pid) = content.trim().parse::<u32>() {
                // Check if the process with that PID is still running
                if is_process_running(old_pid) {
                    // Another instance is still running
                    return false;
                }
            }
        }
        // Old lock file is stale, try to remove it
        let _ = fs::remove_file(&lock_path);
    }
    
    // Try to create our lock file
    match fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&lock_path)
    {
        Ok(mut file) => {
            // Write our PID
            let pid = std::process::id();
            let _ = writeln!(file, "{}", pid);
            true
        }
        Err(_) => {
            // Failed to create lock file
            false
        }
    }
}

/// Signal the existing instance to show itself
fn signal_existing_instance() {
    let signal_path = get_signal_file_path();
    let _ = fs::write(&signal_path, "show");
}

/// Check for and clear the show signal
fn check_show_signal() -> bool {
    let signal_path = get_signal_file_path();
    if signal_path.exists() {
        let _ = fs::remove_file(&signal_path);
        return true;
    }
    false
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Try to acquire the instance lock
    if !acquire_instance_lock() {
        eprintln!("Another instance of Flint Launcher is already running!");
        // Signal the existing instance to show itself
        signal_existing_instance();
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            window_manager::handle_window_event(window, event);
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            // Get the main window and ensure it's visible and focused
            if let Some(window) = app.get_webview_window("main") {
                // Ensure window is visible
                let _ = window.show();
                // Set focus to bring it to front
                let _ = window.set_focus();
                // Unminimize if minimized
                let _ = window.unminimize();
            }
            
            // Set up a timer to check for show signals from other instances
            tauri::async_runtime::spawn(async move {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    
                    if check_show_signal() {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.unminimize();
                        }
                    }
                }
            });
            
            Ok(())
        })
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
            load_game_settings,
            save_game_settings,
            reset_game_settings,
            window_manager::show_main_window,
            window_manager::quit_app,
            check_for_updates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}