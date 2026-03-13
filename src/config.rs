use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    #[serde(default = "default_true")]
    pub effort_level: bool,
    #[serde(default = "default_true")]
    pub context_bar: bool,
    #[serde(default = "default_true")]
    pub rate_limit_current: bool,
    #[serde(default = "default_true")]
    pub rate_limit_weekly: bool,
    #[serde(default = "default_true")]
    pub rate_limit_extra: bool,
    #[serde(default = "default_true")]
    pub cost: bool,
    #[serde(default = "default_true")]
    pub git_info: bool,
    #[serde(default = "default_true")]
    pub duration: bool,
    #[serde(default = "default_true")]
    pub model_name: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            effort_level: true,
            context_bar: true,
            rate_limit_current: true,
            rate_limit_weekly: true,
            rate_limit_extra: true,
            cost: true,
            git_info: true,
            duration: true,
            model_name: true,
        }
    }
}

/// All valid item names for use in `config set` / `config list`.
pub const ITEM_NAMES: &[&str] = &[
    "effort_level",
    "context_bar",
    "rate_limit_current",
    "rate_limit_weekly",
    "rate_limit_extra",
    "cost",
    "git_info",
    "duration",
    "model_name",
];

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".config")
    });
    base.join("ccsl").join("config.toml")
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    display: DisplayConfig,
}

impl DisplayConfig {
    /// Load from the config file. Falls back to defaults on missing file or parse error.
    pub fn load() -> Self {
        Self::load_from(&config_path())
    }

    /// Load from a specific path. Falls back to defaults on missing file or parse error.
    pub fn load_from(path: &std::path::Path) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Self::default(),
            Err(_) => return Self::default(),
        };
        match toml::from_str::<ConfigFile>(&content) {
            Ok(cf) => cf.display,
            Err(_) => Self::default(),
        }
    }

    /// Persist to the config file, creating the directory if needed.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let cf = ConfigFile {
            display: self.clone(),
        };
        let content = toml::to_string_pretty(&cf)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Get the value of a named item. Returns `None` for unknown names.
    pub fn get(&self, name: &str) -> Option<bool> {
        match name {
            "effort_level" => Some(self.effort_level),
            "context_bar" => Some(self.context_bar),
            "rate_limit_current" => Some(self.rate_limit_current),
            "rate_limit_weekly" => Some(self.rate_limit_weekly),
            "rate_limit_extra" => Some(self.rate_limit_extra),
            "cost" => Some(self.cost),
            "git_info" => Some(self.git_info),
            "duration" => Some(self.duration),
            "model_name" => Some(self.model_name),
            _ => None,
        }
    }

    /// Set the value of a named item. Returns `false` for unknown names.
    pub fn set(&mut self, name: &str, value: bool) -> bool {
        match name {
            "effort_level" => self.effort_level = value,
            "context_bar" => self.context_bar = value,
            "rate_limit_current" => self.rate_limit_current = value,
            "rate_limit_weekly" => self.rate_limit_weekly = value,
            "rate_limit_extra" => self.rate_limit_extra = value,
            "cost" => self.cost = value,
            "git_info" => self.git_info = value,
            "duration" => self.duration = value,
            "model_name" => self.model_name = value,
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_all_true() {
        let cfg = DisplayConfig::default();
        assert!(cfg.effort_level);
        assert!(cfg.context_bar);
        assert!(cfg.rate_limit_current);
        assert!(cfg.rate_limit_weekly);
        assert!(cfg.rate_limit_extra);
        assert!(cfg.cost);
        assert!(cfg.git_info);
        assert!(cfg.duration);
        assert!(cfg.model_name);
    }

    #[test]
    fn test_load_missing_file_returns_defaults() {
        let path = std::path::Path::new("/tmp/ccsl-test-nonexistent-999/config.toml");
        let cfg = DisplayConfig::load_from(path);
        assert!(cfg.model_name);
        assert!(cfg.effort_level);
    }

    #[test]
    fn test_load_partial_config_defaults_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "[display]\ncost = false\n").unwrap();

        let cfg = DisplayConfig::load_from(&config_path);
        assert!(!cfg.cost);
        assert!(cfg.model_name); // defaulted to true
        assert!(cfg.effort_level);
    }

    #[test]
    fn test_load_malformed_toml_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(&config_path, "[display]\ncost = NOTABOOL\n").unwrap();

        let cfg = DisplayConfig::load_from(&config_path);
        assert!(cfg.cost); // fell back to default
    }

    #[test]
    fn test_get_known_item() {
        let mut cfg = DisplayConfig::default();
        cfg.cost = false;
        assert_eq!(cfg.get("cost"), Some(false));
        assert_eq!(cfg.get("model_name"), Some(true));
    }

    #[test]
    fn test_get_unknown_item() {
        let cfg = DisplayConfig::default();
        assert_eq!(cfg.get("nonexistent"), None);
    }

    #[test]
    fn test_set_known_item() {
        let mut cfg = DisplayConfig::default();
        assert!(cfg.set("cost", false));
        assert!(!cfg.cost);
        assert!(cfg.set("cost", true));
        assert!(cfg.cost);
    }

    #[test]
    fn test_set_unknown_item() {
        let mut cfg = DisplayConfig::default();
        assert!(!cfg.set("nonexistent", false));
    }
}
