use async_crawler::crawl;
use std::env;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let mut args: Vec<String> = env::args().collect();
    if !args.is_empty() {
        args.remove(0); // drop binary name
    }

    if args.is_empty() {
        eprintln!("Usage: async_crawler <seed_file> [max_depth]");
        return Ok(());
    }

    let seed = PathBuf::from(&args[0]);
    let max_depth: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let results = crawl(vec![seed], max_depth, worker_count)?;
    for page in results {
        println!(
            "{} (links: {}, bytes: {})",
            page.path.display(),
            page.links.len(),
            page.content_len
        );
        for link in page.links {
            println!("  -> {}", link.display());
        }
    }

    Ok(())
}
