use regex::Regex;

/// Smart punctuation processor
pub struct SmartPunctuation {
    double_quote_state: bool,
    single_quote_state: bool,
    double_quote_regex: Regex,
    single_quote_regex: Regex,
    apostrophe_regex: Regex,
    dash_regex: Regex,
    ellipsis_regex: Regex,
}

impl SmartPunctuation {
    /// Create new smart punctuation processor
    pub fn new() -> Self {
        Self {
            double_quote_state: false,
            single_quote_state: false,
            double_quote_regex: Regex::new(r#""([^"]*)""#).unwrap(),
            single_quote_regex: Regex::new(r"'([^']*)'").unwrap(),
            apostrophe_regex: Regex::new(r"\b([a-zA-Z]+)'([a-zA-Z]+)\b").unwrap(),
            dash_regex: Regex::new(r" -- ").unwrap(),
            ellipsis_regex: Regex::new(r"\.\.\.").unwrap(),
        }
    }

    /// Process text with smart punctuation
    pub fn process(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Replace straight quotes with smart quotes
        result = self.replace_double_quotes(&result);
        result = self.replace_single_quotes(&result);

        // Fix apostrophes
        result = self.fix_apostrophes(&result);

        // Replace double hyphens with em dash
        result = self.dash_regex.replace_all(&result, " — ").to_string();

        // Replace three dots with ellipsis
        result = self.ellipsis_regex.replace_all(&result, "…").to_string();

        result
    }

    /// Replace straight double quotes with smart quotes
    fn replace_double_quotes(&self, text: &str) -> String {
        let mut result = String::new();
        let mut in_quote = false;

        for ch in text.chars() {
            if ch == '"' {
                if in_quote {
                    result.push('"'); // Closing quote
                } else {
                    result.push('"'); // Opening quote
                }
                in_quote = !in_quote;
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Replace straight single quotes with smart quotes
    fn replace_single_quotes(&self, text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();
        let mut in_quote = false;

        while let Some(ch) = chars.next() {
            if ch == '\'' {
                // Check if it's an apostrophe (has letters on both sides)
                let prev_is_letter = result.chars().last().map(|c| c.is_alphabetic()).unwrap_or(false);
                let next_is_letter = chars.peek().map(|c| c.is_alphabetic()).unwrap_or(false);

                if prev_is_letter && next_is_letter {
                    // It's an apostrophe
                    result.push('\u{2019}');
                } else if in_quote {
                    // Closing quote
                    result.push('\u{2019}');
                    in_quote = false;
                } else {
                    // Opening quote
                    result.push('\u{2018}');
                    in_quote = true;
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Fix apostrophes in contractions
    fn fix_apostrophes(&self, text: &str) -> String {
        text.replace("'", "'")
    }
}

impl Default for SmartPunctuation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_double_quotes() {
        let sp = SmartPunctuation::new();
        let result = sp.process(r#""Hello world""#);
        assert_eq!(result, "\u{201C}Hello world\u{201D}");
    }

    #[test]
    fn test_smart_single_quotes() {
        let sp = SmartPunctuation::new();
        let result = sp.process("'Hello world'");
        assert_eq!(result, "'Hello world'");
    }

    #[test]
    fn test_apostrophes() {
        let sp = SmartPunctuation::new();
        let result = sp.process("don't can't it's");
        assert!(result.contains("'"));
    }

    #[test]
    fn test_em_dash() {
        let sp = SmartPunctuation::new();
        let result = sp.process("Hello -- world");
        assert_eq!(result, "Hello — world");
    }

    #[test]
    fn test_ellipsis() {
        let sp = SmartPunctuation::new();
        let result = sp.process("Wait...");
        assert_eq!(result, "Wait…");
    }

    #[test]
    fn test_mixed_punctuation() {
        let sp = SmartPunctuation::new();
        let result = sp.process(r#""Don't worry" -- it's "fine...""#);
        assert!(result.contains('"') || result.contains('"'));
        assert!(result.contains('\u{2019}'));
        assert!(result.contains('—'));
        assert!(result.contains('…'));
    }
}
