use crate::error_handler::ErrorInfo;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_EXE_NAME: &str = "MakePlace.exe";
const LEGACY_EXE_NAME: &str = "Makeplace.exe";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstallationStatus {
    FreshEmpty,
    ExistingValid,
    ExistingIncomplete,
    InvalidPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationDetection {
    pub status: InstallationStatus,
    pub mode: InstallationMode,
    pub normalized_path: String,
    pub exe_path: Option<String>,
    pub content_path: Option<String>,
    pub datasmith_path: Option<String>,
    pub custom_path: Option<String>,
    pub save_path: Option<String>,
    pub message: String,
    pub details: Vec<String>,
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

            let mut config: Config =
                serde_json::from_str(&content).context("Failed to parse config.json")?;
            if Self::normalize_config(&mut config) {
                Self::save_config(&config)?;
            }

            Ok(config)
        } else {
            let default_config = Self::create_default();
            Self::save_config(&default_config)?;
            Ok(default_config)
        }
    }

    pub fn save_config(config: &Config) -> Result<()> {
        let config_path = Self::get_config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        fs::write(&config_path, content).context("Failed to write config.json")?;

        Ok(())
    }

    pub fn create_default() -> Config {
        Config {
            current_version: "0.0.0".to_string(),
            github_repo: "RemakePlace/app".to_string(),
            installation_path: String::new(),
            exe_path: DEFAULT_EXE_NAME.to_string(),
            preserve_folders: vec!["Makeplace/Custom".to_string(), "Makeplace/Save".to_string()],
            update_check_url: "https://api.github.com/repos/RemakePlace/app/releases/latest"
                .to_string(),
            last_check: chrono::Utc::now().to_rfc3339(),
            auto_check: true,
            installation_mode: InstallationMode::Update,
        }
    }

    fn normalize_config(config: &mut Config) -> bool {
        if config.exe_path == LEGACY_EXE_NAME {
            config.exe_path = DEFAULT_EXE_NAME.to_string();
            return true;
        }

        false
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

        let detection = Self::detect_installation(path, exe_name);
        if detection.status == InstallationStatus::InvalidPath {
            return Err(ErrorInfo {
                category: crate::error_handler::ErrorCategory::Validation,
                user_message: detection.message,
                technical_details: detection.details.join("; "),
                recovery_suggestion:
                    "Select an empty folder for a new install or a ReMakeplace folder to update."
                        .to_string(),
                is_retryable: false,
            });
        }

        if *mode == InstallationMode::FreshInstall
            && detection.status != InstallationStatus::FreshEmpty
        {
            return Err(ErrorInfo {
                category: crate::error_handler::ErrorCategory::Validation,
                user_message: "This folder already looks like a ReMakeplace installation."
                    .to_string(),
                technical_details: detection.details.join("; "),
                recovery_suggestion:
                    "Use repair/update for this folder or choose an empty folder for a new install."
                        .to_string(),
                is_retryable: false,
            });
        }

        Ok(())
    }

    pub fn detect_installation_mode(path: &str, exe_name: &str) -> InstallationMode {
        Self::detect_installation(path, exe_name).mode
    }

    pub fn detect_installation(path: &str, exe_name: &str) -> InstallationDetection {
        if path.trim().is_empty() {
            return InstallationDetection {
                status: InstallationStatus::InvalidPath,
                mode: InstallationMode::FreshInstall,
                normalized_path: String::new(),
                exe_path: None,
                content_path: None,
                datasmith_path: None,
                custom_path: None,
                save_path: None,
                message: "Choose an installation folder.".to_string(),
                details: vec!["Empty path provided".to_string()],
            };
        }

        let path_buf = PathBuf::from(path);
        let normalized_path = path_buf
            .canonicalize()
            .unwrap_or_else(|_| path_buf.clone())
            .to_string_lossy()
            .to_string();

        if !path_buf.exists() {
            return InstallationDetection {
                status: InstallationStatus::InvalidPath,
                mode: InstallationMode::FreshInstall,
                normalized_path,
                exe_path: None,
                content_path: None,
                datasmith_path: None,
                custom_path: None,
                save_path: None,
                message: "The selected folder does not exist.".to_string(),
                details: vec![format!("Path does not exist: {}", path)],
            };
        }

        if !path_buf.is_dir() {
            return InstallationDetection {
                status: InstallationStatus::InvalidPath,
                mode: InstallationMode::FreshInstall,
                normalized_path,
                exe_path: None,
                content_path: None,
                datasmith_path: None,
                custom_path: None,
                save_path: None,
                message: "The selected path is not a folder.".to_string(),
                details: vec![format!("Path is not a directory: {}", path)],
            };
        }

        if let Err(e) = Self::check_write_access(&path_buf) {
            return InstallationDetection {
                status: InstallationStatus::InvalidPath,
                mode: InstallationMode::FreshInstall,
                normalized_path,
                exe_path: None,
                content_path: None,
                datasmith_path: None,
                custom_path: None,
                save_path: None,
                message: "Cannot write to the selected folder.".to_string(),
                details: vec![format!("Write permission test failed: {}", e)],
            };
        }

        let exe_path = Self::find_child_case_insensitive(&path_buf, exe_name);
        let content_path = Self::find_game_content_dir(&path_buf);
        let datasmith_path =
            Self::find_descendant_dir_case_insensitive(&path_buf, "DatasmithContent", 8);
        let game_dir = content_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(Path::to_path_buf);
        let custom_path = game_dir.as_ref().map(|p| p.join("Custom"));
        let save_path = game_dir.as_ref().map(|p| p.join("Save"));

        let is_empty = fs::read_dir(&path_buf)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);

        if exe_path.is_some() && content_path.is_some() {
            return InstallationDetection {
                status: InstallationStatus::ExistingValid,
                mode: InstallationMode::Update,
                normalized_path,
                exe_path: Self::path_to_string(exe_path),
                content_path: Self::path_to_string(content_path),
                datasmith_path: Self::path_to_string(datasmith_path),
                custom_path: Self::path_to_string(custom_path),
                save_path: Self::path_to_string(save_path),
                message: "Existing ReMakeplace installation detected.".to_string(),
                details: vec!["Required executable and Content folder were found.".to_string()],
            };
        }

        let game_like = exe_path.is_some()
            || content_path.is_some()
            || datasmith_path.is_some()
            || Self::find_child_case_insensitive(&path_buf, "Makeplace").is_some()
            || Self::find_child_case_insensitive(&path_buf, "MakePlace").is_some();

        if game_like {
            let mut details = Vec::new();
            if exe_path.is_none() {
                details.push(format!("Missing executable: {}", exe_name));
            }
            if content_path.is_none() {
                details.push("Missing game content folder: Makeplace/Content".to_string());
            }

            return InstallationDetection {
                status: InstallationStatus::ExistingIncomplete,
                mode: InstallationMode::Update,
                normalized_path,
                exe_path: Self::path_to_string(exe_path),
                content_path: Self::path_to_string(content_path),
                datasmith_path: Self::path_to_string(datasmith_path),
                custom_path: Self::path_to_string(custom_path),
                save_path: Self::path_to_string(save_path),
                message: "Existing installation appears incomplete and can be repaired."
                    .to_string(),
                details,
            };
        }

        if is_empty {
            return InstallationDetection {
                status: InstallationStatus::FreshEmpty,
                mode: InstallationMode::FreshInstall,
                normalized_path,
                exe_path: None,
                content_path: None,
                datasmith_path: None,
                custom_path: Some(
                    path_buf
                        .join("Makeplace")
                        .join("Custom")
                        .to_string_lossy()
                        .to_string(),
                ),
                save_path: Some(
                    path_buf
                        .join("Makeplace")
                        .join("Save")
                        .to_string_lossy()
                        .to_string(),
                ),
                message: "Empty folder ready for a fresh install.".to_string(),
                details: Vec::new(),
            };
        }

        InstallationDetection {
            status: InstallationStatus::InvalidPath,
            mode: InstallationMode::FreshInstall,
            normalized_path,
            exe_path: None,
            content_path: None,
            datasmith_path: None,
            custom_path: None,
            save_path: None,
            message: "This folder is not empty and does not look like ReMakeplace.".to_string(),
            details: vec![format!(
                "Choose an empty folder for a new install or the folder that contains {}.",
                DEFAULT_EXE_NAME
            )],
        }
    }

    pub fn validate_installation_structure(
        path: &Path,
        exe_name: &str,
    ) -> Result<InstallationDetection> {
        let detection = Self::detect_installation(&path.to_string_lossy(), exe_name);
        if detection.status == InstallationStatus::ExistingValid {
            return Ok(detection);
        }

        let detail = if detection.details.is_empty() {
            detection.message.clone()
        } else {
            detection.details.join("; ")
        };

        Err(anyhow::anyhow!(
            "Installed game files failed validation: {}",
            detail
        ))
    }

    pub fn find_installation_root(path: &Path, exe_name: &str) -> Option<PathBuf> {
        let detection = Self::detect_installation(&path.to_string_lossy(), exe_name);
        if detection.status == InstallationStatus::ExistingValid {
            return Some(path.to_path_buf());
        }

        let entries = fs::read_dir(path).ok()?;
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                let detection = Self::detect_installation(&child.to_string_lossy(), exe_name);
                if detection.status == InstallationStatus::ExistingValid {
                    return Some(child);
                }
            }
        }

        None
    }

    pub fn resolve_game_data_folder(installation_path: &str, folder_name: &str) -> Result<PathBuf> {
        let install = PathBuf::from(installation_path);
        let content = Self::find_game_content_dir(&install)
            .unwrap_or_else(|| install.join("Makeplace").join("Content"));
        let game_dir = content
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| install.join("Makeplace"));
        Ok(game_dir.join(folder_name))
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

    pub fn get_config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let new_path = config_dir
            .join("ReMakeplaceAutoupdater")
            .join("config.json");
        let legacy_path = PathBuf::from("config.json");

        if !new_path.exists() && legacy_path.exists() {
            if let Some(parent) = new_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(&legacy_path, &new_path);
        }

        new_path
    }

    fn check_write_access(path: &Path) -> std::io::Result<()> {
        let test_file = path.join(".rmp_write_test");
        fs::write(&test_file, "test")?;
        fs::remove_file(&test_file)
    }

    fn find_game_content_dir(path: &Path) -> Option<PathBuf> {
        for game_dir in ["Makeplace", "MakePlace"] {
            if let Some(dir) = Self::find_child_case_insensitive(path, game_dir) {
                let content = Self::find_child_case_insensitive(&dir, "Content");
                if content.as_ref().is_some_and(|p| p.is_dir()) {
                    return content;
                }
            }
        }
        None
    }

    fn find_child_case_insensitive(path: &Path, name: &str) -> Option<PathBuf> {
        let wanted = name.to_lowercase();
        let entries = fs::read_dir(path).ok()?;
        for entry in entries.flatten() {
            let child_name = entry.file_name().to_string_lossy().to_lowercase();
            if child_name == wanted {
                return Some(entry.path());
            }
        }
        None
    }

    fn find_descendant_dir_case_insensitive(
        path: &Path,
        name: &str,
        max_depth: usize,
    ) -> Option<PathBuf> {
        let wanted = name.to_lowercase();
        let mut stack = vec![(path.to_path_buf(), 0usize)];

        while let Some((dir, depth)) = stack.pop() {
            if depth > max_depth {
                continue;
            }

            let entries = match fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let child = entry.path();
                if !child.is_dir() {
                    continue;
                }

                let child_name = entry.file_name().to_string_lossy().to_lowercase();
                if child_name == wanted {
                    return Some(child);
                }

                stack.push((child, depth + 1));
            }
        }

        None
    }

    fn path_to_string(path: Option<PathBuf>) -> Option<String> {
        path.map(|p| p.to_string_lossy().to_string())
    }
}

