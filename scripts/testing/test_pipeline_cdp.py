#!/usr/bin/env python3
"""Test Pelendur pipeline via CDP WebSocket."""
import asyncio
import json
import urllib.request
import websockets

CDP_URL = "http://127.0.0.1:9224/json"

async def run():
    # Find HUD page
    pages = json.loads(urllib.request.urlopen(CDP_URL, timeout=5).read())
    hud = [p for p in pages if p["title"] == "Pelendur HUD"]
    if not hud:
        print("HUD_NOT_FOUND")
        return
    ws_url = hud[0]["webSocketDebuggerUrl"]
    print(f"HUD_WS: {ws_url}")

    async with websockets.connect(ws_url) as ws:
        # Check initial HUD state
        cmd = {
            "id": 1,
            "method": "Runtime.evaluate",
            "params": {
                "expression": "document.getElementById('main-suggestion')?.textContent || 'NO_SUGGESTION'",
                "returnByValue": True
            }
        }
        await ws.send(json.dumps(cmd))
        resp = json.loads(await ws.recv())
        print(f"SUGGESTION: {resp.get('result', {}).get('value', 'N/A')}")

        # Check system status dots
        cmd2 = {
            "id": 2,
            "method": "Runtime.evaluate",
            "params": {
                "expression": """(function() {
                    const s = document.getElementById('pl-system-status');
                    if (!s) return 'NO_STATUS_BAR';
                    const dots = {};
                    s.querySelectorAll('[data-component]').forEach(el => {
                        const dot = el.querySelector('.pl-status-dot');
                        dots[el.dataset.component] = dot ? dot.className : 'NO_DOT';
                    });
                    return JSON.stringify({class: s.className, dots: dots});
                })()""",
                "returnByValue": True
            }
        }
        await ws.send(json.dumps(cmd2))
        resp2 = json.loads(await ws.recv())
        print(f"STATUS: {resp2.get('result', {}).get('value', 'N/A')}")

        # Inject test audio via invoke
        audio_path = "C:\\Proyectos\\Pelendur\\scripts\\testing\\audio\\what_is_your_greatest_strength.wav"
        cmd3 = {
            "id": 3,
            "method": "Runtime.evaluate",
            "params": {
                "expression": f"""(async () => {{
                    const {{ invoke }} = await import('@tauri-apps/api/core');
                    return await invoke('inject_test_audio', {{path: '{audio_path}'}});
                }})()""",
                "awaitPromise": True,
                "returnByValue": True,
                "timeout": 30000
            }
        }
        await ws.send(json.dumps(cmd3))
        try:
            resp3 = json.loads(await asyncio.wait_for(ws.recv(), timeout=35))
            val = resp3.get("result", {}).get("value", "NO_VALUE")
            print(f"STT_RESULT: {val[:200] if val and len(val) > 200 else val}")
        except asyncio.TimeoutError:
            print("STT_TIMEOUT (35s)")

        # Check test metrics
        cmd4 = {
            "id": 4,
            "method": "Runtime.evaluate",
            "params": {
                "expression": """(async () => {
                    const { invoke } = await import('@tauri-apps/api/core');
                    return JSON.stringify(await invoke('get_test_metrics'));
                })()""",
                "awaitPromise": True,
                "returnByValue": True,
                "timeout": 10000
            }
        }
        await ws.send(json.dumps(cmd4))
        try:
            resp4 = json.loads(await asyncio.wait_for(ws.recv(), timeout=12))
            val4 = resp4.get("result", {}).get("value", "NO_VALUE")
            print(f"METRICS: {val4}")
        except asyncio.TimeoutError:
            print("METRICS_TIMEOUT")

    # Check transcription feed
    async with websockets.connect(ws_url) as ws2:
        cmd5 = {
            "id": 5,
            "method": "Runtime.evaluate",
            "params": {
                "expression": "document.getElementById('transcription-feed')?.innerHTML?.substring(0, 300) || 'EMPTY'",
                "returnByValue": True
            }
        }
        await ws2.send(json.dumps(cmd5))
        resp5 = json.loads(await ws2.recv())
        print(f"FEED: {resp5.get('result', {}).get('value', 'N/A')}")

    print("DONE")

if __name__ == "__main__":
    asyncio.run(run())
