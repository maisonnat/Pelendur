Write-Host "=== PELENDUR DIAGNOSTIC ===" -ForegroundColor Cyan
Write-Host ""

$bin = "C:\Proyectos\Pelendur\target-pelendur\release\pelendur-overlay.exe"
if (Test-Path $bin) {
    $info = Get-Item $bin
    Write-Host "1. BINARY: $($info.Name) - $([math]::Round($info.Length/1MB,1))MB - $($info.LastWriteTime)"
} else { Write-Host "1. BINARY: NOT FOUND" }

$modelDir = "$env:APPDATA\pelendur\models\parakeet"
if (Test-Path $modelDir) {
    Write-Host "2. MODEL: $modelDir"
    Get-ChildItem $modelDir | ForEach-Object { Write-Host "   $($_.Name) - $([math]::Round($_.Length/1MB,1))MB" }
} else { Write-Host "2. MODEL: NOT FOUND at $modelDir" }

$envFile = "C:\Proyectos\Pelendur\.env"
if (Test-Path $envFile) {
    Write-Host "3. CONFIG (.env): OK"
    Get-Content $envFile | Select-String "STT_PROVIDER|PARAKEET_MODEL_DIR|OPENAI_BASE_URL|OPENAI_MODEL" | ForEach-Object { Write-Host "   $_" }
} else { Write-Host "3. CONFIG: NOT FOUND" }

Write-Host "4. OLLAMA:"
$ollama = & ollama list 2>&1 | Out-String
if ($ollama -match "qwen3") { Write-Host "   OK - qwen3 available" }
elseif ($ollama -match "Error") { Write-Host "   FAIL - Ollama not running" }
else { Write-Host "   WARNING - qwen3 not found: $ollama" }

$svc = Get-Service audiosrv -ErrorAction SilentlyContinue
if ($svc.Status -eq "Running") { Write-Host "5. AUDIO SERVICE: Running" }
else { Write-Host "5. AUDIO SERVICE: NOT Running" }

$procs = Get-Process pelendur-overlay -ErrorAction SilentlyContinue
if ($procs) {
    Write-Host "6. PELENDUR PROCESSES: $($procs.Count) running"
    $procs | ForEach-Object { Write-Host "   PID $($_.Id) - $([math]::Round($_.WorkingSet64/1MB,1))MB" }
} else { Write-Host "6. PELENDUR PROCESSES: None" }

Write-Host ""
Write-Host "=== DONE ===" -ForegroundColor Cyan
Write-Host "To test: open PowerShell, cd C:\Proyectos\Pelendur, then run .\target-pelendur\release\pelendur-overlay.exe"
