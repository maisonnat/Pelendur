# PILOT.md — GhostAI Audio Pilot

## Objetivo
Capturar audio del sistema en tiempo real, transcribirlo con STT, y generar respuestas de IA automáticamente. Sin UI bonita — solo el pipeline funcional.

## Flujo
```
[Audio del sistema] → [cpal capture 16kHz] → [VAD detect speech] → [STT (Groq Whisper API)] → [LLM (OpenAI-compatible)] → [Response in terminal]
```

## Stack mínimo
- Rust binary (no Tauri todavía — solo CLI)
- `cpal` — captura de audio
- `rodio` — opcional, para playback testing
- `reqwest` + `tokio` — HTTP async para STT + LLM
- `serde` + `serde_json` — serialization
- `hound` — WAV encoding para enviar a STT API
- `dotenvy` — cargar API keys desde .env

## Archivos
```
pilot/
├── Cargo.toml
├── .env.example
├── src/
│   ├── main.rs          # Entry point, orchestration loop
│   ├── audio.rs         # System audio capture via cpal
│   ├── vad.rs           # Voice Activity Detection (energy-based)
│   ├── stt.rs           # Groq Whisper API transcription
│   ├── llm.rs           # OpenAI-compatible chat completion
│   └── config.rs        # Config from env vars
```

## Tests manuales
1. Ejecutar binary
2. Abrir Zoom/YouTube, reproducir audio
3. Ver transcripción en terminal en tiempo real
4. Ver respuesta de IA generada automáticamente

## Windows setup
Ver [WINDOWS_AUDIO_SETUP.md](WINDOWS_AUDIO_SETUP.md) para configurar captura de audio del sistema en Windows.

Quick start: `run.bat` (compila y ejecuta automáticamente)
