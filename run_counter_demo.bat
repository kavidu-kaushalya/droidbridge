@echo off
title DroidBridge Counter App Launcher
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Launching Counter Demo on Windows Host
echo ===============================================================
echo.
target\debug\droidbridge_cli.exe run fixtures\counter_app.apk
pause
