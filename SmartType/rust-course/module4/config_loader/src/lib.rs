use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub debug: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    MissingKey(&'static str),
    InvalidPort(String),
    InvalidBool(String),
    Io(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingKey(key) => write!(f, "missing required key: {key}"),
            ConfigError::InvalidPort(val) => write!(f, "invalid port: {val}"),
            ConfigError::InvalidBool(val) => write!(f, "invalid bool: {val}"),
            ConfigError::Io(err) => write!(f, "io error: {err}"),
        }
    }
}

impl Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::Io(e.to_string())
    }
}

/// Load configuration from a key=value file, then apply environment overrides.
/// Required keys: host, port. Optional: debug (default false).
pub fn load_config(path: &str) -> Result<AppConfig, ConfigError> {
    let contents = fs::read_to_string(path)?;
    let mut map = parse_kv(&contents);
    apply_env_overrides(&mut map);
    build_config(&map)
}

fn parse_kv(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_lowercase(), value.trim().to_string());
        }
    }
    map
}

fn apply_env_overrides(map: &mut HashMap<String, String>) {
    if let Ok(host) = env::var("APP_HOST") {
        map.insert("host".to_string(), host);
    }
    if let Ok(port) = env::var("APP_PORT") {
        map.insert("port".to_string(), port);
    }
    if let Ok(debug) = env::var("APP_DEBUG") {
        map.insert("debug".to_string(), debug);
    }
}

fn build_config(map: &HashMap<String, String>) -> Result<AppConfig, ConfigError> {
    let host = map
        .get("host")
        .cloned()
        .ok_or(ConfigError::MissingKey("host"))?;

    let port_raw = map
        .get("port")
        .cloned()
        .ok_or(ConfigError::MissingKey("port"))?;
    let port = port_raw
        .parse::<u16>()
        .map_err(|_| ConfigError::InvalidPort(port_raw.clone()))?;

    let debug = match map.get("debug") {
        Some(val) => parse_bool(val).map_err(|_| ConfigError::InvalidBool(val.clone()))?,
        None => false,
    };

    Ok(AppConfig { host, port, debug })
}

fn parse_bool(s: &str) -> Result<bool, ()> {
    match s.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "y" => Ok(true),
        "false" | "0" | "no" | "n" => Ok(false),
        _ => Err(()),
    }
}

pub fn exit_code_for_error(err: &ConfigError) -> i32 {
    match err {
        ConfigError::MissingKey(_) => 2,
        ConfigError::InvalidPort(_) | ConfigError::InvalidBool(_) => 2,
        ConfigError::Io(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_app_env() {
        unsafe {
            env::remove_var("APP_HOST");
            env::remove_var("APP_PORT");
            env::remove_var("APP_DEBUG");
        }
    }

    fn write_temp_file(content: &str) -> PathBuf {
        let mut path = env::temp_dir();
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        path.push(format!("config_test_{id}.txt"));
        let mut file = fs::File::create(&path).unwrap();
        writeln!(file, "{}", content).unwrap();
        path
    }

    #[test]
    fn parses_basic_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_app_env();
        let path = write_temp_file("host=localhost\nport=8080\ndebug=true\n");
        let cfg = load_config(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.port, 8080);
        assert!(cfg.debug);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn applies_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_app_env();
        let path = write_temp_file("host=localhost\nport=8080\n");
        unsafe {
            env::set_var("APP_HOST", "example.com");
            env::set_var("APP_PORT", "9090");
            env::set_var("APP_DEBUG", "yes");
        }

        let cfg = load_config(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.host, "example.com");
        assert_eq!(cfg.port, 9090);
        assert!(cfg.debug);

        clear_app_env();
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_missing_keys() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_app_env();
        let path = write_temp_file("host=localhost\n");
        let err = load_config(path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, ConfigError::MissingKey("port")));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_invalid_values() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_app_env();
        unsafe {
            env::set_var("APP_PORT", "notanumber");
        }
        let path = write_temp_file("host=localhost\nport=notanumber\n");
        let err = load_config(path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, ConfigError::InvalidPort(_)));
        clear_app_env();
        let _ = fs::remove_file(path);
    }
}
