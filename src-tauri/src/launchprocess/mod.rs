mod accountRetrieval;
mod classpathBuilder;
mod javaDiscovery;
mod gameSpawning;
mod pathManagement;
mod processDetection;

use std::fs;
use std::path::PathBuf;
use serde_json::Value;

use accountRetrieval::get_current_account_with_log;
use classpathBuilder::{build_classpath, get_asset_index, get_main_class, merge_version_json};
use javaDiscovery::find_java_executable;
use gameSpawning::{spawn_minecraft_process, LaunchConfig};
use pathManagement::{emit_log, setup_directories};
use processDetection::is_minecraft_running;

/// Main command for launching Minecraft
/// 
/// Supports two formats:
/// 1. Profile-based launch: `launchprocess(profileName: "MyProfile")`
/// 2. Direct version launch: `launchprocess(version: "1.20.1")`
#[tauri::command]
pub async fn launchprocess(
    app: tauri::AppHandle,
    profileName: Option<String>,
    version: Option<String>,
) -> Result<(), String> {
    // Support both old format (version) and new format (profileName)
    let (actual_version, ram_mb, profile_name_for_log, is_profile) = if let Some(pname) = profileName {
        // New format: profile-based launch
        let profiles = crate::libraryManagement::get_all_profiles().await?;
        let profile = profiles
            .iter()
            .find(|p| p.name == pname)
            .ok_or(format!("Profile '{}' not found", pname))?;

        crate::libraryManagement::update_profile_last_played(pname.clone()).await?;
        (
            profile.base_version.clone(),
            profile.ram_mb,
            Some(pname.clone()),
            true,
        )
    } else if let Some(ver) = version {
        // Old format: direct version launch
        (ver.clone(), 2048, None, false)
    } else {
        return Err("Either profileName or version must be provided".to_string());
    };

    // Emit launch started event
    let profile_info = if let Some(pname) = &profile_name_for_log {
        format!("(Profile: {})", pname)
    } else {
        String::new()
    };
    emit_log(
        &app,
        format!("Starting Minecraft {} {}...", actual_version, profile_info),
    );

    // Check if Minecraft is already running
    if is_minecraft_running()? {
        let msg = "Minecraft is already running";
        emit_log(&app, format!("[ERROR] {}", msg));
        return Err(msg.to_string());
    }

    // Get APPDATA path
    let appdata = std::env::var("APPDATA").map_err(|e| {
        emit_log(&app, format!("[ERROR] Failed to get APPDATA: {}", e));
        e.to_string()
    })?;

    let base_dir = PathBuf::from(&appdata).join(".flint");

    // Setup directory structure
    let dirs = setup_directories(
        base_dir.clone(),
        is_profile,
        profile_name_for_log.as_deref(),
        &actual_version,
    );

    emit_log(&app, format!("Game directory: {}", dirs.mc_dir.display()));

    // Find Java executable
    let java_exe = find_java_executable(&dirs.base_dir, &actual_version).map_err(|e| {
        emit_log(&app, format!("[ERROR] {}", e));
        e
    })?;

    emit_log(&app, format!("Using Java: {}", java_exe));

    // Get current account
    let accounts_path = base_dir.join("accounts.json");
    let username = get_current_account_with_log(&app, &accounts_path)?;

    emit_log(&app, format!("Player: {}", username));

    // Read version JSON
    emit_log(&app, "Building classpath...");

    // Try to find the actual version directory (including Fabric/Forge variants)
    let versions_dir = base_dir.join("versions");
    let mut version_json_path = versions_dir
        .join(&actual_version)
        .join(format!("{}.json", &actual_version));
    
    let mut actual_version_id = actual_version.clone();
    
    // If vanilla version doesn't exist, check for Fabric/Forge versions
    if !version_json_path.exists() {
        emit_log(&app, format!("Vanilla version {} not found, looking for modloader variants...", actual_version));
        
        // Check for Fabric versions (they have names like fabric-loader-X.X.X-1.20.1)
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                if let Ok(filename) = entry.file_name().into_string() {
                    if filename.contains("fabric-loader") && filename.contains(&actual_version) {
                        version_json_path = versions_dir
                            .join(&filename)
                            .join(format!("{}.json", &filename));
                        actual_version_id = filename;
                        emit_log(&app, format!("Found Fabric variant: {}", actual_version_id));
                        break;
                    }
                }
            }
        }
    }

    let json_content = fs::read_to_string(&version_json_path).map_err(|e| {
        emit_log(&app, format!("[ERROR] Failed to read version JSON at {}: {}", version_json_path.display(), e));
        e.to_string()
    })?;

    let version_json: Value = serde_json::from_str(&json_content).map_err(|e| {
        emit_log(&app, format!("[ERROR] Failed to parse version JSON: {}", e));
        e.to_string()
    })?;

    // Log the actual structure for debugging
    if let Some(obj) = version_json.as_object() {
        let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        emit_log(&app, format!("[DEBUG] Version JSON keys: {:?}", keys));
        
        // Check for specific expected fields
        if version_json.get("assetIndex").is_some() {
            emit_log(&app, "[DEBUG] ✓ assetIndex found");
        } else {
            emit_log(&app, "[DEBUG] ✗ MISSING: assetIndex");
        }
        
        if version_json.get("mainClass").is_some() {
            emit_log(&app, "[DEBUG] ✓ mainClass found");
        } else {
            emit_log(&app, "[DEBUG] ✗ MISSING: mainClass");
        }
        
        if version_json.get("libraries").is_some() {
            emit_log(&app, "[DEBUG] ✓ libraries found");
        } else {
            emit_log(&app, "[DEBUG] ✗ MISSING: libraries");
        }
        
        if version_json.get("downloads").is_some() {
            emit_log(&app, "[DEBUG] ✓ downloads found");
        } else {
            emit_log(&app, "[DEBUG] ✗ MISSING: downloads");
        }
    }

    // Merge inherited versions (for Fabric/Forge which inherit from vanilla)
    let original_version = version_json.clone();
    let version_json = match merge_version_json(&version_json, &base_dir) {
        Ok(merged) => {
            emit_log(&app, "[DEBUG] Successfully merged inherited version");
            merged
        },
        Err(e) => {
            emit_log(&app, format!("[WARN] Failed to merge parent version, using current version: {}", e));
            original_version
        }
    };

    // Log merged version structure
    if version_json.get("assetIndex").is_some() {
        emit_log(&app, "[DEBUG] ✓ assetIndex found (after merge if applicable)");
    } else {
        emit_log(&app, "[DEBUG] ✗ assetIndex missing even after merge attempt");
    }
    
    // Debug: log what libraries are in the merged version
    if let Some(libs) = version_json["libraries"].as_array() {
        emit_log(&app, format!("[DEBUG] Merged version has {} libraries", libs.len()));
        // Log first few library names for debugging
        for (idx, lib) in libs.iter().take(5).enumerate() {
            if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                emit_log(&app, format!("[DEBUG]   Lib {}: {}", idx, name));
            }
        }
        if libs.len() > 5 {
            emit_log(&app, format!("[DEBUG]   ... and {} more", libs.len() - 5));
        }
    }

    // Extract metadata from version JSON
    let asset_index = get_asset_index(&version_json).map_err(|e| {
        emit_log(&app, format!("[ERROR] {}", e));
        e
    })?;

    let main_class = get_main_class(&version_json).map_err(|e| {
        emit_log(&app, format!("[ERROR] {}", e));
        e
    })?;

    // Build classpath
    let main_jar = base_dir
        .join("versions")
        .join(&actual_version_id)
        .join(format!("{}.jar", &actual_version_id));

    let classpath = build_classpath(&version_json, &dirs.libraries_dir, &main_jar).map_err(|e| {
        emit_log(&app, format!("[ERROR] {}", e));
        e
    })?;

    emit_log(&app, format!("Loaded {} libraries", count_jars(&classpath)));

    // Prepare native library path
    let java_library_path = format!("-Djava.library.path={}", dirs.natives_dir.display());

    // Launch the game
    let config = LaunchConfig {
        java_exe,
        main_class,
        classpath,
        java_library_path,
        version: actual_version.to_string(),
        username,
        asset_index,
        game_dir: dirs.mc_dir,
        assets_dir: dirs.assets_dir,
        ram_mb,
    };

    spawn_minecraft_process(&app, config).await?;

    Ok(())
}

/// Helper function to count JARs in classpath
fn count_jars(classpath: &str) -> usize {
    classpath.split(';').count()
}
