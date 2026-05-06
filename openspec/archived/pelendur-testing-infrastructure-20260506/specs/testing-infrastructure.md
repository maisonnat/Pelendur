## ADDED: `testing` feature flag

**Files**: `ghostai-pilot/Cargo.toml`, `src-tauri/Cargo.toml`

- ADD `testing = []` to ghostai-pilot features
- ADD `testing = ["ghostai-pilot/testing"]` to src-tauri features
- Both crates: feature is additive, no impact on default builds

## ADDED: Test Metrics State

**Files**: `src-tauri/src/state.rs`

- ADD `TestMetrics` struct with fields: stt_latency_ms (Vec<(String,u64)>), pipeline_count (u64), capture_mode (String), uptime_seconds (u64), transcription_count (u64), errors (Vec<String>)
- ADD `test_metrics: Arc<Mutex<TestMetrics>>` to AppState, cfg-gated with `#[cfg(feature = "testing")]`
- ADD `HudState` struct for serialized HUD state response

## ADDED: Testing Commands

**Files**: `src-tauri/src/commands/testing.rs`, `src-tauri/src/main.rs`

- ADD new module `commands::testing`
- ADD command `get_test_metrics()` → TestMetrics
- ADD command `inject_test_audio(path: String)` → Result<String, String>
- ADD command `get_hud_state()` → HudState
- ADD command `simulate_keyboard(shortcut: String)` → Result<(), String>
- ADD command `set_mode(mode: String)` → Result<(), String>
- ADD command `reset_metrics()`
- MODIFY main.rs: register all testing commands behind `#[cfg(feature = "testing")]`

## ADDED: test_metrics Table

**File**: `pelendur.db schema (migration.rs or SQL)`

- ADD table `test_metrics` with columns: id (PK), test_name (TEXT), timestamp (TEXT), stt_latency_ms (REAL), pipeline_ms (REAL), mode (TEXT), passed (INTEGER), details (TEXT)

## ADDED: STT Latency Tracking

**File**: `src-tauri/src/commands/audio.rs`

- MODIFY STT call site: wrap with `std::time::Instant::now()` timing
- Store latency in `state.test_metrics`
- Only active when feature "testing" is enabled

## ADDED: CDP Python Scripts

**Files**: `scripts/testing/cdp_utils.py`, `scripts/testing/run_tests.py`

- ADD cdp_utils.py: CDP connection, invoke/eval/screenshot helpers
- ADD run_tests.py: test suite (STT pipeline, modes, UI, shortcuts, visual)

## ADDED: Test Audio Data

**Files**: `scripts/testing/audio/`

- ADD WAV test files for STT pipeline testing
- Generated via Python TTS, 16kHz mono, <100KB each

## ADDED: CI Workflow

**Files**: `.github/workflows/test.yml`

- ADD workflow that builds with `--features testing` and runs test suite on each PR
