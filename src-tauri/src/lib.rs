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

/// Load the app icon from PNG file and convert to RGBA format for the system tray
fn load_app_icon_rgba(icon_path: &std::path::Path) -> Option<Vec<u8>> {
    // Try to load and decode the PNG file using the image crate
    // Use the full crate path to avoid confusion with tauri::image
    if let Ok(img) = ::image::open(icon_path) {
        let rgba_img = img.to_rgba8();
        let rgba_bytes = rgba_img.to_vec();
        return Some(rgba_bytes);
    }
    None
}

/// Create a 32x32 RGBA icon (green background with white F letter) as a fallback
fn create_fallback_icon_rgba() -> Vec<u8> {
    // 32x32 pixels = 1024 pixels, each pixel is 4 bytes (RGBA)
    let mut data = vec![0u8; 32 * 32 * 4];
    
    // Fill with base green color
    for i in (0..data.len()).step_by(4) {
        data[i] = 34;      // R - dark gray
        data[i + 1] = 197; // G - green
        data[i + 2] = 94;  // B - dark green
        data[i + 3] = 255; // A - fully opaque
    }
    
    // Create white F letter
    // Top horizontal line
    for y in 4..8 {
        for x in 8..24 {
            let idx = (y * 32 + x) * 4;
            data[idx] = 255;     // R
            data[idx + 1] = 255; // G
            data[idx + 2] = 255; // B
        }
    }
    
    // Vertical line
    for y in 8..28 {
        for x in 8..12 {
            let idx = (y * 32 + x) * 4;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
        }
    }
    
    // Middle horizontal line
    for y in 14..18 {
        for x in 8..20 {
            let idx = (y * 32 + x) * 4;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
        }
    }
    
    data
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
            
            // Setup system tray icon
            if let Ok(resource_path) = app.path().resource_dir() {
                let icon_path = resource_path.join("icons/32x32.png");
                
                // Try to load the actual icon, fall back to programmatic one if it fails
                let icon_rgba = load_app_icon_rgba(&icon_path)
                    .unwrap_or_else(|| create_fallback_icon_rgba());
                
                // Create tray menu items with explicit IDs
                let show_item = tauri::menu::MenuItem::with_id(app, "show_window", "Show Window", true, None::<String>);
                let quit_item = tauri::menu::MenuItem::with_id(app, "quit_launcher", "Quit Launcher", true, None::<String>);
                
                if let (Ok(show), Ok(quit)) = (show_item, quit_item) {
                    if let Ok(tray_menu) = tauri::menu::Menu::with_items(app, &[&show, &quit]) {
                        let icon = tauri::image::Image::new_owned(icon_rgba, 32, 32);
                        let _tray = tauri::tray::TrayIconBuilder::new()
                            .on_menu_event(|app, event| {
                                match event.id.as_ref() {
                                    "show_window" => {
                                        if let Some(window) = app.get_webview_window("main") {
                                            let _ = window.show();
                                            let _ = window.set_focus();
                                        }
                                    }
                                    "quit_launcher" => {
                                        let _ = app.exit(0);
                                    }
                                    _ => {}
                                }
                            })
                            .icon(icon)
                            .menu(&tray_menu)
                            .build(app);
                    }
                }
            }
            
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