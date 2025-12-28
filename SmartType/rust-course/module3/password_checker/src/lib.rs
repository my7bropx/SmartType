#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Weak,
    Medium,
    Strong,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RuleResult {
    pub description: &'static str,
    pub passed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Assessment {
    pub password: String,
    pub strength: Strength,
    pub score: u8,
    pub max_score: u8,
    pub results: Vec<RuleResult>,
}

const MIN_LENGTH: usize = 12;

pub fn assess(password: &str) -> Assessment {
    let mut results = Vec::new();

    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    results.push(RuleResult {
        description: "has lowercase letter",
        passed: has_lower,
    });

    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    results.push(RuleResult {
        description: "has uppercase letter",
        passed: has_upper,
    });

    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    results.push(RuleResult {
        description: "has digit",
        passed: has_digit,
    });

    let has_symbol = password
        .chars()
        .any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace());
    results.push(RuleResult {
        description: "has symbol",
        passed: has_symbol,
    });

    let long_enough = password.chars().count() >= MIN_LENGTH;
    results.push(RuleResult {
        description: "length >= 12",
        passed: long_enough,
    });

    let score = results.iter().filter(|r| r.passed).count() as u8;
    let max_score = results.len() as u8;

    let strength = match score {
        0..=2 => Strength::Weak,
        3..=4 => Strength::Medium,
        _ => Strength::Strong,
    };

    Assessment {
        password: password.to_string(),
        strength,
        score,
        max_score,
        results,
    }
}

pub fn exit_code(strength: Strength) -> i32 {
    match strength {
        Strength::Weak => 2,
        Strength::Medium => 1,
        Strength::Strong => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_password_scores_high() {
        let assessment = assess("Tru5tworthy!!");
        assert_eq!(assessment.strength, Strength::Strong);
        assert_eq!(assessment.score, assessment.max_score);
        assert!(assessment.results.iter().all(|r| r.passed));
    }

    #[test]
    fn medium_password_scores_medium() {
        let assessment = assess("Abcdef123");
        assert_eq!(assessment.strength, Strength::Medium);
        assert_eq!(assessment.score, 3);
    }

    #[test]
    fn weak_password_scores_low() {
        let assessment = assess("password");
        assert_eq!(assessment.strength, Strength::Weak);
        assert!(assessment.score < assessment.max_score);
    }

    #[test]
    fn exit_codes_match_spec() {
        assert_eq!(exit_code(Strength::Strong), 0);
        assert_eq!(exit_code(Strength::Medium), 1);
        assert_eq!(exit_code(Strength::Weak), 2);
    }
}
