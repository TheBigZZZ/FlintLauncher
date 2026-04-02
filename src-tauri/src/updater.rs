use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Emitter;

const GITHUB_OWNER: &str = "FaizeenHoque";
const GITHUB_REPO: &str = "FlintLauncher";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: Option<String>,
    pub release_name: Option<String>,
    pub release_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    prerelease: bool,
    draft: bool,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Check for updates from GitHub releases
#[tauri::command]
pub async fn check_for_updates(current_version: String) -> Result<UpdateInfo, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        GITHUB_OWNER, GITHUB_REPO
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "flint-launcher")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch releases: {}", e))?;

    // Check for HTTP errors (rate limits, API errors, etc.)
    let status = response.status();
    if !status.is_success() {
        return Ok(UpdateInfo {
            current_version,
            latest_version: "unknown".to_string(),
            update_available: false,
            download_url: None,
            release_name: None,
            release_notes: Some(format!("Failed to check for updates: HTTP {} - may be rate limited", status)),
        });
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release data: {}. Check GitHub API rate limits.", e))?;

    // Skip prerelease and draft versions
    if release.prerelease || release.draft {
        return Ok(UpdateInfo {
            current_version,
            latest_version: release.tag_name.clone(),
            update_available: false,
            download_url: None,
            release_name: None,
            release_notes: None,
        });
    }

    let latest_version = release.tag_name.replace("v", "");
    let current = current_version.split('.').collect::<Vec<&str>>();
    let latest = latest_version.split('.').collect::<Vec<&str>>();

    // Simple version comparison (e.g., "1.2.3" vs "1.2.4")
    let update_available = is_version_newer(&latest, &current);

    // Find the Windows installer asset
    let windows_asset = release.assets.iter().find(|asset| {
        asset.name.contains("x64-setup") || asset.name.contains(".exe")
    });

    Ok(UpdateInfo {
        current_version,
        latest_version,
        update_available,
        download_url: windows_asset.map(|a| a.browser_download_url.clone()),
        release_name: release.name,
        release_notes: release.body,
    })
}

/// Download and install update
#[tauri::command]
pub async fn download_and_install_update(
    app: tauri::AppHandle,
    download_url: String,
) -> Result<String, String> {
    use std::fs;
    use std::io::Write;

    // Get temp directory for download
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join("flint-launcher-update.exe");

    // Download the installer
    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download update: {}", e))?;

    let mut file = fs::File::create(&installer_path)
        .map_err(|e| format!("Failed to create installer file: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    file.write_all(&bytes)
        .map_err(|e| format!("Failed to write installer: {}", e))?;

    // Emit event that update is ready
    let _ = app.emit("update-ready", json!({
        "installer_path": installer_path.to_string_lossy().to_string()
    }));

    Ok(format!(
        "Update downloaded. Please restart the launcher to install."
    ))
}

/// Helper function to compare semantic versions
fn is_version_newer(latest: &[&str], current: &[&str]) -> bool {
    for i in 0..std::cmp::max(latest.len(), current.len()) {
        let latest_part = latest
            .get(i)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let current_part = current
            .get(i)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        if latest_part > current_part {
            return true;
        } else if latest_part < current_part {
            return false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_version_newer(&["1", "2", "3"], &["1", "2", "2"]));
        assert!(is_version_newer(&["1", "3", "0"], &["1", "2", "9"]));
        assert!(!is_version_newer(&["1", "2", "3"], &["1", "2", "3"]));
        assert!(!is_version_newer(&["1", "2", "2"], &["1", "2", "3"]));
    }
}
