@echo off
echo.
echo ============================================
echo    GhostAI Pilot - Meeting Mode
echo ============================================
echo.
echo [1/2] Pre-warming LLM (gemma4:e4b)...
curl -s -X POST http://localhost:11434/api/generate -d "{\"model\":\"gemma4:e4b\",\"prompt\":\"hi\",\"stream\":false}" >nul 2>&1

echo [2/2] Starting GhostAI Pilot...
echo.
echo ============================================
echo   Select mode when prompted:
echo   [1] Single device (mic only)
echo   [2] Meeting Mode (mic + app loopback)
echo ============================================
echo.
cd /d C:\Proyectos\Albatroz
.\target\release\ghostai-pilot.exe
