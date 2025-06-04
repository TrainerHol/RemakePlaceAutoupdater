#!/usr/bin/env python3
"""
Setup script for building ReMakeplace Updater executable
"""

import sys
from cx_Freeze import setup, Executable

# Dependencies to include
packages = [
    "customtkinter",
    "requests",
    "py7zr",
    "packaging",
    "PIL",
    "tkinter",
    "json",
    "threading",
    "subprocess",
    "pathlib",
    "shutil",
    "datetime"
]

# Files to include
include_files = [
    "config.json"
]

# Build options
build_exe_options = {
    "packages": packages,
    "include_files": include_files,
    "excludes": ["unittest", "email", "html", "http", "urllib", "xml"],
    "zip_include_packages": ["encodings", "PySide6"],
    "build_exe": "dist"
}

# Executable configuration
exe = Executable(
    script="updater.py",
    base="Win32GUI" if sys.platform == "win32" else None,
    target_name="ReMakeplaceUpdater.exe",
    icon=None  # Add icon path here if you have one
)

setup(
    name="ReMakeplace Updater",
    version="1.0.0",
    description="Modern auto-updater for ReMakeplace",
    options={"build_exe": build_exe_options},
    executables=[exe]
) 