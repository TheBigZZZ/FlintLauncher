use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use reqwest::Client;
use tokio::fs as tokio_fs;
use std::io::Write;
use sha1::{Sha1, Digest};
use tauri::Manager;
use tauri::Emitter;
use tokio::sync::Semaphore;
use std::sync::Arc;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

// Shared HTTP client for connection pooling
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new())
});

// Global flag for cancelling downloads
static DOWNLOAD_CANCEL_FLAG: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));

fn get_client() -> &'static Client {
    &HTTP_CLIENT
}

fn should_cancel_download() -> bool {
    DOWNLOAD_CANCEL_FLAG.load(Ordering::Relaxed)
}

fn reset_cancel_flag() {
    DOWNLOAD_CANCEL_FLAG.store(false, Ordering::Relaxed);
}

const FLINT_DIR: &str = ".flint";
const MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
const JAVA_MANIFEST_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub version_type: String,
    pub release_time: String,
    pub installed: bool,
}

fn get_flint_dir() -> Result<PathBuf, String> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        Ok(PathBuf::from(appdata).join(FLINT_DIR))
    } else {
        Err("Could not find APPDATA".to_string())
    }
}

fn get_bundled_java_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .resource_dir()
        .ok()
        .map(|p: PathBuf| p.join("java-runtime"))
}

fn copy_bundled_java(component: &str, app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(bundled_dir) = get_bundled_java_dir(app) {
        let src = bundled_dir.join(component);
        if src.exists() {
            let flint_dir = get_flint_dir()?;
            let dest = flint_dir.join("runtime").join(component);
            
            if !dest.exists() {
                copy_dir_all(&src, &dest)
                    .map_err(|e| format!("Failed to copy bundled Java: {}", e))?;
            }
        }
    }
    Ok(())
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn fetch_available_versions() -> Result<Vec<VersionInfo>, String> {
    let client = get_client();
    let response = client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    let manifest: Value = {
        let text = response.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse manifest: {}", e))?
    };

    let installed = get_installed_versions().await.unwrap_or_default();
    let installed_set: std::collections::HashSet<_> = installed.iter().cloned().collect();

    let versions = manifest["versions"]
        .as_array()
        .ok_or("No versions in manifest")?
        .iter()
        .map(|v| VersionInfo {
            id: v["id"].as_str().unwrap_or("").to_string(),
            version_type: v["type"].as_str().unwrap_or("").to_string(),
            release_time: v["releaseTime"].as_str().unwrap_or("").to_string(),
            installed: installed_set.contains(v["id"].as_str().unwrap_or("")),
        })
        .filter(|v| !v.id.is_empty())
        .collect();

    Ok(versions)
}

#[tauri::command]
pub async fn get_installed_versions() -> Result<Vec<String>, String> {
    let flint_dir = get_flint_dir()?;
    let versions_dir = flint_dir.join("versions");

    if !versions_dir.exists() {
        return Ok(vec![]);
    }

    let mut installed = Vec::new();
    let mut entries = tokio_fs::read_dir(&versions_dir)
        .await
        .map_err(|e| e.to_string())?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        if let Some(name) = entry.file_name().into_string().ok() {
            let version_json = versions_dir.join(&name).join(format!("{}.json", name));
            if version_json.exists() {
                installed.push(name);
            }
        }
    }

    Ok(installed)
}

#[tauri::command]
pub async fn get_installed_versions_info() -> Result<Vec<VersionInfo>, String> {
    let installed_ids = get_installed_versions().await?;
    
    // Fetch all versions from manifest to get type and release_time
    let client = get_client();
    let response = client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    let manifest: Value = {
        let text = response.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse manifest: {}", e))?
    };

    let installed_set: std::collections::HashSet<_> = installed_ids.iter().cloned().collect();

    let versions = manifest["versions"]
        .as_array()
        .ok_or("No versions in manifest")?
        .iter()
        .filter(|v| installed_set.contains(v["id"].as_str().unwrap_or("")))
        .map(|v| VersionInfo {
            id: v["id"].as_str().unwrap_or("").to_string(),
            version_type: v["type"].as_str().unwrap_or("").to_string(),
            release_time: v["releaseTime"].as_str().unwrap_or("").to_string(),
            installed: true,
        })
        .collect();

    Ok(versions)
}

#[tauri::command]
pub async fn is_version_installed(version: String) -> Result<bool, String> {
    let flint_dir = get_flint_dir()?;
    let version_dir = flint_dir.join("versions").join(&version);
    let version_json = version_dir.join(format!("{}.json", version));
    let client_jar = version_dir.join(format!("{}.jar", version));

    Ok(version_json.exists() && client_jar.exists())
}

