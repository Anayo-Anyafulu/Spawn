use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub search_dir: PathBuf,
    pub install_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            search_dir: dirs_next::download_dir().unwrap_or_else(|| PathBuf::from(".")),
            install_dir: dirs_next::home_dir().map(|h| h.join("Games")).unwrap_or_else(|| PathBuf::from(".")),
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs_next::config_dir()
        .ok_or_else(|| anyhow!("Could not find config directory"))?
        .join("spawn");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir.join("config.toml"))
}

pub fn load_config() -> Config {
    let path = match get_config_path() {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };
    
    fs::read_to_string(path)
        .and_then(|s| toml::from_str(&s).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)))
        .unwrap_or_else(|_| Config::default())
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = get_config_path()?;
    let s = toml::to_string(config).map_err(|e| anyhow!("Failed to serialize config: {}", e))?;
    fs::write(path, s).context("Failed to write config file")
}
