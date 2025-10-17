use anyhow::{Result, Context};
use std::path::{PathBuf};
use uuid::Uuid;
use crate::downloader::Downloader;
use crate::gallery;

#[derive(Debug, serde::Deserialize)]
pub struct ImportPayload {
    #[serde(rename = "type")] pub kind: String,
    pub title: String,
    pub author: String,
    pub jsonUrl: String,
    pub imageUrl: Option<String>,
}

pub async fn import_design(config: &crate::config::Config, payload: ImportPayload) -> Result<(String, Option<String>)> {
    let install = PathBuf::from(&config.installation_path);
    let target_dir = if payload.kind.to_lowercase() == "layout" { install.join("Makeplace").join("Save") } else { install.join("Makeplace").join("Custom") };
    std::fs::create_dir_all(&target_dir).context("Failed to create target dir")?;

    let sanitized_title = sanitize(&payload.title);
    let sanitized_author = sanitize(&payload.author);
    let label = if payload.kind.to_lowercase() == "layout" { "Layout" } else { "Design" };
    let filename = format!("[{}] {} by {}.json", label, sanitized_title, sanitized_author);
    let json_path = target_dir.join(&filename);

    // Download JSON
    Downloader::download_file_with_resume(&payload.jsonUrl, &json_path, false, |_| {}).await
        .context("Failed to download JSON")?;

    // Download image (optional)
    let mut saved_image: Option<String> = None;
    if let Some(url) = payload.imageUrl.as_ref() {
        let images_dir = gallery::get_images_dir();
        let img_name = format!("{}.jpg", Uuid::new_v4());
        let img_path = images_dir.join(img_name);
        let _ = Downloader::download_file_with_resume(url, &img_path, false, |_| {}).await;
        if img_path.exists() {
            saved_image = Some(img_path.to_string_lossy().to_string());
        }
    }

    // Add to gallery DB
    let id = Uuid::new_v4().to_string();
    gallery::add_entry(&id, &payload.title, &payload.kind, &payload.author, &json_path.to_string_lossy(), saved_image.as_deref())?;

    Ok((json_path.to_string_lossy().to_string(), saved_image))
}

fn sanitize(s: &str) -> String {
    let mut out = s.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-' || *c == '_').collect::<String>();
    out = out.trim().to_string();
    if out.is_empty() { "Untitled".to_string() } else { out }
}


