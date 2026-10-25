@echo off
title DroidBridge WhatsApp Native Desktop App
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - WhatsApp Native Windows Compatibility Runner
echo   Zero Hypervisors ^| Zero Emulators ^| Native Desktop Window
echo ===============================================================
echo.
echo [*] Step 1: Installing WhatsApp into DroidBridge Sandboxed Container...
target\debug\droidbridge_cli.exe install "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk"
echo.
echo [*] Step 2: Launching WhatsApp Activity Window directly on Windows Desktop...
target\debug\droidbridge_cli.exe run com.whatsapp
pause
