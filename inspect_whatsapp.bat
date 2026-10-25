@echo off
title DroidBridge WhatsApp Inspector
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Inspecting WhatsApp APK
echo ===============================================================
echo.
target\debug\droidbridge_cli.exe inspect "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk"
pause
