# Pelendur UI Redesign — Specs

## ADDED

### CSS Design System
- `ui/style.css`: token system with 40+ CSS custom properties
  - Color palette: zinc-black base, copper accent, cyan AI accent
  - Glassmorphism utility classes
  - Typography scale (Inter UI, JetBrains Mono)
  - Animation keyframes (fade-in, slide-up, scale-in, glow-pulse, shimmer)
  - Spacing, border-radius, shadow tokens
- Component styles:
  - `.pl-control-bar` — glass bar with icon buttons
  - `.pl-btn--icon` — icon buttons with hover/active/disabled states
  - `.pl-btn--primary` — primary action button (e.g., Start Interview)
  - `.pl-card--suggestion` — suggestion card with copper border + glow
  - `.pl-transcription` — transcription feed items
  - `.pl-status-bar` — interview status bar with pill badge
  - `.pl-modal` — modal component with glass overlay
  - `.pl-status-dot` — recording/idle/error indicator
  - `.pl-minimal-dot` — minimal mode indicator
  - `.pl-badge` — tag/pill elements
  - `.pl-panel` — collapsible knowledge panel

### Animations (keyframes)
- `pl-fade-in` — opacity + translateY for entry
- `pl-slide-up` — for transcription items
- `pl-scale-in` — for modals and cards
- `pl-glow-pulse` — for minimal dot and recording indicator
- `pl-shimmer` — loading state effect

## MODIFIED

### ui/index.html
- All elements get BEM-style class names (old class names REPLACED)
- Structure preserved, IDs preserved, data-tauri-drag-region preserved
- Control bar wrapped in `.pl-control-bar` container
- Buttons use `.pl-btn--icon` class
- Status indicator uses `.pl-status-dot` class  
- Suggestion card uses `.pl-card--suggestion`
- Transcription items use `.pl-transcription` + `.pl-transcription--interviewer` | `.pl-transcription--me`
- Interview status bar uses `.pl-status-bar` + `.pl-status-bar--active`
- Modals use `.pl-modal` + `.pl-modal--*` and overlay `.pl-modal-overlay`
- Knowledge panel uses `.pl-panel` classes

### ui/main.js
- Class toggling updated to use new BEM classes
- Button click animations (add/remove `.pl-btn--active` class with timeout)
- Spring animation for knowledge panel toggle
- Smooth transitions for interview mode
- ALL business logic, Tauri IPC, event listeners PRESERVED — zero changes to functionality

## REMOVED

- Old inline CSS variables (`--bg-color`, `--accent-color`, etc.) replaced by Design System tokens
- Old class names removed: `.hud-container`, `.icon-btn`, `.controls`, `.minimal-icon`, `.interview-status-bar`, `.transcription-item`, `.interviewer`, `.me`, `.modal-overlay`, `.modal`, etc.
- Old keyframe names: `pulse`, `minimalPulse`, `fadeIn`, `slideUp` replaced with `pl-*` prefixed versions