#[tauri::command]
pub async fn delete_version(version: String) -> Result<(), String> {
    let flint_dir = get_flint_dir()?;
    let version_dir = flint_dir.join("versions").join(&version);

    if version_dir.exists() {
        fs::remove_dir_all(&version_dir).map_err(|e| e.to_string())?;
    }

    Ok(())
}

async fn download_file(
    url: &str,
    path: &PathBuf,
    expected_sha1: Option<&str>,
) -> Result<(), String> {
    // Check if file exists and SHA1 is valid
    if path.exists() {
        if let Some(expected) = expected_sha1 {
            let content = tokio_fs::read(path).await.map_err(|e| e.to_string())?;
            let mut hasher = Sha1::new();
            hasher.update(&content);
            let hash = format!("{:x}", hasher.finalize());
            if hash == expected {
                return Ok(());
            }
        }
    }

    // Create parent directory
    if let Some(parent) = path.parent() {
        tokio_fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Download with retries
    for attempt in 0..3 {
        match download_with_retry(url, path).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt == 2 => return Err(format!("Failed after 3 attempts: {}", e)),
            Err(_) => continue,
        }
    }

    Ok(())
}

async fn download_with_retry(url: &str, path: &PathBuf) -> Result<(), String> {
    let client = get_client();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let content = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    file.write_all(&content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

fn extract_natives(jar_path: &PathBuf, natives_dir: &PathBuf) -> Result<(), String> {
    let file = std::fs::File::open(jar_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    fs::create_dir_all(natives_dir).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name();

        if name.starts_with("META-INF") {
            continue;
        }

        let outpath = natives_dir.join(name);

        if name.ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = std::fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// Parallel download task structure
#[derive(Clone)]
struct DownloadTask {
    url: String,
    path: PathBuf,
    sha1: Option<String>,
    name: String, // File name for logging
}

// Download multiple files with 32 concurrent threads
async fn download_files_parallel(tasks: Vec<DownloadTask>, app: tauri::AppHandle) -> Result<(), String> {
    if tasks.is_empty() {
        return Ok(());
    }

    let total = tasks.len();
    let semaphore = Arc::new(Semaphore::new(32)); // 32 concurrent downloads
    let mut join_set = tokio::task::JoinSet::new();
    let downloaded_counter = Arc::new(tokio::sync::Mutex::new(0usize));
    
    for task in tasks {
        // Check if download was cancelled
        if should_cancel_download() {
            let _ = app.emit("download-progress", serde_json::json!({
                "status": "cancelled",
                "message": "Download cancelled by user"
            }));
            return Err("Download cancelled".to_string());
        }

        let permit = semaphore.clone().acquire_owned().await.map_err(|e| e.to_string())?;
        let app_clone = app.clone();
        let counter = downloaded_counter.clone();
        
        join_set.spawn(async move {
            let _guard = permit;
            let filename = task.name.clone();
            
            match download_file(&task.url, &task.path, task.sha1.as_deref()).await {
                Ok(()) => {
                    let mut count = counter.lock().await;
                    *count += 1;
                    let _ = app_clone.emit("download-progress", serde_json::json!({
                        "filename": filename,
                        "status": "completed",
                        "current": *count,
                        "total": total
                    }));
                    Ok(())
                }
                Err(e) => {
                    let _ = app_clone.emit("download-progress", serde_json::json!({
                        "filename": filename,
                        "status": "failed",
                        "error": e.clone()
                    }));
                    Err(e)
                }
            }
        });
    }

    let mut failed = 0;
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(())) => {
                // Success tracked in spawned task
            }
            Ok(Err(_)) => {
                failed += 1;
                // Continue on individual failures
            }
            Err(e) => {
                let _ = app.emit("download-progress", serde_json::json!({
                    "status": "task-error",
                    "error": e.to_string()
                }));
                failed += 1;
            }
        }
    }

    if failed > 0 && failed == total {
        return Err(format!("All {} downloads failed", failed));
    }

    Ok(())
}

#[tauri::command]
pub async fn install_version(
    app: tauri::AppHandle,
    version: String,
) -> Result<String, String> {
    // Reset cancellation flag at start
    reset_cancel_flag();

    let flint_dir = get_flint_dir()?;
    let version_dir = flint_dir.join("versions").join(&version);
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

    // Fetch version manifest
    let client = get_client();
    let manifest_response = client
        .get(MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    let manifest: Value = {
        let text = manifest_response.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse manifest: {}", e))?
    };

    let version_url = manifest["versions"]
        .as_array()
        .ok_or("No versions in manifest")?
        .iter()
        .find(|v| v["id"].as_str() == Some(&version))
        .and_then(|v| v["url"].as_str())
        .ok_or("Version not found")?;

    // Fetch version JSON
    let version_response = client
        .get(version_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version JSON: {}", e))?;

    let vj: Value = {
        let text = version_response.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse version JSON: {}", e))?
    };

    // Save version JSON
    let vj_path = version_dir.join(format!("{}.json", version));
    fs::write(&vj_path, serde_json::to_string_pretty(&vj).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    // Download client JAR
    if let Some(client_obj) = vj["downloads"]["client"].as_object() {
        let client_url = client_obj.get("url").and_then(|u| u.as_str()).ok_or("No client URL")?;
        let client_sha1 = client_obj.get("sha1").and_then(|s| s.as_str());
        let client_jar = version_dir.join(format!("{}.jar", version));
        download_file(client_url, &client_jar, client_sha1).await?;
    }

    // Download libraries
    let libraries_dir = flint_dir.join("libraries");
    let natives_dir = version_dir.join("natives");
    fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;

    let mut library_tasks = Vec::new();

    if let Some(libs) = vj["libraries"].as_array() {
        for lib in libs {
            // Check OS rules
            let mut allowed = true;
            if let Some(rules) = lib["rules"].as_array() {
                allowed = false;
                for rule in rules {
                    if let Some(action) = rule["action"].as_str() {
                        match action {
                            "allow" => {
                                if !rule["os"].is_object() || 
                                   rule["os"]["name"].as_str() == Some("windows") {
                                    allowed = true;
                                }
                            }
                            "disallow" => {
                                if !rule["os"].is_object() || 
                                   rule["os"]["name"].as_str() == Some("windows") {
                                    allowed = false;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            if !allowed {
                continue;
            }

            // Collect artifact downloads
            if let Some(artifact) = lib["downloads"]["artifact"].as_object() {
                let art_url = artifact.get("url")
                    .and_then(|u| u.as_str())
                    .ok_or("No artifact URL")?;
                let art_path = artifact.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or("No artifact path")?;
                let art_sha1 = artifact.get("sha1").and_then(|s| s.as_str()).map(|s| s.to_string());
                
                let full_path = libraries_dir.join(art_path);
                let filename = art_path.split('/').last().unwrap_or("unknown").to_string();
                library_tasks.push(DownloadTask {
                    url: art_url.to_string(),
                    path: full_path,
                    sha1: art_sha1,
                    name: filename,
                });
            }

            // Collect native downloads
            if let Some(classifiers) = lib["downloads"]["classifiers"].as_object() {
                if let Some(native) = classifiers.get("natives-windows") {
                    if let Some(native_obj) = native.as_object() {
                        let nat_url = native_obj.get("url")
                            .and_then(|u| u.as_str())
                            .ok_or("No native URL")?;
                        let nat_path = native_obj.get("path")
                            .and_then(|p| p.as_str())
                            .ok_or("No native path")?;
                        let nat_sha1 = native_obj.get("sha1").and_then(|s| s.as_str()).map(|s| s.to_string());

                        let full_path = libraries_dir.join(nat_path);
                        let filename = nat_path.split('/').last().unwrap_or("unknown").to_string();
                        library_tasks.push(DownloadTask {
                            url: nat_url.to_string(),
                            path: full_path,
                            sha1: nat_sha1,
                            name: filename,
                        });
                    }
                }
            }
        }
    }

    // Download all libraries in parallel
    download_files_parallel(library_tasks, app.clone()).await?;

    // Extract natives after all libraries are downloaded
    if let Some(libs) = vj["libraries"].as_array() {
        for lib in libs {
            if let Some(classifiers) = lib["downloads"]["classifiers"].as_object() {
                if let Some(native) = classifiers.get("natives-windows") {
                    if let Some(native_obj) = native.as_object() {
                        if let Some(nat_path) = native_obj.get("path").and_then(|p| p.as_str()) {
                            let full_path = libraries_dir.join(nat_path);
                            if full_path.exists() {
                                let _ = extract_natives(&full_path, &natives_dir);
                            }
                        }
                    }
                }
            }
        }
    }

    // Download assets
    if let Some(asset_index) = vj["assetIndex"].as_object() {
        let asset_index_id = asset_index.get("id").and_then(|id| id.as_str()).ok_or("No asset ID")?;
        let asset_index_url = asset_index.get("url").and_then(|u| u.as_str()).ok_or("No asset index URL")?;
        
        let asset_index_dir = flint_dir.join("assets").join("indexes");
        fs::create_dir_all(&asset_index_dir).map_err(|e| e.to_string())?;
        let asset_index_path = asset_index_dir.join(format!("{}.json", asset_index_id));
        
        download_file(asset_index_url, &asset_index_path, None).await?;

        let index_content = fs::read_to_string(&asset_index_path).map_err(|e| e.to_string())?;
        let assets: Value = serde_json::from_str(&index_content).map_err(|e| e.to_string())?;

        let objects_dir = flint_dir.join("assets").join("objects");
        let mut asset_tasks = Vec::new();
        
        if let Some(objects) = assets["objects"].as_object() {
            for (_, obj) in objects.iter() {
                if let Some(hash) = obj["hash"].as_str() {
                    let asset_url = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
                    let asset_path = objects_dir.join(&hash[..2]).join(hash);
                    
                    if !asset_path.exists() {
                        asset_tasks.push(DownloadTask {
                            url: asset_url.clone(),
                            path: asset_path,
                            sha1: None,
                            name: hash.to_string(),
                        });
                    }
                }
            }
        }
        
        // Download all assets in parallel
        download_files_parallel(asset_tasks, app.clone()).await?;
    }

    // Create default options.txt
    let game_dir = flint_dir.join("instances").join("default");
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;
    let options_file = game_dir.join("options.txt");
    
    if !options_file.exists() {
        let default_options = "key_key.attack:key.mouse.left\nkey_key.use:key.mouse.right\n";
        fs::write(&options_file, default_options).map_err(|e| e.to_string())?;
    }

    // Install Java
    if let Some(java_info) = vj.get("javaVersion") {
        let component = java_info.get("component").and_then(|c| c.as_str());
        let major = java_info.get("majorVersion").and_then(|m| m.as_u64()).unwrap_or(8) as u32;
        
        if let Some(comp) = component {
            let java_exe = install_java_component(app.clone(), comp.to_string(), major).await?;
            
            // Save metadata
            let meta = json!({
                "javaExe": java_exe,
                "javaVersion": major
            });
            
            let meta_path = version_dir.join("flint_meta.json");
            fs::write(&meta_path, serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(format!("Version {} installed successfully", version))
}

#[tauri::command]
pub async fn install_java_component(app: tauri::AppHandle, component: String, _major_version: u32) -> Result<String, String> {
    let flint_dir = get_flint_dir()?;
    let java_dir = flint_dir.join("runtime").join(&component);
    let java_exe = java_dir.join("bin").join("java.exe");

    // Check if local bundled Java already exists
    if java_exe.exists() {
        return Ok(java_exe.to_string_lossy().to_string());
    }

    // Check if system Java exists (in PATH)
    if let Ok(output) = std::process::Command::new("where").arg("java.exe").output() {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Some(java_path) = stdout.lines().next() {
                    return Ok(java_path.trim().to_string());
                }
            }
        }
    }

    // Try to copy bundled Java first
    if copy_bundled_java(&component, &app).is_ok() && java_exe.exists() {
        return Ok(java_exe.to_string_lossy().to_string());
    }

    // Download Java from internet only if not found
    let client = get_client();
    
    let manifest_response = client
        .get(JAVA_MANIFEST_URL)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Java manifest: {}", e))?;

    let all_runtimes: Value = {
        let text = manifest_response.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse Java manifest: {}", e))?
    };

    let runtime_list = all_runtimes["windows-x64"][&component]
        .as_array()
        .ok_or(format!("No Java component {} found", component))?;

    let runtime = runtime_list
        .last()
        .ok_or("Empty runtime list")?;

    let java_manifest_url = runtime["manifest"]["url"]
        .as_str()
        .ok_or("No manifest URL")?;

    let files_response = get_client()
        .get(java_manifest_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Java files: {}", e))?;

    let java_manifest: Value = {
        let text = files_response.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse Java manifest: {}", e))?
    };

    let files = java_manifest["files"]
        .as_object()
        .ok_or("No files in Java manifest")?;

    fs::create_dir_all(&java_dir).map_err(|e| e.to_string())?;

    for (path, info) in files.iter() {
        let path_str: &str = path;
        if let Some(file_type) = info["type"].as_str() {
            match file_type {
                "directory" => {
                    let dir_path = java_dir.join(path_str.replace("/", "\\"));
                    let _ = fs::create_dir_all(&dir_path);
                }
                "file" => {
                    if info["downloads"]["raw"]["url"].is_string() {
                        if let Some(url) = info["downloads"]["raw"]["url"].as_str() {
                            let file_path = java_dir.join(path_str.replace("/", "\\"));
                            let _ = download_file(url, &file_path, None).await;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(java_exe.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_java_path(version: String) -> Result<String, String> {
    let flint_dir = get_flint_dir()?;
    let meta_path = flint_dir.join("versions").join(&version).join("flint_meta.json");

    if !meta_path.exists() {
        return Err("Version metadata not found".to_string());
    }

    let content = fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let meta: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    meta["javaExe"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or("No Java path found".to_string())
}

#[tauri::command]
pub fn cancel_download() -> Result<(), String> {
    DOWNLOAD_CANCEL_FLAG.store(true, Ordering::Relaxed);
    Ok(())
}

