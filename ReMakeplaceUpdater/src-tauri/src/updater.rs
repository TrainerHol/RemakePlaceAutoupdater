use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
    pub is_available: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

pub struct UpdateManager;

impl UpdateManager {
    pub async fn check_for_updates(config: &Config) -> Result<UpdateInfo> {
        let latest_release = Self::get_latest_release(&config.update_check_url).await?;
        let latest_version = latest_release.tag_name.trim_start_matches('v').to_string();

        let is_available = Self::compare_versions(&config.current_version, &latest_version)?;

        let download_url = if is_available {
            Self::find_7z_asset(&latest_release.assets).unwrap_or_else(|| "".to_string())
        } else {
            "".to_string()
        };

        Ok(UpdateInfo {
            latest_version,
            download_url,
            is_available,
        })
    }

    pub fn compare_versions(current: &str, latest: &str) -> Result<bool> {
        let current_version =
            semver::Version::parse(current).context("Failed to parse current version")?;
        let latest_version =
            semver::Version::parse(latest).context("Failed to parse latest version")?;

        Ok(latest_version > current_version)
    }

    async fn get_latest_release(url: &str) -> Result<GitHubRelease> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "ReMakeplace-Updater")
            .send()
            .await
            .context("Failed to fetch latest release")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "GitHub API returned status: {}",
                response.status()
            ));
        }

        let release: GitHubRelease = response
            .json()
            .await
            .context("Failed to parse GitHub API response")?;

        Ok(release)
    }

    fn find_7z_asset(assets: &[GitHubAsset]) -> Option<String> {
        // Look for .7z files first (preferred)
        for asset in assets {
            if asset.name.ends_with(".7z") {
                println!("Found .7z asset: {}", asset.name);
                return Some(asset.browser_download_url.clone());
            }
        }

        // Fallback to .zip files
        for asset in assets {
            if asset.name.ends_with(".zip") {
                println!("Found .zip asset (fallback): {}", asset.name);
                return Some(asset.browser_download_url.clone());
            }
        }

        // Additional fallbacks for other supported formats
        for asset in assets {
            if asset.name.ends_with(".tar.gz") || asset.name.ends_with(".tgz") {
                println!("Found .tar.gz asset (fallback): {}", asset.name);
                return Some(asset.browser_download_url.clone());
            }
        }

        for asset in assets {
            if asset.name.ends_with(".tar.zst") || asset.name.ends_with(".tar.zstd") {
                println!("Found .tar.zst asset (fallback): {}", asset.name);
                return Some(asset.browser_download_url.clone());
            }
        }

        // Log what assets are available for debugging
        println!("No supported archive format found. Available assets:");
        for asset in assets {
            println!("  - {}", asset.name);
        }

        None
    }
}
