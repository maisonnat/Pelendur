<#
.SYNOPSIS
    Build Pelendur desde PowerShell (NO desde WSL)
.DESCRIPTION
    Compila el .exe de Windows con features audio,testing.
    Mata procesos previos automáticamente.
#>

$ProjectRoot = Split-Path -Parent $PSScriptRoot

Write-Host "`n ===== Pelendur Build (PowerShell) =====`n" -ForegroundColor Cyan

Write-Host "[1/3] Matando procesos previos..." -NoNewline
Get-Process pelendur-overlay -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1
Write-Host " OK" -ForegroundColor Green

Write-Host "[2/3] Build debug + testing..." -NoNewline
Set-Location "$ProjectRoot\src-tauri"
$result = cargo build --features audio,testing 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host " FAILED" -ForegroundColor Red
    $result | Select-String -Pattern "error"
    exit 1
}
Write-Host " OK" -ForegroundColor Green

Write-Host "[3/3] Build release (opcional, solo si --release)..." -NoNewline
Write-Host " SKIP (usa -Release flag)" -ForegroundColor Yellow

Write-Host "`n ✅ Build exitoso!" -ForegroundColor Green
Write-Host "   Binario: $ProjectRoot\target-pelendur\debug\pelendur-overlay.exe`n"

# --- Release build (comentado, descomentar para release) ---
# Write-Host "[3/3] Build release + testing..."
# cargo build --features audio,testing --release
# Write-Host "   Release: $ProjectRoot\target-pelendur\release\pelendur-overlay.exe"

# --- MSI Installer (comentado, descomentar para MSI) ---
# Write-Host "[4/4] Generando MSI installer..."
# cargo tauri build --features audio,testing
# Write-Host "   MSI: $ProjectRoot\target-pelendur\release\bundle\msi\*.msi"
