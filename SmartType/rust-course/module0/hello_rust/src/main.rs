use std::env;

const STUDENT_NAME: &str = "SmartType Learner";

#[derive(Debug, PartialEq, Eq)]
struct RunInfo {
    name: String,
    os: String,
    args: Vec<String>,
}

impl RunInfo {
    /// Construct run info using real environment values.
    fn from_env() -> Self {
        Self::from_iter(STUDENT_NAME, env::consts::OS, env::args())
    }

    /// Construct run info from provided data (useful for testing and examples).
    fn from_iter<I, S>(name: &str, os: &str, iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut args: Vec<String> = iter.into_iter().map(Into::into).collect();
        if !args.is_empty() {
            args.remove(0); // Drop the binary name.
        }

        Self {
            name: name.to_string(),
            os: os.to_string(),
            args,
        }
    }
}

impl std::fmt::Display for RunInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args_display = if self.args.is_empty() {
            "none".to_string()
        } else {
            self.args.join(", ")
        };

        writeln!(f, "Hello, {}!", self.name)?;
        writeln!(f, "Operating system: {}", self.os)?;
        write!(f, "Arguments: {}", args_display)
    }
}

fn main() {
    let info = RunInfo::from_env();
    println!("{}", info);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_run_info() {
        let args = vec!["hello_rust", "first", "second"];
        let info = RunInfo::from_iter("Tester", "linux", args);

        assert_eq!(info.args, vec!["first", "second"]);

        let output = info.to_string();
        assert!(output.contains("Hello, Tester!"));
        assert!(output.contains("Operating system: linux"));
        assert!(output.contains("Arguments: first, second"));
    }

    #[test]
    fn handles_no_args() {
        let args = vec!["hello_rust"];
        let info = RunInfo::from_iter("Tester", "macos", args);

        assert!(info.args.is_empty());

        let output = info.to_string();
        assert!(output.contains("Arguments: none"));
    }
}
