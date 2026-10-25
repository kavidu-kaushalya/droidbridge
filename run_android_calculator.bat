@echo off
title DroidBridge Android Calculator Launcher
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Native Android Calculator on Windows Host
echo   Zero Hypervisors ^| Zero Emulators ^| Native Win32 ^& ART
echo ===============================================================
echo.
echo Launching com.android.calculator2...
target\debug\droidbridge_cli.exe calculator
pause
