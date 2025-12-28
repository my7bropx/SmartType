use parallel_scanner::scan_dir;
use std::env;
use std::io;

fn main() -> io::Result<()> {
    let dir = env::args().nth(1).unwrap_or_else(|| ".".into());
    let results = scan_dir(&dir)?;

    for file_hash in results {
        println!("{}  {}", file_hash.hash_hex, file_hash.path.display());
    }

    Ok(())
}
