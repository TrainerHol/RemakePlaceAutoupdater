# ReMakeplace Auto-Updater (Tauri/Rust)

A modern, fast, and reliable auto-updater for ReMakeplace built with Tauri and Rust. This replaces the previous Python version with better performance, smaller size, and native system integration.

## Features

### ✨ Core Features

- **Automatic Updates**: Checks GitHub releases for new versions
- **Smart Caching**: Downloads are cached and reused
- **Data Preservation**: User data (Custom/Save folders) is backed up during updates
- **Multiple Archive Formats**: Supports .7z and .zip with multiple fallback extraction methods
- **Game Launcher**: Launch ReMakeplace directly from the updater
- **Modern UI**: Dark theme with orange accents matching ReMakeplace branding

### 🔧 Technical Features

- **Single Executable**: No external dependencies required
- **Cross-platform**: Built with Rust for Windows, macOS, and Linux
- **Memory Efficient**: Streams large downloads without loading everything into memory
- **Robust Error Handling**: Graceful fallbacks and user-friendly error messages
- **Background Operations**: Non-blocking UI with real-time progress updates

## Installation

### From Release

1. Download the latest release from the GitHub releases page
2. Extract and run `ReMakeplace Launcher.exe`
3. On first run, select your ReMakeplace installation folder

### Building from Source

```bash
# Prerequisites: Node.js, npm, Rust
git clone <repository-url>
cd ReMakeplaceUpdater
npm install
npm run tauri-build
```

## Usage

### First Run Setup

1. Launch the updater
2. Click "⚙️ Settings" to configure your installation path
3. Browse to your ReMakeplace installation folder (must contain `Makeplace.exe`)
4. Click "Save & Continue"

### Normal Operation

- **Check for Updates**: Click "Check for Updates" or it happens automatically on startup
- **Install Updates**: When available, click "Update Now" to download and install
- **Launch Game**: Click "Launch ReMakeplace" to start the game
- **Change Settings**: Click "⚙️ Settings" to modify the installation path

## Configuration

The updater uses a `config.json` file with the following structure:

```json
{
  "current_version": "7.25.0",
  "github_repo": "RemakePlace/app",
  "installation_path": "path/to/installation",
  "exe_path": "Makeplace.exe",
  "preserve_folders": ["Makeplace/Custom", "Makeplace/Save"],
  "update_check_url": "https://api.github.com/repos/RemakePlace/app/releases/latest",
  "last_check": "2025-06-03T23:01:28.574023",
  "auto_check": true
}
```

### Configuration Options

- `current_version`: Currently installed version
- `installation_path`: Path to ReMakeplace installation
- `preserve_folders`: Folders to backup during updates
- `auto_check`: Enable automatic update checking on startup

## Architecture

### Backend (Rust)

- **Config Management**: Handles configuration loading/saving and validation
- **Update Manager**: GitHub API integration and version comparison
- **Download Manager**: Streaming downloads with progress tracking
- **Archive Extractor**: Multi-format extraction with fallbacks
- **Game Launcher**: Process management for launching ReMakeplace

### Frontend (TypeScript/HTML/CSS)

- **Modern UI**: Clean, responsive interface with dark theme
- **Real-time Updates**: Progress bars and status messages
- **Settings Dialog**: Path configuration with validation
- **Event-Driven**: Reactive UI responding to backend events

### Communication

- **Tauri Commands**: Frontend → Backend function calls
- **Tauri Events**: Backend → Frontend real-time updates
- **Type Safety**: Full TypeScript definitions for all interfaces

## Update Process

1. **Check**: Query GitHub API for latest release
2. **Compare**: Use semantic versioning to determine if update needed
3. **Download**: Stream download to cache directory with progress tracking
4. **Backup**: Preserve user data (Custom/Save folders)
5. **Extract**: Extract archive using native libraries or system tools
6. **Restore**: Restore user data to new installation
7. **Update**: Update configuration with new version
8. **Cleanup**: Remove old cache files and temporary directories

## Error Handling

The updater includes comprehensive error handling:

- **Network Issues**: Retry logic and offline graceful degradation
- **File System**: Permission errors and disk space handling
- **Archive Problems**: Multiple extraction method fallbacks
- **Data Safety**: Backup restoration on failed updates

## Development

### Project Structure

```
ReMakeplaceUpdater/
├── src/                    # Frontend TypeScript
│   ├── main.ts            # Main application logic
│   ├── types.ts           # TypeScript definitions
│   └── style.css          # UI styling
├── src-tauri/             # Rust backend
│   └── src/
│       ├── lib.rs         # Main Tauri application
│       ├── config.rs      # Configuration management
│       ├── updater.rs     # Update logic
│       ├── downloader.rs  # Download management
│       ├── extractor.rs   # Archive extraction
│       └── launcher.rs    # Game launcher
├── index.html             # Frontend entry point
└── config.json           # User configuration
```

### Available Scripts

- `npm run dev`: Start frontend development server
- `npm run build`: Build frontend for production
- `npm run tauri-dev`: Start Tauri development mode
- `npm run tauri-build`: Build production executable

### Dependencies

- **Frontend**: Vite, TypeScript, Tauri API
- **Backend**: Tauri, Tokio, Reqwest, 7z/ZIP libraries

## Migration from Python Version

The Tauri version is designed to be a drop-in replacement:

- ✅ Same UI layout and functionality
- ✅ Compatible with existing `config.json` files
- ✅ Identical user experience
- ✅ Better performance and smaller size

Simply replace the Python executable with the new Tauri version.

## Troubleshooting

### Common Issues

- **"Makeplace.exe not found"**: Ensure the installation path points to the correct folder
- **Download fails**: Check internet connection and GitHub API availability
- **Extraction fails**: Ensure sufficient disk space and proper permissions
- **Game won't launch**: Verify the executable path is correct

### Logs and Debugging

The application outputs debug information to the console in development mode. For production issues, check:

- Network connectivity to GitHub
- File system permissions
- Available disk space
- Antivirus software blocking operations

## License

This project is licensed under the same terms as ReMakeplace.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Support

For issues related to:

- **The updater itself**: Open an issue on this repository
- **ReMakeplace game**: Contact the ReMakeplace developers
- **Installation problems**: Check the troubleshooting section above
