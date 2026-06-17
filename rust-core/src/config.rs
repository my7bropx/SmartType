use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub smart_punctuation: bool,
    pub autocorrect: bool,
    pub min_word_length: usize,
    pub applications: HashMap<String, AppConfig>,
    pub custom_typos: HashMap<String, String>,
    pub hotkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub enabled: bool,
    pub smart_quotes: Option<bool>,
    pub autocorrect: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        let mut applications = HashMap::new();

        // Default app configurations
        applications.insert(
            "firefox".to_string(),
            AppConfig {
                enabled: true,
                smart_quotes: Some(true),
                autocorrect: Some(true),
            },
        );

        applications.insert(
            "qterminal".to_string(),
            AppConfig {
                enabled: true,
                smart_quotes: Some(false), // Disable smart quotes in terminal
                autocorrect: Some(true),
            },
        );

        applications.insert(
            "kitty".to_string(),
            AppConfig {
                enabled: true,
                smart_quotes: Some(false),
                autocorrect: Some(true),
            },
        );

        applications.insert(
            "alacritty".to_string(),
            AppConfig {
                enabled: true,
                smart_quotes: Some(false),
                autocorrect: Some(true),
            },
        );

        applications.insert(
            "code".to_string(), // VS Code
            AppConfig {
                enabled: true,
                smart_quotes: Some(false),
                autocorrect: Some(true),
            },
        );

        let mut custom_typos = HashMap::new();
        custom_typos.insert("hte".to_string(), "the".to_string());
        custom_typos.insert("becuase".to_string(), "because".to_string());

        Self {
            enabled: true,
            smart_punctuation: true,
            autocorrect: true,
            min_word_length: 2,
            applications,
            custom_typos,
            hotkey: "Super+Shift+A".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            let config: Config = serde_yaml::from_str(&content)
                .context("Failed to parse config file")?;
            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure config directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_yaml::to_string(self)
            .context("Failed to serialize config")?;

        fs::write(&config_path, content)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Get config file path
    fn config_path() -> Result<PathBuf> {
        let config_dir = directories::BaseDirs::new()
            .context("Failed to get home directory")?
            .config_dir()
            .join("smarttype");

        Ok(config_dir.join("config.yaml"))
    }

    /// Get configuration for specific application
    pub fn get_app_config(&self, app_name: &str) -> Option<&AppConfig> {
        self.applications.get(app_name)
    }

    /// Update application configuration
    pub fn set_app_config(&mut self, app_name: String, config: AppConfig) {
        self.applications.insert(app_name, config);
    }

    /// Add custom typo correction
    pub fn add_custom_typo(&mut self, typo: String, correction: String) {
        self.custom_typos.insert(typo, correction);
    }

    /// Remove custom typo correction
    pub fn remove_custom_typo(&mut self, typo: &str) {
        self.custom_typos.remove(typo);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.enabled);
        assert!(config.smart_punctuation);
        assert!(config.autocorrect);
        assert_eq!(config.min_word_length, 2);
    }

    #[test]
    fn test_app_config() {
        let config = Config::default();
        let firefox_config = config.get_app_config("firefox");
        assert!(firefox_config.is_some());
        assert!(firefox_config.unwrap().enabled);
    }

    #[test]
    fn test_serialization() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.enabled, deserialized.enabled);
    }
}
