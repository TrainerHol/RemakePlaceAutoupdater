# ReMakeplace Auto-Updater 🚀

A modern, native auto-updater for ReMakeplace built with Tauri (Rust + TypeScript). Download updates directly from GitHub releases with smart caching and data preservation.

## Features ✨

- **🔒 Data Preservation**: Automatically preserves Custom and Save folders
- **⚙️ Easy Setup**: Browse and configure installation path with validation

## Installation 📦

### Option 1: Download Pre-built Binary (Recommended)

1. **Download from [Releases](https://github.com/TrainerHol/RemakePlaceAutoupdater/releases)**

   **Installers (Recommended for most users):**

   - **Windows**: `.msi` (Windows Installer) or `.exe` (NSIS Setup)
   - **macOS**: `.dmg` (Disk Image)
   - **Linux**: `.deb` (Debian/Ubuntu), `.rpm` (RedHat/Fedora), or `.AppImage` (Universal)

   **Portable Executables (Advanced users):**

   - **Windows**: `.exe` (Portable - no installation required)
   - **macOS**: `.app.tar.gz` (Portable app bundle)
   - **Linux**: Binary executable (Portable - no installation required)

2. **Installation Options**

   - **Installers**: Run the installer and follow the setup wizard
   - **Portable**: Extract (if needed) and run directly - no installation required

3. **Launch and configure**
   - On first run, click "Browse" to select your ReMakeplace installation folder
   - The app validates that `Makeplace.exe` exists in the selected folder
   - Click "Save & Continue" to proceed

### Option 2: Build from Source

#### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific dependencies:
  - **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
  - **Windows**: No additional dependencies needed
  - **macOS**: No additional dependencies needed

#### Build Steps

```bash
# Clone the repository
git clone https://github.com/TrainerHol/RemakePlaceAutoupdater.git
cd RemakePlaceAutoupdater/ReMakeplaceUpdater

# Install dependencies
npm install

# Development mode
npm run tauri dev

# Build release
npm run tauri build
```

## Configuration ⚙️

The `config.json` file is automatically created with sensible defaults and you can change the repo to any other fork of Makeplace in the future:

```json
{
  "current_version": "0.0.0",
  "github_repo": "RemakePlace/app",
  "installation_path": "",
  "exe_path": "Makeplace.exe",
  "preserve_folders": ["Makeplace/Custom", "Makeplace/Save"],
  "update_check_url": "https://api.github.com/repos/RemakePlace/app/releases/latest",
  "last_check": "2024-01-01T00:00:00Z",
  "auto_check": true
}
```

### Data Protection 🔒

Your important data is automatically preserved during updates:

- **`/Makeplace/Custom/`** - Your custom layouts and configurations
- **`/Makeplace/Save/`** - Your saved data

## Development 👨‍💻

### Project Structure

```
ReMakeplaceUpdater/
├── src/                    # Frontend (TypeScript)
│   ├── main.ts            # Main application logic
│   ├── style.css          # Styling
│   └── types.ts           # Type definitions
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── config.rs      # Configuration management
│   │   ├── updater.rs     # Update checking logic
│   │   ├── downloader.rs  # Download management
│   │   ├── extractor.rs   # Archive extraction
│   │   ├── launcher.rs    # Game launching
│   │   └── lib.rs         # Main Tauri app
│   └── tauri.conf.json    # Tauri configuration
├── dist/                  # Built frontend
├── package.json           # Node.js dependencies
└── README.md             # This file
```

### Key Technologies

- **Frontend**: TypeScript, HTML5, CSS3
- **Backend**: Rust with Tauri framework
- **HTTP Client**: reqwest for GitHub API and downloads
- **Archive Handling**: sevenz-rust for 7z, zip for ZIP files
- **Version Management**: semver for version comparison
- **UI Framework**: Native OS webview with custom styling

### Available Scripts

```bash
# Development
npm run tauri dev          # Start development server
npm run dev               # Frontend only development
npm run build             # Build frontend
npm run tauri build       # Build complete application

# Maintenance
cargo clean               # Clean Rust build cache (in src-tauri/)
npm ci                    # Clean install dependencies
```

## GitHub Actions CI/CD 🔄

The repository includes automated build and release workflows:

- **Multi-platform builds**: Windows, macOS (Intel + Apple Silicon), Linux
- **Automatic releases**: Triggered by version tags (`v*`)
- **Draft releases**: For non-tag pushes
- **Asset uploading**: All platform binaries included

To create a release:

```bash
git tag v1.0.0
git push origin v1.0.0
```

## License 📄

This updater is provided as-is for the ReMakeplace community. Use at your own risk. Feel free to modify and distribute.

---
