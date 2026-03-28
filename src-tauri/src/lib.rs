use tauri::Manager;
use std::fs;
use std::path::PathBuf;

fn accounts_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok(PathBuf::from(appdata).join(".flint").join("accounts.json"));
        }
    }

    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("accounts.json"))
}

#[tauri::command]
fn accountcreate(app: tauri::AppHandle, username: String) -> Result<(), String> {
    let trimmed = username.trim().to_string();
    if trimmed.is_empty() {
        return Err("Empty username".into());
    }

    let path = accounts_file_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut accounts: Vec<String> = if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        vec![]
    };

    if accounts.contains(&trimmed) {
        return Err("Username already exists".into());
    }

    accounts.push(trimmed);
    fs::write(&path, serde_json::to_string(&accounts).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn accountget(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let path = accounts_file_path(&app)?;

    if !path.exists() {
        return Ok(vec![]);
    }

    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let accounts: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
    Ok(accounts)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![accountcreate, accountget])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}