use port_scanner::scan;
use std::env;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    if !args.is_empty() {
        args.remove(0);
    }

    if args.len() < 2 {
        eprintln!("Usage: port_scanner <host> <port1,port2,port3> [timeout_ms] [workers]");
        return;
    }

    let host = &args[0];
    let ports: Vec<u16> = args[1]
        .split(',')
        .filter_map(|p| p.parse::<u16>().ok())
        .collect();
    let timeout_ms = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
    let workers = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);

    let results = scan(host, &ports, timeout_ms, workers);
    for r in results {
        let status = if r.open { "open" } else { "closed" };
        println!("{}:{} -> {}", r.addr.ip(), r.addr.port(), status);
    }
}
