use std::fs;
use std::io;
use std::path::Path;

pub fn count_words_in_str(input: &str) -> usize {
    input.split_whitespace().count()
}

pub fn count_words_in_file<P: AsRef<Path>>(path: P) -> io::Result<usize> {
    let content = fs::read_to_string(path)?;
    Ok(count_words_in_str(&content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn counts_words_in_str() {
        let input = "rust is fast and fearless";
        assert_eq!(count_words_in_str(input), 5);
    }

    #[test]
    fn counts_words_in_file() {
        let mut path = std::env::temp_dir();
        path.push("wc_test_file.txt");

        {
            let mut file = File::create(&path).unwrap();
            writeln!(file, "one two three").unwrap();
        }

        let count = count_words_in_file(&path).unwrap();
        assert_eq!(count, 3);

        // Clean up best-effort
        let _ = std::fs::remove_file(path);
    }
}
