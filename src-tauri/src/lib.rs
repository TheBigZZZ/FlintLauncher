use tauri::Manager;
use std::fs;
use std::path::PathBuf;
use serde_json::{json, Value};

mod launchprocess;
mod library;
use launchprocess::launchprocess;
use library::{
    fetch_available_versions, get_installed_versions, get_installed_versions_info, is_version_installed, delete_version,
    install_version, get_java_path, install_java_component,
};

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
fn accountcreate(app: tauri::AppHandle, username: String) -> Result<String, String> {
    let trimmed = username.trim().to_string();
    if trimmed.is_empty() {
        return Err("Empty username".into());
    }

    let path = accounts_file_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut data = if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let parsed: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        
        // Handle migration from old array format to new object format
        if parsed.is_array() {
            json!({"accounts": parsed, "current": null})
        } else {
            parsed
        }
    } else {
        json!({"accounts": [], "current": null})
    };

    let accounts = data["accounts"].as_array_mut().ok_or("Invalid data structure")?;

    if accounts.len() >= 6 {
        return Err("Maximum 6 accounts allowed".into());
    }

    if accounts.iter().any(|acc| acc.as_str() == Some(&trimmed)) {
        return Err("Username already exists".into());
    }

    accounts.push(Value::String(trimmed.clone()));
    fs::write(&path, serde_json::to_string(&data).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(trimmed)
}

#[tauri::command]
fn accountget(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let path = accounts_file_path(&app)?;

    if !path.exists() {
        return Ok(vec![]);
    }

    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let parsed: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    
    // Handle migration from old array format to new object format
    let data = if parsed.is_array() {
        json!({"accounts": parsed, "current": null})
    } else {
        parsed
    };
    
    if let Some(accounts) = data["accounts"].as_array() {
        Ok(accounts.iter().filter_map(|v| v.as_str().map(String::from)).collect())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
fn accountgetcurrent(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let path = accounts_file_path(&app)?;

    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let parsed: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    
    // Handle migration from old array format to new object format
    let data = if parsed.is_array() {
        json!({"accounts": parsed, "current": null})
    } else {
        parsed
    };
    
    Ok(data["current"].as_str().map(String::from))
}

#[tauri::command]
fn accountsetcurrent(app: tauri::AppHandle, username: String) -> Result<(), String> {
    let path = accounts_file_path(&app)?;

    let mut data = if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let parsed: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        
        // Handle migration from old array format to new object format
        if parsed.is_array() {
            json!({"accounts": parsed, "current": null})
        } else {
            parsed
        }
    } else {
        json!({"accounts": [], "current": null})
    };

    let accounts = data["accounts"].as_array().ok_or("Invalid data structure")?;
    if !accounts.iter().any(|acc| acc.as_str() == Some(&username)) {
        return Err("Account not found".into());
    }

    data["current"] = Value::String(username);
    fs::write(&path, serde_json::to_string(&data).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn accountdelete(app: tauri::AppHandle, username: String) -> Result<(), String> {
    let path = accounts_file_path(&app)?;

    let mut data = if path.exists() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let parsed: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        
        // Handle migration from old array format to new object format
        if parsed.is_array() {
            json!({"accounts": parsed, "current": null})
        } else {
            parsed
        }
    } else {
        return Err("No accounts to delete".into());
    };

    let current = data["current"].as_str();
    if current == Some(&username) {
        return Err("Cannot delete currently selected account".into());
    }

    let accounts = data["accounts"].as_array_mut().ok_or("Invalid data structure")?;
    accounts.retain(|acc| acc.as_str() != Some(&username));

    fs::write(&path, serde_json::to_string(&data).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    Ok(())
}

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}