"""Pelendur test with proper await on __TAURI_INTERNALS__.invoke."""
import asyncio, json, urllib.request

CDP_URL = "http://127.0.0.1:9224/json"

def p(s):
    try: print(s)
    except: print(str(s).encode("ascii","replace").decode("ascii"))

async def main():
    pages = json.loads(urllib.request.urlopen(CDP_URL, timeout=5).read())
    hud = [p for p in pages if p["title"] == "Pelendur HUD"]
    if not hud:
        p("FAIL: No HUD")
        return

    import websockets
    async with websockets.connect(hud[0]["webSocketDebuggerUrl"]) as ws:
        async def js(expr, timeout=15):
            cmd = {"id":1,"method":"Runtime.evaluate","params":{"expression":expr,"returnByValue":True,"awaitPromise":True,"timeout":timeout*1000}}
            await ws.send(json.dumps(cmd))
            r = json.loads(await asyncio.wait_for(ws.recv(), timeout=timeout+3))
            exc = r.get("result",{}).get("exceptionDetails")
            if exc:
                return {"error": exc.get("text","?"), "desc": str(exc.get("exception",{}).get("description",""))[:400]}
            val = r.get("result",{}).get("result",{})
            return val.get("value", "")

        # 1. Check system status bar
        bar = await js("document.getElementById('pl-system-status')?.className || 'N/A'")
        p(f"Status bar: {bar}")

        # 2. Get dots
        dots = await js("""(function(){
            const s=document.getElementById('pl-system-status');
            if(!s)return'{}';
            const d={};
            s.querySelectorAll('[data-component]').forEach(e=>{
                const dot=e.querySelector('.pl-status-dot');
                d[e.dataset.component]=dot?dot.className:'?';
            });
            return JSON.stringify(d);
        })()""")
        p(f"Dots: {dots}")

        # 3. Get suggestion
        sug = await js("document.getElementById('main-suggestion')?.textContent || 'N/A'")
        p(f"Suggestion: {sug}")

        # 4. Try get_system_status via __TAURI_INTERNALS__ with AWAIT
        status = await js("(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('get_system_status');return JSON.stringify(s);})()")
        p(f"SystemStatus: {status}")

        # 5. Try get_hud_state with AWAIT
        hud_state = await js("(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('get_hud_state');return JSON.stringify(s);})()")
        p(f"HUDState: {hud_state}")

        # 6. Inject test audio with AWAIT
        audio_path = r"C:\\Proyectos\\Pelendur\\scripts\\testing\\audio\\what_is_your_greatest_strength.wav"
        stt = await js(f"(async()=>{{const s=await window.__TAURI_INTERNALS__.invoke('inject_test_audio',{{path:'{audio_path}'}});return s;}})", timeout=60)
        p(f"STT ({len(str(stt))}ch): {str(stt)[:300]}")

        # 7. Get metrics
        metrics = await js("(async()=>{const s=await window.__TAURI_INTERNALS__.invoke('get_test_metrics');return JSON.stringify(s);})()")
        p(f"Metrics: {metrics}")

    p("DONE")

asyncio.run(main())
