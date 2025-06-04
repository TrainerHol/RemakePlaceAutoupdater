@echo off
title Build ReMakeplace Updater
echo ====================================
echo Building ReMakeplace Updater
echo ====================================
echo.

echo Installing/updating dependencies...
pip install -r requirements.txt
if %errorlevel% neq 0 (
    echo Failed to install dependencies!
    pause
    exit /b 1
)

echo.
echo Building standalone executable...
python build.py
if %errorlevel% neq 0 (
    echo Build failed!
    pause
    exit /b 1
)

echo.
echo ====================================
echo Build completed successfully!
echo Check the 'release' folder for files
echo ====================================
pause 