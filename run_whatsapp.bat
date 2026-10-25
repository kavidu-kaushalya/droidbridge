@echo off
title DroidBridge Real Android Runtime Runner
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Running Real APK on Native Windows Desktop
echo   Zero Hypervisors ^| Zero Emulators ^| Direct Win32 Execution
echo ===============================================================
echo.
target\debug\droidbridge_cli.exe run "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk"
pause
