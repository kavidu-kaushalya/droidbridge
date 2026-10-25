@echo off
title DroidBridge WhatsApp Launcher
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Launching WhatsApp on Windows Host
echo ===============================================================
echo.
target\debug\droidbridge_cli.exe run "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk"
pause
