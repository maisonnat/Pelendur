# Pelendur Design System — Proposal

## Intent
Crear un sistema de diseño visual para Pelendur que enamore al usuario. Inspirado en:
- **Axur Design System**: Cyber Noir Intelligence (zinc-950, orange #FF671F, glassmorphism, tipografía Inter)
- **Cluely**: Overlay limpio, dark mode, asistencia en tiempo real sin fricción
- **Meetily**: Tauri+Rust nativo, Parakeet STT, privacy-first
- **Natively AI**: Pixel-perfect UI, stealth mode, proceso disfrazado

## Design Philosophy: "Stealth Interview Intelligence"

Fusión de Cyber Noir Intelligence con un propósito completamente distinto: **claridad absoluta bajo presión**. Donde Axur persuade a un CISO, Pelendur asiste a un candidato en una entrevista.

### Core Tenets
1. **Invisibilidad como poder** — El HUD debe ser invisible hasta que se necesita. Glassmorphism extremo, sin bordes duros.
2. **Información jerárquica** — La transcripción en vivo es secundaria. La sugerencia de IA es primaria. Timing y modo son terciarios.
3. **Micro-interacciones que deleitan** — Transiciones suaves, glow cuando hay transcripción activa, pulse cuando hay sugerencia nueva.
4. **Diseño que no distrae** — Zero animaciones decorativas. Todo movimiento tiene un propósito informativo.

### Paleta Tentativa
| Token | Color | Uso |
|---|---|---|
| `--bg-base` | `#09090b` (zinc-950) | Fondo del HUD |
| `--bg-surface` | `rgba(9,9,11,0.85)` | Paneles glass |
| `--accent` | `#FF671F` (Axur orange) | Transcripción activa, hero |
| `--accent-glow` | `#FF8A4C` | Glow en sugerencias |
| `--text-primary` | `#FFFFFF` | Transcripción |
| `--text-secondary` | `#a1a1aa` (zinc-400) | Metadatos, timestamps |
| `--text-suggestion` | `#FF671F` | Texto de sugerencia IA |
| `--success` | `#22c55e` | Estado "grabando" |
| `--border` | `rgba(255,255,255,0.06)` | Bordes sutiles glass |

### Tipografía
- **Inter** (ya cargada en el sistema): weight 300 para números grandes, 400 para cuerpo, 600 para sugerencias
- Tracking: `tracking-widest` para badges de modo, `tracking-tight` para transcripción

### Arquitectura del HUD Rediseñado
```
┌─────────────────────────────────────────────┐
│  [🎧 System] [🎙️] [🔘] [🔓] [✕]    ●     │  ← Barra de control glass
│  ─────────────────────────────────────────── │
│                                              │
│   💬 "A discounted cash flow model..."       │  ← Transcripción en vivo
│                                              │
│   ┌─────────────────────────────────┐        │
│   │ Assist                          │        │  ← Sugerencia IA (card glass con glow)
│   │ "A DCF model values a company   │        │
│   │  by projecting future free      │        │
│   │  cash flows..."                 │        │
│   │                          [Copy] │        │
│   └─────────────────────────────────┘        │
│                                              │
│   📋 Follow-up: What about WACC?             │  ← Sugerencias de seguimiento
│                                              │
│   00:00  Interview Active — Acme Corp        │  ← Barra de estado inferior
└─────────────────────────────────────────────┘
```

## Próximos Pasos
1. Investigar a fondo Cluely/Meetily/Natively via screenshots reales
2. Escribir OpenSpec specs con diseño detallado
3. Implementar CSS del nuevo HUD en `ui/style.css`
4. Validar con test suite de Track A (screenshots + métricas)
