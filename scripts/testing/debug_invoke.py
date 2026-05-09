"""Debug: show full error info for invoke failures."""
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

        # Get raw response for get_system_status
        r = await js_expr("(async()=>await window.__TAURI_INTERNALS__.invoke('get_system_status'))()")
        print(f"STATUS_RESP: {json.dumps(r, indent=2)[:1000]}")

        # Get raw response for inject_test_audio
        r2 = await js_expr("(async()=>await window.__TAURI_INTERNALS__.invoke('inject_test_audio',{path:'C:\\\\Proyectos\\\\Pelendur\\\\scripts\\\\testing\\\\audio\\\\what_is_your_greatest_strength.wav'}))()", 60)
        print(f"STT_RESP: {json.dumps(r2, indent=2)[:1500]}")

        # Check if whisper model is loaded
        r3 = await js_expr("(async()=>{try{const r=await window.__TAURI_INTERNALS__.invoke('get_test_metrics');return JSON.stringify(r)}catch(e){return 'ERR:'+e}})()")
        val = r3.get("result",{}).get("result",{}).get("value","N/A")
        print(f"METRICS_VAL: {val}")

    print("DONE")

asyncio.run(main())
