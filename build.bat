@echo off
REM =====================================================
REM  Pelendur Build Script — CORRER SOLO DESDE POWERSHELL
REM  No correr desde WSL! WSL produce binarios Linux.
REM =====================================================
echo.
echo  ===== Pelendur Build =====
echo.
echo  [1/3] Matando procesos previos...
taskkill /f /im pelendur-overlay.exe >nul 2>&1

echo  [2/3] Build debug + testing...
cd /d "%~dp0src-tauri"
cargo build --features audio,testing
if %ERRORLEVEL% NEQ 0 (
    echo  ❌ BUILD FALLIDO
    pause
    exit /b 1
)

echo  [3/3] Copiando .env al dir de salida...
copy "%~dp0.env" "%~dp0target-pelendur\debug\.env" >nul 2>&1

echo.
echo  ✅ Build exitoso!
echo  Binario: %~dp0target-pelendur\debug\pelendur-overlay.exe
echo.
echo  Para ejecutar:
echo    cd /d "%~dp0"
echo    start target-pelendur\debug\pelendur-overlay.exe
echo.
pause
