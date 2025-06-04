# ReMakeplace Auto-Updater 🚀

Launcher for ReMakeplace with automatic update functionality from GitHub releases.

## Installation 📦

### Prerequisites

- Python 3.8 or higher
- Windows (tested on 10)

### Setup Instructions

1. **Download the updater files to any folder** (doesn't need to be in ReMakeplace folder)

   ```
   ReMakeplaceUpdater/
   ├── updater.py
   ├── config.json
   ├── requirements.txt
   ├── setup.py
   ├── launch_updater.bat
   └── README.md
   ```

2. **Install Python dependencies**

   ```bash
   pip install -r requirements.txt
   ```

3. **Run the updater**

   ```bash
   python updater.py
   ```

   Or double-click `launch_updater.bat`

4. **First Run Setup**
   - On first launch, you'll see a welcome dialog
   - Click "Browse" to select your ReMakeplace installation folder
   - The updater will validate that `Makeplace.exe` exists in the selected folder
   - Click "Save & Continue" to proceed

## Building Executable (Optional) 🔨

To create a standalone `.exe` file:

```bash
python setup.py build
```

The executable will be created in the `dist/` folder. You can then run `ReMakeplaceUpdater.exe` directly.

### Data Protection

Your important data is automatically preserved during updates:

- `/Makeplace/Custom/` - Your custom layouts and configurations
- `/Makeplace/Save/` - Your saved data

## Configuration ⚙️

The `config.json` file contains all settings:

```json
{
  "current_version": "7.2.5", // Your current version
  "github_repo": "RemakePlace/app", // GitHub repository
  "installation_path": "C:/Games/ReMakeplace", // Path to ReMakeplace installation
  "exe_path": "Makeplace.exe", // Main executable filename
  "preserve_folders": [
    // Folders to preserve during updates
    "Makeplace/Custom",
    "Makeplace/Save"
  ],
  "update_check_url": "https://api.github.com/repos/RemakePlace/app/releases/latest",
  "last_check": "", // Last update check timestamp
  "auto_check": true // Auto-check on startup
}
```

## Development 👨‍💻

### Project Structure

```
RemakePlaceUpdater/
├── updater.py          # Main application
├── config.json        # Configuration
├── requirements.txt    # Python dependencies
├── setup.py           # Build script
├── launch_updater.bat # Windows launcher
└── README.md          # This file
```

### Key Components

- **UI Framework**: CustomTkinter for modern appearance
- **Path Management**: Path validation and folder browsing
- **HTTP Client**: Requests for GitHub API and downloads
- **Archive Handling**: py7zr for .7z file extraction
- **Version Management**: Packaging for version comparison

## License 📄

This updater is provided as-is for the ReMakeplace community. Use at your own risk. Feel free to modify and distribute.

---
