# 🚀 Pelendur Roadmap

## ✅ Implementado

### 1. Pipeline de Audio
- Captura WASAPI loopback (sistema) y cpal (micrófono)
- Dual capture: sistema + micrófono mezclado 50/50
- Voice Activity Detection (VAD) energy-based
- STT local con whisper.cpp tiny (GPU CUDA, <100ms)
- STT cloud con Groq Whisper API
- Resampling automático 48→16kHz

### 2. Perfil Personal
- Perfil personal integrado desde knowledge/personal/
- Matching de historias STAR en tiempo real
- Motor de contexto modular (archivos locales + Engram)

### 3. Auto-aprendizaje
- Graceful shutdown con resumen de entrevista
- Persistencia dual: knowledge/interviews/ + Engram

### 4. Research de Empresa
- Base de datos estática (knowledge/companies/)
- Detección de keywords para inyectar contexto

### 5. UX/UI (Tauri HUD)
- Glass Style UI con desenfoque nativo (Acrylic)
- Stealth Mode (invisible para Zoom/Teams)
- Ctrl+Shift+Q para cerrar
- Live stream de transcripción + sugerencias

## 🛠 Infraestructura
- Audio: WASAPI loopback (Windows), cpal (cross-platform)
- STT: whisper.cpp (local), Groq Whisper API (cloud)
- LLM: OpenAI-compatible (Ollama, z.ai, etc.)
- Build: Tauri v2, ghostai-pilot lib con feature flags

## 📋 Pendientes
1. **Pipeline LLM** — testear que la transcripción → LLM → sugerencias funcione en el HUD
2. **Dual capture con AW520H+VoiceMeeter** — testear en setup real del usuario
3. **Precisión español/portugués** — Parakeet ONNX o whisper multilingual
4. **Capa 7 (Earbuds)** — TTS local (Piper) para sugerencias por auricular
