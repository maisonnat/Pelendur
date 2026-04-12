@echo off
echo.
echo ============================================
echo    GhostAI Pilot - Quick Toggle
echo ============================================
echo.
echo What do you want to do?
echo.
echo   [1] Start Meeting Mode (ON)
echo   [2] Stop Meeting Mode (OFF)
echo   [3] Pre-warm LLM only
echo.
set /p choice="Select [1-3]: "

if "%choice%"=="1" call "%~dp0ghostai-start.bat"
if "%choice%"=="2" call "%~dp0ghostai-stop.bat"
if "%choice%"=="3" goto warm

echo Invalid choice.
pause
exit /b

:warm
echo.
echo Pre-warming gemma4:e4b in VRAM...
curl -s -X POST http://localhost:11434/api/generate -d "{\"model\":\"gemma4:e4b\",\"prompt\":\"hello\",\"stream\":false,\"keep_alive\":\"24h\"}" >nul 2>&1
echo Done! Model loaded and will stay in VRAM for 24h.
echo.
pause
