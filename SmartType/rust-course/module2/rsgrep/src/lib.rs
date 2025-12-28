use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub query: String,
    pub filename: String,
    pub ignore_case: bool,
}

impl Config {
    pub fn from_args(mut args: impl Iterator<Item = String>) -> Result<Self, String> {
        // drop binary name
        args.next();

        let query = args
            .next()
            .ok_or_else(|| "missing query string".to_string())?;
        let filename = args
            .next()
            .ok_or_else(|| "missing file name to search".to_string())?;

        let ignore_case = args.any(|arg| arg == "--ignore-case" || arg == "-i");

        Ok(Self {
            query,
            filename,
            ignore_case,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub line_number: usize,
    pub line: String,
}

pub fn search_in_reader<R: BufRead>(query: &str, reader: R, ignore_case: bool) -> Vec<Match> {
    let matcher = |line: &str| {
        if ignore_case {
            line.to_lowercase().contains(&query.to_lowercase())
        } else {
            line.contains(query)
        }
    };

    reader
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| match line {
            Ok(line_content) if matcher(&line_content) => Some(Match {
                line_number: idx + 1,
                line: line_content,
            }),
            _ => None,
        })
        .collect()
}

pub fn search_file(config: &Config) -> Result<Vec<Match>, Box<dyn Error>> {
    let file = File::open(&config.filename)?;
    let reader = BufReader::new(file);
    Ok(search_in_reader(&config.query, reader, config.ignore_case))
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let matches = search_file(&config)?;

    if matches.is_empty() {
        println!("No matches found.");
        return Ok(());
    }

    for m in matches {
        println!("{}:{}", m.line_number, m.line);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_parses_args() {
        let args = vec![
            "rsgrep".to_string(),
            "needle".to_string(),
            "file.txt".to_string(),
            "--ignore-case".to_string(),
        ];
        let cfg = Config::from_args(args.into_iter()).unwrap();
        assert_eq!(cfg.query, "needle");
        assert_eq!(cfg.filename, "file.txt");
        assert!(cfg.ignore_case);
    }

    #[test]
    fn search_finds_case_sensitive() {
        let input = b"Rust is great\nTrust the process\n";
        let reader = BufReader::new(&input[..]);
        let results = search_in_reader("Rust", reader, false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 1);
    }

    #[test]
    fn search_finds_ignore_case() {
        let input = b"rustacean\nFerris\nRUST rules\n";
        let reader = BufReader::new(&input[..]);
        let results = search_in_reader("rust", reader, true);
        assert_eq!(results.len(), 2);
        assert_eq!(results[1].line_number, 3);
    }
}
