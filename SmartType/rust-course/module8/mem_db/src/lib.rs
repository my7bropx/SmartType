use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone, Debug, Default)]
pub struct Database {
    inner: Arc<RwLock<HashMap<String, String>>>,
    stats: Arc<Mutex<Stats>>, // Stats updated via interior mutability
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub inserts: usize,
    pub deletes: usize,
    pub lookups: usize,
}

impl Database {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(Stats::default())),
        }
    }

    pub fn insert(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut map = self.inner.write().expect("lock poisoned");
        map.insert(key.into(), value.into());
        if let Ok(mut stats) = self.stats.lock() {
            stats.inserts += 1;
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let map = self.inner.read().expect("lock poisoned");
        let value = map.get(key).cloned();
        if let Ok(mut stats) = self.stats.lock() {
            stats.lookups += 1;
        }
        value
    }

    pub fn delete(&self, key: &str) -> bool {
        let mut map = self.inner.write().expect("lock poisoned");
        let removed = map.remove(key).is_some();
        if removed {
            if let Ok(mut stats) = self.stats.lock() {
                stats.deletes += 1;
            }
        }
        removed
    }

    pub fn stats(&self) -> Stats {
        self.stats
            .lock()
            .map(|s| Stats {
                inserts: s.inserts,
                deletes: s.deletes,
                lookups: s.lookups,
            })
            .unwrap_or_else(|_| Stats::default())
    }

    pub fn keys(&self) -> Vec<String> {
        let map = self.inner.read().expect("lock poisoned");
        map.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn insert_and_get() {
        let db = Database::new();
        db.insert("user", "ferris");
        assert_eq!(db.get("user"), Some("ferris".into()));
        assert_eq!(db.stats().inserts, 1);
        assert_eq!(db.stats().lookups, 1);
    }

    #[test]
    fn delete_updates_stats() {
        let db = Database::new();
        db.insert("temp", "value");
        assert!(db.delete("temp"));
        assert!(!db.delete("missing"));
        let stats = db.stats();
        assert_eq!(stats.deletes, 1);
    }

    #[test]
    fn concurrent_access_is_safe() {
        let db = Database::new();
        let db1 = db.clone();
        let db2 = db.clone();

        let t1 = thread::spawn(move || {
            for i in 0..50 {
                db1.insert(format!("k{i}"), format!("v{i}"));
            }
        });

        let t2 = thread::spawn(move || {
            for i in 0..50 {
                let _ = db2.get(&format!("k{i}"));
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let keys = db.keys();
        assert!(keys.len() <= 50);
        assert!(db.stats().lookups >= 50);
    }
}
