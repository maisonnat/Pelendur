# Proposal: Pelendur Autonomous Testing Infrastructure

## Intent
Darle a Hermes la capacidad de probar TODO en Pelendur de forma autónoma desde WSL — tiempos de respuesta, pipeline STT, modos de captura, estado del HUD, colores de UI, y regresión visual — sin necesidad de que Alejandro abra la app manualmente.

## Problem
Actualmente Hermes no puede verificar si Pelendur funciona correctamente en Windows. Solo sabe si compila (cargo check). No puede:
- Medir latencia STT real
- Saber si el HUD muestra la transcripción correcta
- Probar modos (system/mic/dual) sin asistencia manual
- Detectar regresiones visuales (colores, textos truncados, botones invisibles)
- Verificar el pipeline end-to-end

## Solution
Sistema de testing en 4 fases:

**F0: CDP Connection from WSL** (zero Rust code)
Scripts Python que se conectan al WebView2 via Chrome DevTools Protocol (puerto 9224, ya configurado) para: tomar screenshots, leer DOM, invocar commands, medir tiempos.

**F1: Tauri Instrumentation** (feature "testing")
Commands Tauri detrás de `#[cfg(feature = "testing")]`:
- `get_test_metrics()` → latencias, contadores, estado
- `inject_test_audio(path)` → by-passea WASAPI, inyecta WAV directo al STT
- `get_hud_state()` → estado serializado del HUD
- `simulate_keyboard(shortcut)` → simula atajos
- `set_mode(mode)` → cambia modo programáticamente
- `reset_metrics()` → resetea contadores
Métricas persisten en tabla `test_metrics` de `pelendur.db`.

**F2: Test Suite Script** (Python)
Script autónomo que: lanza Pelendur en modo testing → conecta CDP → ejecuta batería de tests (pipeline, modos, UI, performance) → genera reporte con screenshots y métricas → persiste en DB.

**F3: CI Integration** (GitHub Actions)
Workflow que corre en cada PR: build → deploy → test suite → reporte.

## Scope
- **IN**: HUD overlay + Profile window (ambas ventanas WebView2)
- **IN**: Pipeline STT (whisper.cpp), captura (system/mic/dual), UI rendering
- **IN**: Métricas de performance y persistencia en DB
- **OUT**: Tests de componentes unitarios Rust (eso es cargo test)
- **OUT**: UI/UX redesign (Track B, sesión separada)

## Approach
1. Feature flag `testing` en Cargo.toml (ambos crates)
2. Nuevo archivo `src-tauri/src/commands/testing.rs` con todos los commands
3. Tabla `test_metrics` en el schema SQLite
4. Scripts Python en `scripts/testing/` dentro del repo
5. CI workflow `.github/workflows/test.yml`
