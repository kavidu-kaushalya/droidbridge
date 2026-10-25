@echo off
title DroidBridge Real Layout Inflater Test
cd /d "%~dp0"
echo ===============================================================
echo   DroidBridge - Android Real Layout Inflater ^& ARSC Test
echo   Zero Hypervisors ^| Zero Emulators ^| Real APK Bytecode ^& Layouts
echo ===============================================================
echo.
echo [*] Step 1: Resolving WhatsApp App Label from resources.arsc (0x7f125623)...
target\debug\droidbridge_cli.exe resolve-res "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk" 0x7f125623
echo.
echo [*] Step 2: Inflating Real Android Binary Layout XML (0x7f0e0001 -> res/ges.xml)...
target\debug\droidbridge_cli.exe inflate-layout "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk" 0x7f0e0001
echo.
echo [*] Step 3: Inflating Real Android Binary Layout XML (0x7f0e0005 -> res/gew.xml)...
target\debug\droidbridge_cli.exe inflate-layout "C:\Users\nactro\Downloads\com.whatsapp-v2.26.41.71_264107133-Android_6.0.apk" 0x7f0e0005
pause
