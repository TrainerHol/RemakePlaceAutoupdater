# ReMakeplace Autoupdater

ReMakeplace Autoupdater is a Tauri desktop app for installing, updating, repairing, and launching ReMakeplace. It downloads the latest ReMakeplace game archive from GitHub, validates the extracted files, and preserves user folders such as `Custom` and `Save` during updates.

It also integrates with [ffxivhousing.com](https://www.ffxivhousing.com). Designs and layouts from the site can be opened in the updater and downloaded directly into the selected ReMakeplace install: layouts go to `Save`, and other designs go to `Custom`.

## Download

Get the latest updater from the [Releases page](https://github.com/TrainerHol/RemakePlaceAutoupdater/releases). Release assets are built from the GitHub Actions workflow for Windows x64, Linux x64, macOS Apple Silicon, and macOS Intel.

- Windows x64: use the setup `.exe` or `.msi` installer from the release, or `ReMakeplace.Autoupdater_<version>_portable_windows_x64.exe` if you want a portable updater.
- macOS Apple Silicon: use the Apple Silicon `.dmg`, or `ReMakeplace.Autoupdater_<version>_portable_macos_aarch64.app.tar.gz`.
- macOS Intel: use the Intel `.dmg`, or `ReMakeplace.Autoupdater_<version>_portable_macos_x64.app.tar.gz`.
- Linux x64: use the `.AppImage`, `.deb`, `.rpm`, or `ReMakeplace.Autoupdater_<version>_portable_linux_x64.tar.gz`.

Linux users still need Wine to launch ReMakeplace, because ReMakeplace itself is a Windows executable. The updater installs and repairs the game files normally, then launches with `wine MakePlace.exe` from the selected install folder. If Wine is missing, the app shows a Wine setup error instead of reporting a missing executable.

## First Run

When the app opens, choose an installation folder.

- Pick an empty folder for a new install.
- Pick the folder that contains `MakePlace.exe` for an existing install.
- If the folder looks like ReMakeplace but is missing required files, the app will offer a repair instead of treating it as a fresh install.

The main screen shows the selected install path, the stored current version, the latest available version, and the validation state for the selected folder. The current version is the version recorded in the updater config. If the folder cannot be validated, that version is shown as unverified instead of being hidden.

## Updating And Repairing

For normal updates, click `Update Now`. The updater backs up configured user data, extracts the downloaded archive into a staging folder, validates the staged game structure, copies the files into the install folder, restores user data, and only then saves the new version.

For repairs, click `Repair Install`. Repairs always download the latest full archive, even when the stored current version matches the latest release. This is intentional: repairs are for missing or incomplete game files.

The app checks for:

- `MakePlace.exe`
- `Makeplace/Content` or `MakePlace/Content`

If the executable or game content folder is missing after extraction, the updater reports that directly. That can mean the archive was packaged incorrectly upstream, or that extraction failed. The message is meant to point at the missing structure without assuming which side caused it.

## Folders

The main screen and Settings include shortcuts for:

- `Custom`
- `Save`
- the app config folder

The updater resolves `Custom` and `Save` from the selected ReMakeplace install. If the folders do not exist yet, opening them creates the expected folder path.

The Gallery tab can open [ffxivhousing.com](https://www.ffxivhousing.com). When a design or layout is sent to the updater, the JSON file is downloaded into the correct game folder automatically: layouts are saved under `Makeplace/Save`, and designs are saved under `Makeplace/Custom`.

## Settings

Settings lets you:

- change the install folder
- verify or repair the selected install
- set the recorded current version to the latest release
- open the config folder
- open `Custom`
- open `Save`

Use `Set current version to latest` only when you know the installed game is already current but the updater config is stale, for example after a manual install.

## Troubleshooting

### The app says the install needs repair

The selected folder contains some ReMakeplace files, but not enough to launch safely. Use `Repair Install`. The updater will preserve `Custom` and `Save` while restoring missing game files from the latest full archive.

### The app reports missing `Makeplace/Content`

The extracted or selected install folder does not contain the required game content directory. Run a repair first. If the same error happens with a fresh download, check the release archive in GitHub and report the package structure.

### Linux launch fails because Wine is missing

Install Wine through your Linux distribution, then launch again. The updater does not bundle Wine.

### Linux launch fails with error 71 (protocol error) dispatching to wayland display

This is due to a bug on Nvidia systems with WebkitGTK. Launch the application with the following environment variable `__NV_DISABLE_EXPLICIT_SYNC=1`.

If that doesn't work try the [known workarounds here](https://v2.tauri.app/develop/debug/linux-graphics/#workarounds).

### The current version looks wrong

The current version comes from the updater config, not from the game executable. If the install is valid and you know it is up to date, use Settings and choose `Set current version to latest`.

### Downloads or updates fail

Try `Clear Cache`, then update again. The updater validates cached archives by file size before reusing them, so an interrupted or mismatched download should be rejected.

For unresolved issues, use [GitHub Issues](https://github.com/TrainerHol/RemakePlaceAutoupdater/issues) or the ReMakeplace [Discord community](https://discord.gg/f2VAqXKWUw).

## Build From Source

Requirements:

- Node.js 20 or newer
- Rust stable
- Linux packages: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`

```bash
git clone https://github.com/TrainerHol/RemakePlaceAutoupdater.git
cd RemakePlaceAutoupdater/ReMakeplaceUpdater
npm ci
npm run tauri-dev
```

Build a release binary:

```bash
npm run build
npm run tauri-build
```

Run checks:

```bash
npm run build
npm run test:release
cd src-tauri
cargo test
```

## Project Layout

```text
ReMakeplaceUpdater/
├── src/                    # Frontend TypeScript
│   ├── main.ts
│   ├── style.css
│   └── types.ts
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── config.rs       # Config and install detection
│   │   ├── downloader.rs   # Download and cache handling
│   │   ├── extractor.rs    # Archive extraction
│   │   ├── launcher.rs     # Launch behavior, including Wine on Linux
│   │   ├── updater.rs      # GitHub release metadata
│   │   └── lib.rs          # Tauri commands
│   └── tauri.conf.json
├── scripts/                # Release helper scripts
└── public/metadata.json
```

## Releases

Releases are created manually from GitHub Actions.

1. Open the `Draft Release` workflow.
2. Run it with a stable SemVer value such as `1.3.0`.
3. Review the draft release.
4. Publish the draft when the assets look correct.

The workflow bumps versions in `package.json`, `package-lock.json`, `tauri.conf.json`, `Cargo.toml`, and `Cargo.lock`, then creates a draft release tagged `remakeplace-updater-vX.Y.Z`.

The workflow matrix builds:

- Windows x64 on `windows-latest`
- Linux x64 on `ubuntu-22.04`
- macOS Apple Silicon with `--target aarch64-apple-darwin`
- macOS Intel with `--target x86_64-apple-darwin`

The draft includes Tauri installer/package outputs plus portable builds:

- Windows x64 setup `.exe` and `.msi` installer assets from Tauri, plus `ReMakeplace.Autoupdater_<version>_portable_windows_x64.exe`
- Linux x64 `.AppImage`, `.deb`, and `.rpm`, plus `ReMakeplace.Autoupdater_<version>_portable_linux_x64.tar.gz`
- macOS Apple Silicon `.dmg`, plus `ReMakeplace.Autoupdater_<version>_portable_macos_aarch64.app.tar.gz`
- macOS Intel `.dmg`, plus `ReMakeplace.Autoupdater_<version>_portable_macos_x64.app.tar.gz`

Portable assets are uploaded to the exact draft release tag produced by the workflow.

## License

This updater is provided as-is for the ReMakeplace community. Use it at your own risk. You may modify and distribute it.
