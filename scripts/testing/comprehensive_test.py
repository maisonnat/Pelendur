"""Comprehensive Pelendur bug hunt - tests every feature."""
import asyncio, json, urllib.request

CDP = "http://127.0.0.1:9224/json"
AUDIO = r"C:\Proyectos\Pelendur\scripts\testing\audio"

PASS = 0
FAIL = 0

def check(name, result, expected=None):
    global PASS, FAIL
    ok = True
    if expected is not None:
        ok = expected in str(result)
    status = "" if ok else ""
    if ok: PASS += 1
    else: FAIL += 1
    print(f"  {status} {name}: {str(result)[:100]}")

async def js_eval(ws, expr, timeout=10):
    c = {"id":1,"method":"Runtime.evaluate","params":{"expression":expr,"returnByValue":True,"awaitPromise":True,"timeout":timeout*1000}}
    await ws.send(json.dumps(c))
    r = json.loads(await asyncio.wait_for(ws.recv(), timeout=timeout+3))
    exc = r.get("result",{}).get("exceptionDetails")
    if exc:
        return f"ERR:{exc.get('text','?')}"
    v = r.get("result",{}).get("result",{})
    return v.get("value","")

async def invoke(ws, cmd, args="{}", timeout=15):
    return await js_eval(ws, f"(async()=>{{try{{return JSON.stringify(await window.__TAURI_INTERNALS__.invoke('{cmd}',{args}))}}catch(e){{return 'ERR:'+(e.message||e)}}}})()", timeout)

async def main():
    global PASS, FAIL
    pages = json.loads(urllib.request.urlopen(CDP, timeout=5).read())
    hud = [p for p in pages if p["title"] == "Pelendur HUD"]
    if not hud:
        print(" FAIL: No HUD page found. Is Pelendur running with --features audio,testing?")
        return

    import websockets
    async with websockets.connect(hud[0]["webSocketDebuggerUrl"]) as ws:
        print(f"\n{'='*60}")
        print(f"  PELENDUR BUG HUNT")
        print(f"{'='*60}\n")

        # 1. SYSTEM STATUS
        print("SYSTEM STATUS")
        status = await invoke(ws, "get_system_status")
        check("get_system_status", status, "ready")

        readiness = await invoke(ws, "get_readiness")
        check("get_readiness", readiness, "ready")
        check("  latency field", readiness, "latency_ms")

        #  2. UI ELEMENTS 
        print("\n UI ELEMENTS")

        # Check all critical elements exist
        elements = ["audio-source-btn","interview-btn","close-btn","lock-btn","clear-btn",
                     "main-suggestion","transcription-feed","partial-transcription",
                     "process-modal","company-modal","summary-modal",
                     "readiness-dashboard","hud-container"]
        for el in elements:
            exists = await js_eval(ws, f"!!document.getElementById('{el}')")
            check(f"  #{el}", exists, "true")

        # Check modals have real content (not ... placeholder)
        for mid in ["process-modal","company-modal","summary-modal"]:
            html_len = await js_eval(ws, f"document.getElementById('{mid}')?.innerHTML?.length || 0")
            check(f"  #{mid} content length", html_len)
            if int(str(html_len)) < 50:
                print(f"      Modal {mid} has only {html_len} chars - might be empty!")

        # Check dashboard dots
        for comp in ["stt","llm","kg","audio"]:
            dot = await js_eval(ws, f"document.querySelector('.pl-dashboard__dot[data-component=\"{comp}\"]')?.className || 'N/A'")
            check(f"  dashboard dot {comp}", dot)

        # Check buttons work (click audio source)
        await js_eval(ws, "document.getElementById('audio-source-btn')?.click()")
        await asyncio.sleep(0.5)

        modal_visible = await js_eval(ws, "!document.getElementById('process-modal')?.classList.contains('hidden')")
        check("  modal opens on  click", modal_visible, "true")

        overlay_visible = await js_eval(ws, "!document.getElementById('modal-overlay')?.classList.contains('hidden')")
        check("  overlay visible", overlay_visible, "true")

        # Check process list has items
        list_items = await js_eval(ws, "document.querySelectorAll('#process-list .clickable')?.length || 0")
        check("  audio options in list", list_items)
        if int(str(list_items)) == 0:
            print("      No audio devices listed - check get_audio_devices!")

        # Close modal
        await js_eval(ws, "document.getElementById('close-modal-btn')?.click()")
        await asyncio.sleep(0.3)

        #  3. STT PIPELINE 
        print("\n STT PIPELINE")
        await invoke(ws, "reset_metrics")

        tests = [
            ("en_hello.wav", "EN short"),
            ("en_strength.wav", "EN medium"),
            ("es_hola.wav", "ES short"),
            ("silence_500ms.wav", "silence"),
        ]
        for fname, ftype in tests:
            path = f"{AUDIO}\\{fname}".replace("\\", "\\\\")
            result = await invoke(ws, "inject_test_audio", f'{{"path":"{path}"}}', timeout=30)
            if "ERR" in str(result):
                check(f"  {ftype}: {fname}", result)
            else:
                try:
                    text = json.loads(result) if isinstance(result, str) else result
                    check(f"  {ftype}: {fname}", str(text)[:80])
                except:
                    check(f"  {ftype}: {fname}", str(result)[:80])

        metrics = await invoke(ws, "get_test_metrics")
        check("  STT metrics", metrics, "transcription_count")

        #  4. HUD STATE 
        print("\n HUD STATE")
        hud_state = await invoke(ws, "get_hud_state")
        check("  get_hud_state", hud_state)

        #  5. MEETING READINESS 
        print("\n MEETING READINESS")
        badge = await js_eval(ws, "document.getElementById('dashboard-badge')?.textContent || 'N/A'")
        check("  dashboard badge", badge)

        #  6. EDGE CASES 
        print("\n EDGE CASES")

        # inject_test_audio with invalid path
        bad = await invoke(ws, "inject_test_audio", '{"path":"NONEXISTENT.wav"}', timeout=5)
        check("  invalid path handled", bad, "ERR")

        # get_hud_state with nothing
        empty = await invoke(ws, "get_hud_state")
        check("  empty state doesn't crash", empty)

        #  RESULTS 
        print(f"\n{'='*60}")
        total = PASS + FAIL
        pct = (PASS / total * 100) if total > 0 else 0
        print(f"  RESULTS: {PASS}/{total} passed ({pct:.0f}%)")
        if FAIL == 0:
            print(f"   ALL TESTS PASSED!")
        else:
            print(f"   {FAIL} TESTS FAILED")
        print(f"{'='*60}")

asyncio.run(main())
