use std::path::{PathBuf};
use rusqlite::{Connection, params};
use anyhow::{Result, Context};
use chrono::Utc;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GalleryItemDto {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub author: String,
    pub json_path: String,
    pub image_path: Option<String>,
    pub added_at: i64,
}

pub fn get_app_data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("ReMakeplaceLauncher").join("gallery")
}

fn get_db_path() -> PathBuf {
    get_app_data_dir().join("gallery.db")
}

pub fn init_db() -> Result<()> {
    let dir = get_app_data_dir();
    std::fs::create_dir_all(&dir).context("Failed to create gallery data dir")?;
    let conn = Connection::open(get_db_path()).context("Failed to open gallery DB")?;
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        CREATE TABLE IF NOT EXISTS designs (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            kind TEXT NOT NULL,
            author TEXT NOT NULL,
            json_path TEXT NOT NULL,
            image_path TEXT,
            added_at INTEGER NOT NULL
        );
        "#,
    ).context("Failed to create table")?;
    Ok(())
}

pub fn add_entry(id: &str, title: &str, kind: &str, author: &str, json_path: &str, image_path: Option<&str>) -> Result<()> {
    let conn = Connection::open(get_db_path()).context("Failed to open DB for insert")?;
    conn.execute(
        "INSERT OR REPLACE INTO designs (id, title, kind, author, json_path, image_path, added_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![id, title, kind, author, json_path, image_path, Utc::now().timestamp()],
    ).context("Failed to insert gallery entry")?;
    Ok(())
}

pub fn list_entries() -> Result<Vec<GalleryItemDto>> {
    let conn = Connection::open(get_db_path()).context("Failed to open DB for list")?;
    let mut stmt = conn.prepare("SELECT id, title, kind, author, json_path, image_path, added_at FROM designs ORDER BY added_at DESC")
        .context("Prepare list query failed")?;
    let rows = stmt.query_map([], |row| {
        Ok(GalleryItemDto {
            id: row.get(0)?,
            title: row.get(1)?,
            kind: row.get(2)?,
            author: row.get(3)?,
            json_path: row.get(4)?,
            image_path: row.get(5).ok(),
            added_at: row.get(6)?,
        })
    }).context("Query map failed")?;

    let mut items = Vec::new();
    for item in rows {
        items.push(item.context("Row mapping failed")?);
    }
    Ok(items)
}

pub fn get_images_dir() -> PathBuf {
    let dir = get_app_data_dir().join("images");
    let _ = std::fs::create_dir_all(&dir);
    dir
}


