"""Complete Pelendur pipeline benchmark: STT latency + LLM response time."""
import asyncio, json, urllib.request, time, sys

CDP_URL = "http://127.0.0.1:9224/json"
AUDIO_DIR = r"C:\Proyectos\Pelendur\scripts\testing\audio"

async def js_invoke(ws, cmd, args="{}", timeout=30):
    """Execute Tauri command via __TAURI_INTERNALS__ with await."""
    expr = f"(async()=>{{try{{const r=await window.__TAURI_INTERNALS__.invoke('{cmd}',{args});return JSON.stringify(r)}}catch(e){{return 'ERR:'+(e.message||e.toString())}}}})()"
    c = {"id":1,"method":"Runtime.evaluate","params":{"expression":expr,"returnByValue":True,"awaitPromise":True,"timeout":timeout*1000}}
    await ws.send(json.dumps(c))
    r = json.loads(await asyncio.wait_for(ws.recv(), timeout=timeout+5))
    exc = r.get("result",{}).get("exceptionDetails")
    if exc:
        return f"EXC:{exc.get('text','?')}:{exc.get('exception',{}).get('description','')[:200]}"
    val = r.get("result",{}).get("result",{}).get("value","")
    return val

async def benchmark():
    pages = json.loads(urllib.request.urlopen(CDP_URL, timeout=5).read())
    hud = [p for p in pages if p["title"] == "Pelendur HUD"]
    if not hud:
        print("FAIL: HUD not found")
        return

    import websockets
    async with websockets.connect(hud[0]["webSocketDebuggerUrl"]) as ws:
        # Check readiness
        status = await js_invoke(ws, "get_system_status")
        print(f"\n{'='*60}")
        print(f"  SYSTEM STATUS: {status}")
        print(f"{'='*60}\n")

        # Reset metrics
        await js_invoke(ws, "reset_metrics")

        # Benchmark each WAV
        audio_files = [
            ("en_hello.wav", "EN short"),
            ("en_strength.wav", "EN medium"),
            ("en_challenge.wav", "EN long"),
            ("es_hola.wav", "ES short"),
            ("es_fortaleza.wav", "ES medium"),
            ("es_experiencia.wav", "ES long"),
            ("silence_500ms.wav", "silence"),
        ]

        print(f"{'WAV':30s} {'Type':12s} {'Result':50s} {'Time':>8s}")
        print(f"{'-'*30} {'-'*12} {'-'*50} {'-'*8}")

        for fname, ftype in audio_files:
            path = f"{AUDIO_DIR}\\{fname}"
            # Escape backslashes for JSON: C:\path -> C:\\path
            escaped = path.replace("\\", "\\\\")
            start = time.time()
            result = await js_invoke(ws, "inject_test_audio", f'{{"path":"{escaped}"}}', timeout=30)
            elapsed = (time.time() - start) * 1000

            # Get metrics for STT latency
            metrics_raw = await js_invoke(ws, "get_test_metrics")
            try:
                metrics = json.loads(metrics_raw) if isinstance(metrics_raw, str) else {}
                stt_lat = metrics.get("stt_latency_ms", [])
                last_lat = stt_lat[-1][1] if stt_lat else elapsed
            except:
                last_lat = elapsed

            # Truncate result for display
            display = result[:50] if result and "ERR" not in result else result[:60]
            if result.startswith('"') and result.endswith('"'):
                display = result[1:51]
            print(f"{fname:30s} {ftype:12s} {str(display):50s} {last_lat:6.0f}ms")

        # Test LLM response time (start interview + query)
        print(f"\n{'='*60}")
        print(f"  LLM RESPONSE BENCHMARK")
        print(f"{'='*60}")

        start = time.time()
        # Inject a question for LLM
        await js_invoke(ws, "inject_test_audio", f'{{"path":"{AUDIO_DIR}\\en_hello.wav"}}', timeout=30)
        # Check metrics for transcription
        metrics2 = await js_invoke(ws, "get_test_metrics")
        try:
            m = json.loads(metrics2) if isinstance(metrics2, str) else {}
            count = m.get("transcription_count", 0)
            print(f"  Transcriptions processed: {count}")
        except:
            pass

        # Final metrics
        final = await js_invoke(ws, "get_test_metrics")
        print(f"\n  FINAL METRICS: {final[:300]}")

    print(f"\n{'='*60}")
    print(f"  BENCHMARK COMPLETE")
    print(f"{'='*60}")

asyncio.run(benchmark())
