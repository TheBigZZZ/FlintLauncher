use std::process::{Command, Stdio};
use std::path::PathBuf;
use std::fs;
use serde_json::Value;
use tauri::Emitter;

#[allow(dead_code)]
fn is_minecraft_running() -> Result<bool, String> {
    // Check if java.exe is running
    let output = Command::new("tasklist")
        .output()
        .map_err(|e| e.to_string())?;
    
    let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    Ok(stdout.to_lowercase().contains("java.exe"))
}

fn find_java_executable(flint_dir: &PathBuf, version: &str) -> Result<String, String> {
    // First, try to get Java path from version metadata
    let meta_path = flint_dir.join("versions").join(version).join("flint_meta.json");
    if meta_path.exists() {
        if let Ok(content) = fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<Value>(&content) {
                if let Some(java_path) = meta["javaExe"].as_str() {
                    if PathBuf::from(java_path).exists() {
                        return Ok(java_path.to_string());
                    }
                }
            }
        }
    }

    // Try bundled Java installations
    let bundled_java = flint_dir.join("runtime").join("java-runtime-gamma").join("bin").join("java.exe");
    if bundled_java.exists() {
        return Ok(bundled_java.to_string_lossy().to_string());
    }

    let bundled_java = flint_dir.join("runtime").join("java-runtime-alpha").join("bin").join("java.exe");
    if bundled_java.exists() {
        return Ok(bundled_java.to_string_lossy().to_string());
    }

    let bundled_java = flint_dir.join("runtime").join("jre-legacy").join("bin").join("java.exe");
    if bundled_java.exists() {
        return Ok(bundled_java.to_string_lossy().to_string());
    }

    // Fall back to system Java (check PATH)
    match Command::new("where").arg("java.exe").output() {
        Ok(output) => {
            let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
            let java_path = stdout.lines().next().ok_or("Java not found in PATH")?;
            Ok(java_path.trim().to_string())
        }
        Err(_) => {
            Err("Java executable not found. Please install Java or ensure it is in your PATH.".to_string())
        }
    }
}

