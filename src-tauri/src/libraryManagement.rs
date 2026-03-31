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

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new())
});

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
pub struct GameProfile {
    pub name: String,
    pub base_version: String,
    pub created_date: String,
    pub last_played: Option<String>,
    pub ram_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub version_type: String,
    pub release_time: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderInfo {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricVersionEntry {
    pub loader: FabricLoaderInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeVersion {
    pub version: String,
    pub latest: bool,
}

#[derive(Clone)]
struct DownloadTask {
    url: String,
    path: PathBuf,
    sha1: Option<String>,
    name: String,
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
    // Try to get bundled Java from app resources first
    if let Some(bundled_dir) = get_bundled_java_dir(app) {
        let src = bundled_dir.join(component);
        eprintln!("[JAVA] Checking bundled Java at: {}", src.display());
        if src.exists() {
            let flint_dir = get_flint_dir()?;
            let dest = flint_dir.join("runtime").join(component);
            if !dest.exists() {
                eprintln!("[JAVA] Copying bundled {} from {} to {}", component, src.display(), dest.display());
                copy_dir_all(&src, &dest)
                    .map_err(|e| format!("Failed to copy bundled Java: {}", e))?;
                eprintln!("[JAVA] ✓ Successfully copied {} to .flint/runtime", component);
            } else {
                eprintln!("[JAVA] Destination already exists: {}", dest.display());
            }
            return Ok(());
        } else {
            eprintln!("[JAVA] ✗ Bundled Java not found at app resources: {}", src.display());
        }
    } else {
        eprintln!("[JAVA] ✗ Could not determine bundled Java directory from app");
    }

    // Fallback: try source resources (for development)
    let source_dir = PathBuf::from("src-tauri/resources/java-runtime").join(component);
    if source_dir.exists() {
        eprintln!("[JAVA] Found Java in source resources: {}", source_dir.display());
        let flint_dir = get_flint_dir()?;
        let dest = flint_dir.join("runtime").join(component);
        if !dest.exists() {
            eprintln!("[JAVA] Copying source {} from {} to {}", component, source_dir.display(), dest.display());
            copy_dir_all(&source_dir, &dest)
                .map_err(|e| format!("Failed to copy Java from source: {}", e))?;
            eprintln!("[JAVA] ✓ Successfully copied {} from source to .flint/runtime", component);
        }
        return Ok(());
    }

    eprintln!("[JAVA] ⚠ Could not find bundled Java {} anywhere, will attempt download", component);
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

fn maven_to_url(base_url: &str, maven: &str) -> (String, String) {
    let parts: Vec<&str> = maven.split(':').collect();
    if parts.len() != 3 {
        return (String::new(), String::new());
    }
    let group_path = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let rel_path = format!(
        "{}/{}/{}/{}-{}.jar",
        group_path, artifact, version, artifact, version
    );
    let base = if base_url.ends_with('/') {
        base_url.to_string()
    } else {
        format!("{}/", base_url)
    };
    let url = format!("{}{}", base, rel_path);
    (url, rel_path)
}

async fn download_file(
    url: &str,
    path: &PathBuf,
    expected_sha1: Option<&str>,
) -> Result<(), String> {
    if path.exists() {
        if let Some(expected) = expected_sha1 {
            let content = tokio_fs::read(path).await.map_err(|e| e.to_string())?;
            let mut hasher = Sha1::new();
            hasher.update(&content);
            let hash = format!("{:x}", hasher.finalize());
            if hash == expected {
                return Ok(());
            }
        } else {
            // No sha1, trust existing file
            return Ok(());
        }
    }

    if let Some(parent) = path.parent() {
        tokio_fs::create_dir_all(parent)
            .await
            .map_err(|e| e.to_string())?;
    }

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

    if !response.status().is_success() {
        return Err(format!("HTTP {} for {}", response.status(), url));
    }

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
        let name = file.name().to_string();

        if name.starts_with("META-INF") {
            continue;
        }

        let outpath = natives_dir.join(&name);

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

async fn download_files_parallel(tasks: Vec<DownloadTask>, app: tauri::AppHandle) -> Result<(), String> {
    if tasks.is_empty() {
        return Ok(());
    }

    let total = tasks.len();
    let semaphore = Arc::new(Semaphore::new(24));
    let mut join_set = tokio::task::JoinSet::new();
    let downloaded_counter = Arc::new(tokio::sync::Mutex::new(0usize));

    for task in tasks {
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

    let mut errors: Vec<String> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        if should_cancel_download() {
            let _ = app.emit("download-progress", serde_json::json!({
                "status": "cancelled",
                "message": "Download cancelled by user"
            }));
            return Err("Download cancelled".to_string());
        }

        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => errors.push(e),
            Err(e) => {
                let _ = app.emit("download-progress", serde_json::json!({
                    "status": "task-error",
                    "error": e.to_string()
                }));
                errors.push(e.to_string());
            }
        }
    }

    if !errors.is_empty() {
        eprintln!("{} download(s) failed:", errors.len());
        for e in &errors {
            eprintln!("  - {}", e);
        }
        // Only hard-fail if everything failed
        if errors.len() == total {
            return Err(format!("All {} downloads failed", total));
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

#[tauri::command]
pub async fn install_version(
    app: tauri::AppHandle,
    version: String,
) -> Result<String, String> {
    reset_cancel_flag();

    let flint_dir = get_flint_dir()?;
    let version_dir = flint_dir.join("versions").join(&version);
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

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

    let version_response = client
        .get(version_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch version JSON: {}", e))?;

    let vj: Value = {
        let text = version_response.text().await.map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse version JSON: {}", e))?
    };

    // Log the structure for debugging
    if let Some(obj) = vj.as_object() {
        let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
        println!("[INSTALL] Downloaded version JSON for {} with keys: {:?}", version, keys);
        
        // Show what was downloaded
        if vj.get("assetIndex").is_some() {
            println!("[INSTALL] ✓ assetIndex: {:?}", vj["assetIndex"]);
        }
        if vj.get("mainClass").is_some() {
            println!("[INSTALL] ✓ mainClass: {}", vj["mainClass"]);
        }
        if let Some(libs) = vj.get("libraries").and_then(|l| l.as_array()) {
            println!("[INSTALL] ✓ libraries: {} entries", libs.len());
        }
    }

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

    download_files_parallel(library_tasks, app.clone()).await?;

    // Extract natives
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
                    let asset_url = format!(
                        "https://resources.download.minecraft.net/{}/{}",
                        &hash[..2], hash
                    );
                    let asset_path = objects_dir.join(&hash[..2]).join(hash);
                    if !asset_path.exists() {
                        asset_tasks.push(DownloadTask {
                            url: asset_url,
                            path: asset_path,
                            sha1: Some(hash.to_string()),
                            name: hash.to_string(),
                        });
                    }
                }
            }
        }

        download_files_parallel(asset_tasks, app.clone()).await?;
    }

    // Create default game dir
    let game_dir = flint_dir.join("instances").join("default");
    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;
    let options_file = game_dir.join("options.txt");
    if !options_file.exists() {
        fs::write(&options_file, "key_key.attack:key.mouse.left\nkey_key.use:key.mouse.right\n")
            .map_err(|e| e.to_string())?;
    }

    // Install Java
    if let Some(java_info) = vj.get("javaVersion") {
        let component = java_info.get("component").and_then(|c| c.as_str());
        let major = java_info.get("majorVersion").and_then(|m| m.as_u64()).unwrap_or(8) as u32;

        if let Some(comp) = component {
            let java_exe = install_java_component(app.clone(), comp.to_string(), major).await?;

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
pub async fn install_java_component(app: tauri::AppHandle, component: String, major_version: u32) -> Result<String, String> {
    let flint_dir = get_flint_dir()?;
    let java_dir = flint_dir.join("runtime").join(&component);
    let java_exe = java_dir.join("bin").join("java.exe");

    eprintln!("[JAVA] install_java_component called for {} (Java {})", component, major_version);

    if java_exe.exists() {
        eprintln!("[JAVA] ✓ Java {} already cached at {}", component, java_exe.display());
        return Ok(java_exe.to_string_lossy().to_string());
    }

    // Try to copy bundled Java FIRST - this is our preference
    eprintln!("[JAVA] Attempting to copy bundled Java {}...", component);
    if copy_bundled_java(&component, &app).is_ok() && java_exe.exists() {
        eprintln!("[JAVA] ✓ Bundled Java {} installed to .flint/runtime", component);
        return Ok(java_exe.to_string_lossy().to_string());
    } else {
        eprintln!("[JAVA] ✗ Bundled Java {} not available from resources", component);
    }

    // Download from Mojang if bundled isn't available
    eprintln!("[JAVA] Downloading {} from Mojang for Java {}...", component, major_version);

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

    let runtime = runtime_list.last().ok_or("Empty runtime list")?;

    let java_manifest_url = runtime["manifest"]["url"]
        .as_str()
        .ok_or("No manifest URL")?;

    eprintln!("[JAVA] Downloading Java manifest from Mojang...");
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

    eprintln!("[JAVA] Downloading {} Java files...", files.len());
    for (path, info) in files.iter() {
        let path_str: &str = path;
        if let Some(file_type) = info["type"].as_str() {
            match file_type {
                "directory" => {
                    let dir_path = java_dir.join(path_str.replace("/", "\\"));
                    let _ = fs::create_dir_all(&dir_path);
                }
                "file" => {
                    if let Some(url) = info["downloads"]["raw"]["url"].as_str() {
                        let file_path = java_dir.join(path_str.replace("/", "\\"));
                        let _ = download_file(url, &file_path, None).await;
                    }
                }
                _ => {}
            }
        }
    }

    eprintln!("[JAVA] ✓ Java {} downloaded and installed to .flint/runtime", component);
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
pub async fn get_all_profiles() -> Result<Vec<GameProfile>, String> {
    let flint_dir = get_flint_dir()?;
    let profiles_file = flint_dir.join("profiles.json");

    if !profiles_file.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&profiles_file).map_err(|e| e.to_string())?;

    match serde_json::from_str::<Vec<GameProfile>>(&content) {
        Ok(profiles) => Ok(profiles),
        Err(_) => {
            match serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                Ok(old_profiles) => {
                    let mut migrated = Vec::new();
                    for profile_json in old_profiles {
                        let profile = GameProfile {
                            name: profile_json.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown")
                                .to_string(),
                            base_version: profile_json.get("base_version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("1.20.1")
                                .to_string(),
                            created_date: profile_json.get("created_date")
                                .and_then(|v| v.as_str())
                                .unwrap_or("2026-01-01 00:00:00")
                                .to_string(),
                            last_played: profile_json.get("last_played")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            ram_mb: profile_json.get("ram_mb")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(2048) as u32,
                        };
                        migrated.push(profile);
                    }
                    let _ = fs::write(
                        &profiles_file,
                        serde_json::to_string_pretty(&migrated).map_err(|e| e.to_string())?
                    );
                    Ok(migrated)
                }
                Err(e) => Err(format!("Failed to parse profiles: {}", e))
            }
        }
    }
}

#[tauri::command]
pub async fn create_profile(
    name: String,
    base_version: String,
) -> Result<GameProfile, String> {
    let flint_dir = get_flint_dir()?;

    if !is_version_installed(base_version.clone()).await? {
        return Err(format!("Version {} not installed", base_version));
    }

    if name.trim().is_empty() {
        return Err("Profile name cannot be empty".to_string());
    }

    let existing_profiles = get_all_profiles().await?;
    if existing_profiles.iter().any(|p| p.name == name) {
        return Err(format!("Profile '{}' already exists", name));
    }

    let profile = GameProfile {
        name: name.clone(),
        base_version: base_version.clone(),
        created_date: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        last_played: None,
        ram_mb: 2048,
    };

    let profile_dir = flint_dir.join("instances").join(&name);
    fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;

    let options_file = profile_dir.join("options.txt");
    if !options_file.exists() {
        fs::write(&options_file, "key_key.attack:key.mouse.left\nkey_key.use:key.mouse.right\n")
            .map_err(|e| e.to_string())?;
    }

    let mut profiles = get_all_profiles().await?;
    profiles.push(profile.clone());
    let profiles_file = flint_dir.join("profiles.json");
    fs::write(
        &profiles_file,
        serde_json::to_string_pretty(&profiles).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;

    Ok(profile)
}

#[tauri::command]
pub async fn delete_profile(name: String) -> Result<(), String> {
    let flint_dir = get_flint_dir()?;

    let profile_dir = flint_dir.join("instances").join(&name);
    if profile_dir.exists() {
        fs::remove_dir_all(&profile_dir).map_err(|e| e.to_string())?;
    }

    let mut profiles = get_all_profiles().await?;
    profiles.retain(|p| p.name != name);
    let profiles_file = flint_dir.join("profiles.json");
    fs::write(
        &profiles_file,
        serde_json::to_string_pretty(&profiles).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_profile_last_played(name: String) -> Result<(), String> {
    let flint_dir = get_flint_dir()?;
    let mut profiles = get_all_profiles().await?;

    for profile in &mut profiles {
        if profile.name == name {
            profile.last_played = Some(chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
            break;
        }
    }

    let profiles_file = flint_dir.join("profiles.json");
    fs::write(
        &profiles_file,
        serde_json::to_string_pretty(&profiles).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn update_profile_ram(name: String, ram_mb: u32) -> Result<(), String> {
    if ram_mb < 512 || ram_mb > 16384 {
        return Err("RAM must be between 512MB and 16GB".to_string());
    }

    let flint_dir = get_flint_dir()?;
    let mut profiles = get_all_profiles().await?;

    for profile in &mut profiles {
        if profile.name == name {
            profile.ram_mb = ram_mb;
            break;
        }
    }

    let profiles_file = flint_dir.join("profiles.json");
    fs::write(
        &profiles_file,
        serde_json::to_string_pretty(&profiles).map_err(|e| e.to_string())?
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn cancel_download() -> Result<(), String> {
    DOWNLOAD_CANCEL_FLAG.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn get_fabric_versions(minecraft_version: String) -> Result<Vec<FabricLoaderInfo>, String> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{}", minecraft_version);

    let response = get_client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Fabric versions: {}", e))?;

    let entries: Vec<FabricVersionEntry> = response
        .json::<Vec<FabricVersionEntry>>()
        .await
        .map_err(|e| format!("Failed to parse Fabric versions: {}", e))?;

    let mut loaders: Vec<FabricLoaderInfo> = entries
        .into_iter()
        .map(|entry| entry.loader)
        .collect();

    loaders.sort_by(|a, b| {
        if a.stable != b.stable {
            b.stable.cmp(&a.stable)
        } else {
            let a_parts: Vec<&str> = a.version.split('.').collect();
            let b_parts: Vec<&str> = b.version.split('.').collect();
            for (ap, bp) in a_parts.iter().zip(b_parts.iter()) {
                if let (Ok(an), Ok(bn)) = (ap.parse::<u32>(), bp.parse::<u32>()) {
                    let cmp = bn.cmp(&an);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                } else {
                    let cmp = bp.cmp(ap);
                    if cmp != std::cmp::Ordering::Equal {
                        return cmp;
                    }
                }
            }
            std::cmp::Ordering::Equal
        }
    });

    Ok(loaders)
}

#[tauri::command]
pub async fn get_forge_versions(minecraft_version: String) -> Result<Vec<ForgeVersion>, String> {
    let url = "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";

    let response = get_client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Forge versions: {}", e))?;

    let xml_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read Forge response: {}", e))?;

    let mut forge_versions: Vec<ForgeVersion> = Vec::new();

    for line in xml_text.lines() {
        if line.contains("<version>") {
            if let Some(start) = line.find("<version>") {
                if let Some(end) = line.find("</version>") {
                    let version_str = &line[start + 9..end];
                    if version_str.starts_with(&format!("{}-", minecraft_version)) {
                        forge_versions.push(ForgeVersion {
                            version: version_str.to_string(),
                            latest: false,
                        });
                    }
                }
            }
        }
    }

    forge_versions.sort_by(|a, b| {
        let a_parts: Vec<&str> = a.version.split('-').collect();
        let b_parts: Vec<&str> = b.version.split('-').collect();
        if a_parts.len() > 1 && b_parts.len() > 1 {
            let a_num: Vec<u32> = a_parts[1].split('.').filter_map(|s| s.parse().ok()).collect();
            let b_num: Vec<u32> = b_parts[1].split('.').filter_map(|s| s.parse().ok()).collect();
            b_num.cmp(&a_num)
        } else {
            b.version.cmp(&a.version)
        }
    });

    if !forge_versions.is_empty() {
        forge_versions[0].latest = true;
    }

    Ok(forge_versions)
}

#[tauri::command]
pub async fn install_fabric_version(
    app: tauri::AppHandle,
    minecraft_version: String,
    fabric_version: String,
) -> Result<(), String> {
    let flint_dir = get_flint_dir()?;

    // Require vanilla to be installed first
    let vanilla_jar = flint_dir
        .join("versions")
        .join(&minecraft_version)
        .join(format!("{}.jar", minecraft_version));

    if !vanilla_jar.exists() {
        return Err(format!(
            "Vanilla version {} must be installed before installing Fabric",
            minecraft_version
        ));
    }

    // Fetch Fabric profile JSON
    let profile_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        minecraft_version, fabric_version
    );

    let client = get_client();
    let profile_resp = client
        .get(&profile_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Fabric profile: {}", e))?;

    if !profile_resp.status().is_success() {
        return Err(format!(
            "Fabric profile fetch failed: HTTP {}",
            profile_resp.status()
        ));
    }

    let profile_json: Value = profile_resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Fabric profile: {}", e))?;

    let profile_id = profile_json["id"]
        .as_str()
        .ok_or("No id in Fabric profile")?
        .to_string();

    // Save version JSON
    let version_dir = flint_dir.join("versions").join(&profile_id);
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

    let version_json_path = version_dir.join(format!("{}.json", profile_id));
    fs::write(
        &version_json_path,
        serde_json::to_string_pretty(&profile_json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    // Copy vanilla jar — Fabric needs a jar named after its profile id
    let fabric_jar = version_dir.join(format!("{}.jar", profile_id));
    if !fabric_jar.exists() {
        fs::copy(&vanilla_jar, &fabric_jar)
            .map_err(|e| format!("Failed to copy vanilla jar for Fabric: {}", e))?;
    }

    // Queue Fabric library downloads
    let libraries_dir = flint_dir.join("libraries");
    let mut tasks: Vec<DownloadTask> = Vec::new();

    if let Some(libs) = profile_json["libraries"].as_array() {
        for lib in libs {
            let name = lib["name"].as_str().unwrap_or("");
            let base_url = lib["url"].as_str().unwrap_or("https://maven.fabricmc.net/");

            if name.is_empty() {
                continue;
            }

            let (download_url, rel_path) = maven_to_url(base_url, name);
            if download_url.is_empty() {
                eprintln!("Skipping bad maven coord: {}", name);
                continue;
            }

            let full_path = libraries_dir.join(&rel_path);
            let sha1 = lib["sha1"].as_str().map(|s| s.to_string());

            // Skip if already valid
            if full_path.exists() {
                if let Some(ref expected) = sha1 {
                    if let Ok(content) = std::fs::read(&full_path) {
                        let mut hasher = Sha1::new();
                        hasher.update(&content);
                        let hash = format!("{:x}", hasher.finalize());
                        if &hash == expected {
                            continue;
                        }
                    }
                } else {
                    continue;
                }
            }

            tasks.push(DownloadTask {
                url: download_url,
                path: full_path,
                sha1,
                name: rel_path.split('/').last().unwrap_or(name).to_string(),
            });
        }
    }

    let _ = app.emit("download-progress", serde_json::json!({
        "status": "starting",
        "message": format!("Downloading {} Fabric libraries...", tasks.len())
    }));

    download_files_parallel(tasks, app.clone()).await?;

    let _ = app.emit("download-progress", serde_json::json!({
        "status": "done",
        "message": format!("Fabric {} for {} installed", fabric_version, minecraft_version)
    }));

    Ok(())
}

#[tauri::command]
pub async fn install_forge_version(_minecraft_version: String, _forge_version: String) -> Result<(), String> {
    Err("Forge installation is not yet implemented. Use Fabric or vanilla for now.".to_string())
}