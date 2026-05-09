# Comprehensive pipeline test via CDP
$ErrorActionPreference = "Stop"

function Send-CdpCommand($ws, $cmd) {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($cmd)
    $ws.SendAsync([ArraySegment[byte]]::new($bytes), 1, $true, [System.Threading.CancellationToken]::None).Wait()
    Start-Sleep -Milliseconds 800
    $buf = New-Object byte[] 262144
    $result = $ws.ReceiveAsync([ArraySegment[byte]]::new($buf), [System.Threading.CancellationToken]::None).Result
    return [System.Text.Encoding]::UTF8.GetString($buf, 0, $result.Count)
}

function Get-CdpWs {
    param($title)
    $pages = Invoke-RestMethod -Uri "http://127.0.0.1:9224/json"
    $page = $pages | Where-Object { $_.title -eq $title }
    if (-not $page) { throw "Page '$title' not found" }
    $ws = New-Object System.Net.WebSockets.ClientWebSocket
    $ws.ConnectAsync($page.webSocketDebuggerUrl, [System.Threading.CancellationToken]::None).Wait()
    Start-Sleep -Milliseconds 200
    return $ws
}

# Start Pelendur
Write-Output "=== Starting Pelendur ==="
Get-Process pelendur-overlay -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 2
cd C:\Proyectos\Pelendur
$proc = Start-Process -FilePath "C:\Proyectos\Pelendur\target-pelendur\debug\pelendur-overlay.exe" -WorkingDirectory "C:\Proyectos\Pelendur" -PassThru -RedirectStandardOutput "C:\Proyectos\Pelendur\pelendur-out2.txt" -RedirectStandardError "C:\Proyectos\Pelendur\pelendur-err2.txt"
Start-Sleep -Seconds 10
Write-Output "PID: $($proc.Id)"

# Connect to HUD
Write-Output "=== Connecting to HUD ==="
$ws = Get-CdpWs -title "Pelendur HUD"

# Check initial HUD state
Write-Output "=== Initial HUD State ==="
$evalCmd = '{"id":1,"method":"Runtime.evaluate","params":{"expression":"document.getElementById(\"main-suggestion\").textContent","returnByValue":true}}'
$resp = Send-CdpCommand $ws $evalCmd
Write-Output "Suggestion: $($resp | ConvertFrom-Json | Select -ExpandProperty result | Select -ExpandProperty value)"

$ws.CloseAsync(1000, "", [System.Threading.CancellationToken]::None).Wait()

# Inject test audio via Tauri invoke
Write-Output "=== Injecting Test Audio ==="
$wsHud = Get-CdpWs -title "Pelendur HUD"
$audioPath = "C:\Proyectos\Pelendur\scripts\testing\audio\what_is_your_greatest_strength.wav"
$injectCmd = @"
{"id":2,"method":"Runtime.evaluate","params":{"expression":"(async () => { const { invoke } = await import('@tauri-apps/api/core'); return await invoke('inject_test_audio', {path: '$audioPath'}); })()","awaitPromise":true,"returnByValue":true}}
"@
Write-Output "Injecting: $audioPath"
$resp = Send-CdpCommand $wsHud $injectCmd
$result = $resp | ConvertFrom-Json
if ($result.id -eq 2 -and $result.result) {
    Write-Output "STT Result: $($result.result.value)"
} else {
    Write-Output "STT RAW: $resp"
}
$wsHud.CloseAsync(1000, "", [System.Threading.CancellationToken]::None).Wait()

# Check test metrics
Start-Sleep -Seconds 3
Write-Output "=== Test Metrics ==="
$wsMetrics = Get-CdpWs -title "Pelendur HUD"
$metricsCmd = '{"id":3,"method":"Runtime.evaluate","params":{"expression":"(async () => { const { invoke } = await import(\"@tauri-apps/api/core\"); return await invoke(\"get_test_metrics\"); })()","awaitPromise":true,"returnByValue":true}}'
$resp = Send-CdpCommand $wsMetrics $metricsCmd
try {
    $metrics = $resp | ConvertFrom-Json
    $metricsVal = $metrics.result.value
    Write-Output "Metrics: $($metricsVal | ConvertTo-Json)"
} catch {
    Write-Output "METRICS_RAW: $resp"
}
$wsMetrics.CloseAsync(1000, "", [System.Threading.CancellationToken]::None).Wait()

# Cleanup
Write-Output "=== Done ==="
