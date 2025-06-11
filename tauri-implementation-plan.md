# ReMakeplace Auto-Updater - Tauri/Rust Implementation Guide

## Project Structure

```
ReMakeplaceUpdater/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # Main entry point
│   │   ├── lib.rs            # Core app logic
│   │   ├── config.rs         # Configuration management
│   │   ├── updater.rs        # Update logic
│   │   ├── downloader.rs     # Download manager
│   │   ├── extractor.rs      # Archive extraction
│   │   └── launcher.rs       # Game launcher
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
├── src/
│   ├── main.ts               # Frontend entry
│   ├── style.css             # Styling
│   └── types.ts              # TypeScript definitions
├── dist/                     # Frontend build output
├── config.json               # User configuration (preserved)
└── package.json
```

## Required Dependencies

### Rust Crates (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2.0", features = ["protocol-asset", "window-create", "process-command"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "stream"] }
sevenz-rust = "0.4"                    # 7z extraction
zip = "0.6"                            # ZIP fallback
semver = "1.0"                         # Version comparison
dirs = "5.0"                           # Standard directories
anyhow = "1.0"                         # Error handling
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
```

### Tauri Plugins

```bash
pnpm tauri add fs              # File system operations
pnpm tauri add http            # HTTP client for downloads
pnpm tauri add dialog          # Dialog for folder selection
pnpm tauri add shell           # Process management for launching game
pnpm tauri add notification    # Notifications
```

## Rust Backend Implementation

### Configuration Management (`config.rs`)

```rust
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
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn load_config() -> Result<Config, anyhow::Error>
    pub fn save_config(config: &Config) -> Result<(), anyhow::Error>
    pub fn create_default() -> Config
    pub fn validate_installation_path(path: &str, exe_name: &str) -> bool
}
```

### Update Manager (`updater.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub download_url: String,
    pub is_available: bool,
}

pub struct UpdateManager;

impl UpdateManager {
    pub async fn check_for_updates(config: &Config) -> Result<UpdateInfo, anyhow::Error>
    pub fn compare_versions(current: &str, latest: &str) -> Result<bool, anyhow::Error>
    pub async fn get_download_url(repo_url: &str) -> Result<String, anyhow::Error>
}
```

### Download Manager (`downloader.rs`)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ProgressInfo {
    pub percentage: f64,
    pub speed: f64,         // MB/s
    pub downloaded: u64,
    pub total: u64,
}

pub struct Downloader;

impl Downloader {
    pub async fn download_file<F>(
        url: &str,
        filepath: &Path,
        progress_callback: F
    ) -> Result<(), anyhow::Error>
    where F: Fn(ProgressInfo)

    pub fn verify_download(filepath: &Path, expected_size: u64) -> Result<bool, anyhow::Error>
    pub fn manage_cache(cache_dir: &Path, keep_current: bool) -> Result<(), anyhow::Error>
}
```

### Archive Extractor (`extractor.rs`)

```rust
pub struct Extractor;

impl Extractor {
    pub async fn extract_archive(
        archive_path: &Path,
        destination: &Path
    ) -> Result<(), anyhow::Error>

    async fn extract_7z(archive_path: &Path, destination: &Path) -> Result<(), anyhow::Error>
    async fn extract_zip(archive_path: &Path, destination: &Path) -> Result<(), anyhow::Error>
    async fn extract_with_system_7z(archive_path: &Path, destination: &Path) -> Result<(), anyhow::Error>
    pub fn validate_archive(archive_path: &Path) -> Result<bool, anyhow::Error>
}
```

### Game Launcher (`launcher.rs`)

```rust
pub struct Launcher;

impl Launcher {
    pub async fn launch_game(
        installation_path: &Path,
        exe_name: &str
    ) -> Result<(), anyhow::Error>

    pub fn validate_executable(exe_path: &Path) -> Result<bool, anyhow::Error>
}
```

## Tauri Commands (Rust ↔ Frontend Interface)

```rust
#[tauri::command]
async fn load_config() -> Result<Config, String>

#[tauri::command]
async fn save_config(config: Config) -> Result<(), String>

#[tauri::command]
async fn validate_path(path: String) -> Result<bool, String>

#[tauri::command]
async fn check_updates() -> Result<UpdateInfo, String>

#[tauri::command]
async fn start_download(url: String, path: String) -> Result<(), String>

#[tauri::command]
async fn install_update(archive_path: String) -> Result<(), String>

#[tauri::command]
async fn launch_game() -> Result<(), String>

#[tauri::command]
async fn browse_folder() -> Result<Option<String>, String>
```

