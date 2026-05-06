# Implementation Tasks — Pelendur UI Redesign (Track B)

## Task 1: Replace style.css with Design System

- [ ] Replace entire `ui/style.css` with the new Pelendur Design System
- [ ] Add all CSS custom properties from design.md
- [ ] Implement glassmorphism utility classes
- [ ] Redesign control bar with new buttons, hover states, status dot
- [ ] Redesign suggestion card with copper accent, glow shadow, scale animation
- [ ] Redesign transcription feed with glass bg, gradient mask, bubble items
- [ ] Redesign interview status bar with pill badge, monospace timer
- [ ] Redesign knowledge panel with collapsible sections, smooth transitions
- [ ] Redesign modals with glass overlay, scale-in animation
- [ ] Redesign minimal mode dot with glow pulse animation
- [ ] Add all keyframe animations (fade-in, slide-up, scale-in, glow-pulse, shimmer)
- [ ] Add @media for edge cases (small windows, high DPI)
- [ ] Apply `will-change: transform, opacity` to animated elements for GPU acceleration
- [ ] Ensure `backdrop-filter` doesn't break on Windows WebView2 (test -webkit- prefix)

## Task 2: Update index.html with BEM structure

- [ ] Update `ui/index.html` to use BEM-style class names matching new CSS
- [ ] Control bar: wrap in `<div class="pl-control-bar">`
- [ ] Buttons: `<button class="pl-btn pl-btn--icon" data-action="...">`
- [ ] Suggestion card: `<div class="pl-card pl-card--suggestion">`
- [ ] Transcription items: `<div class="pl-transcription pl-transcription--interviewer">`
- [ ] Status bar: `<div class="pl-status-bar pl-status-bar--active">`
- [ ] Modals: `<div class="pl-modal pl-modal--company">`
- [ ] Keep all IDs and data-* attributes intact for JS compatibility
- [ ] Add semantic ARIA labels where appropriate
- [ ] Ensure drag-region attributes are preserved

## Task 3: Update main.js for new UI interactions

- [ ] Verify all DOM queries (getElementById) still work with updated HTML
- [ ] Update any class toggling to use new BEM classes
- [ ] Add spring animation for knowledge panel toggle
- [ ] Add smooth transition for interview mode enter/exit
- [ ] Add scale animation on button click (add/remove .pl-btn--active class)
- [ ] Keep ALL existing Tauri IPC calls intact — do NOT change business logic
- [ ] Keep ALL event listeners intact
- [ ] Do NOT change the internal state management logic

## Task 4: Verify build and functionality

- [ ] `cargo check` from project root (verify Tauri commands still work)
- [ ] `cd src-tauri && cargo check` (verify Tauri overlay compiles)
- [ ] Verify all CSS animations look correct (will be tested on Windows later)
- [ ] No JavaScript console errors
- [ ] All classes properly reference CSS custom properties
