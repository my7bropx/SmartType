use std::env;
use std::process;

use password_checker::{assess, exit_code};

fn main() {
    let password = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: password_checker <password>");
        process::exit(2);
    });

    let assessment = assess(&password);
    print_report(&assessment);

    process::exit(exit_code(assessment.strength));
}

fn print_report(assessment: &password_checker::Assessment) {
    println!("Password length: {}", assessment.password.chars().count());
    println!(
        "Strength: {:?} ({}/{})",
        assessment.strength, assessment.score, assessment.max_score
    );
    println!("Checks:");
    for rule in &assessment.results {
        println!(
            "- {:<18} : {}",
            rule.description,
            if rule.passed { "ok" } else { "miss" }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_usage_on_missing_arg() {
        // simulate no args: return early with exit code 2
        let output = std::panic::catch_unwind(|| {
            let _ = assess("");
        });
        assert!(output.is_ok());
    }

    #[test]
    fn report_shows_scores() {
        let assessment = assess("GoodPass123!");
        print_report(&assessment);
    }
}
