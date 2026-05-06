import asyncio
import json
import sys
import os
import urllib.request

sys.path.insert(0, os.path.dirname(__file__))
from cdp_utils import CDPConnection

async def run_tests():
    print("=" * 50)
    print("  Pelendur Quick Test Suite")
    print("=" * 50)
    
    cdp = CDPConnection("127.0.0.1", 9224)
    
    try:
        await cdp.connect()
        print("  ✅ CDP Connected\n")
        
        # Test 1: UI Elements
        print("  ▶️  test_ui_elements...")
        count = await cdp.eval('document.querySelectorAll("button").length')
        print(f"     Buttons found: {count}")
        assert count is not None and count >= 4, f"Expected >=4 buttons, got {count}"
        
        body_text = await cdp.eval("document.body.innerText.length")
        assert body_text is not None and body_text > 0, "Empty body"
        print(f"  ✅ UI Elements OK (body: {body_text} chars)\n")
        
        # Test 2: HUD Container
        print("  ▶️  test_hud_container...")
        container = await cdp.eval('document.getElementById("hud-container") !== null')
        assert container, "HUD container not found"
        minimal_icon = await cdp.eval('document.getElementById("minimal-icon") !== null')
        assert minimal_icon, "Minimal icon not found"
        print(f"  ✅ HUD Structure OK\n")
        
        # Test 3: Design System Classes
        print("  ▶️  test_design_system...")
        control_bar = await cdp.eval('document.querySelector(".pl-control-bar") !== null')
        pl_btns = await cdp.eval('document.querySelectorAll(".pl-btn").length')
        pl_card = await cdp.eval('document.querySelector(".pl-card--suggestion") !== null')
        pl_status_dot = await cdp.eval('document.querySelector(".pl-status-dot") !== null')
        print(f"     Control bar: {control_bar}")
        print(f"     pl-btn count: {pl_btns}")
        print(f"     Suggestion card: {pl_card}")
        print(f"     Status dot: {pl_status_dot}")
        assert control_bar, "pl-control-bar not found"
        assert pl_btns >= 5, f"Expected >=5 pl-btn, got {pl_btns}"
        assert pl_card, "Suggestion card not found"
        assert pl_status_dot, "Status dot not found"
        print(f"  ✅ Design System OK\n")
        
        # Test 4: Transcription Feature
        print("  ▶️  test_transcription...")
        trans_feed = await cdp.eval('document.getElementById("transcription-feed") !== null')
        partial = await cdp.eval('document.getElementById("partial-transcription") !== null')
        suggestions = await cdp.eval('document.getElementById("suggestions-feed") !== null')
        print(f"     Transcription feed: {trans_feed}")
        print(f"     Partial transcription: {partial}")
        print(f"     Suggestions feed: {suggestions}")
        assert trans_feed and partial and suggestions, "Missing transcription elements"
        print(f"  ✅ Transcription OK\n")
        
        # Test 5: Visual Regression (Screenshot)
        print("  ▶️  test_visual...")
        img = await cdp.screenshot()
        assert len(img) > 1000, f"Screenshot too small: {len(img)} bytes"
        report_dir = os.path.join(os.path.dirname(__file__), "test-report")
        os.makedirs(report_dir, exist_ok=True)
        ss_path = os.path.join(report_dir, f"screenshot_{int(asyncio.get_event_loop().time())}.png")
        with open(ss_path, "wb") as f:
            f.write(img)
        print(f"  ✅ Screenshot saved: {ss_path} ({len(img)//1024}KB)")
        
        print("\n" + "=" * 50)
        print("  ✅ ALL TESTS PASSED")
        print("=" * 50)
        
    except Exception as e:
        print(f"\n  ❌ TEST FAILED: {e}")
        sys.exit(1)
    finally:
        await cdp.close()

if __name__ == "__main__":
    asyncio.run(run_tests())