## Tauri Events (Backend → Frontend)

```typescript
// Progress updates
listen<ProgressInfo>("download-progress", (event) => {
  updateProgressBar(event.payload.percentage, event.payload.speed);
});

// Status updates
listen<string>("status-update", (event) => {
  updateStatusMessage(event.payload);
});

// Error notifications
listen<string>("error", (event) => {
  showError(event.payload);
});
```

## Frontend Implementation

### TypeScript Types (`types.ts`)

```typescript
export interface Config {
  current_version: string;
  github_repo: string;
  installation_path: string;
  exe_path: string;
  preserve_folders: string[];
  update_check_url: string;
  last_check: string;
  auto_check: boolean;
}

export interface UpdateInfo {
  latest_version: string;
  download_url: string;
  is_available: boolean;
}

export interface ProgressInfo {
  percentage: number;
  speed: number;
  downloaded: number;
  total: number;
}
```

### UI Components Structure

```typescript
// Main UI elements to implement
class HeaderComponent {
  // Orange gradient header with title
}

class SettingsDialog {
  // Modal dialog for path configuration
  // Real-time path validation with visual feedback
}

class VersionDisplay {
  // Current and latest version information
}

class ProgressBar {
  // Download/install progress with speed indicator
}

class ButtonGroup {
  // Update and Launch buttons
}

class StatusDisplay {
  // Status messages and error display
}
```

## Window Configuration (`tauri.conf.json`)

```json
{
  "build": {
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build",
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "tauri": {
    "bundle": {
      "identifier": "com.remakeplace.updater",
      "icon": ["icons/icon.png"],
      "targets": ["msi", "nsis"]
    },
    "windows": [
      {
        "title": "ReMakeplace Launcher",
        "width": 600,
        "height": 550,
        "minWidth": 600,
        "minHeight": 500,
        "resizable": true,
        "center": true,
        "decorations": true,
        "theme": "Dark"
      }
    ]
  }
}
```

## CSS Styling Requirements

```css
/* Color scheme */
:root {
  --primary-orange: #ff8c42;
  --secondary-orange: #e6732a;
  --dark-bg: #2b2b2b;
  --darker-bg: #1a1a1a;
  --text-primary: #ffffff;
  --text-secondary: #cccccc;
  --success-green: #4caf50;
  --error-red: #f44336;
}

/* Layout requirements */
.header {
  background: linear-gradient(135deg, var(--primary-orange), var(--secondary-orange));
  height: 80px;
  color: var(--text-primary);
}

.progress-bar {
  background-color: var(--primary-orange);
}

/* Modern, MMO-style appearance with smooth animations */
```

## Key Implementation Details

### Update Process Flow

1. **Check for updates** via GitHub API
2. **Compare versions** using semantic versioning
3. **Download to cache** with progress tracking
4. **Backup user data** (Custom/Save folders)
5. **Extract archive** with multiple fallback methods
6. **Restore user data** after extraction
7. **Update configuration** with new version
8. **Clean up cache** and temporary files

### Archive Extraction Strategy

1. **Primary**: Use `sevenz-rust` crate for 7z files
2. **Fallback 1**: Shell out to system 7zip.exe if available
3. **Fallback 2**: Try standard zip extraction for compatibility
4. **Error handling**: User-friendly messages for BCJ2 compression issues

### Data Preservation

- **Preserve folders**: `Makeplace/Custom`, `Makeplace/Save`
- **Backup to**: `temp_backup/` directory during updates
- **Atomic operations**: Ensure no data loss during failures
- **Restore process**: Copy back preserved data after extraction

### Cache Management

- **Cache location**: `update_cache/` directory
- **Filename format**: `v{version}_{original_filename}`
- **Reuse logic**: Check cache before downloading
- **Cleanup**: Remove old versions, keep current

### Error Handling

- **User-friendly messages** with actionable guidance
- **Graceful degradation** when features fail
- **Retry mechanisms** for transient failures
- **Recovery options** for common error scenarios

## Build Configuration

### Development

```bash
pnpm tauri dev
```

### Production Build

```bash
pnpm tauri build
```

### Output Targets

- Single executable (.exe for Windows)
- MSI installer
- NSIS installer
- Portable version (standalone .exe)

## Requirements Summary

- **Complete feature parity** with Python version
- **Identical UI/UX** - users should notice no difference
- **Better performance** - faster startup and downloads
- **Smaller size** - single executable with no dependencies
- **Windows-first** - optimized for Windows 10+
- **Backward compatible** - preserve existing config.json
