"""Debug STT with full error details."""
import asyncio, json, urllib.request

CDP_URL = "http://127.0.0.1:9224/json"

async def main():
    pages = json.loads(urllib.request.urlopen(CDP_URL, timeout=5).read())
    hud = [p for p in pages if p["title"] == "Pelendur HUD"]

    import websockets
    async with websockets.connect(hud[0]["webSocketDebuggerUrl"]) as ws:
        async def js_expr(expr, timeout=30):
            cmd = {"id":1,"method":"Runtime.evaluate","params":{"expression":expr,"returnByValue":True,"awaitPromise":True,"timeout":timeout*1000}}
            await ws.send(json.dumps(cmd))
            r = json.loads(await asyncio.wait_for(ws.recv(), timeout=timeout+5))
            return r

        # 1. Check if file exists (from Rust perspective)
        r = await js_expr("(async()=>{try{const r=await window.__TAURI_INTERNALS__.invoke('get_test_metrics');return JSON.stringify(r)}catch(e){return 'ERR:'+e}})()")
        val = r.get("result",{}).get("result",{}).get("value","N/A")
        print(f"Metrics: {val}")

        # 2. Inject test audio with error catching
        audio = "C:\\\\Proyectos\\\\Pelendur\\\\scripts\\\\testing\\\\audio\\\\what_is_your_greatest_strength.wav"
        r = await js_expr(f"(async()=>{{try{{const r=await window.__TAURI_INTERNALS__.invoke('inject_test_audio',{{path:'{audio}'}});return JSON.stringify(r)}}catch(e){{return 'ERR:'+e.message||e}}}})()", 60)
        val2 = r.get("result",{}).get("result",{}).get("value","N/A")
        print(f"STT: {val2}")

        # 3. Check metrics after STT
        r = await js_expr("(async()=>{try{const r=await window.__TAURI_INTERNALS__.invoke('get_test_metrics');return JSON.stringify(r)}catch(e){return 'ERR:'+e}})()")
        val3 = r.get("result",{}).get("result",{}).get("value","N/A")
        print(f"After: {val3}")

    print("DONE")

asyncio.run(main())
