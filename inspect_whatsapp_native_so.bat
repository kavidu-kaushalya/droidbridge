@echo off
title DroidBridge WhatsApp Native ELF Library Inspector
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Inspecting WhatsApp x86_64 Native ELF (.so)
echo ===============================================================
echo.
target\debug\droidbridge_cli.exe inspect-elf fixtures\lib\x86_64\libs.so
pause
