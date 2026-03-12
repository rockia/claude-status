use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const BINARY_NAME: &str = "csl";
const SETTINGS_FILE: &str = "settings.json";
const BACKUP_FILE: &str = "statusline-backup.json";
const TMP_DIR: &str = "/tmp/csl";

fn claude_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("Could not determine home directory")?;
    Ok(home.join(".claude"))
}

fn expected_status_line() -> Value {
    serde_json::json!({
        "type": "command",
        "command": "$HOME/.claude/csl"
    })
}

fn read_settings(path: &std::path::Path) -> Result<Value, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid JSON in {}: {e}", path.display()))
}

fn write_settings(path: &std::path::Path, value: &Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Failed to serialize settings: {e}"))?;
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

fn is_already_installed(claude_dir: &std::path::Path) -> bool {
    let binary_path = claude_dir.join(BINARY_NAME);
    let settings_path = claude_dir.join(SETTINGS_FILE);

    if !binary_path.exists() || !settings_path.exists() {
        return false;
    }

    let Ok(settings) = read_settings(&settings_path) else {
        return false;
    };

    settings.get("statusLine") == Some(&expected_status_line())
}

pub fn install() -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Failed to determine current executable path: {e}"))?;

    let claude_dir = claude_dir()?;
    let binary_path = claude_dir.join(BINARY_NAME);
    let settings_path = claude_dir.join(SETTINGS_FILE);
    let backup_path = claude_dir.join(BACKUP_FILE);

    // Check if already installed with correct configuration
    if is_already_installed(&claude_dir) {
        println!("csl is already installed and configured.");
        return Ok(());
    }

    // Create ~/.claude/ if it doesn't exist
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)
            .map_err(|e| format!("Failed to create {}: {e}", claude_dir.display()))?;
    }

    // Copy the running binary to ~/.claude/csl
    fs::copy(&current_exe, &binary_path).map_err(|e| {
        format!(
            "Failed to copy {} to {}: {e}",
            current_exe.display(),
            binary_path.display()
        )
    })?;

    // Read existing settings.json if present
    let mut settings = if settings_path.exists() {
        let parsed = read_settings(&settings_path)?;

        // Back up existing statusLine if present
        if let Some(status_line) = parsed.get("statusLine") {
            let backup = serde_json::json!({ "statusLine": status_line.clone() });
            write_settings(&backup_path, &backup)?;
            println!("Backed up existing statusLine to {}", backup_path.display());
        }

        parsed
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Write the statusLine configuration, preserving other keys
    if let Some(obj) = settings.as_object_mut() {
        obj.insert("statusLine".to_string(), expected_status_line());
    }

    write_settings(&settings_path, &settings)?;

    println!("Installed csl successfully.");
    println!("  Binary: {}", binary_path.display());
    println!("  Settings: {}", settings_path.display());

    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    let claude_dir = claude_dir()?;
    let binary_path = claude_dir.join(BINARY_NAME);
    let settings_path = claude_dir.join(SETTINGS_FILE);
    let backup_path = claude_dir.join(BACKUP_FILE);

    let binary_exists = binary_path.exists();
    let settings_has_status_line = settings_path.exists()
        && read_settings(&settings_path)
            .map(|s| s.get("statusLine").is_some())
            .unwrap_or(false);

    // If nothing is installed, say so and return
    if !binary_exists && !settings_has_status_line && !backup_path.exists() {
        println!("csl is not currently installed. Nothing to do.");
        return Ok(());
    }

    // Restore or remove statusLine from settings.json
    if settings_path.exists() {
        if let Ok(mut settings) = read_settings(&settings_path) {
            if backup_path.exists() {
                // Restore statusLine from backup
                if let Ok(backup) = read_settings(&backup_path) {
                    if let Some(obj) = settings.as_object_mut() {
                        if let Some(backed_up) = backup.get("statusLine") {
                            obj.insert("statusLine".to_string(), backed_up.clone());
                        } else {
                            obj.remove("statusLine");
                        }
                    }
                    println!("Restored previous statusLine from backup.");
                }
                let _ = fs::remove_file(&backup_path);
            } else if let Some(obj) = settings.as_object_mut() {
                obj.remove("statusLine");
            }

            write_settings(&settings_path, &settings)?;
        }
    } else if backup_path.exists() {
        // No settings file but backup exists -- clean it up
        let _ = fs::remove_file(&backup_path);
    }

    // Delete the binary (ignore if it doesn't exist)
    if binary_exists {
        let _ = fs::remove_file(&binary_path);
    }

    // Delete /tmp/csl/ directory (ignore errors)
    let tmp_dir = PathBuf::from(TMP_DIR);
    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    println!("csl has been uninstalled.");

    Ok(())
}
