use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub current_version: String,
    pub github_repo: String,
    pub installation_path: String,
    pub exe_path: String,
    pub preserve_folders: Vec<String>,
    pub update_check_url: String,
    pub last_check: String,
    pub auto_check: bool,
    #[serde(default = "default_installation_mode")]
    pub installation_mode: InstallationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallationMode {
    Update,
    FreshInstall,
}

fn default_installation_mode() -> InstallationMode {
    InstallationMode::Update
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn load_config() -> Result<Config> {
        let config_path = Self::get_config_path();
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config.json")?;
            
            let config: Config = serde_json::from_str(&content)
                .context("Failed to parse config.json")?;
            
            Ok(config)
        } else {
            let default_config = Self::create_default();
            Self::save_config(&default_config)?;
            Ok(default_config)
        }
    }

    pub fn save_config(config: &Config) -> Result<()> {
        let config_path = Self::get_config_path();
        let content = serde_json::to_string_pretty(config)
            .context("Failed to serialize config")?;
        
        fs::write(&config_path, content)
            .context("Failed to write config.json")?;
        
        Ok(())
    }

    pub fn create_default() -> Config {
        Config {
            current_version: "0.0.0".to_string(),
            github_repo: "RemakePlace/app".to_string(),
            installation_path: String::new(),
            exe_path: "Makeplace.exe".to_string(),
            preserve_folders: vec![
                "Makeplace/Custom".to_string(),
                "Makeplace/Save".to_string(),
            ],
            update_check_url: "https://api.github.com/repos/RemakePlace/app/releases/latest".to_string(),
            last_check: chrono::Utc::now().to_rfc3339(),
            auto_check: true,
            installation_mode: InstallationMode::Update,
        }
    }

    pub fn validate_installation_path(path: &str, exe_name: &str, mode: &InstallationMode) -> bool {
        if path.is_empty() {
            return false;
        }

        let path_buf = PathBuf::from(path);
        if !path_buf.exists() || !path_buf.is_dir() {
            return false;
        }

        match mode {
            InstallationMode::Update => {
                // For updates, exe must exist
                let exe_path = path_buf.join(exe_name);
                exe_path.exists() && exe_path.is_file()
            }
            InstallationMode::FreshInstall => {
                // For fresh installs, directory just needs to exist and be writable
                true
            }
        }
    }

    pub fn detect_installation_mode(path: &str, exe_name: &str) -> InstallationMode {
        if path.is_empty() {
            return InstallationMode::FreshInstall;
        }

        let path_buf = PathBuf::from(path);
        if !path_buf.exists() {
            return InstallationMode::FreshInstall;
        }

        let exe_path = path_buf.join(exe_name);
        if exe_path.exists() && exe_path.is_file() {
            InstallationMode::Update
        } else {
            InstallationMode::FreshInstall
        }
    }

    fn get_config_path() -> PathBuf {
        // Use current directory for config.json to maintain compatibility
        PathBuf::from("config.json")
    }
}

impl Default for Config {
    fn default() -> Self {
        ConfigManager::create_default()
    }
} 