impl Default for Config {
    fn default() -> Self {
        ConfigManager::create_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_valid_install(root: &Path) {
        fs::write(root.join(DEFAULT_EXE_NAME), "exe").unwrap();
        fs::create_dir_all(root.join("MakePlace").join("Content")).unwrap();
    }

    #[test]
    fn detects_empty_folder_as_fresh() {
        let dir = TempDir::new().unwrap();
        let detection =
            ConfigManager::detect_installation(&dir.path().to_string_lossy(), "Makeplace.exe");
        assert_eq!(detection.status, InstallationStatus::FreshEmpty);
        assert_eq!(detection.mode, InstallationMode::FreshInstall);
    }

    #[test]
    fn detects_valid_install_case_insensitively() {
        let dir = TempDir::new().unwrap();
        create_valid_install(dir.path());

        let detection =
            ConfigManager::detect_installation(&dir.path().to_string_lossy(), "makeplace.exe");
        assert_eq!(detection.status, InstallationStatus::ExistingValid);
        assert_eq!(detection.mode, InstallationMode::Update);
        assert!(detection.content_path.unwrap().contains("MakePlace"));
        assert!(detection.datasmith_path.is_none());
    }

    #[test]
    fn normalizes_legacy_executable_name() {
        let mut config = ConfigManager::create_default();
        config.exe_path = LEGACY_EXE_NAME.to_string();

        assert!(ConfigManager::normalize_config(&mut config));
        assert_eq!(config.exe_path, DEFAULT_EXE_NAME);
    }

    #[test]
    fn detects_datasmith_when_present_but_does_not_require_it() {
        let dir = TempDir::new().unwrap();
        create_valid_install(dir.path());
        fs::create_dir_all(
            dir.path()
                .join("MakePlace")
                .join("Plugins")
                .join("DatasmithContent"),
        )
        .unwrap();

        let detection =
            ConfigManager::detect_installation(&dir.path().to_string_lossy(), "Makeplace.exe");
        assert_eq!(detection.status, InstallationStatus::ExistingValid);
        assert!(detection.datasmith_path.is_some());
    }

    #[test]
    fn detects_incomplete_install_instead_of_fresh() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Makeplace.exe"), "exe").unwrap();

        let detection =
            ConfigManager::detect_installation(&dir.path().to_string_lossy(), "Makeplace.exe");
        assert_eq!(detection.status, InstallationStatus::ExistingIncomplete);
        assert_eq!(detection.mode, InstallationMode::Update);
        assert!(detection.details.iter().any(|d| d.contains("Content")));
    }

    #[test]
    fn finds_installation_root_under_single_top_level_folder() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("ReMakeplace-V7-50-0");
        fs::create_dir_all(&nested).unwrap();
        create_valid_install(&nested);

        let found = ConfigManager::find_installation_root(dir.path(), "Makeplace.exe").unwrap();
        assert_eq!(found, nested);
    }
}
