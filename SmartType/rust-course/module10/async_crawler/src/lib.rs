use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageResult {
    pub path: PathBuf,
    pub links: Vec<PathBuf>,
    pub content_len: usize,
}

/// Crawl files level by level, following `link:<path>` markers up to `max_depth`.
/// Work for each level is fanned out across threads (capped by `worker_count`).
pub fn crawl(
    seeds: Vec<PathBuf>,
    max_depth: usize,
    worker_count: usize,
) -> io::Result<Vec<PageResult>> {
    if seeds.is_empty() {
        return Ok(Vec::new());
    }

    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut current: Vec<PathBuf> = seeds;
    let mut results = Vec::new();

    for depth in 0..=max_depth {
        // Deduplicate at this level
        current.retain(|p| visited.insert(p.clone()));
        if current.is_empty() {
            break;
        }

        let mut next_level: Vec<PathBuf> = Vec::new();
        let mut handles = Vec::new();
        let chunks = current.chunks(worker_count.max(1));

        for chunk in chunks {
            for path in chunk.iter().cloned() {
                handles.push(thread::spawn(move || {
                    process_file(&path).map(|p| (p, depth))
                }));
            }
        }

        for handle in handles {
            if let Ok(res) = handle.join() {
                if let Ok((page, d)) = res {
                    if d < max_depth {
                        for link in &page.links {
                            next_level.push(link.clone());
                        }
                    }
                    results.push(page);
                }
            }
        }

        current = next_level;
    }

    Ok(results)
}

fn process_file(path: &Path) -> io::Result<PageResult> {
    let content = fs::read_to_string(path)?;
    let links = extract_links(&content, path);
    Ok(PageResult {
        path: path.to_path_buf(),
        links,
        content_len: content.len(),
    })
}

fn extract_links(content: &str, base: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for line in content.lines() {
        if let Some(pos) = line.find("link:") {
            let link_part = line[pos + 5..].trim();
            if !link_part.is_empty() {
                let target = base
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(link_part);
                out.push(target);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("{prefix}_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn temp_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        write!(f, "{}", content).unwrap();
        path
    }

    #[test]
    fn crawls_links_up_to_depth() {
        let dir = temp_dir("crawler_a");
        let file_a = temp_file(&dir, "a.txt", "hello\nlink:b.txt\n");
        let file_b = temp_file(&dir, "b.txt", "child\n");

        let seeds = vec![file_a.clone()];
        let results = crawl(seeds, 1, 4).unwrap();

        assert!(results.iter().any(|p| p.path == file_a));
        assert!(results.iter().any(|p| p.path == file_b));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stops_at_depth_limit() {
        let dir = temp_dir("crawler_b");
        let file_a = temp_file(&dir, "a.txt", "link:b.txt\n");
        let file_b = temp_file(&dir, "b.txt", "link:c.txt\n");
        let file_c = temp_file(&dir, "c.txt", "leaf\n");

        let results = crawl(vec![file_a.clone()], 1, 2).unwrap();
        assert!(results.iter().any(|p| p.path == file_a));
        assert!(results.iter().any(|p| p.path == file_b));
        assert!(!results.iter().any(|p| p.path == file_c));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn handles_missing_files() {
        let missing = PathBuf::from("/surely/missing/file.txt");
        let results = crawl(vec![missing], 1, 2).unwrap();
        assert!(results.is_empty());
    }
}
