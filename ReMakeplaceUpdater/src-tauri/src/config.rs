use crate::error_handler::{ErrorHandler, ErrorInfo};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
            let content = fs::read_to_string(&config_path).context("Failed to read config.json")?;

            let config: Config =
                serde_json::from_str(&content).context("Failed to parse config.json")?;

            Ok(config)
        } else {
            let default_config = Self::create_default();
            Self::save_config(&default_config)?;
            Ok(default_config)
        }
    }

    pub fn save_config(config: &Config) -> Result<()> {
        let config_path = Self::get_config_path();
        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&config_path, content).context("Failed to write config.json")?;

        Ok(())
    }

    pub fn create_default() -> Config {
        Config {
            current_version: "0.0.0".to_string(),
            github_repo: "RemakePlace/app".to_string(),
            installation_path: String::new(),
            exe_path: "Makeplace.exe".to_string(),
            preserve_folders: vec!["Makeplace/Custom".to_string(), "Makeplace/Save".to_string()],
            update_check_url: "https://api.github.com/repos/RemakePlace/app/releases/latest"
                .to_string(),
            last_check: chrono::Utc::now().to_rfc3339(),
            auto_check: true,
            installation_mode: InstallationMode::Update,
        }
    }

    pub fn validate_installation_path(path: &str, exe_name: &str, mode: &InstallationMode) -> bool {
        Self::validate_installation_path_detailed(path, exe_name, mode).is_ok()
    }

    /// Enhanced path validation that provides detailed error information
    pub fn validate_installation_path_detailed(
        path: &str,
        exe_name: &str,
        mode: &InstallationMode,
    ) -> Result<(), ErrorInfo> {
        if path.is_empty() {
            return Err(ErrorInfo {
                category: crate::error_handler::ErrorCategory::Validation,
                user_message: "Please select an installation directory.".to_string(),
                technical_details: "Empty path provided".to_string(),
                recovery_suggestion: "Use the Browse button to select a folder.".to_string(),
                is_retryable: false,
            });
        }

        let path_buf = PathBuf::from(path);

        // Check if path exists
        if !path_buf.exists() {
            return Err(ErrorInfo {
                category: crate::error_handler::ErrorCategory::FileSystem,
                user_message: "The selected directory does not exist.".to_string(),
                technical_details: format!("Path does not exist: {}", path),
                recovery_suggestion: "Create the directory or select an existing one.".to_string(),
                is_retryable: false,
            });
        }

        // Check if it's a directory
        if !path_buf.is_dir() {
            return Err(ErrorInfo {
                category: crate::error_handler::ErrorCategory::Validation,
                user_message: "The selected path is not a directory.".to_string(),
                technical_details: format!("Path is not a directory: {}", path),
                recovery_suggestion: "Select a directory, not a file.".to_string(),
                is_retryable: false,
            });
        }

        // Test write permissions
        let test_file = path_buf.join(".write_test");
        if let Err(e) = std::fs::write(&test_file, "test") {
            return Err(ErrorInfo {
                category: crate::error_handler::ErrorCategory::Permission,
                user_message: "Cannot write to the selected directory.".to_string(),
                technical_details: format!("Write permission test failed: {}", e),
                recovery_suggestion: "Choose a different directory or run as administrator."
                    .to_string(),
                is_retryable: false,
            });
        } else {
            let _ = std::fs::remove_file(&test_file); // Clean up test file
        }

        match mode {
            InstallationMode::Update => {
                // For updates, exe must exist
                let exe_path = path_buf.join(exe_name);
                if !exe_path.exists() {
                    return Err(ErrorInfo {
                        category: crate::error_handler::ErrorCategory::Validation,
                        user_message: format!(
                            "Could not find {} in the selected directory.",
                            exe_name
                        ),
                        technical_details: format!("Executable not found: {}", exe_path.display()),
                        recovery_suggestion:
                            "Select the directory containing your ReMakeplace installation."
                                .to_string(),
                        is_retryable: false,
                    });
                }

                if !exe_path.is_file() {
                    return Err(ErrorInfo {
                        category: crate::error_handler::ErrorCategory::Validation,
                        user_message: format!(
                            "{} exists but is not a valid executable file.",
                            exe_name
                        ),
                        technical_details: format!("Path is not a file: {}", exe_path.display()),
                        recovery_suggestion:
                            "Select the correct directory containing the ReMakeplace executable."
                                .to_string(),
                        is_retryable: false,
                    });
                }
            }
            InstallationMode::FreshInstall => {
                // For fresh installs, check if directory is empty or warn if it contains files
                if let Ok(entries) = std::fs::read_dir(&path_buf) {
                    let count = entries.count();
                    if count > 0 {
                        // This is a warning, not an error - we still allow it
                        println!("Warning: Directory is not empty, installation will merge files");
                    }
                }
            }
        }

        Ok(())
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

    /// Get a user-friendly description of the detected installation mode
    pub fn get_mode_description(mode: &InstallationMode) -> &'static str {
        match mode {
            InstallationMode::Update => {
                "Existing installation detected - updates will preserve your data"
            }
            InstallationMode::FreshInstall => {
                "No existing installation found - will perform fresh install"
            }
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
