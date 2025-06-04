@echo off
title ReMakeplace Updater Launcher
echo Starting ReMakeplace Updater...
python updater.py
if %errorlevel% neq 0 (
    echo.
    echo Error occurred while running the updater.
    echo Please make sure Python is installed and dependencies are installed:
    echo pip install -r requirements.txt
    echo.
    pause
) 