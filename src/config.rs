use serde::Deserialize;
use config::{Config as ConfigLib, ConfigError, Environment, File};
use std::net::SocketAddr;
use once_cell::sync::Lazy;

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn address(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Failed to parse server address")
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggerConfig {
    pub level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub logger: LoggerConfig,
    pub mock_event_min_delay_secs: u64,
    pub mock_event_max_delay_secs: u64,
}

impl AppConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = ConfigLib::builder()
            // Start off by merging in the default configuration file
            .add_source(File::with_name("config/default"))
            // Add in the current environment file
            // Default to 'development' env
            // Note that this file is optional, and may not be present
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_SERVER_PORT=8080` would set `config.server.port`
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        s.try_deserialize()
    }
}

// Global application configuration instance
pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    AppConfig::new().expect("Failed to load application configuration")
});

// Function to create default config files if they don't exist
pub fn ensure_config_files_exist() -> std::io::Result<()> {
    std::fs::create_dir_all("config")?;
    let default_config_path = "config/default.toml";
    if std::fs::metadata(default_config_path).is_err() {
        let default_toml_content = r#"
[server]
host = "127.0.0.1"
port = 8080

[logger]
level = "info"

mock_event_min_delay_secs = 5
mock_event_max_delay_secs = 15
        "#;
        std::fs::write(default_config_path, default_toml_content)?;
        println!("Created default configuration file: {}", default_config_path);
    }

    let dev_config_path = "config/development.toml";
    if std::fs::metadata(dev_config_path).is_err() {
        let dev_toml_content = r#"
# Development specific settings can override defaults here
# Example:
# [logger]
# level = "debug"
        "#;
        std::fs::write(dev_config_path, dev_toml_content)?;
        println!("Created development configuration file: {}", dev_config_path);
    }
    Ok(())
} 