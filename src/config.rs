use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    pub providers: Vec<ProviderConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(auth: &str) -> Result<Config> {
        let config: Config = toml::from_str(&format!(
            "[[providers]]\nname = 'test'\ntype = 'cloudflare'\napi_key = 'token'\nzone_id = 'zone'\n{auth}"
        ))?;
        config.validate()?;
        Ok(config)
    }

    #[test]
    fn authentication_config_requires_selected_credentials() {
        for invalid in [
            "",
            "auth = 'none'",
            "auth = 'key'",
            "auth = 'basic'",
            "auth = 'basic'\nusername = 'user'",
            "auth = 'basic'\npassword = 'secret'",
            "auth = 'key'\nkey = ''",
            "auth = 'basic'\nusername = ''\npassword = 'secret'",
            "auth = 'basic'\nusername = 'user'\npassword = ''",
            "auth = 'basic'\nusername = 'user:name'\npassword = 'secret'",
        ] {
            assert!(parse(invalid).is_err(), "accepted invalid auth: {invalid}");
        }
    }

    #[test]
    fn valid_basic_and_key_configs_are_accepted() {
        assert!(parse("auth = 'basic'\nusername = 'user'\npassword = 'pass:word'").is_ok());
        assert!(
            parse("auth = 'basic'\nusername = 'user'\npassword = 'secret'\nkey = 'unused'").is_ok()
        );
        assert!(parse("auth = 'key'\nkey = 'secret'").is_ok());
        let example: Config = toml::from_str(include_str!("../config.example.toml")).unwrap();
        example.validate().unwrap();
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            log_level: default_log_level(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    #[serde(flatten)]
    pub auth: AuthConfig,
    pub api_key: String,
    pub zone_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "auth", rename_all = "lowercase")]
pub enum AuthConfig {
    Basic { username: String, password: String },
    Key { key: String },
}

impl AuthConfig {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Basic { username, password } => {
                anyhow::ensure!(!username.is_empty(), "username must not be empty");
                anyhow::ensure!(!username.contains(':'), "username must not contain ':'");
                anyhow::ensure!(!password.is_empty(), "password must not be empty");
            }
            Self::Key { key } => anyhow::ensure!(!key.is_empty(), "key must not be empty"),
        }
        Ok(())
    }
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        for provider in &self.providers {
            provider.auth.validate().with_context(|| {
                format!(
                    "Invalid authentication config for provider: {}",
                    provider.name
                )
            })?;
        }
        Ok(())
    }

    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.name == name)
    }
}
