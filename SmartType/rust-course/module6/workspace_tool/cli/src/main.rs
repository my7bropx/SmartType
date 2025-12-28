use corelib::{add, greet};
use std::env;

fn print_usage() {
    eprintln!("Usage:\n  workspace-tool greet <name>\n  workspace-tool add <a> <b>");
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    if !args.is_empty() {
        args.remove(0); // drop binary name
    }

    let Some(command) = args.get(0).cloned() else {
        print_usage();
        std::process::exit(1);
    };
    args.remove(0);

    match command.as_str() {
        "greet" => {
            if let Some(name) = args.get(0) {
                println!("{}", greet(name));
            } else {
                print_usage();
                std::process::exit(1);
            }
        }
        "add" => {
            if args.len() < 2 {
                print_usage();
                std::process::exit(1);
            }
            let a: i32 = args[0].parse().unwrap_or(0);
            let b: i32 = args[1].parse().unwrap_or(0);
            println!("{} + {} = {}", a, b, add(a, b));
        }
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_helper() {
        assert!(greet("Ferris").contains("Ferris"));
    }

    #[test]
    fn add_helper() {
        assert_eq!(add(2, 3), 5);
    }
}
