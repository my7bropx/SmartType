use std::collections::HashMap;
use std::fmt::Display;

/// Compute the mean of a list of numbers.
pub fn mean(data: &[f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    let sum: f64 = data.iter().sum();
    Some(sum / data.len() as f64)
}

/// Compute the median of a list of numbers.
pub fn median(data: &[f64]) -> Option<f64> {
    if data.is_empty() {
        return None;
    }
    let mut values: Vec<f64> = data.to_vec();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}

/// Compute the mode(s) of a list of numbers (all values with highest frequency).
pub fn mode(data: &[f64]) -> Vec<f64> {
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for &value in data {
        let key = value.to_bits();
        *counts.entry(key).or_insert(0) += 1;
    }
    let max_count = counts.values().copied().max().unwrap_or(0);
    if max_count == 0 {
        return vec![];
    }
    let mut modes: Vec<f64> = counts
        .into_iter()
        .filter_map(|(value, count)| {
            if count == max_count {
                Some(f64::from_bits(value))
            } else {
                None
            }
        })
        .collect();
    modes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    modes
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Stack<T> {
    items: Vec<T>,
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, item: T) {
        self.items.push(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    pub fn peek(&self) -> Option<&T> {
        self.items.last()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub trait Summarize {
    fn summarize(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub title: String,
    pub author: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tweet {
    pub username: String,
    pub text: String,
}

impl Summarize for Article {
    fn summarize(&self) -> String {
        format!("{} — by {}", self.title, self.author)
    }
}

impl Summarize for Tweet {
    fn summarize(&self) -> String {
        format!("@{}: {}", self.username, self.text)
    }
}

pub fn print_summaries<T: Summarize + Display>(items: &[T]) {
    for item in items {
        println!("{}", item.summarize());
        println!("display: {}", item);
    }
}

impl Display for Article {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Article('{}', by {})", self.title, self.author)
    }
}

impl Display for Tweet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Tweet by @{}", self.username)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_median_mode() {
        let data = [1.0, 2.0, 2.0, 3.0, 4.0];
        assert_eq!(mean(&data), Some(2.4));
        assert_eq!(median(&data), Some(2.0));
        assert_eq!(mode(&data), vec![2.0]);
    }

    #[test]
    fn stack_push_pop_peek() {
        let mut stack = Stack::new();
        assert!(stack.is_empty());
        stack.push(1);
        stack.push(2);
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek(), Some(&2));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert!(stack.pop().is_none());
    }

    #[test]
    fn summarize_implementations() {
        let article = Article {
            title: "Rust 2024".into(),
            author: "Ferris".into(),
            content: "All about Rust".into(),
        };
        let tweet = Tweet {
            username: "rustacean".into(),
            text: "Rust is great".into(),
        };

        assert!(article.summarize().contains("Rust 2024"));
        assert!(tweet.summarize().starts_with("@rustacean"));
    }
}
