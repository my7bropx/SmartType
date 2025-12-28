# Final Capstone Projects

Choose one of the capstone directions and evolve it into a polished project.

## Options
- **Endpoint telemetry agent**: watch processes and file changes, output JSON events, async pipeline + rules engine.
- **Pentest tooling**: fast port scanner with async scanning, banners, output formats.
- **Git-like CLI tool**: subcommands, config, logging, tests, async network.

## Implemented starter: Port Scanner (`capstone/port_scanner/`)
- Scans a host over a list of ports using scoped threads and connection timeouts.
- CLI: `cargo run --manifest-path capstone/port_scanner/Cargo.toml -- 127.0.0.1 22,80,443 200 8`
- Library exposes `scan` returning per-port results; tests spin up a local listener to validate open/closed detection.

Next steps: add banner grabbing, JSON/CSV output, rate limiting, and async runtime integration.
