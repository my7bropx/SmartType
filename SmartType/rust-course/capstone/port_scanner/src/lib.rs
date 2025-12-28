use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanResult {
    pub addr: SocketAddr,
    pub open: bool,
}

/// Parse a comma-separated list of ports/ranges like "80,443,8000-8005".
/// Duplicates are removed and the result is sorted.
pub fn parse_ports(spec: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for part in spec.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((start, end)) = trimmed.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.parse::<u16>(), end.parse::<u16>()) {
                let (low, high) = if s <= e { (s, e) } else { (e, s) };
                for p in low..=high {
                    ports.push(p);
                }
                continue;
            }
        }

        if let Ok(p) = trimmed.parse::<u16>() {
            ports.push(p);
        }
    }

    ports.sort_unstable();
    ports.dedup();
    ports
}

/// Scan a host over a set of ports using worker threads.
pub fn scan(host: &str, ports: &[u16], timeout_ms: u64, workers: usize) -> Vec<ScanResult> {
    let worker_count = workers.max(1);
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let results = Arc::new(Mutex::new(Vec::with_capacity(ports.len())));

    // chunk ports for threads
    let chunks: Vec<Vec<u16>> = ports.chunks(worker_count).map(|c| c.to_vec()).collect();

    thread::scope(|scope| {
        for chunk in chunks {
            let host = host.to_string();
            let res = Arc::clone(&results);
            scope.spawn(move || {
                for port in chunk {
                    if let Some(addr) = resolve_addr(&host, port) {
                        let open = is_open(&addr, timeout);
                        res.lock().unwrap().push(ScanResult { addr, open });
                    }
                }
            });
        }
    });

    let mut out = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
    out.sort_by(|a, b| a.addr.port().cmp(&b.addr.port()));
    out
}

fn resolve_addr(host: &str, port: u16) -> Option<SocketAddr> {
    let target = format!("{host}:{port}");
    target.to_socket_addrs().ok()?.next()
}

fn is_open(addr: &SocketAddr, timeout: Duration) -> bool {
    if let Ok(stream) = TcpStream::connect_timeout(addr, timeout) {
        let _ = stream.set_read_timeout(Some(timeout));
        let _ = stream.set_write_timeout(Some(timeout));
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn detects_open_and_closed_ports() {
        let open_port = free_port();
        let listener = TcpListener::bind(("127.0.0.1", open_port)).unwrap();
        let closed_port = free_port();

        let ports = vec![open_port, closed_port];
        let results = scan("127.0.0.1", &ports, 200, 4);

        let mut map = std::collections::HashMap::new();
        for r in results {
            map.insert(r.addr.port(), r.open);
        }

        assert_eq!(map.get(&open_port), Some(&true));
        assert_eq!(map.get(&closed_port), Some(&false));

        drop(listener);
    }
}
