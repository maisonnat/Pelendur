"""Pelendur STT + LLM Pipeline Test — standalone script for Windows Python"""
import asyncio
import json
import os
import subprocess
import sys
import time

# Fix console encoding for emoji/unicode
if sys.platform == "win32":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")

PROJECT = r"C:\Proyectos\Pelendur"

async def main():
    print("=" * 60)
    print("  Pelendur Pipeline Test - STT + LLM")
    print("=" * 60)
    passed = 0
    failed = 0
    test_num = [0]

    def step(name):
        test_num[0] += 1
        print(f"\n  [{test_num[0]}] {name}...")
    
    def ok(msg):
        nonlocal passed
        passed += 1
        # Strip or replace unicode for Windows console
        safe = msg.encode("utf-8", errors="replace").decode("utf-8", errors="replace")
        print(f"     [PASS] {safe}")
    
    def fail(msg):
        nonlocal failed
        failed += 1
        safe = msg.encode("utf-8", errors="replace").decode("utf-8", errors="replace")
        print(f"     [FAIL] {safe}")

    # ─── TEST 1: Whisper CLI direct ───
    step("Whisper CLI — Direct Smoke Test")
    whisper_exe = os.path.join(PROJECT, "whisper.cpp", "build", "bin", "Release", "whisper-cli.exe")
    whisper_model = os.path.join(PROJECT, "whisper.cpp", "models", "ggml-tiny.en.bin")
    wav_path = os.path.join(PROJECT, "scripts", "testing", "audio", "tell_me_about_yourself.wav")
    
    if not os.path.exists(whisper_exe):
        fail(f"whisper-cli.exe not found at {whisper_exe}")
    elif not os.path.exists(whisper_model):
        fail(f"Whisper model not found at {whisper_model}")
    elif not os.path.exists(wav_path):
        fail(f"Test WAV not found at {wav_path}")
    else:
        try:
            result = subprocess.run(
                [whisper_exe, "-m", whisper_model, "-f", wav_path, "--no-timestamps"],
                capture_output=True, text=True, timeout=30
            )
            output = result.stdout.strip() or result.stderr.strip()
            if len(output) > 10:
                ok(f"Whisper returned: {output[:100]}...")
            else:
                fail(f"Whisper output too short: '{output}'")
        except subprocess.TimeoutExpired:
            fail("Whisper CLI timed out (>30s)")
        except Exception as e:
            fail(f"Whisper error: {e}")

    # ─── TEST 2: CDP + inject_test_audio ───
    step("STT via inject_test_audio")
    try:
        import json
        import urllib.request
        
        # Use urllib instead of httpx to avoid Python 3.13 incompatibility
        with urllib.request.urlopen("http://127.0.0.1:9224/json", timeout=5) as resp:
            pages = json.loads(resp.read().decode())
        
        hud_page = None
        for p in pages:
            if "Pelendur HUD" in p.get("title", ""):
                hud_page = p
                break
        if not hud_page:
            hud_page = pages[0] if pages else None
        
        if not hud_page:
            fail("No CDP pages found (Pelendur running?)")
        else:
            ok(f"CDP connected: {hud_page.get('title', '?')}")
            
            # Connect WebSocket
            import websockets
            ws_url = hud_page["webSocketDebuggerUrl"]
            async with websockets.connect(ws_url) as ws:
                msg_id = [100]
                
                async def cdp(method, params=None):
                    msg_id[0] += 1
                    cmd = json.dumps({"id": msg_id[0], "method": method, "params": params or {}})
                    await ws.send(cmd)
                    resp = await ws.recv()
                    data = json.loads(resp)
                    return data.get("result", {})
                
                async def invoke(cmd_name, args=None):
                    # Use stringify to ensure we always get a string back
                    js = f"window.__TAURI__.core.invoke('{cmd_name}', {json.dumps(args or {})}).then(r => JSON.stringify(r))"
                    result = await cdp("Runtime.evaluate", {"expression": js, "awaitPromise": True})
                    exc = result.get("exceptionDetails")
                    if exc:
                        err_text = exc.get("text", str(exc))
                        return {"_error": err_text}
                    val = result.get("result", {}).get("value")
                    if isinstance(val, str):
                        try:
                            parsed = json.loads(val)
                            return parsed
                        except (json.JSONDecodeError, TypeError):
                            pass
                    return val if val is not None else {"_null": True}
                
                ok("WebSocket CDP connected")
                
                # Inject test audio
                wav_inject = os.path.join(PROJECT, "scripts", "testing", "audio", "what_is_your_greatest_strength.wav").replace("\\", "/")
                transcript = await invoke("inject_test_audio", {"path": wav_inject})
                if transcript and len(str(transcript)) > 0:
                    ok(f"Transcription: {str(transcript)[:100]}")
                else:
                    fail(f"Empty transcript: {transcript}")
                
                # Get metrics
                time.sleep(2)
                metrics = await invoke("get_test_metrics")
                if metrics and isinstance(metrics, dict) and "_error" not in metrics and "_null" not in metrics:
                    ok(f"transcription_count={metrics.get('transcription_count', '?')}")
                    latencies = metrics.get("stt_latency_ms", [])
                    if latencies:
                        last_lat = latencies[-1][1] if isinstance(latencies[-1], (list, tuple)) else 0
                        ok(f"Last STT latency: {last_lat}ms")
                    else:
                        ok("No STT latency recorded (first run)")
                else:
                    fail(f"get_test_metrics: {str(metrics)[:100]}")
                
                # ─── TEST 3: LLM — Start Interview ───
                step("LLM — Start Interview")
                start_result = await invoke("start_interview", {"companyName": "Test Company"})
                ok(f"Interview started")
                
                time.sleep(2)
                
                # Inject transcription to trigger LLM
                wav_inject2 = os.path.join(PROJECT, "scripts", "testing", "audio", "describe_a_challenge.wav").replace("\\", "/")
                trans2 = await invoke("inject_test_audio", {"path": wav_inject2})
                if trans2:
                    ok(f"Transcription: {str(trans2)[:60]}")
                
                # Wait for LLM
                print("     Waiting 15s for LLM to generate suggestion...")
                await asyncio.sleep(15)
                
                # Check HUD suggestion
                try:
                    result = await cdp("Runtime.evaluate", {
                        "expression": "document.getElementById('main-suggestion').innerText",
                        "returnByValue": True
                    })
                    suggestion = result.get("result", {}).get("value", "")
                    if suggestion and len(suggestion) > 10 and "Esperando" not in suggestion and "Cargando" not in suggestion:
                        ok(f"Suggestion: {suggestion[:150]}")
                    else:
                        fail(f"Suggestion unchanged: '{suggestion[:60]}'")
                except Exception as e:
                    fail(f"Suggestion eval error: {e}")
                
                # End interview
                summary = await invoke("end_interview")
                if summary and isinstance(summary, dict) and "_error" not in summary:
                    summary_text = summary.get("summary_text", "")
                    if summary_text:
                        ok(f"Summary generated ({len(summary_text)} chars)")
                    else:
                        ok("Interview ended (summary received)")
                else:
                    err = summary.get("_error", str(summary)) if isinstance(summary, dict) else str(summary)
                    fail(f"end_interview: {err}")
                
                # ─── TEST 4: HUD State ───
                step("HUD State Check")
                hud_state = await invoke("get_hud_state")
                if hud_state:
                    ok(f"is_locked={hud_state.get('is_locked')} is_minimal={hud_state.get('is_minimal')}")
                else:
                    fail("get_hud_state failed")
                
                # Reset
                await invoke("reset_metrics")
                ok("Metrics reset")
                
    except ImportError as e:
        fail(f"Missing Python dep: {e}")
        print("  Run: pip install websockets httpx")
    except Exception as e:
        fail(f"CDP test error: {e}")

    print(f"\n{'=' * 60}")
    if failed == 0:
        print(f"  [PASS] ALL {passed} TESTS PASSED")
    else:
        print(f"  [PASS] {passed} passed, [FAIL] {failed} failed")
    print(f"{'=' * 60}")
    return 0 if failed == 0 else 1

if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
