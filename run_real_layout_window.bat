@echo off
title DroidBridge Dynamic Android Layout Window
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Dynamic Android Layout Window Renderer
echo   Zero Hypervisors ^| Zero Emulators ^| Direct Native GDI/Win32
echo ===============================================================
echo.
echo [*] Launching Real Android Layout (0x7f0e0001) on Windows Desktop...
target\debug\droidbridge_cli.exe render-layout "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk" 0x7f0e0001
pause
