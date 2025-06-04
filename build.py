#!/usr/bin/env python3
"""
Build script for ReMakeplace Updater using PyInstaller
Creates a standalone executable with no external dependencies
"""

import os
import sys
import subprocess
import shutil
from pathlib import Path

def build_exe():
    """Build standalone executable using PyInstaller"""
    
    print(">> Building ReMakeplace Updater standalone executable...")
    
    # Clean previous builds
    for folder in ['build', 'dist']:
        if Path(folder).exists():
            print(f">> Cleaning {folder}/")
            shutil.rmtree(folder)
    
    # PyInstaller command with optimized settings
    cmd = [
        'pyinstaller',
        '--onefile',                    # Single executable file
        '--windowed',                   # No console window (GUI app)
        '--name=ReMakeplaceUpdater',    # Output executable name
        '--icon=NONE',                  # No icon (you can add one later)
        
        # Include all necessary modules explicitly
        '--hidden-import=customtkinter',
        '--hidden-import=PIL',
        '--hidden-import=PIL._tkinter_finder',
        '--hidden-import=requests',
        '--hidden-import=py7zr',
        '--hidden-import=packaging',
        '--hidden-import=packaging.version',
        '--hidden-import=tkinter',
        '--hidden-import=tkinter.filedialog',
        '--hidden-import=tkinter.messagebox',
        
        # Add data files
        '--add-data=config.json;.',
        
        # Optimization
        '--optimize=2',
        '--strip',
        
        # Clean build
        '--clean',
        
        # Main script
        'updater.py'
    ]
    
    print(">> Running PyInstaller...")
    print(f"Command: {' '.join(cmd)}")
    
    try:
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
        print(">> Build successful!")
        
        # Check if executable was created
        exe_path = Path('dist/ReMakeplaceUpdater.exe')
        if exe_path.exists():
            size_mb = exe_path.stat().st_size / (1024 * 1024)
            print(f">> Executable created: {exe_path}")
            print(f">> Size: {size_mb:.1f} MB")
            
            # Create release folder with all necessary files
            release_folder = Path('release')
            if release_folder.exists():
                shutil.rmtree(release_folder)
            release_folder.mkdir()
            
            # Copy files to release folder
            files_to_copy = [
                ('dist/ReMakeplaceUpdater.exe', 'ReMakeplaceUpdater.exe'),
                ('config.json', 'config.json'),
                ('README.md', 'README.md'),
                ('launch_updater.bat', 'launch_updater.bat'),
            ]
            
            for src, dst in files_to_copy:
                src_path = Path(src)
                if src_path.exists():
                    shutil.copy2(src_path, release_folder / dst)
                    print(f">> Copied {src} -> release/{dst}")
            
            print(f">> Release package ready in: {release_folder.absolute()}")
            print(">> You can now distribute the entire 'release' folder!")
            
        else:
            print("ERROR: Executable not found in dist/")
            return False
            
    except subprocess.CalledProcessError as e:
        print(f"ERROR: Build failed: {e}")
        if e.stdout:
            print(f"STDOUT: {e.stdout}")
        if e.stderr:
            print(f"STDERR: {e.stderr}")
        return False
    
    return True

def install_dependencies():
    """Install required dependencies"""
    print(">> Installing dependencies...")
    try:
        subprocess.run([sys.executable, '-m', 'pip', 'install', '-r', 'requirements.txt'], check=True)
        print(">> Dependencies installed!")
        return True
    except subprocess.CalledProcessError as e:
        print(f"ERROR: Failed to install dependencies: {e}")
        return False

if __name__ == "__main__":
    print("ReMakeplace Updater Build Script")
    print("=" * 50)
    
    # Check if we're in the right directory
    if not Path('updater.py').exists():
        print("ERROR: updater.py not found! Please run this script from the project root.")
        sys.exit(1)
    
    # Install dependencies if needed
    if '--install-deps' in sys.argv:
        if not install_dependencies():
            sys.exit(1)
    
    # Build executable
    if build_exe():
        print("\n>> Build completed successfully!")
        print(">> Check the 'release' folder for distributable files")
    else:
        print("\nERROR: Build failed!")
        sys.exit(1) 