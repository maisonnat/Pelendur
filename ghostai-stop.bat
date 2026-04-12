@echo off
echo.
echo ============================================
echo    GhostAI Pilot - Meeting Mode OFF
echo ============================================
echo.
echo Closing GhostAI Pilot...
taskkill /IM ghostai-pilot.exe /F 2>nul

echo Closing VoiceMeeter...
taskkill /IM voicemeeter_x64.exe /F 2>nul

echo Unloading LLM from VRAM...
curl -s -X POST http://localhost:11434/api/generate -d "{\"model\":\"gemma4:e4b\",\"keep_alive\":0}" >nul 2>&1

echo.
echo ============================================
echo   All stopped. VRAM freed.
echo ============================================
echo.
pause
