use std::process::Command;
use std::path::PathBuf;
use std::fs;
use serde_json::Value;

/// Finds the Java executable to use for launching Minecraft
/// 
/// Tries these locations in order:
/// 1. Bundled Java runtime in .flint/runtime (installed during version setup)
/// 2. System Java (from PATH)
pub fn find_java_executable(flint_dir: &PathBuf, version: &str) -> Result<String, String> {
    // First, try bundled Java in .flint/runtime (in priority order - newest first)
    let bundled_runtimes = vec![
        ("java-runtime-epsilon", "Epsilon runtime - Java 25"),
        ("java-runtime-delta", "Delta runtime - Java 21"),
        ("java-runtime-gamma", "Gamma runtime - Java 17"),
        ("java-runtime-alpha", "Alpha runtime - Java 16"),
        ("jre-legacy", "Legacy JRE - Java 8"),
    ];

    for (runtime_name, _label) in bundled_runtimes {
        let bundled_java = flint_dir.join("runtime").join(runtime_name).join("bin").join("java.exe");
        if bundled_java.exists() {
            eprintln!("[JAVA] ✓ Found bundled Java: {}", bundled_java.display());
            return Ok(bundled_java.to_string_lossy().to_string());
        }
    }

    // Try system Java if bundled isn't available
    eprintln!("[JAVA] No bundled Java found, checking system PATH...");
    if let Ok(output) = std::process::Command::new("where").arg("java.exe").output() {
        if output.status.success() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Some(java_path) = stdout.lines().next() {
                    let trimmed = java_path.trim();
                    eprintln!("[JAVA] ✓ Using system Java: {}", trimmed);
                    return Ok(trimmed.to_string());
                }
            }
        }
    }

    // Try to get Java path from version metadata (fallback for old installs)
    let meta_path = flint_dir.join("versions").join(version).join("flint_meta.json");
    if meta_path.exists() {
        if let Ok(content) = fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<Value>(&content) {
                if let Some(java_path) = meta["javaExe"].as_str() {
                    if PathBuf::from(java_path).exists() {
                        eprintln!("[JAVA] Using saved metadata Java: {}", java_path);
                        return Ok(java_path.to_string());
                    }
                }
            }
        }
    }

    Err("Java executable not found. Please install Java or ensure it is in your PATH.".to_string())
}
