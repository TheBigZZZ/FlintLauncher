use std::path::PathBuf;
use serde_json::Value;

/// Convert Maven coordinates to relative jar path
/// Examples:
/// "net.fabricmc:fabric-loader:0.14.0" -> "net/fabricmc/fabric-loader/0.14.0/fabric-loader-0.14.0.jar"
fn maven_coords_to_path(maven_coords: &str) -> Option<String> {
    let parts: Vec<&str> = maven_coords.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let group_path = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    Some(format!(
        "{}/{}/{}/{}-{}.jar",
        group_path, artifact, version, artifact, version
    ))
}

/// Merges parent and child version JSONs (for inherited versions like Fabric)
pub fn merge_version_json(version_json: &Value, base_dir: &PathBuf) -> Result<Value, String> {
    // Check if this version inherits from another
    if let Some(parent_id) = version_json.get("inheritsFrom").and_then(|v| v.as_str()) {
        // Load parent version JSON
        let parent_path = base_dir
            .join("versions")
            .join(parent_id)
            .join(format!("{}.json", parent_id));
        
        let parent_content = std::fs::read_to_string(&parent_path)
            .map_err(|e| format!("Failed to read parent version JSON {}: {}", parent_id, e))?;
        
        let parent_json: Value = serde_json::from_str(&parent_content)
            .map_err(|e| format!("Failed to parse parent version JSON: {}", e))?;
        
        // Merge: child overrides parent
        let mut merged = parent_json.clone();
        
        // Merge libraries (combine both arrays)
        let mut all_libs = Vec::new();
        if let Some(parent_libs) = parent_json.get("libraries").and_then(|l| l.as_array()) {
            all_libs.extend(parent_libs.clone());
        }
        if let Some(child_libs) = version_json.get("libraries").and_then(|l| l.as_array()) {
            all_libs.extend(child_libs.clone());
        }
        if !all_libs.is_empty() {
            merged["libraries"] = Value::Array(all_libs);
        }
        
        // Override with child values where present
        if let Some(main_class) = version_json.get("mainClass") {
            merged["mainClass"] = main_class.clone();
        }
        if let Some(args) = version_json.get("arguments") {
            merged["arguments"] = args.clone();
        }
        if let Some(id) = version_json.get("id") {
            merged["id"] = id.clone();
        }
        
        return Ok(merged);
    }
    
    // No inheritance, return as-is
    Ok(version_json.clone())
}

/// Builds the Java classpath from version JSON metadata
/// 
/// Collects all library JARs and the main game JAR, joining them with semicolons (Windows)
/// Handles both Minecraft format (downloads.artifact.path) and Fabric format (Maven coordinates)
pub fn build_classpath(
    version_json: &Value,
    libraries_dir: &PathBuf,
    main_jar: &PathBuf,
) -> Result<String, String> {
    let mut jars = Vec::new();

    // Process all libraries from the version JSON
    if let Some(libs) = version_json["libraries"].as_array() {
        eprintln!("[DEBUG] Found {} libraries in version JSON", libs.len());
        for (idx, lib) in libs.iter().enumerate() {
            // Some libraries are conditional (natives, etc.) - skip if they don't apply to this platform
            if lib.get("natives").is_some() && lib.get("natives").and_then(|n| n.get("windows")).is_none() {
                eprintln!("[DEBUG] Lib {}: Skipping (natives for other platform)", idx);
                continue;
            }
            
            let mut jar_path: Option<PathBuf> = None;
            
            // Try Minecraft format first (downloads.artifact.path)
            if let Some(artifact) = lib["downloads"]["artifact"].as_object() {
                if let Some(path_str) = artifact.get("path").and_then(|p| p.as_str()) {
                    jar_path = Some(libraries_dir.join(path_str));
                }
            }
            
            // If not Minecraft format, try Fabric format (Maven coordinates in "name" field)
            if jar_path.is_none() {
                if let Some(name) = lib.get("name").and_then(|n| n.as_str()) {
                    if let Some(rel_path) = maven_coords_to_path(name) {
                        jar_path = Some(libraries_dir.join(&rel_path));
                        eprintln!("[DEBUG] Lib {}: Converted Maven coords '{}' to '{}'", idx, name, rel_path);
                    }
                }
            }
            
            // Check if jar exists and add to classpath
            if let Some(path) = jar_path {
                if let Some(name) = lib.get("name") {
                    if path.exists() {
                        eprintln!("[DEBUG] Lib {}: Found {:?}", idx, name);
                        jars.push(path);
                    } else {
                        eprintln!("[DEBUG] Lib {}: Missing {:?} at {:?}", idx, name, path);
                    }
                }
            }
        }
    } else {
        eprintln!("[DEBUG] No libraries array found in version JSON");
    }

    // Add main game JAR
    if main_jar.exists() {
        eprintln!("[DEBUG] Main JAR found: {}", main_jar.display());
        jars.push(main_jar.clone());
    } else {
        eprintln!("[DEBUG] Main JAR missing: {} (not downloaded)", main_jar.display());
    }

    // Build classpath string (semicolon-separated for Windows)
    let classpath = jars
        .iter()
        .filter_map(|p| p.to_str())
        .collect::<Vec<_>>()
        .join(";");
    
    eprintln!("[DEBUG] Final classpath has {} JARs, total paths: {}", jars.len(), classpath.len());

    Ok(classpath)
}

/// Extracts the asset index from version JSON with detailed debugging
pub fn get_asset_index(version_json: &Value) -> Result<String, String> {
    // Log the structure for debugging
    if let Some(asset_obj) = version_json.get("assetIndex") {
        if let Some(id) = asset_obj.get("id") {
            if let Some(id_str) = id.as_str() {
                return Ok(id_str.to_string());
            }
        }
        // Log what we actually found for debugging
        eprintln!("[DEBUG] assetIndex exists but 'id' is not a string: {:?}", asset_obj);
        return Err(format!("assetIndex.id is not a string: {:?}", asset_obj));
    }
    
    // Check if assetIndex exists at all
    eprintln!("[DEBUG] Version JSON keys: {:?}", version_json.as_object().map(|o| o.keys().collect::<Vec<_>>()));
    eprintln!("[DEBUG] Full assetIndex value: {:?}", version_json.get("assetIndex"));
    
    Err("Missing assetIndex.id".to_string())
}

/// Extracts the main class from version JSON
pub fn get_main_class(version_json: &Value) -> Result<String, String> {
    version_json["mainClass"]
        .as_str()
        .ok_or_else(|| "Missing mainClass".to_string())
        .map(|s| s.to_string())
}
