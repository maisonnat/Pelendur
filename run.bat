@echo off
REM GhostAI Pilot - Windows Quick Start
echo.
echo ====================================
echo    GhostAI Pilot - Windows Setup
echo ====================================
echo.

REM Check if Rust is installed
where cargo >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Rust not found. Install from https://rustup.rs
    echo Then restart this script.
    pause
    exit /b 1
)

REM Check if .env exists
if not exist .env (
    echo [SETUP] Creating .env from template...
    copy .env.example .env
    echo.
    echo [ACTION] Edit .env and add your API keys:
    echo   - GROQ_API_KEY  (from https://console.groq.com)
    echo   - OPENAI_API_KEY (or use Ollama locally)
    echo.
    notepad .env
    echo.
    pause
)

echo [BUILD] Compiling with audio support...
cargo build --features audio
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Build failed. Common issues:
    echo   1. Install Visual Studio Build Tools:
    echo      https://visualstudio.microsoft.com/visual-cpp-build-tools/
    echo   2. Or install LLVM: https://releases.llvm.org/
    pause
    exit /b 1
)

echo.
echo ====================================
echo    Ready to run!
echo ====================================
echo.
echo On Windows, system audio capture needs ONE of:
echo   1. "Stereo Mix" enabled (Sound Settings ^> Recording)
echo   2. VB-Cable installed (https://vb-audio.com/Cable/)
echo   3. VoiceMeeter installed
echo.
echo If none of these are set up, it will use your microphone.
echo.
pause
echo.
echo [RUN] Starting GhostAI Pilot...
cargo run --features audio
