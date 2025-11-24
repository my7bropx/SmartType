pub mod engine;
pub mod dictionary;
pub mod smart_punctuation;
pub mod config;
pub mod hook;

pub use engine::AutocorrectEngine;
pub use dictionary::Dictionary;
pub use smart_punctuation::SmartPunctuation;
pub use config::Config;

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main SmartType library interface
pub struct SmartType {
    engine: Arc<RwLock<AutocorrectEngine>>,
    config: Arc<RwLock<Config>>,
}

impl SmartType {
    /// Create a new SmartType instance
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let engine = AutocorrectEngine::new(config.clone())?;

        Ok(Self {
            engine: Arc::new(RwLock::new(engine)),
            config: Arc::new(RwLock::new(config)),
        })
    }

    /// Process input text and return corrected version
    pub async fn process(&self, input: &str) -> Result<String> {
        let engine = self.engine.read().await;
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(input.to_string());
        }

        engine.process(input)
    }

    /// Process a single word
    pub async fn correct_word(&self, word: &str) -> Result<Option<String>> {
        let engine = self.engine.read().await;
        Ok(engine.correct_word(word))
    }

    /// Reload configuration
    pub async fn reload_config(&self) -> Result<()> {
        let new_config = Config::load()?;
        let mut config = self.config.write().await;
        *config = new_config.clone();

        let mut engine = self.engine.write().await;
        engine.update_config(new_config)?;

        Ok(())
    }

    /// Add custom correction
    pub async fn add_correction(&self, typo: &str, correction: &str) -> Result<()> {
        let mut engine = self.engine.write().await;
        engine.add_custom_correction(typo, correction)
    }

    /// Remove custom correction
    pub async fn remove_correction(&self, typo: &str) -> Result<()> {
        let mut engine = self.engine.write().await;
        engine.remove_custom_correction(typo)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<Stats> {
        let engine = self.engine.read().await;
        Ok(engine.get_stats())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub total_corrections: u64,
    pub session_corrections: u64,
    pub dictionary_size: usize,
    pub custom_corrections: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_correction() {
        let smarttype = SmartType::new().await.unwrap();
        let result = smarttype.correct_word("teh").await.unwrap();
        assert_eq!(result, Some("the".to_string()));
    }

    #[tokio::test]
    async fn test_smart_punctuation() {
        let smarttype = SmartType::new().await.unwrap();
        let result = smarttype.process("\"hello world\"").await.unwrap();
        assert!(result.contains('"') || result.contains('"'));
    }
}
