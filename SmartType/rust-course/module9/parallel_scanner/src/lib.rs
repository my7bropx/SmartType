use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHash {
    pub path: PathBuf,
    pub hash_hex: String,
}

/// Parallel scan with scoped threads.
pub fn scan_dir(root: impl AsRef<Path>) -> io::Result<Vec<FileHash>> {
    let entries = walk_files(root.as_ref())?;
    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .min(entries.len().max(1));

    let results: Arc<Mutex<Vec<io::Result<FileHash>>>> = Arc::new(Mutex::new(Vec::new()));

    thread::scope(|scope| {
        for chunk in entries.chunks(worker_count.max(1)) {
            let paths: Vec<PathBuf> = chunk.to_vec();
            let out = Arc::clone(&results);
            scope.spawn(move || {
                for path in paths {
                    let res = hash_file(&path);
                    out.lock().unwrap().push(res);
                }
            });
        }
    });

    let mut final_results = Vec::new();
    for res in results.lock().unwrap().drain(..) {
        final_results.push(res?);
    }
    Ok(final_results)
}

/// Sequential scan (useful for baselines and testing).
pub fn scan_dir_sequential(root: impl AsRef<Path>) -> io::Result<Vec<FileHash>> {
    let entries = walk_files(root.as_ref())?;
    let mut out = Vec::with_capacity(entries.len());
    for path in entries {
        out.push(hash_file(&path)?);
    }
    Ok(out)
}

fn hash_file(path: &Path) -> io::Result<FileHash> {
    let mut file = BufReader::new(File::open(path)?);
    let mut hasher = Fnv64::new();

    let mut buf = [0u8; 8192];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    let hash_hex = hasher.finish_hex();

    Ok(FileHash {
        path: path.to_path_buf(),
        hash_hex,
    })
}

fn walk_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// Simple deterministic 64-bit FNV-1a hasher (no external deps).
#[derive(Default)]
struct Fnv64 {
    state: u64,
}

impl Fnv64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::OFFSET_BASIS,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.state ^= u64::from(*b);
            self.state = self.state.wrapping_mul(Self::PRIME);
        }
    }

    fn finish_hex(&self) -> String {
        format!("{:016x}", self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("{prefix}_{nanos}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn temp_file_with(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = File::create(&path).unwrap();
        write!(f, "{}", content).unwrap();
        path
    }

    #[test]
    fn hashes_single_file() {
        let dir = temp_dir("scanner_single");
        let path = temp_file_with(&dir, "a.txt", "hello");
        let hashes = scan_dir(&dir).unwrap();
        assert!(hashes.iter().any(|h| h.path == path));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn errors_on_missing_dir() {
        let missing = PathBuf::from("/definitely/missing/path");
        let err = scan_dir(&missing).unwrap_err();
        assert!(err.kind() == io::ErrorKind::NotFound || err.kind() == io::ErrorKind::Other);
    }

    #[test]
    fn hashes_are_deterministic() {
        let dir = temp_dir("scanner_deterministic");
        let path1 = temp_file_with(&dir, "a.txt", "hello world");
        let path2 = temp_file_with(&dir, "b.txt", "hello world");

        let h1 = hash_file(&path1).unwrap().hash_hex;
        let h2 = hash_file(&path2).unwrap().hash_hex;
        assert_eq!(h1, h2);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parallel_and_sequential_match() {
        let dir = temp_dir("scanner_compare");
        temp_file_with(&dir, "a.txt", "a");
        temp_file_with(&dir, "b.txt", "b");

        let mut parallel = scan_dir(&dir).unwrap();
        let mut sequential = scan_dir_sequential(&dir).unwrap();

        parallel.sort_by(|a, b| a.path.cmp(&b.path));
        sequential.sort_by(|a, b| a.path.cmp(&b.path));

        assert_eq!(parallel, sequential);

        let _ = fs::remove_dir_all(dir);
    }
}
