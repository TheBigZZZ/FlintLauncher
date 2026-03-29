use std::process::Command;
use std::path::PathBuf;
use std::fs;
use serde_json::Value;

#[allow(dead_code)]
fn is_minecraft_running() -> Result<bool, String> {
    // Check if java.exe is running
    let output = Command::new("tasklist")
        .output()
        .map_err(|e| e.to_string())?;
    
    let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    Ok(stdout.to_lowercase().contains("java.exe"))
}

#[tauri::command]
pub fn launchprocess(_app: tauri::AppHandle, version: String) -> Result<(), String> {
    // Check if Minecraft is already running
    if is_minecraft_running()? {
        return Err("Minecraft is already running".to_string());
    }
    
    // Get APPDATA path
    let appdata = std::env::var("APPDATA").map_err(|e| e.to_string())?;
    let mc_dir = PathBuf::from(&appdata).join(".flint");
    
    // Read version JSON
    let version_json_path = mc_dir.join("versions").join(&version).join(format!("{}.json", &version));
    let json_content = fs::read_to_string(&version_json_path).map_err(|e| e.to_string())?;
    let version_json: Value = serde_json::from_str(&json_content).map_err(|e| e.to_string())?;
    
    // Extract asset index and main class
    let asset_index = version_json["assetIndex"]["id"]
        .as_str()
        .ok_or("Missing assetIndex.id")?;
    let main_class = version_json["mainClass"]
        .as_str()
        .ok_or("Missing mainClass")?;
    
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
    
    // Build classpath string (semicolon-separated for Windows)
    let classpath = jars
        .iter()
        .filter_map(|p| p.to_str())
        .collect::<Vec<_>>()
        .join(";");
    
    // Prepare native library path
    let natives_path = mc_dir.join("versions").join(&version).join("natives");
    let java_library_path = format!("-Djava.library.path={}", natives_path.display());
    
    // Run Java
    Command::new("java")
        .arg("-Xmx2G")
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
        .arg("Fynr1x")
        .arg("--userType")
        .arg("legacy")
        .arg("--versionType")
        .arg("release")
        .spawn()
        .map_err(|e| e.to_string())?;
    
    println!("Game launched successfully");
    Ok(())
}