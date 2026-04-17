# 🚀 Pelendur: Roadmap GSD (48h MVP)

Este documento detalla el estado actual del proyecto Pelendur siguiendo la metodología **Get Shit Done (GSD)** para la reunión en 48 horas.

## ✅ Implementado (Listo para la reunión)

### 1. El Copilot que te Conoce (Cerebro)
- **Perfil Personal Integrado**: El LLM ahora recibe tu CV, historias STAR y skills dominados en cada prompt.
- **Matching en Tiempo Real**: El sistema busca automáticamente historias STAR relevantes basándose en lo que dice el entrevistador.
- **Motor de Contexto Modular**: Capaz de leer de archivos locales (knowledge/) y de tu proyecto **The Crab Engram**.

### 2. Auto-aprendizaje (Capa 5)
- **Graceful Shutdown**: Al cerrar con Ctrl+C, el sistema genera un resumen de la entrevista.
- **Persistencia Dual**: Los aprendizajes se guardan en knowledge/interviews/ y se envían a **The Crab Engram** automáticamente.

### 3. Research de Empresa (Capa 3)
- **Base de Datos Estática**: Soporte para cargar investigación de empresas (ej: knowledge/companies/datadog/).
- **Detección de Keywords**: Si se menciona la empresa, el sistema inyecta su cultura y stack técnico en el contexto.

### 4. UX/UI Invisible (Tauri HUD)
- **Glass Style UI**: HUD moderno con desenfoque nativo (Acrylic/Blur).
- **Stealth Mode**: Invisible para herramientas de captura (Zoom/Teams).
- **Control Global**: Atajo Ctrl+Alt+L para bloquear/desbloquear el HUD y botones de acción.
- **Live Stream**: Transcripción y sugerencias en tiempo real desde Rust via Tauri events.

## 🛠 Estado de la Infraestructura
- **Audio**: Auto-config implementado. WASAPI polling (Windows) + PulseAudio/PipeWire (Linux). Feature flags: wasapi_loopback, linux_audio.
- **STT**: Whisper.cpp (local) y Groq (cloud) funcionales.
- **LLM**: Pipeline de streaming optimizado para latencia mínima.

## 📋 Próximos Pasos (Post-reunión)
1. **Capa 7 (Earbuds)**: Integración de TTS (Piper) para recibir sugerencias por audio sin desviar la mirada.

---
**GSD Mode Active**: Enfocados en la victoria en 48 horas. 🦀🔥
