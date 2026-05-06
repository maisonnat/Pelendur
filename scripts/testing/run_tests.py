#!/usr/bin/env python3
"""Pelendur Autonomous Test Suite — runs against running Pelendur instance via CDP."""

import asyncio
import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from cdp_utils import CDPConnection, CDPError

REPORT_DIR = Path("test-report")


class PelendurTest:
    def __init__(self, host: str = "127.0.0.1", port: int = 9224):
        self.cdp = CDPConnection(host, port)
        self.results: list[dict] = []
        self.start_time = time.time()

    async def connect(self) -> None:
        print("  🔌 Connecting to Pelendur via CDP...")
        await self.cdp.connect()
        print("  ✅ Connected")

    async def run_all(self) -> bool:
        tests = [
            ("stt_pipeline", self.test_stt_pipeline),
            ("capture_modes", self.test_capture_modes),
            ("ui_elements", self.test_ui_elements),
            ("shortcuts", self.test_shortcuts),
            ("visual_regression", self.test_visual_regression),
        ]
        all_passed = True
        for name, test_fn in tests:
            print(f"\n  ▶️  {name}...")
            try:
                await test_fn()
                self.results.append({"test": name, "passed": True, "error": None})
                print(f"  ✅ {name} PASSED")
            except Exception as e:
                self.results.append({"test": name, "passed": False, "error": str(e)})
                print(f"  ❌ {name} FAILED: {e}")
                all_passed = False
        return all_passed

    async def test_stt_pipeline(self) -> None:
        wav_path = str(Path(__file__).parent / "audio" / "what_is_your_greatest_strength.wav")
        if not os.path.exists(wav_path):
            raise FileNotFoundError(f"Test audio not found: {wav_path}")
        result = await self.cdp.invoke("inject_test_audio", {"path": wav_path})
        assert isinstance(result, str), f"Expected string, got {type(result)}"
        assert len(result) > 0, "Empty transcript"
        metrics = await self.cdp.invoke("get_test_metrics", {})
        assert metrics["transcription_count"] >= 1
        assert len(metrics["stt_latency_ms"]) >= 1
        latency = metrics["stt_latency_ms"][0][1]
        assert latency < 30000, f"STT too slow: {latency}ms"

    async def test_capture_modes(self) -> None:
        for mode in ["system", "mic", "dual"]:
            result = await self.cdp.invoke("set_mode", {"mode": mode})
            assert result is None, f"set_mode failed for {mode}: {result}"
            await asyncio.sleep(0.5)
            if mode != "system":
                hud = await self.cdp.invoke("get_hud_state", {})
                assert hud.get("capture_mode") == mode, f"Expected mode {mode}, got {hud}"

    async def test_ui_elements(self) -> None:
        count = await self.cdp.eval('document.querySelectorAll(".icon-btn").length')
        assert count is not None, "Could not query UI elements"
        assert count >= 4, f"Expected >=4 icon buttons, got {count}"
        body_text = await self.cdp.eval("document.body.innerText.length")
        assert body_text is not None and body_text > 0, "Empty body"

    async def test_shortcuts(self) -> None:
        hud_before = await self.cdp.invoke("get_hud_state", {})
        was_locked = hud_before.get("is_locked", False)
        await self.cdp.invoke("simulate_keyboard", {"shortcut": "Ctrl+Alt+L"})
        await asyncio.sleep(0.3)
        hud_after = await self.cdp.invoke("get_hud_state", {})
        assert hud_after["is_locked"] != was_locked, "Lock state did not toggle"
        await self.cdp.invoke("simulate_keyboard", {"shortcut": "Ctrl+Alt+L"})

    async def test_visual_regression(self) -> None:
        img = await self.cdp.screenshot()
        assert len(img) > 1000, f"Screenshot too small: {len(img)} bytes"
        REPORT_DIR.mkdir(parents=True, exist_ok=True)
        screenshot_path = REPORT_DIR / f"screenshot_{int(time.time())}.png"
        with open(screenshot_path, "wb") as f:
            f.write(img)
        print(f"    📸 Saved: {screenshot_path}")

    async def close(self) -> None:
        await self.cdp.close()

    def generate_report(self, all_passed: bool) -> str:
        elapsed = time.time() - self.start_time
        report = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "duration_seconds": round(elapsed, 2),
            "passed": all_passed,
            "results": self.results,
        }
        REPORT_DIR.mkdir(parents=True, exist_ok=True)
        report_path = REPORT_DIR / "report.json"
        with open(report_path, "w") as f:
            json.dump(report, f, indent=2)
        print(f"\n  📊 Report: {report_path}")
        return str(report_path)


async def main():
    print("=" * 50)
    print("  Pelendur Test Suite")
    print("=" * 50)
    tester = PelendurTest()
    try:
        await tester.connect()
        all_passed = await tester.run_all()
        report = tester.generate_report(all_passed)
        sys.exit(0 if all_passed else 1)
    except CDPError as e:
        print(f"  ❌ CDP Error: {e}")
        print("  ℹ️  Make sure Pelendur is running with --remote-debugging-port=9224")
        sys.exit(1)
    finally:
        await tester.close()


if __name__ == "__main__":
    asyncio.run(main())
