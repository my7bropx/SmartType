use crate::dictionary::Dictionary;
use crate::smart_punctuation::SmartPunctuation;
use crate::config::Config;
use crate::Stats;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

/// Core autocorrect engine
pub struct AutocorrectEngine {
    dictionary: Dictionary,
    smart_punctuation: SmartPunctuation,
    config: Config,
    custom_corrections: HashMap<String, String>,
    stats: EngineStats,
    word_boundary: Regex,
}

#[derive(Debug, Clone, Default)]
struct EngineStats {
    total_corrections: u64,
    session_corrections: u64,
}

impl AutocorrectEngine {
    /// Create new engine instance
    pub fn new(config: Config) -> Result<Self> {
        let dictionary = Dictionary::load()?;
        let smart_punctuation = SmartPunctuation::new();
        let custom_corrections = Self::load_custom_corrections(&config)?;

        Ok(Self {
            dictionary,
            smart_punctuation,
            config,
            custom_corrections,
            stats: EngineStats::default(),
            word_boundary: Regex::new(r"\b\w+\b").unwrap(),
        })
    }

    /// Process input text
    pub fn process(&self, input: &str) -> Result<String> {
        let mut output = input.to_string();

        // Apply autocorrect if enabled
        if self.config.autocorrect {
            output = self.apply_autocorrect(&output)?;
        }

        // Apply smart punctuation if enabled
        if self.config.smart_punctuation {
            output = self.smart_punctuation.process(&output);
        }

        Ok(output)
    }

    /// Correct a single word
    pub fn correct_word(&self, word: &str) -> Option<String> {
        if word.len() < self.config.min_word_length {
            return None;
        }

        // Check custom corrections first
        if let Some(correction) = self.custom_corrections.get(&word.to_lowercase()) {
            return Some(self.preserve_case(word, correction));
        }

        // Check dictionary
        if let Some(correction) = self.dictionary.get(&word.to_lowercase()) {
            return Some(self.preserve_case(word, correction));
        }

        None
    }

    /// Apply autocorrect to text
    fn apply_autocorrect(&self, text: &str) -> Result<String> {
        let mut result = text.to_string();
        let words: Vec<_> = self.word_boundary.find_iter(text).collect();

        // Process in reverse to maintain correct indices
        for word_match in words.iter().rev() {
            let word = word_match.as_str();
            if let Some(correction) = self.correct_word(word) {
                result.replace_range(word_match.range(), &correction);
            }
        }

        Ok(result)
    }

    /// Preserve original case pattern
    fn preserve_case(&self, original: &str, correction: &str) -> String {
        if original.chars().all(|c| c.is_uppercase()) {
            // ALL CAPS
            correction.to_uppercase()
        } else if original.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            // Title Case
            let mut chars = correction.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        } else {
            // lowercase
            correction.to_lowercase()
        }
    }

    /// Load custom corrections from config
    fn load_custom_corrections(config: &Config) -> Result<HashMap<String, String>> {
        Ok(config.custom_typos.clone())
    }

    /// Update configuration
    pub fn update_config(&mut self, config: Config) -> Result<()> {
        self.custom_corrections = Self::load_custom_corrections(&config)?;
        self.config = config;
        Ok(())
    }

    /// Add custom correction
    pub fn add_custom_correction(&mut self, typo: &str, correction: &str) -> Result<()> {
        self.custom_corrections.insert(
            typo.to_lowercase(),
            correction.to_string(),
        );
        Ok(())
    }

    /// Remove custom correction
    pub fn remove_custom_correction(&mut self, typo: &str) -> Result<()> {
        self.custom_corrections.remove(&typo.to_lowercase());
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> Stats {
        Stats {
            total_corrections: self.stats.total_corrections,
            session_corrections: self.stats.session_corrections,
            dictionary_size: self.dictionary.len(),
            custom_corrections: self.custom_corrections.len(),
        }
    }

    /// Increment correction counter
    pub fn record_correction(&mut self) {
        self.stats.total_corrections += 1;
        self.stats.session_corrections += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_engine() -> AutocorrectEngine {
        let config = Config::default();
        AutocorrectEngine::new(config).unwrap()
    }

    #[test]
    fn test_word_correction() {
        let engine = create_test_engine();
        assert_eq!(engine.correct_word("teh"), Some("the".to_string()));
        assert_eq!(engine.correct_word("recieve"), Some("receive".to_string()));
    }

    #[test]
    fn test_case_preservation() {
        let engine = create_test_engine();
        assert_eq!(engine.preserve_case("TEH", "the"), "THE");
        assert_eq!(engine.preserve_case("Teh", "the"), "The");
        assert_eq!(engine.preserve_case("teh", "the"), "the");
    }

    #[test]
    fn test_sentence_correction() {
        let engine = create_test_engine();
        let result = engine.process("teh quick borwn fox").unwrap();
        assert!(result.contains("the"));
        assert!(result.contains("brown"));
    }

    #[test]
    fn test_min_word_length() {
        let mut config = Config::default();
        config.min_word_length = 3;
        let engine = AutocorrectEngine::new(config).unwrap();
        assert_eq!(engine.correct_word("ab"), None);
    }
}
