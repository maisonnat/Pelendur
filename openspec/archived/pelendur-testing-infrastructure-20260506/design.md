# Design: Pelendur Testing Infrastructure

## Architecture

```
WSL (Hermes)
  │
  ├── scripts/testing/run_tests.py  ← Test suite (Python)
  │       │
  │       ├── CDP connect → WebView2 (127.0.0.1:9224)
  │       ├── invoke("get_test_metrics") → Tauri command
  │       ├── invoke("inject_test_audio", "test.wav") → Tauri command
  │       ├── CDP Runtime.evaluate() → Read HUD DOM
  │       ├── CDP Page.captureScreenshot() → Visual check
  │       └── Save metrics to pelendur.db
  │
  └── scripts/testing/cdp_utils.py  ← CDP helpers
```

## F0: CDP Scripts (Python, ~50 lines)

**File**: `scripts/testing/cdp_utils.py`
- Connect to `ws://127.0.0.1:9224/devtools/page/<id>`
- `hud_invoke(command, args)` → calls `window.__TAURI__.invoke(command, args)` via Runtime.evaluate
- `hud_eval(js)` → runs JS in HUD context
- `hud_screenshot()` → captures screenshot via CDP
- Auto-discovers HUD tab from page list

**File**: `scripts/testing/run_tests.py`
Test functions, one per feature area.

## F1: Tauri Instrumentation

### Cargo.toml changes

**ghostai-pilot/Cargo.toml** — ADD:
```toml
testing = []
```

**src-tauri/Cargo.toml** — ADD:
```toml
testing = ["ghostai-pilot/testing"]
```

Also add `tempfile` dep (for inject_test_audio).

### New struct: TestMetrics (in state.rs)

```rust
#[derive(Default, Serialize)]
pub struct TestMetrics {
    pub stt_latency_ms: Vec<(String, u64)>,  // (text, latency_ms)
    pub pipeline_count: u64,
    pub capture_mode: String,
    pub uptime_seconds: u64,
    pub transcription_count: u64,
    pub errors: Vec<String>,
}
```

Add to AppState:
```rust
#[cfg(feature = "testing")]
pub test_metrics: Arc<Mutex<TestMetrics>>,
```

Atomic counters in a global or in AppState for tracking:
- STT latency (timestamp when audio captured → when transcript received)
- Pipeline count
- Current mode

### New file: src-tauri/src/commands/testing.rs

Commands:

1. `get_test_metrics() -> TestMetrics`
   Returns current accumulated metrics from AppState.

2. `inject_test_audio(path: String) -> Result<String, String>`
   Reads WAV file from disk, feeds it directly into the STT pipeline (bypassing WASAPI/audio capture).
   Uses `stt::transcribe_local_sync()` directly.
   Returns transcript text.

3. `get_hud_state() -> HudState`
   Returns serialized state: current mode, is_locked, is_minimal, interview_active, last_transcript, last_suggestion.

4. `simulate_keyboard(shortcut: String) -> Result<(), String>`
   Triggers the global shortcut handler with the given shortcut.
   Shortcuts supported: "Ctrl+Alt+L" (lock), "Ctrl+Shift+Q" (quit).

5. `set_mode(mode: String) -> Result<(), String>`
   Changes capture mode: "system", "mic", "dual".
   Stops current capture and starts new one with specified mode.

6. `reset_metrics()`
   Resets all test counters to zero.

### DB Table: test_metrics

```sql
CREATE TABLE IF NOT EXISTS test_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_name TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    stt_latency_ms REAL,
    pipeline_ms REAL,
    mode TEXT,
    passed INTEGER,
    details TEXT  -- JSON blob
);
```

### main.rs registration

```rust
#[cfg(feature = "testing")]
commands::testing::get_test_metrics,
#[cfg(feature = "testing")]
commands::testing::inject_test_audio,
#[cfg(feature = "testing")]
commands::testing::get_hud_state,
#[cfg(feature = "testing")]
commands::testing::simulate_keyboard,
#[cfg(feature = "testing")]
commands::testing::set_mode,
#[cfg(feature = "testing")]
commands::testing::reset_metrics,
```

### STT Pipeline Changes

`stt.rs` already has `transcribe_local_sync(config, wav_bytes)` that works synchronously.
The `inject_test_audio` command will:
1. Read WAV file with `hound`
2. Call `transcribe_local_sync()` with the bytes
3. Record latency
4. Return transcript text

### STT Latency Timing

In `audio.rs` where STT is called, wrap with timing:

```rust
let start = std::time::Instant::now();
let result = stt::transcribe_local_sync(&config, &wav_bytes);
let latency = start.elapsed();
// Store latency in test_metrics
```

## F2: Test Suite Script (Python)

**File**: `scripts/testing/run_tests.py`

```python
# Architecture
class PelendurTest:
    def __init__(self):
        self.cdp = CDPConnection("127.0.0.1:9224")
        self.metrics = []
    
    def test_stt_pipeline(self):
        """Inject test audio and verify transcript"""
        wav_path = "scripts/testing/audio/what_is_your_strength.wav"
        result = self.cdp.invoke("inject_test_audio", {"path": wav_path})
        assert "strength" in result.lower()
        self.record_metric("stt_pipeline", latency=...)
    
    def test_capture_modes(self):
        """Test system/mic/dual mode switching"""
        for mode in ["system", "mic", "dual"]:
            self.cdp.invoke("set_mode", {"mode": mode})
            time.sleep(1)
            hud = self.cdp.invoke("get_hud_state", {})
            assert hud["mode"] == mode
    
    def test_ui_elements(self):
        """Verify HUD DOM elements are visible"""
        buttons = self.cdp.eval(
            'document.querySelectorAll(".icon-btn").length'
        )
        assert buttons >= 6
    
    def test_shortcuts(self):
        """Test keyboard shortcuts"""
        self.cdp.invoke("simulate_keyboard", {"shortcut": "Ctrl+Alt+L"})
        hud = self.cdp.invoke("get_hud_state", {})
        assert hud["is_locked"] == True
    
    def test_visual_regression(self):
        """Screenshot + basic checks"""
        img = self.cdp.screenshot()
        # Check image is not blank
        assert img.size > 1000
    
    def run_all(self):
        """Run all tests and generate report"""
        ...
```

## F3: CI Integration

**.github/workflows/test.yml**

```yaml
name: Testing

on: [pull_request]

jobs:
  test:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - name: Build with testing feature
        run: cd src-tauri && cargo build --features testing
      - name: Run test suite
        run: python scripts/testing/run_tests.py
      - name: Upload test report
        uses: actions/upload-artifact@v4
        with:
          name: test-report
          path: test-report/
```

## Test Audio Files

- `scripts/testing/audio/what_is_your_greatest_strength.wav` — ~3s, 16kHz mono
- `scripts/testing/audio/tell_me_about_yourself.wav` — ~5s, 16kHz mono  
- Generated via Python TTS (pyttsx3/gTTS) or bundled with the repo
- Small files (< 100KB each) to keep in git
