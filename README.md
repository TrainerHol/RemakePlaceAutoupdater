# ReMakeplace Auto-Updater 🚀

A modern, native auto-updater for ReMakeplace built with Tauri (Rust + TypeScript). Download updates directly from GitHub releases with smart caching and data preservation.

## Features ✨

- **🔒 Data Preservation**: Automatically preserves Custom, Makeplace Config, and Save folders
- **⚙️ Easy Setup**: Browse and configure installation path with validation

## Installation Guide 📦

### For New Users (Fresh Installation)

If you don't have ReMakeplace installed yet, the updater can handle everything for you!

1. **Download the Updater**

   - Go to the [Releases page](https://github.com/TrainerHol/RemakePlaceAutoupdater/releases)
   - Download the file for your operating system:
     - **Windows**: Download `remakeplaceupdater-setup.msi` (installer) or `remakeplaceupdater-portable.exe` (no installation needed)
     - **macOS**: Download `remakeplaceupdater.dmg` (installer) or `remakeplaceupdater.app.tar.gz` (portable)
     - **Linux**: Download `remakeplaceupdater.AppImage` (works on most distributions)

2. **Run the Updater**

   - Double-click the downloaded file
   - If Windows shows a security warning, click "More info" then "Run anyway" (this is normal for new apps)

3. **Choose Installation Location**

   - When the app opens, you'll see "Welcome to ReMakeplace Launcher"
   - Click the "Browse" button
   - Select or create a folder where you want ReMakeplace installed (e.g., `C:\Games\ReMakeplace`)
   - The app will show "✅ Valid folder for fresh installation"
   - Click "Save"

4. **Install ReMakeplace**
   - The app will check for the latest version
   - Click "Install Now" (green button)
   - A confirmation dialog will show the installation location - click "Yes" to proceed
   - Wait for the download and installation to complete
   - Once done, click "Launch ReMakeplace" to start playing!

### For Existing Users (Updates)

If you already have ReMakeplace installed:

1. **Download and Run the Updater** (same as step 1 above)

2. **Select Your ReMakeplace Folder**

   - Click "Browse" and navigate to your existing ReMakeplace installation
   - The folder should contain `Makeplace.exe`
   - The app will show "✅ Valid installation path"
   - Click "Save"

3. **Check for Updates**
   - The app automatically checks if updates are available
   - If an update is found, click "Update Now"
   - Your saved data and custom settings are automatically preserved!

**Note**: You do not need to delete your itch.io Makeplace/ReMakeplace installation, it will work for updating as long as there's a Makeplace.exe file.

### Special Cases

#### "My installation shows version 0.0.0"

If you have ReMakeplace installed but the updater shows version 0.0.0:

1. Click the settings button (⚙️)
2. Check the "Set current version to latest" option
3. This syncs your version without downloading the current update again.

#### "I want to change my installation folder"

1. Click the settings button (⚙️) at any time
2. Browse to select a new folder
3. If you select an empty folder, the app will offer to do a fresh installation there
4. For existing installations, you'll need to manually move/delete your files.

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

## Troubleshooting 🛠️

### Common Issues

**"The app won't open" (Windows)**

- Right-click the app and select "Run as administrator"
- Check if Windows Defender or antivirus is blocking it
- Try the portable version instead of the installer

**"Can't find installation folder"**

- Look for a folder containing `Makeplace.exe`
- Common locations:
  - Windows: `C:\Program Files\ReMakeplace` or `C:\Games\ReMakeplace`
  - macOS: `/Applications/ReMakeplace.app`
  - Linux: `/home/username/ReMakeplace`

**"Update failed"**

- Click "Clear Cache" and try again
- Check your internet connection
- Temporarily disable antivirus during update

**"Version shows 0.0.0"**

- Go to Settings (⚙️)
- Enable "Set current version to latest"
- This is common for manual installations

### Need More Help?

Visit our [GitHub Issues](https://github.com/TrainerHol/RemakePlaceAutoupdater/issues) page or ask in the ReMakeplace [discord community](https://discord.gg/f2VAqXKWUw)!

## Advanced Configuration ⚙️

For advanced users, the `config.json` file can be manually edited:

```json
{
  "current_version": "0.0.0",
  "github_repo": "RemakePlace/app",
  "installation_path": "",
  "exe_path": "Makeplace.exe",
  "preserve_folders": ["Makeplace/Custom", "Makeplace/Save"], // Also preserves config.json
  "update_check_url": "https://api.github.com/repos/RemakePlace/app/releases/latest",
  "release_page_url": "https://github.com/RemakePlace/app/releases/latest",
  "last_check": "2024-01-01T00:00:00Z",
  "auto_check": true,
  "installation_mode": "update"
}
```

### Your Data is Safe! 🔒

The updater automatically protects your important files during updates:

- **`Makeplace/Custom/`** - All your custom house layouts and designs
- **`Makeplace/Save/`** - Your saved game data and settings
- **`Makeplace/config.json`** - Your ReMakeplace configuration file

These folders and files are backed up before updates and restored afterward, so you never lose your creations or settings!

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
