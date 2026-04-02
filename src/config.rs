use crate::constants;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CONFIG_FILENAME: &str = ".sfcompact.yaml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SfCompactConfig {
    #[serde(default = "default_format")]
    pub default_format: String,

    #[serde(default)]
    pub formats: BTreeMap<String, String>,

    #[serde(default)]
    pub skip: Vec<String>,

    #[serde(default)]
    pub preserve_comments: bool,

    #[serde(default = "default_indent")]
    pub indent: u8,
}

fn default_format() -> String {
    constants::FORMAT_YAML.to_string()
}

fn default_indent() -> u8 {
    4
}

impl Default for SfCompactConfig {
    fn default() -> Self {
        Self {
            default_format: default_format(),
            formats: BTreeMap::new(),
            skip: Vec::new(),
            preserve_comments: false,
            indent: default_indent(),
        }
    }
}

impl SfCompactConfig {
    /// Create a config with smart defaults based on manifest metadata.
    /// Default is YAML (best for small types where JSON overhead exceeds savings).
    /// Order-sensitive types (Flow, FlexiPage, Layout) get yaml-ordered to
    /// preserve element order while still avoiding JSON overhead.
    pub fn with_smart_defaults(
        entries: &[(String, bool, Vec<String>)], // (type_name, order_sensitive, supported_formats)
    ) -> Self {
        let mut formats = BTreeMap::new();
        for (type_name, order_sensitive, _supported) in entries {
            if *order_sensitive {
                formats.insert(
                    type_name.clone(),
                    constants::FORMAT_YAML_ORDERED.to_string(),
                );
            }
        }
        Self {
            default_format: default_format(),
            formats,
            skip: Vec::new(),
            preserve_comments: false,
            indent: default_indent(),
        }
    }

    /// Return the format configured for a given metadata type name.
    /// Falls back to default_format if no type-specific override exists.
    pub fn format_for_type(&self, type_name: &str) -> &str {
        self.formats
            .get(type_name)
            .map(|s| s.as_str())
            .unwrap_or(&self.default_format)
    }

    /// Check if a metadata type should be skipped.
    pub fn should_skip(&self, type_name: &str) -> bool {
        self.skip.iter().any(|s| s == type_name)
    }

    /// Validate that all format values in the config are valid.
    pub fn validate(&self) -> Result<()> {
        let valid = constants::VALID_FORMATS;
        if !valid.contains(&self.default_format.as_str()) {
            anyhow::bail!(
                "Invalid default_format '{}' in config. Valid formats: {}",
                self.default_format,
                valid.join(", ")
            );
        }
        for (type_name, format) in &self.formats {
            if !valid.contains(&format.as_str()) {
                anyhow::bail!(
                    "Invalid format '{}' for type '{}' in config. Valid formats: {}",
                    format,
                    type_name,
                    valid.join(", ")
                );
            }
        }
        Ok(())
    }
}

/// Find `.sfcompact.yaml` starting from `start_dir` and walking up to parent directories.
pub fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir.to_path_buf();
    loop {
        let candidate = dir.join(CONFIG_FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Load config from the given directory (or parent dirs). Returns default if not found.
pub fn load_config(start_dir: &Path) -> Result<SfCompactConfig> {
    match find_config_file(start_dir) {
        Some(path) => {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            let config: SfCompactConfig = serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            Ok(config)
        }
        None => Ok(SfCompactConfig::default()),
    }
}

/// Load config from a specific file path.
pub fn load_config_from(path: &Path) -> Result<SfCompactConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: SfCompactConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// Save config to a specific file path.
pub fn save_config(config: &SfCompactConfig, path: &Path) -> Result<()> {
    let yaml = serde_yaml::to_string(config).context("Failed to serialize config to YAML")?;
    std::fs::write(path, &yaml)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;
    Ok(())
}

/// Save config to `.sfcompact.yaml` in the given directory.
pub fn save_config_to_dir(config: &SfCompactConfig, dir: &Path) -> Result<PathBuf> {
    let path = dir.join(CONFIG_FILENAME);
    save_config(config, &path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = SfCompactConfig::default();
        assert_eq!(config.default_format, "yaml");
        assert!(config.formats.is_empty());
        assert!(config.skip.is_empty());
    }

    #[test]
    fn format_for_type_uses_override() {
        let mut config = SfCompactConfig::default();
        config
            .formats
            .insert("flow".to_string(), "yaml-ordered".to_string());
        assert_eq!(config.format_for_type("flow"), "yaml-ordered");
        assert_eq!(config.format_for_type("profile"), "yaml");
    }

    #[test]
    fn should_skip() {
        let mut config = SfCompactConfig::default();
        config.skip.push("customMetadata".to_string());
        assert!(config.should_skip("customMetadata"));
        assert!(!config.should_skip("profile"));
    }

    #[test]
    fn roundtrip_yaml() {
        let mut config = SfCompactConfig::default();
        config.default_format = "yaml".to_string();
        config
            .formats
            .insert("flow".to_string(), "json".to_string());
        config
            .formats
            .insert("flexipage".to_string(), "json".to_string());
        config.skip.push("customMetadata".to_string());

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: SfCompactConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn smart_defaults_assigns_formats_by_order_sensitivity() {
        let entries = vec![
            (
                "Flow".to_string(),
                true,
                vec![
                    "yaml".to_string(),
                    "yaml-ordered".to_string(),
                    "json".to_string(),
                ],
            ),
            (
                "Profile".to_string(),
                false,
                vec![
                    "yaml".to_string(),
                    "yaml-ordered".to_string(),
                    "json".to_string(),
                ],
            ),
        ];
        let config = SfCompactConfig::with_smart_defaults(&entries);
        // Order-sensitive Flow gets yaml-ordered; Profile falls through to default (yaml)
        assert_eq!(config.format_for_type("Flow"), "yaml-ordered");
        assert_eq!(config.format_for_type("Profile"), "yaml");
    }
}