#[tauri::command]
pub async fn launchprocess(app: tauri::AppHandle, version: String) -> Result<(), String> {
    // Emit launch started event
    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": format!("Starting Minecraft {}...", version)
    }));

    // Check if Minecraft is already running
    if is_minecraft_running()? {
        let msg = "Minecraft is already running";
        let _ = app.emit("launch-log", serde_json::json!({
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "message": format!("[ERROR] {}", msg)
        }));
        return Err(msg.to_string());
    }
    
    // Get APPDATA path
    let appdata = std::env::var("APPDATA").map_err(|e| {
        let _ = app.emit("launch-log", serde_json::json!({
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "message": format!("[ERROR] Failed to get APPDATA: {}", e)
        }));
        e.to_string()
    })?;
    let mc_dir = PathBuf::from(&appdata).join(".flint");
    
    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": format!("Game directory: {}", mc_dir.display())
    }));
    
    // Find Java executable
    let java_exe = find_java_executable(&mc_dir, &version).map_err(|e| {
        let _ = app.emit("launch-log", serde_json::json!({
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "message": format!("[ERROR] {}", e)
        }));
        e
    })?;

    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": format!("Using Java: {}", java_exe)
    }));
    
    // Read current account from accounts.json
    let accounts_path = mc_dir.join("accounts.json");
    let username = if accounts_path.exists() {
        let raw = fs::read_to_string(&accounts_path).map_err(|e| {
            let _ = app.emit("launch-log", serde_json::json!({
                "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
                "message": format!("[ERROR] Failed to read accounts: {}", e)
            }));
            e.to_string()
        })?;
        let data: Value = serde_json::from_str(&raw).map_err(|e| {
            let _ = app.emit("launch-log", serde_json::json!({
                "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
                "message": format!("[ERROR] Failed to parse accounts: {}", e)
            }));
            e.to_string()
        })?;
        
        // Handle migration from old array format to new object format
        let data = if data.is_array() {
            serde_json::json!({"accounts": data, "current": null})
        } else {
            data
        };
        
        data["current"]
            .as_str()
            .ok_or_else(|| {
                let msg = "No account selected";
                let _ = app.emit("launch-log", serde_json::json!({
                    "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
                    "message": format!("[ERROR] {}", msg)
                }));
                msg.to_string()
            })?
            .to_string()
    } else {
        let msg = "No accounts found";
        let _ = app.emit("launch-log", serde_json::json!({
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "message": format!("[ERROR] {}", msg)
        }));
        return Err(msg.to_string());
    };
    
    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": format!("Player: {}", username)
    }));
    
    // Read version JSON
    let version_json_path = mc_dir.join("versions").join(&version).join(format!("{}.json", &version));
    let json_content = fs::read_to_string(&version_json_path).map_err(|e| {
        let _ = app.emit("launch-log", serde_json::json!({
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "message": format!("[ERROR] Failed to read version JSON: {}", e)
        }));
        e.to_string()
    })?;
    let version_json: Value = serde_json::from_str(&json_content).map_err(|e| {
        let _ = app.emit("launch-log", serde_json::json!({
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "message": format!("[ERROR] Failed to parse version JSON: {}", e)
        }));
        e.to_string()
    })?;
    
    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": "Building classpath..."
    }));
    
    // Extract asset index and main class
    let asset_index = version_json["assetIndex"]["id"]
        .as_str()
        .ok_or_else(|| {
            let _ = app.emit("launch-log", serde_json::json!({
                "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
                "message": "[ERROR] Missing assetIndex.id"
            }));
            "Missing assetIndex.id".to_string()
        })?;
    let main_class = version_json["mainClass"]
        .as_str()
        .ok_or_else(|| {
            let _ = app.emit("launch-log", serde_json::json!({
                "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
                "message": "[ERROR] Missing mainClass"
            }));
            "Missing mainClass".to_string()
        })?;
    
    // Build classpath
    let assets_dir = mc_dir.join("assets");
    let libraries_dir = mc_dir.join("libraries");
    let mut jars = Vec::new();
    
    if let Some(libs) = version_json["libraries"].as_array() {
        for lib in libs {
            if let Some(artifact) = lib["downloads"]["artifact"].as_object() {
                if let Some(path_str) = artifact.get("path").and_then(|p| p.as_str()) {
                    let jar_path = libraries_dir.join(path_str);
                    if jar_path.exists() {
                        jars.push(jar_path);
                    }
                }
            }
        }
    }
    
    // Add main jar
    let main_jar = mc_dir.join("versions").join(&version).join(format!("{}.jar", &version));
    jars.push(main_jar);
    
    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": format!("Loaded {} libraries", jars.len())
    }));
    
    // Build classpath string (semicolon-separated for Windows)
    let classpath = jars
        .iter()
        .filter_map(|p| p.to_str())
        .collect::<Vec<_>>()
        .join(";");
    
    // Prepare native library path
    let natives_path = mc_dir.join("versions").join(&version).join("natives");
    let java_library_path = format!("-Djava.library.path={}", natives_path.display());
    
    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": "Launching game..."
    }));
    
    // Spawn Java process with visible console window
    let mut cmd = Command::new(&java_exe);
    cmd.arg("-Xmx2G")
        .arg("-Xms1G")
        .arg(&java_library_path)
        .arg(format!("-cp"))
        .arg(&classpath)
        .arg(main_class)
        .arg("--version")
        .arg(&version)
        .arg("--gameDir")
        .arg(mc_dir.to_str().ok_or("Invalid path")?)
        .arg("--assetsDir")
        .arg(assets_dir.to_str().ok_or("Invalid path")?)
        .arg("--assetIndex")
        .arg(asset_index)
        .arg("--uuid")
        .arg("00000000-0000-0000-0000-000000000000")
        .arg("--accessToken")
        .arg("0")
        .arg("--enable-native-access=ALL-UNNAMED")
        .arg("--username")
        .arg(&username)
        .arg("--userType")
        .arg("legacy")
        .arg("--versionType")
        .arg("release")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    
    let mut child = cmd.spawn().map_err(|e| {
        let msg = format!("Failed to launch game: {}", e);
        let _ = app.emit("launch-log", serde_json::json!({
            "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
            "message": format!("[ERROR] {}", msg)
        }));
        msg
    })?;
    
    let _ = app.emit("launch-log", serde_json::json!({
        "timestamp": chrono::Local::now().format("%H:%M:%S").to_string(),
        "message": format!("Game launched with PID: {} - Terminal window will stay open", child.id())
    }));
    
    Ok(())
}