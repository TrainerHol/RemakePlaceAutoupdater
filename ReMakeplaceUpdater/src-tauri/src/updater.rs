use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
    pub asset_name: String,
    pub asset_size: u64,
    pub release_url: String,
    pub release_notes: String,
    pub is_available: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

pub struct UpdateManager;

impl UpdateManager {
    pub async fn check_for_updates(config: &Config) -> Result<UpdateInfo> {
        let latest_release = Self::get_latest_release(&config.update_check_url).await?;
        let latest_version = latest_release.tag_name.trim_start_matches('v').to_string();

        let is_available = Self::compare_versions(&config.current_version, &latest_version)?;

        let asset = Self::find_supported_asset(&latest_release.assets).ok_or_else(|| {
            anyhow::anyhow!(
                "No supported ReMakeplace archive was found on the latest GitHub release"
            )
        })?;

        Ok(UpdateInfo {
            latest_version,
            download_url: asset.browser_download_url,
            asset_name: asset.name,
            asset_size: asset.size,
            release_url: latest_release.html_url,
            release_notes: latest_release.body.unwrap_or_default(),
            is_available,
        })
    }

    pub fn compare_versions(current: &str, latest: &str) -> Result<bool> {
        let current_version = match semver::Version::parse(current) {
            Ok(version) => version,
            Err(_) => return Ok(true),
        };
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

    fn find_supported_asset(assets: &[GitHubAsset]) -> Option<GitHubAsset> {
        // Look for .7z files first (preferred)
        for asset in assets {
            if asset.name.ends_with(".7z") {
                println!("Found .7z asset: {}", asset.name);
                return Some(asset.clone());
            }
        }

        // Fallback to .zip files
        for asset in assets {
            if asset.name.ends_with(".zip") {
                println!("Found .zip asset (fallback): {}", asset.name);
                return Some(asset.clone());
            }
        }

        // Additional fallbacks for other supported formats
        for asset in assets {
            if asset.name.ends_with(".tar.gz") || asset.name.ends_with(".tgz") {
                println!("Found .tar.gz asset (fallback): {}", asset.name);
                return Some(asset.clone());
            }
        }

        for asset in assets {
            if asset.name.ends_with(".tar.zst") || asset.name.ends_with(".tar.zstd") {
                println!("Found .tar.zst asset (fallback): {}", asset.name);
                return Some(asset.clone());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str, size: u64) -> GitHubAsset {
        GitHubAsset {
            name: name.to_string(),
            browser_download_url: format!("https://example.invalid/{}", name),
            size,
        }
    }

    #[test]
    fn compare_versions_treats_invalid_current_as_update_available() {
        assert!(UpdateManager::compare_versions("unknown", "7.50.0").unwrap());
    }

    #[test]
    fn supported_asset_prefers_7z_and_preserves_size() {
        let assets = vec![asset("ReMakeplace.zip", 10), asset("ReMakeplace.7z", 20)];
        let selected = UpdateManager::find_supported_asset(&assets).unwrap();
        assert_eq!(selected.name, "ReMakeplace.7z");
        assert_eq!(selected.size, 20);
    }

    #[test]
    fn unsupported_assets_are_rejected() {
        let assets = vec![asset("readme.txt", 10), asset("installer.exe", 20)];
        assert!(UpdateManager::find_supported_asset(&assets).is_none());
    }
}
