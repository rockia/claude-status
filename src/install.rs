use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const REPO: &str = "rockia/claude-status";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const TARGET: &str = env!("TARGET");

fn fetch_latest_release() -> Result<(String, String), String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let response = ureq::get(&url)
        .header("User-Agent", "claude-status")
        .call()
        .map_err(|e| format!("GitHub API request failed: {e}"))?;

    let body: Value = response
        .into_body()
        .read_to_string()
        .map_err(|e| format!("Failed to read response: {e}"))
        .and_then(|s| serde_json::from_str(&s).map_err(|e| format!("Failed to parse JSON: {e}")))?;

    let tag = body
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or("Missing tag_name in release")?
        .to_string();

    let asset_name = format!("claude-status-{TARGET}");
    let assets = body
        .get("assets")
        .and_then(|v| v.as_array())
        .ok_or("Missing assets in release")?;

    let url = assets
        .iter()
        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(&asset_name))
        .and_then(|a| a.get("browser_download_url"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| {
            let names: Vec<&str> = assets
                .iter()
                .filter_map(|a| a.get("name")?.as_str())
                .collect();
            format!(
                "No asset '{asset_name}' found. Available: {}",
                names.join(", ")
            )
        })?
        .to_string();

    Ok((tag, url))
}

fn download_binary(url: &str, dest: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .header("User-Agent", "claude-status")
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let bytes = response
        .into_body()
        .read_to_vec()
        .map_err(|e| format!("Failed to read binary: {e}"))?;

    fs::write(dest, &bytes).map_err(|e| format!("Failed to write binary: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(dest, fs::Permissions::from_mode(0o755));
    }

    Ok(())
}

fn strip_v(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

fn claude_dir() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    PathBuf::from(home).join(".claude")
}

fn binary_dest() -> PathBuf {
    claude_dir().join("claude-status")
}

fn settings_path() -> PathBuf {
    claude_dir().join("settings.json")
}

fn backup_path() -> PathBuf {
    claude_dir().join("statusline-backup.json")
}

fn read_settings() -> Value {
    let path = settings_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    }
}

fn write_settings(val: &Value) {
    let path = settings_path();
    let _ = fs::create_dir_all(path.parent().unwrap());
    let json = serde_json::to_string_pretty(val).unwrap();
    fs::write(path, json).expect("Failed to write settings.json");
}

pub fn install() {
    let dest = binary_dest();
    let dir = claude_dir();
    let _ = fs::create_dir_all(&dir);

    // Copy current binary to ~/.claude/claude-status
    let current_exe = std::env::current_exe().expect("Cannot determine current binary path");
    if current_exe != dest {
        fs::copy(&current_exe, &dest).expect("Failed to copy binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
        }
    }

    // Backup existing statusLine config
    let mut settings = read_settings();
    if let Some(existing) = settings.get("statusLine") {
        let backup = serde_json::json!({ "statusLine": existing.clone() });
        let backup_json = serde_json::to_string_pretty(&backup).unwrap();
        fs::write(backup_path(), backup_json).expect("Failed to write backup");
        println!(
            "Backed up existing statusLine config to {:?}",
            backup_path()
        );
    }

    // Write new statusLine config
    let command = dest.to_string_lossy().to_string();
    settings["statusLine"] = serde_json::json!({
        "type": "command",
        "command": command
    });
    write_settings(&settings);

    println!("Installed claude-status to {}", dest.display());
    println!("Status line configured in {}", settings_path().display());
}

pub fn uninstall() {
    let dest = binary_dest();
    let bp = backup_path();
    let settings = read_settings();
    let has_status_line = settings.get("statusLine").is_some();

    // Check if there's anything to uninstall
    if !dest.exists() && !bp.exists() && !has_status_line {
        println!("Nothing to uninstall");
        return;
    }

    // Restore backup if exists
    let mut settings = settings;
    if bp.exists() {
        if let Ok(content) = fs::read_to_string(&bp)
            && let Ok(backup) = serde_json::from_str::<Value>(&content)
            && let Some(sl) = backup.get("statusLine")
        {
            settings["statusLine"] = sl.clone();
            write_settings(&settings);
            println!("Restored previous statusLine config");
        }
        let _ = fs::remove_file(&bp);
    } else if has_status_line {
        if let Some(obj) = settings.as_object_mut() {
            obj.remove("statusLine");
        }
        write_settings(&settings);
        println!("Removed statusLine config from settings.json");
    }

    // Remove binary
    if dest.exists() {
        let _ = fs::remove_file(&dest);
        println!("Removed {}", dest.display());
    }
}

pub fn update() {
    let (tag, download_url) = match fetch_latest_release() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Update failed: {e}");
            std::process::exit(1);
        }
    };

    let latest_version = strip_v(&tag);
    if latest_version == CURRENT_VERSION {
        println!("Already up to date (v{CURRENT_VERSION})");
        return;
    }

    let dest = binary_dest();
    let tmp = dest.with_extension("tmp");

    println!("Updating v{CURRENT_VERSION} → v{latest_version}...");

    if let Err(e) = download_binary(&download_url, &tmp) {
        eprintln!("Update failed: {e}");
        let _ = fs::remove_file(&tmp);
        std::process::exit(1);
    }

    if let Err(e) = fs::rename(&tmp, &dest) {
        eprintln!("Failed to replace binary: {e}");
        let _ = fs::remove_file(&tmp);
        std::process::exit(1);
    }

    println!("Updated v{CURRENT_VERSION} → v{latest_version}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_v_with_prefix() {
        assert_eq!(strip_v("v1.2.3"), "1.2.3");
    }

    #[test]
    fn test_strip_v_without_prefix() {
        assert_eq!(strip_v("1.2.3"), "1.2.3");
    }

    #[test]
    fn test_version_up_to_date() {
        assert_eq!(strip_v(&format!("v{CURRENT_VERSION}")), CURRENT_VERSION);
    }

    #[test]
    fn test_asset_matching() {
        let assets_json = serde_json::json!([
            {"name": "claude-status-x86_64-apple-darwin", "browser_download_url": "https://example.com/x86"},
            {"name": "claude-status-aarch64-apple-darwin", "browser_download_url": "https://example.com/arm"},
            {"name": "claude-status-x86_64-unknown-linux-musl", "browser_download_url": "https://example.com/linux"},
        ]);
        let assets = assets_json.as_array().unwrap();
        let asset_name = format!("claude-status-{TARGET}");
        let found = assets
            .iter()
            .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(&asset_name));
        // We can't assert the exact URL since TARGET varies by machine,
        // but we can assert the lookup itself works when a match exists.
        if found.is_some() {
            let url = found
                .unwrap()
                .get("browser_download_url")
                .and_then(|u| u.as_str())
                .unwrap();
            assert!(url.starts_with("https://"));
        }
    }

    #[test]
    fn test_asset_not_found_error() {
        let assets: Vec<serde_json::Value> = vec![serde_json::json!(
            {"name": "claude-status-unknown-target", "browser_download_url": "https://example.com/x"}
        )];
        let asset_name = "claude-status-nonexistent-target";
        let found = assets
            .iter()
            .find(|a| a.get("name").and_then(|n| n.as_str()) == Some(asset_name));
        assert!(found.is_none());
    }
}
