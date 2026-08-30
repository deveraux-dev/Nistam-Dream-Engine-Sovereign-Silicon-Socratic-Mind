//! Configuration -- TOML-based, auto-generated on first run. Trimmed to the
//! connection + TLS fields this port actually uses: the donor's
//! `HttpConfig`/`VoiceConfig`/`CacheConfig`/`NotificationConfig`/
//! `PluginConfig` all belonged to features (HTTP API, voice relay, message
//! cache, notification DND, chat-app plugins) this plan deliberately does
//! not port -- carrying their schema forward with nothing behind it would
//! be dead config, not a real capability.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Pairing/discovery/reconnect settings.
    pub connection: ConnectionConfig,
    /// TLS cert/key file locations.
    pub tls: TlsConfig,
}

/// Pairing, discovery, and reconnect settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// This device's stable UUID.
    pub device_id: String,
    /// This device's display name (defaults to the hostname).
    pub device_name: String,
    /// TCP port the desktop listens on for paired connections.
    pub listen_port: u16,
    /// UDP port the desktop broadcasts discovery announcements on.
    pub discovery_port: u16,
    /// Ceiling for the reconnect exponential backoff, seconds.
    pub reconnect_max_backoff_secs: u64,
}

/// TLS cert/key file locations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to the certificate PEM file.
    pub cert_path: String,
    /// Path to the private key PEM file.
    pub key_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            connection: ConnectionConfig {
                device_id: uuid::Uuid::new_v4().to_string(),
                device_name: hostname(),
                listen_port: 13131,
                discovery_port: 13132,
                reconnect_max_backoff_secs: 60,
            },
            tls: TlsConfig { cert_path: String::new(), key_path: String::new() },
        }
    }
}

impl Config {
    /// Platform config directory (~/.config/13link or equivalent).
    pub fn config_dir() -> PathBuf {
        let base = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("13link")
    }

    /// Path to config.toml.
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Load config from disk, creating default if missing.
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Save config to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}

fn hostname() -> String {
    gethostname::gethostname().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let back: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.connection.listen_port, 13131);
        assert_eq!(back.connection.discovery_port, 13132);
        assert_eq!(back.tls.cert_path, "");
    }
}
