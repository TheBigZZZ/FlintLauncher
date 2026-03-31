use tauri::WindowEvent;
use tauri::Manager;

/// Handle window events, specifically the close request
pub fn handle_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        let window_clone = window.clone();
        
        // Check if keep_launcher_background is enabled by reading the settings file directly
        let should_hide = check_keep_background_setting();
        
        if should_hide {
            api.prevent_close();
            let _ = window_clone.hide();
        }
    }
}

/// Check the keep_launcher_background setting from the settings file
fn check_keep_background_setting() -> bool {
    use std::fs;
    
    // Get APPDATA directory
    let app_data = match std::env::var("APPDATA") {
        Ok(path) => path,
        Err(_) => return false,
    };
    
    let settings_path = format!("{}/.flint/settings.json", app_data);
    
    if !std::path::Path::new(&settings_path).exists() {
        return false;
    }
    
    match fs::read_to_string(&settings_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(settings) => {
                    settings
                        .get("keep_launcher_background")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                }
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

/// Command to show the main window
#[tauri::command]
pub async fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    for window in app.webview_windows().values() {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

/// Command to force quit the app
#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}
