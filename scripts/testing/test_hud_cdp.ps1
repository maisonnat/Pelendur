$ErrorActionPreference = "Stop"
$pages = Invoke-RestMethod -Uri "http://127.0.0.1:9224/json"
$hud = $pages | Where-Object { $_.title -eq "Pelendur HUD" }
Write-Output "HUD_URL: $($hud.url)"
Write-Output "HUD_WS: $($hud.webSocketDebuggerUrl)"

$ws = New-Object System.Net.WebSockets.ClientWebSocket
$ws.ConnectAsync($hud.webSocketDebuggerUrl, [System.Threading.CancellationToken]::None).Wait()
Start-Sleep -Milliseconds 300

$js = @"
(function() {
    const status = document.getElementById('pl-system-status');
    const mainSuggestion = document.getElementById('main-suggestion');
    const partialDiv = document.getElementById('partial-transcription');
    const statusDot = document.getElementById('status-indicator');
    const dots = {};
    if (status) {
        status.querySelectorAll('[data-component]').forEach(el => {
            const dot = el.querySelector('.pl-status-dot');
            dots[el.dataset.component] = dot ? dot.className : 'NO_DOT';
        });
    }
    return JSON.stringify({
        statusBarClass: status ? status.className : 'NOT_FOUND',
        dots: dots,
        mainSuggestion: mainSuggestion ? mainSuggestion.textContent.substring(0,100) : 'NOT_FOUND',
        partialClass: partialDiv ? partialDiv.className : 'NOT_FOUND',
        statusDotClass: statusDot ? statusDot.className : 'NOT_FOUND',
        buttons: {
            audio: !!document.getElementById('audio-source-btn'),
            interview: !!document.getElementById('interview-btn')
        }
    });
})()
"@

$evalCmd = "{""id"":1,""method"":""Runtime.evaluate"",""params"":{""expression"":" + $($js -replace '"', '\"') + ",""returnByValue"":true}}"

$bytes = [System.Text.Encoding]::UTF8.GetBytes($evalCmd)
$ws.SendAsync([ArraySegment[byte]]::new($bytes), 1, $true, [System.Threading.CancellationToken]::None).Wait()
Start-Sleep -Milliseconds 500

$buf = New-Object byte[] 65536
$result = $ws.ReceiveAsync([ArraySegment[byte]]::new($buf), [System.Threading.CancellationToken]::None).Result
$resp = [System.Text.Encoding]::UTF8.GetString($buf, 0, $result.Count)
Write-Output "RAW_RESP: $resp"
$ws.CloseAsync(1000, "", [System.Threading.CancellationToken]::None).Wait()
