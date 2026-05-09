"""Test that audio source modal opens correctly."""
import asyncio, json, urllib.request

CDP_URL = "http://127.0.0.1:9224/json"

async def main():
    pages = json.loads(urllib.request.urlopen(CDP_URL, timeout=5).read())
    hud = [p for p in pages if p["title"] == "Pelendur HUD"]
    if not hud:
        print("FAIL: No HUD")
        return

    import websockets
    async with websockets.connect(hud[0]["webSocketDebuggerUrl"]) as ws:
        async def js(expr, timeout=10):
            c = {"id":1,"method":"Runtime.evaluate","params":{"expression":expr,"returnByValue":True,"awaitPromise":True,"timeout":timeout*1000}}
            await ws.send(json.dumps(c))
            r = json.loads(await asyncio.wait_for(ws.recv(), timeout=timeout+3))
            exc = r.get("result",{}).get("exceptionDetails")
            if exc:
                return f"ERR:{exc.get('text','?')}:{exc.get('exception',{}).get('description','')[:200]}"
            v = r.get("result",{}).get("result",{})
            return v.get("value","")

        # Check if process modal exists and has content
        modal = await js("document.getElementById('process-modal')?.innerHTML?.length || 0")
        print(f"Process modal HTML length: {modal}")

        overlay = await js("document.getElementById('modal-overlay')?.className || 'NOT_FOUND'")
        print(f"Modal overlay class: {overlay}")

        process_list = await js("document.getElementById('process-list')?.innerHTML || 'NOT_FOUND'")
        print(f"Process list HTML: {str(process_list)[:200]}")

        # Check company modal too
        company = await js("document.getElementById('company-modal')?.innerHTML?.length || 0")
        print(f"Company modal HTML length: {company}")

        # Try to invoke get_audio_devices to see if it works
        devices = await js("(async()=>{try{const d=await window.__TAURI_INTERNALS__.invoke('get_audio_devices');return 'OK:'+JSON.stringify(d)}catch(e){return 'ERR:'+e.message||e}})()")
        print(f"Audio devices: {str(devices)[:300]}")

        # Simulate click on audio button
        click = await js("(async()=>{document.getElementById('audio-source-btn')?.click();return 'CLICKED'})()")
        print(f"Click result: {click}")

        await asyncio.sleep(1)

        # Check if modal is now visible
        modal_visible = await js("document.getElementById('process-modal')?.classList.contains('hidden') === false")
        print(f"Modal visible after click: {modal_visible}")

        overlay_visible = await js("document.getElementById('modal-overlay')?.classList.contains('hidden') === false")
        print(f"Overlay visible after click: {overlay_visible}")

        # Check what's in the process list now
        list_html = await js("document.getElementById('process-list')?.innerHTML?.substring(0,500) || 'EMPTY'")
        print(f"Process list after click: {str(list_html)[:500]}")

    print("DONE")

asyncio.run(main())
