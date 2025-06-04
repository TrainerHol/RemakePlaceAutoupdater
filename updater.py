#!/usr/bin/env python3
"""
ReMakeplace Auto-Updater
Modern MMO-style launcher with auto-update functionality
"""

import tkinter as tk
import customtkinter as ctk
from tkinter import filedialog, messagebox
import json
import os
import sys
import threading
import time
import requests
import py7zr
import shutil
from datetime import datetime
from pathlib import Path
from packaging import version
import subprocess

class ReMakeplaceUpdater:
    def __init__(self):
        # Set appearance and theme
        ctk.set_appearance_mode("dark")
        ctk.set_default_color_theme("blue")
        
        # Initialize main window
        self.root = ctk.CTk()
        self.root.title("ReMakeplace Launcher")
        self.root.geometry("600x550")  # Increased height significantly
        self.root.resizable(True, True)  # Make window resizable
        self.root.minsize(600, 500)  # Set minimum size to prevent too small
        
        # Center window on screen
        self.center_window()
        
        # Load configuration
        self.config = self.load_config()
        
        # Initialize UI variables
        self.update_available = False
        self.latest_version = None
        self.download_url = None
        self.download_progress = 0
        self.download_speed = 0
        
        # Check if installation path is configured
        if not self.validate_installation_path():
            self.show_settings_dialog(first_run=True)
        else:
            # Create UI
            self.create_ui()
            # Start update check
            self.check_for_updates()
    
    def center_window(self):
        """Center the window on screen"""
        self.root.update_idletasks()
        width = self.root.winfo_width()
        height = self.root.winfo_height()
        pos_x = (self.root.winfo_screenwidth() // 2) - (width // 2)
        pos_y = (self.root.winfo_screenheight() // 2) - (height // 2)
        self.root.geometry(f"{width}x{height}+{pos_x}+{pos_y}")
    
    def load_config(self):
        """Load configuration from JSON file"""
        try:
            with open("config.json", "r") as f:
                return json.load(f)
        except FileNotFoundError:
            # Create default config if doesn't exist
            default_config = {
                "current_version": "0.0.0",
                "github_repo": "RemakePlace/app",
                "installation_path": "",
                "exe_path": "Makeplace.exe",
                "preserve_folders": ["Makeplace/Custom", "Makeplace/Save"],
                "update_check_url": "https://api.github.com/repos/RemakePlace/app/releases/latest",
                "last_check": "",
                "auto_check": True
            }
            self.save_config(default_config)
            return default_config
    
    def save_config(self, config=None):
        """Save configuration to JSON file"""
        if config is None:
            config = self.config
        with open("config.json", "w") as f:
            json.dump(config, f, indent=4)
    
    def validate_installation_path(self):
        """Validate that the installation path contains MakePlace.exe"""
        if not self.config.get("installation_path"):
            return False
        
        install_path = Path(self.config["installation_path"])
        exe_path = install_path / self.config["exe_path"]
        
        return exe_path.exists() and exe_path.is_file()
    
    def show_settings_dialog(self, first_run=False):
        """Show settings dialog to configure installation path"""
        settings_window = ctk.CTkToplevel(self.root)
        settings_window.title("ReMakeplace Settings")
        settings_window.geometry("600x400")  # Made larger to prevent cropping
        settings_window.resizable(False, False)
        settings_window.transient(self.root)
        settings_window.grab_set()
        
        # Center settings window
        settings_window.update_idletasks()
        x = (settings_window.winfo_screenwidth() // 2) - (settings_window.winfo_width() // 2)
        y = (settings_window.winfo_screenheight() // 2) - (settings_window.winfo_height() // 2)
        settings_window.geometry(f"+{x}+{y}")
        
        # Main frame
        main_frame = ctk.CTkFrame(settings_window, corner_radius=10)
        main_frame.pack(fill="both", expand=True, padx=20, pady=20)
        
        # Title
        title_text = "Welcome! Please configure your ReMakeplace installation." if first_run else "Settings"
        title_label = ctk.CTkLabel(
            main_frame,
            text=title_text,
            font=ctk.CTkFont(size=18, weight="bold")
        )
        title_label.pack(pady=(20, 10))
        
        if first_run:
            info_label = ctk.CTkLabel(
                main_frame,
                text="Select the folder where ReMakeplace is installed\n(the folder containing Makeplace.exe)",
                font=ctk.CTkFont(size=14),
                text_color="gray"
            )
            info_label.pack(pady=(0, 20))
        
        # Installation path frame
        path_frame = ctk.CTkFrame(main_frame)
        path_frame.pack(fill="x", padx=20, pady=10)
        
        path_label = ctk.CTkLabel(
            path_frame,
            text="Installation Path:",
            font=ctk.CTkFont(size=14, weight="bold")
        )
        path_label.pack(anchor="w", padx=10, pady=(10, 5))
        
        # Path entry and browse button frame
        entry_frame = ctk.CTkFrame(path_frame, fg_color="transparent")
        entry_frame.pack(fill="x", padx=10, pady=(0, 10))
        
        path_var = tk.StringVar(value=self.config.get("installation_path", ""))
        path_entry = ctk.CTkEntry(
            entry_frame,
            textvariable=path_var,
            font=ctk.CTkFont(size=12),
            height=35
        )
        path_entry.pack(side="left", fill="x", expand=True, padx=(0, 10))
        
        browse_button = ctk.CTkButton(
            entry_frame,
            text="Browse",
            width=80,
            height=35,
            command=lambda: self.browse_folder(path_var)
        )
        browse_button.pack(side="right")
        
        # Status label
        status_label = ctk.CTkLabel(
            main_frame,
            text="",
            font=ctk.CTkFont(size=12)
        )
        status_label.pack(pady=(10, 20))  # Added bottom padding
        
        # Buttons frame - ensure it's always visible
        button_frame = ctk.CTkFrame(main_frame, fg_color="transparent")
        button_frame.pack(fill="x", padx=20, pady=(0, 20), side="bottom")  # Pack at bottom
        
        def validate_and_save():
            path = path_var.get().strip()
            if not path:
                status_label.configure(text="❌ Please select an installation path", text_color="red")
                return
            
            install_path = Path(path)
            if not install_path.exists():
                status_label.configure(text="❌ Selected path does not exist", text_color="red")
                return
            
            exe_path = install_path / self.config["exe_path"]
            if not exe_path.exists():
                status_label.configure(text="❌ Makeplace.exe not found in selected folder", text_color="red")
                return
            
            # Save configuration
            self.config["installation_path"] = str(install_path)
            self.save_config()
            
            status_label.configure(text="✅ Configuration saved successfully!", text_color="green")
            
            # Close dialog and continue
            settings_window.after(1000, lambda: self.close_settings_and_continue(settings_window, first_run))
        
        def cancel_settings():
            if first_run:
                self.root.quit()
            else:
                settings_window.destroy()
        
        # Save button
        save_button = ctk.CTkButton(
            button_frame,
            text="Save & Continue" if first_run else "Save",
            font=ctk.CTkFont(size=14, weight="bold"),
            height=40,
            width=140,
            fg_color=("#FF8C42", "#E6732A"),
            hover_color=("#E6732A", "#CC5A1A"),
            command=validate_and_save
        )
        save_button.pack(side="right", padx=(10, 0))
        
        # Cancel button
        cancel_button = ctk.CTkButton(
            button_frame,
            text="Exit" if first_run else "Cancel",
            font=ctk.CTkFont(size=14, weight="bold"),
            height=40,
            width=100,
            fg_color=("#666666", "#555555"),
            hover_color=("#777777", "#666666"),
            command=cancel_settings
        )
        cancel_button.pack(side="right")
        
        # Auto-validate on path change
        def on_path_change(*args):
            path = path_var.get().strip()
            if path:
                install_path = Path(path)
                if install_path.exists():
                    exe_path = install_path / self.config["exe_path"]
                    if exe_path.exists():
                        status_label.configure(text="✅ Makeplace.exe found!", text_color="green")
                    else:
                        status_label.configure(text="❌ Makeplace.exe not found", text_color="red")
                else:
                    status_label.configure(text="❌ Path does not exist", text_color="red")
            else:
                status_label.configure(text="")
        
        path_var.trace("w", on_path_change)
        
        # Focus on entry
        path_entry.focus()
    
    def browse_folder(self, path_var):
        """Open folder browser dialog"""
        folder = filedialog.askdirectory(
            title="Select ReMakeplace Installation Folder",
            initialdir=path_var.get() or os.path.expanduser("~")
        )
        if folder:
            path_var.set(folder)
    
    def close_settings_and_continue(self, settings_window, first_run):
        """Close settings window and continue with main app"""
        settings_window.destroy()
        if first_run:
            self.create_ui()
            self.check_for_updates()
        else:
            # Update the path label on the main screen
            if hasattr(self, 'path_label'):
                self.path_label.configure(text=f"Installation: {self.config.get('installation_path', 'Not configured')}")
    
    def create_ui(self):
        """Create the modern UI with orange gradients"""
        # Main frame with gradient background
        self.main_frame = ctk.CTkFrame(self.root, corner_radius=0)
        self.main_frame.pack(fill="both", expand=True, padx=0, pady=0)
        
        # Header frame with title
        self.header_frame = ctk.CTkFrame(
            self.main_frame, 
            height=80, 
            corner_radius=0,
            fg_color=("#FF8C42", "#E6732A")  # Orange gradient
        )
        self.header_frame.pack(fill="x", padx=0, pady=0)
        self.header_frame.pack_propagate(False)
        
        # Title label
        self.title_label = ctk.CTkLabel(
            self.header_frame,
            text="ReMakeplace Launcher",
            font=ctk.CTkFont(size=28, weight="bold"),
            text_color="white"
        )
        self.title_label.pack(expand=True)
        
        # Content frame - use expand and fill to utilize all space
        self.content_frame = ctk.CTkFrame(self.main_frame, corner_radius=0)
        self.content_frame.pack(fill="both", expand=True, padx=20, pady=20)
        
        # Installation path display - more prominent
        self.path_info_frame = ctk.CTkFrame(self.content_frame)
        self.path_info_frame.pack(fill="x", pady=(0, 15))
        
        # Path info section
        path_info_section = ctk.CTkFrame(self.path_info_frame, fg_color="transparent")
        path_info_section.pack(fill="x", padx=15, pady=10)
        
        path_title = ctk.CTkLabel(
            path_info_section,
            text="Installation Path:",
            font=ctk.CTkFont(size=12, weight="bold"),
            text_color="white"
        )
        path_title.pack(anchor="w")
        
        self.path_label = ctk.CTkLabel(
            path_info_section,
            text=f"{self.config.get('installation_path', 'Not configured')}",
            font=ctk.CTkFont(size=11),
            text_color="gray"
        )
        self.path_label.pack(anchor="w", pady=(2, 0))
        
        # Settings button - more prominent
        self.settings_button = ctk.CTkButton(
            self.path_info_frame,
            text="⚙️ Settings",
            font=ctk.CTkFont(size=13, weight="bold"),
            width=100,
            height=35,
            fg_color=("#FF8C42", "#E6732A"),
            hover_color=("#E6732A", "#CC5A1A"),
            command=lambda: self.show_settings_dialog(first_run=False)
        )
        self.settings_button.pack(side="right", padx=15, pady=10)
        
        # Middle content frame for status and version info
        self.middle_frame = ctk.CTkFrame(self.content_frame, fg_color="transparent")
        self.middle_frame.pack(fill="both", expand=True, pady=(0, 20))
        
        # Status label
        self.status_label = ctk.CTkLabel(
            self.middle_frame,
            text="Checking for updates...",
            font=ctk.CTkFont(size=16)
        )
        self.status_label.pack(pady=(20, 15))
        
        # Version info frame
        self.version_frame = ctk.CTkFrame(self.middle_frame)
        self.version_frame.pack(fill="x", pady=(0, 15))
        
        self.current_version_label = ctk.CTkLabel(
            self.version_frame,
            text=f"Current Version: {self.config['current_version']}",
            font=ctk.CTkFont(size=14)
        )
        self.current_version_label.pack(pady=8)
        
        self.latest_version_label = ctk.CTkLabel(
            self.version_frame,
            text="Latest Version: Checking...",
            font=ctk.CTkFont(size=14)
        )
        self.latest_version_label.pack(pady=8)
        
        # Progress frame (initially hidden)
        self.progress_frame = ctk.CTkFrame(self.middle_frame)
        
        self.progress_label = ctk.CTkLabel(
            self.progress_frame,
            text="Downloading update...",
            font=ctk.CTkFont(size=14)
        )
        self.progress_label.pack(pady=(15, 8))
        
        self.progress_bar = ctk.CTkProgressBar(
            self.progress_frame,
            width=400,
            height=20,
            progress_color=("#FF8C42", "#E6732A")
        )
        self.progress_bar.pack(pady=(0, 8))
        self.progress_bar.set(0)
        
        self.speed_label = ctk.CTkLabel(
            self.progress_frame,
            text="Download Speed: 0 MB/s",
            font=ctk.CTkFont(size=12)
        )
        self.speed_label.pack(pady=(0, 15))
        
        # Button frame - always at bottom
        self.button_frame = ctk.CTkFrame(self.content_frame, fg_color="transparent")
        self.button_frame.pack(fill="x", side="bottom", pady=(0, 10))
        
        # Update button
        self.update_button = ctk.CTkButton(
            self.button_frame,
            text="Update Available",
            font=ctk.CTkFont(size=16, weight="bold"),
            height=45,  # Made slightly taller
            width=220,  # Made wider
            fg_color=("#FF8C42", "#E6732A"),
            hover_color=("#E6732A", "#CC5A1A"),
            command=self.start_update,
            state="disabled"
        )
        self.update_button.pack(side="left", padx=(20, 10))
        
        # Launch button
        self.launch_button = ctk.CTkButton(
            self.button_frame,
            text="Launch ReMakeplace",
            font=ctk.CTkFont(size=16, weight="bold"),
            height=45,  # Made slightly taller
            width=220,  # Made wider
            fg_color=("#2B2B2B", "#1A1A1A"),
            hover_color=("#404040", "#2B2B2B"),
            command=self.launch_game
        )
        self.launch_button.pack(side="right", padx=(10, 20))
    
    def check_for_updates(self):
        """Check for updates in a separate thread"""
        def check_thread():
            try:
                self.root.after(0, lambda: self.status_label.configure(text="Checking for updates..."))
                response = requests.get(self.config["update_check_url"], timeout=10)
                response.raise_for_status()
                
                release_data = response.json()
                self.latest_version = release_data["tag_name"].lstrip("v")
                
                # Find the .7z asset
                for asset in release_data["assets"]:
                    if asset["name"].endswith(".7z"):
                        self.download_url = asset["browser_download_url"]
                        break
                
                # Update UI on main thread
                self.root.after(0, self.update_ui_after_check)
                
            except Exception as e:
                self.root.after(0, lambda: self.status_label.configure(
                    text=f"Failed to check for updates: {str(e)}"
                ))
        
        threading.Thread(target=check_thread, daemon=True).start()
    
    def update_ui_after_check(self):
        """Update UI after version check completes"""
        self.latest_version_label.configure(text=f"Latest Version: {self.latest_version}")
        
        current_ver = version.parse(self.config["current_version"]) if self.config["current_version"] != "0.0.0" else version.parse("0.0.0")
        latest_ver = version.parse(self.latest_version)
        
        if latest_ver > current_ver:
            self.update_available = True
            self.status_label.configure(text="🎉 New update available!")
            self.update_button.configure(state="normal", text="Update Now")
        else:
            self.status_label.configure(text="✅ You're up to date!")
            self.update_button.configure(text="No Updates", state="disabled")
    
    def start_update(self):
        """Start the update process"""
        if not self.update_available or not self.download_url:
            return
        
        # Show progress frame
        self.progress_frame.pack(fill="x", pady=10)
        self.update_button.configure(state="disabled")
        self.launch_button.configure(state="disabled")
        
        # Start download in separate thread
        threading.Thread(target=self.download_and_install, daemon=True).start()
    
    def download_and_install(self):
        """Download and install the update"""
        try:
            # Create cache directory (persistent across runs)
            cache_dir = Path("update_cache")
            cache_dir.mkdir(exist_ok=True)
            
            # Generate cache filename with version
            filename = self.download_url.split("/")[-1]
            cache_filepath = cache_dir / f"v{self.latest_version}_{filename}"
            
            # Check if file is already cached
            if cache_filepath.exists():
                self.root.after(0, lambda: self.progress_label.configure(text="Using cached download..."))
                self.root.after(0, lambda: self.progress_bar.set(1.0))
                self.root.after(0, lambda: self.speed_label.configure(text="Using cached file"))
                # Small delay to show the cache message
                time.sleep(0.5)
            else:
                # Download to cache
                self.download_file(self.download_url, cache_filepath)
            
            # Extract and install from cache
            self.root.after(0, lambda: self.progress_label.configure(text="Installing update..."))
            self.install_update(cache_filepath)
            
            # Update version in config
            self.config["current_version"] = self.latest_version
            self.config["last_check"] = datetime.now().isoformat()
            self.save_config()
            
            # Clean up old cache files and temp directories
            self.cleanup_cache(cache_dir, keep_current=True)
            temp_dir = Path("temp_update")
            if temp_dir.exists():
                shutil.rmtree(temp_dir, ignore_errors=True)
            
            # Update UI
            self.root.after(0, self.update_complete)
            
        except Exception as e:
            # Don't clean up cache on error - keep for retry
            error_message = f"Update failed: {str(e)}"
            self.root.after(0, lambda msg=error_message: self.show_error(msg))
    
    def cleanup_cache(self, cache_dir, keep_current=False):
        """Clean up old cached files, optionally keeping the current version"""
        try:
            if not cache_dir.exists():
                return
            
            current_version_prefix = f"v{self.latest_version}_" if keep_current else None
            files_removed = 0
            
            for file in cache_dir.iterdir():
                if file.is_file():
                    # Keep current version if specified
                    if keep_current and current_version_prefix and file.name.startswith(current_version_prefix):
                        continue
                    
                    # Remove file
                    try:
                        file.unlink()
                        files_removed += 1
                    except Exception as e:
                        print(f"Warning: Could not remove cache file {file}: {e}")
            
            # Remove cache directory if empty and we're not keeping anything
            if not keep_current and cache_dir.exists():
                try:
                    # Check if directory is empty
                    if not any(cache_dir.iterdir()):
                        cache_dir.rmdir()
                    else:
                        print(f"Cache directory not empty, keeping: {list(cache_dir.iterdir())}")
                except OSError as e:
                    print(f"Could not remove cache directory: {e}")
            
            if files_removed > 0:
                print(f"Cleaned up {files_removed} cached file(s)")
                    
        except Exception as e:
            print(f"Cache cleanup error: {e}")
    
    def download_file(self, url, filepath):
        """Download file with progress tracking"""
        try:
            self.root.after(0, lambda: self.progress_label.configure(text="Downloading update..."))
            
            response = requests.get(url, stream=True)
            response.raise_for_status()
            
            total_size = int(response.headers.get('content-length', 0))
            downloaded = 0
            start_time = time.time()
            
            # Ensure parent directory exists
            filepath.parent.mkdir(parents=True, exist_ok=True)
            
            with open(filepath, 'wb') as f:
                for chunk in response.iter_content(chunk_size=8192):
                    if chunk:
                        f.write(chunk)
                        downloaded += len(chunk)
                        
                        # Calculate progress and speed
                        progress = downloaded / total_size if total_size > 0 else 0
                        elapsed_time = time.time() - start_time
                        speed = downloaded / elapsed_time / 1024 / 1024 if elapsed_time > 0 else 0  # MB/s
                        
                        # Update UI with captured values
                        self.root.after(0, lambda p=progress, s=speed: self.update_progress(p, s))
                        
            # Verify download completed
            if total_size > 0 and downloaded != total_size:
                raise Exception(f"Download incomplete: {downloaded}/{total_size} bytes")
                
        except Exception as e:
            # Remove incomplete download
            if filepath.exists():
                filepath.unlink(missing_ok=True)
            raise Exception(f"Download failed: {str(e)}")
    
    def update_progress(self, progress, speed):
        """Update progress bar and speed label"""
        self.progress_bar.set(progress)
        self.speed_label.configure(text=f"Download Speed: {speed:.2f} MB/s")
    
    def install_update(self, archive_path):
        """Install the update while preserving user data"""
        try:
            installation_path = Path(self.config["installation_path"])
            
            # Validate installation path exists
            if not installation_path.exists():
                raise Exception(f"Installation path does not exist: {installation_path}")
            
            # Validate archive exists
            if not archive_path.exists():
                raise Exception(f"Update archive not found: {archive_path}")
            
            # Backup user data
            backup_dir = Path("temp_backup")
            backup_dir.mkdir(exist_ok=True)
            
            self.root.after(0, lambda: self.progress_label.configure(text="Backing up user data..."))
            
            for folder in self.config["preserve_folders"]:
                src = installation_path / folder
                if src.exists():
                    dst = backup_dir / folder
                    dst.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copytree(src, dst, dirs_exist_ok=True)
            
            # Extract new files to installation directory
            self.root.after(0, lambda: self.progress_label.configure(text="Extracting update files..."))
            
            # Try multiple extraction methods
            extraction_success = False
            
            # Method 1: Try py7zr first (works for most 7z files)
            try:
                with py7zr.SevenZipFile(archive_path, mode='r') as archive:
                    archive.extractall(path=str(installation_path))
                extraction_success = True
            except Exception as py7zr_error:
                py7zr_error_msg = str(py7zr_error)
                
                # Method 2: Try system 7zip command line as fallback
                if not extraction_success:
                    try:
                        # Look for 7zip in common locations
                        seven_zip_paths = [
                            "7z",  # If in PATH
                            "C:\\Program Files\\7-Zip\\7z.exe",
                            "C:\\Program Files (x86)\\7-Zip\\7z.exe",
                        ]
                        
                        seven_zip_exe = None
                        for path in seven_zip_paths:
                            try:
                                result = subprocess.run([path], capture_output=True, timeout=5)
                                seven_zip_exe = path
                                break
                            except (subprocess.TimeoutExpired, FileNotFoundError, subprocess.CalledProcessError):
                                continue
                        
                        if seven_zip_exe:
                            self.root.after(0, lambda: self.progress_label.configure(text="Using system 7zip for extraction..."))
                            
                            # Extract using system 7zip
                            cmd = [seven_zip_exe, "x", str(archive_path), f"-o{str(installation_path)}", "-y"]
                            result = subprocess.run(cmd, capture_output=True, text=True, timeout=300)
                            
                            if result.returncode == 0:
                                extraction_success = True
                            else:
                                raise Exception(f"7zip extraction failed: {result.stderr}")
                        
                    except Exception as zip_error:
                        # Method 3: Try Python's zipfile as last resort (in case it's actually a zip)
                        try:
                            import zipfile
                            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                                zip_ref.extractall(str(installation_path))
                            extraction_success = True
                        except Exception:
                            pass
            
            # If all extraction methods failed
            if not extraction_success:
                error_msg = f"Failed to extract archive. The file uses an unsupported compression format (likely BCJ2 filter). Please install 7-Zip from https://www.7-zip.org/ and try again."
                if 'py7zr_error_msg' in locals() and 'BCJ2' in py7zr_error_msg:
                    error_msg += f"\n\nTechnical details: {py7zr_error_msg}"
                raise Exception(error_msg)
            
            # Restore user data
            self.root.after(0, lambda: self.progress_label.configure(text="Restoring user data..."))
            
            for folder in self.config["preserve_folders"]:
                backup_src = backup_dir / folder
                if backup_src.exists():
                    dst = installation_path / folder
                    if dst.exists():
                        shutil.rmtree(dst)
                    shutil.copytree(backup_src, dst)
            
            # Cleanup backup
            shutil.rmtree(backup_dir, ignore_errors=True)
            
        except Exception as e:
            # Cleanup on error
            backup_dir = Path("temp_backup")
            if backup_dir.exists():
                shutil.rmtree(backup_dir, ignore_errors=True)
            raise Exception(f"Installation failed: {str(e)}")
    
    def update_complete(self):
        """Handle update completion"""
        self.progress_frame.pack_forget()
        self.status_label.configure(text="✅ Update completed successfully!")
        self.current_version_label.configure(text=f"Current Version: {self.latest_version}")
        self.update_button.configure(text="Up to Date", state="disabled")
        self.launch_button.configure(state="normal")
        self.update_available = False
        
        # Clean up all cache files after successful update
        self.root.after(0, lambda: self.progress_label.configure(text="Cleaning up cache..."))
        try:
            cache_dir = Path("update_cache")
            self.cleanup_cache(cache_dir, keep_current=False)
        except Exception as e:
            print(f"Cache cleanup failed: {e}")
        
        # Small delay to show cleanup message, then hide progress
        self.root.after(1000, lambda: self.progress_frame.pack_forget())
    
    def show_error(self, message):
        """Show error message"""
        self.progress_frame.pack_forget()
        self.status_label.configure(text=f"❌ {message}")
        self.update_button.configure(state="normal" if self.update_available else "disabled")
        self.launch_button.configure(state="normal")
    
    def launch_game(self):
        """Launch the main application"""
        installation_path = Path(self.config["installation_path"])
        exe_path = installation_path / self.config["exe_path"]
        
        if exe_path.exists():
            try:
                subprocess.Popen([str(exe_path)], cwd=str(installation_path))
                self.root.quit()
            except Exception as e:
                self.show_error(f"Failed to launch: {str(e)}")
        else:
            self.show_error("Makeplace.exe not found! Please check your installation path in settings.")
    
    def run(self):
        """Start the application"""
        self.root.mainloop()

if __name__ == "__main__":
    app = ReMakeplaceUpdater()
    app.run() 