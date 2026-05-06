# Tasks: Pelendur Testing Infrastructure

## Phase A: Cargo.toml & Feature Flags
- [ ] ADD `testing = []` to `ghostai-pilot/Cargo.toml` features
- [ ] ADD `testing = ["ghostai-pilot/testing"]` to `src-tauri/Cargo.toml` features

## Phase B: Tauri State & Commands
- [ ] ADD `TestMetrics` struct and `HudState` struct to `src-tauri/src/state.rs` (cfg-gated)
- [ ] ADD `test_metrics` field to `AppState` (cfg-gated)
- [ ] CREATE `src-tauri/src/commands/testing.rs` with all 6 commands
- [ ] MODIFY `src-tauri/src/main.rs` to register testing commands behind cfg-gate
- [ ] MODIFY `src-tauri/src/commands/audio.rs` to add STT latency tracking (cfg-gated)

## Phase C: SQLite + Metrics Storage
- [ ] ADD `test_metrics` table creation to migration or schema init
- [ ] ADD insert/query helpers for test metrics in Rust

## Phase D: Python CDP Scripts
- [ ] CREATE `scripts/testing/cdp_utils.py` with CDP connection helpers
- [ ] CREATE `scripts/testing/run_tests.py` with test suite
- [ ] Include test for: STT pipeline, mode switching, UI elements, shortcuts, visual
- [ ] Generate test audio WAV files

## Phase E: CI Integration
- [ ] CREATE `.github/workflows/test.yml` for PR testing

## Verification
- [ ] `cargo check --features "audio"` passes (default build, no testing)
- [ ] `cargo check --features "audio,testing"` passes (testing build)
- [ ] `cargo check --features "audio,parakeet,testing"` passes (all features)
- [ ] Scripts are executable
