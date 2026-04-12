@echo off
REM GhostAI Pilot - whisper.cpp Setup for Windows
echo.
echo ============================================
echo    whisper.cpp Setup for GhostAI Pilot
echo ============================================
echo.

REM Check prerequisites
where git >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Git not found. Install from https://git-scm.com
    pause
    exit /b 1
)

where cmake >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] CMake not found. Install from https://cmake.org
    echo         Or: winget install Kitware.CMake
    pause
    exit /b 1
)

echo [1/4] Cloning whisper.cpp...
if not exist whisper.cpp (
    git clone https://github.com/ggml-org/whisper.cpp.git
) else (
    echo       Already cloned, pulling latest...
    cd whisper.cpp
    git pull
    cd ..
)

cd whisper.cpp

echo.
echo [2/4] Building with CUDA support (RTX 3080)...
cmake -B build -DGGML_CUDA=1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] CMake failed. Make sure you have:
    echo   - Visual Studio Build Tools (C++ workload)
    echo   - CUDA Toolkit installed
    echo     Download: https://developer.nvidia.com/cuda-downloads
    pause
    exit /b 1
)

cmake --build build -j 2 --config Release
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed.
    pause
    exit /b 1
)

echo.
echo [3/4] Downloading model (base.en - 140MB, fast)...
cd models
call download-ggml-model.cmd base.en
cd ..

echo.
echo [4/4] Testing...
set WHISPER_BIN=%CD%\build\bin\Release\whisper-cli.exe
set WHISPER_MODEL=%CD%\models\ggml-base.bin

if exist samples\jfk.wav (
    echo.
    echo Running test transcription...
    "%WHISPER_BIN%" -m "%WHISPER_MODEL%" -f samples\jfk.wav --no-timestamps
)

cd ..

echo.
echo ============================================
echo    Setup complete!
echo ============================================
echo.
echo Add to your .env file:
echo.
echo   STT_PROVIDER=local
echo   WHISPER_MODEL_PATH=%CD%\whisper.cpp\models\ggml-base.en.bin
echo   WHISPER_LANGUAGE=en
echo   WHISPER_BIN=%WHISPER_BIN%
echo.
echo Or copy .env.example to .env and edit the paths.
echo.
pause
