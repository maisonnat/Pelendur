# Pelendur UI/UX Redesign — Track B

## Intent

Rediseñar completamente la interfaz del HUD overlay de Pelendur para superar visualmente a Meetily, Cluely y Natively AI. El objetivo es un HUD de entrevista que se sienta premium, profesional y sigiloso — como un producto de clase mundial, no un prototipo.

## Alcance

### In Scope
- `ui/style.css` — CSS completo rediseñado con Design System Pelendur
- `ui/index.html` — Estructura HTML actualizada con nuevos componentes
- `ui/main.js` — Lógica UI actualizada para nuevos estilos e interacciones (sin cambiar lógica de negocio ni Tauri IPC)
- Sistema de design tokens vía CSS custom properties en `:root`
- Glassmorphism, blur, gradientes, animaciones GPU-acceleradas
- Nuevo layout: status bar minimal, sugerencias elegantes, transcripción con burbujas
- Animaciones de transición suaves (modos, modales, panels)
- Modo minimal refinado con animaciones
- Estado hover/active/focus para todos los botones e interacciones
- Paleta de colores profesional (inspirada en Axur Design System)

### Out of Scope
- Lógica de negocio (STT, WASAPI, backend Rust)
- Nuevos features (no agregar funcionalidad)
- Perfil de React (ui-profile/) — se rediseñará en fase separada
- Iconos SVG personalizados (se usará Unicode/emoji como ahora, o reemplazar en fase posterior)
- Cambios en Tauri config o backend Rust

## Enfoque

1. **Definir Design System Pelendur** con tokens CSS: colores, tipografía, espaciado, bordes, sombras, animaciones
2. **Rediseñar style.css** completo usando los tokens, con glassmorphism, gradientes sutiles, animaciones fluidas
3. **Actualizar index.html** con estructura semántica moderna y clases BEM
4. **Actualizar main.js** para manejar los nuevos estilos y animaciones (sin cambiar la lógica)
5. **Verificar** que el HUD siga funcionando correctamente con los mismos Tauri commands

## Principios de Diseño

1. **Stealth Premium** — El HUD debe verse invisible hasta que se necesita, luego revelar información con elegancia
2. **Jerarquía Visual Clara** — La sugerencia principal es el elemento central (más grande, más brillante). La transcripción es secundaria
3. **Glassmorphism Funcional** — No por moda: el blur permite leer el contenido debajo del overlay, esencial para un HUD de entrevista
4. **Animaciones con Propósito** — Cada transición comunica un cambio de estado. Nada se mueve sin razón
5. **Consistencia** — Un solo sistema de diseño aplicado a todos los componentes
