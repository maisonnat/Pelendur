# Pelendur - Project Conventions

## Architecture
Rust monolith + Tauri v2 UI with multi-window architecture. Real-time audio processing pipeline with local STT inference and hybrid knowledge storage.

## Layers
- **Core Engine** (`src/`): Rust systems layer — audio pipeline, VAD, STT, LLM orchestration
- **UI** (`ui/`): Dual-window architecture — HUD overlay (vanilla JS/CSS) + Profile window (React + Tailwind)
- **Knowledge** (`knowledge/`): Hybrid storage — SQLite for structured data + Markdown/YAML for human-editable docs
- **Build** (`.bat` scripts): Windows automation for Tauri build + Whisper setup

## Rust Conventions
- Edition 2021+
- `clippy` warnings as errors
- `async` via `tokio` runtime
- Strong typing with `serde` for all IPC and data schemas
- Minimize allocations in audio hot path (cpal callbacks)
- VAD gates STT — never process silence
- whisper.cpp bindings: prefer FFI with zero-copy where possible

## Audio Pipeline
- `cpal` for device abstraction
- WASAPI Loopback on Windows for system audio capture
- Low-latency priority: avoid heap allocations in real-time threads
- Buffer sizes tuned for <50ms end-to-end latency

## UI (Tauri v2 Multi-Window)
### HUD Overlay Window
- **Vanilla JS/CSS only** — no React, no bundler, no runtime overhead in HUD
- Tauri events for Rust → JS IPC (async, non-blocking)
- Overlay mode: transparent, always-on-top, click-through
- Frontend must stay <100KB total
- Target: <500ms latency for context injection

### Profile Management Window (NEW)
- **React + Tailwind + Vite** — only allowed for this dedicated window
- Rich UI for knowledge graph visualization and profile management
- Bundle target: <500KB React bundle
- Load time target: <2s
- IPC communication with HUD window via Tauri events

### Multi-Window Architecture
- Two Tauri windows: HUD overlay + Profile management
- Each window has independent tech stack suited to its purpose
- Rust backend serves both windows via Tauri IPC
- Inter-window communication: Rust backend as message broker

## Knowledge Base (Hybrid Storage)
### SQLite (Structured Data)
- Skills, experiences, STAR stories, relationships stored in SQLite
- Fast querying for knowledge graph visualization
- Schema: nodes, edges, attributes with indexing
- Vector embeddings (future): for semantic search (Task 18)

### Markdown/YAML (Human-Editable)
- Documents, notes, and configuration in `knowledge/`
- Schema validation in Rust (`src/knowledge/`)
- Versioned with Git — human-readable and diffable
- Source of truth for content authoring

### Storage Coordination
- SQLite serves knowledge graph queries and visualization
- Markdown/YAML serves content editing and Git versioning
- Hybrid approach: performance + maintainability
- No external DB — SQLite embedded within app

## Error Handling
- Use `anyhow` for application errors, `thiserror` for library errors
- Never panic in audio or STT threads — graceful degradation
- Log via `tracing`, not `println!`

## Git
- Conventional commits: `feat:`, `fix:`, `perf:`, `refactor:`
- Feature branches for new modules
- Keep commits atomic — one concern per commit

## Testing
- Unit tests for pure functions (VAD, parsing, schema validation)
- Integration tests for audio pipeline (mocked devices)
- Benchmark critical paths with `criterion`

## Performance Invariants
- Audio thread: no allocations, no locks, no syscalls beyond cpal
- STT: local inference only — no network in hot path
- UI: must not block Rust event loop
- Knowledge graph queries: <50ms, handle 200+ nodes at 60fps

## Performance Budgets
### Memory
- **App idle (both windows)**: <400MB
- **App idle (HUD only)**: <200MB
- **HUD overlay**: <100KB vanilla JS/CSS
- **Profile window**: <500KB React bundle

### Latency
- **Audio pipeline**: <50ms end-to-end
- **Context injection to HUD**: <500ms
- **Profile window load**: <2s
- **Knowledge graph queries**: <50ms

### Throughput
- **Knowledge graph rendering**: 200+ nodes at 60fps
- **SQLite queries**: <50ms typical, indexed paths

### Startup
- **App startup time**: <5s with all subsystems loaded
- **HUD overlay ready**: <1s after app launch
- **Profile window**: <2s on-demand open
