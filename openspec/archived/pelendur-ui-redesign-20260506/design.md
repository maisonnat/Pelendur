# Pelendur Design System — Track B

## 1. Design Tokens (CSS Custom Properties)

### 1.1 Color Palette

```css
:root {
  /* Background — deep zinc-black base */
  --pl-bg-base: rgba(10, 10, 15, 0.72);
  --pl-bg-surface: rgba(22, 22, 35, 0.82);
  --pl-bg-card: rgba(30, 30, 50, 0.75);
  --pl-bg-elevated: rgba(40, 40, 65, 0.85);
  --pl-bg-glass: rgba(20, 20, 35, 0.65);

  /* Accent — warm copper (Premium, profesional, distintivo) */
  --pl-accent-primary: #F4A261;
  --pl-accent-hover: #F6B87A;
  --pl-accent-active: #D4874E;
  --pl-accent-glow: rgba(244, 162, 97, 0.3);
  --pl-accent-subtle: rgba(244, 162, 97, 0.12);

  /* Secondary accent — cool cyan (AI, sugerencias, highlights) */
  --pl-ai-primary: #4FC3F7;
  --pl-ai-glow: rgba(79, 195, 247, 0.25);
  --pl-ai-subtle: rgba(79, 195, 247, 0.1);

  /* Text */
  --pl-text-primary: #F0F0F5;
  --pl-text-secondary: #8899BB;
  --pl-text-tertiary: #556688;
  --pl-text-dim: rgba(255, 255, 255, 0.15);

  /* Status */
  --pl-status-active: #4ADE80;
  --pl-status-idle: #8899BB;
  --pl-status-error: #FB7185;
  --pl-status-warning: #FBBF24;

  /* Glass effects */
  --pl-glass-bg: rgba(20, 20, 35, 0.55);
  --pl-glass-border: rgba(255, 255, 255, 0.06);
  --pl-glass-border-hover: rgba(255, 255, 255, 0.12);
  --pl-glass-blur: blur(24px);
  --pl-glass-saturate: saturate(120%);

  /* Shadows */
  --pl-shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.3);
  --pl-shadow-md: 0 8px 32px rgba(0, 0, 0, 0.4);
  --pl-shadow-lg: 0 16px 48px rgba(0, 0, 0, 0.5);
  --pl-shadow-accent: 0 0 20px var(--pl-accent-glow);

  /* Typography */
  --pl-font-ui: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --pl-font-mono: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
  --pl-font-size-xs: 11px;
  --pl-font-size-sm: 12px;
  --pl-font-size-base: 14px;
  --pl-font-size-lg: 18px;
  --pl-font-size-xl: 24px;
  --pl-font-size-2xl: 32px;
  --pl-font-weight-normal: 400;
  --pl-font-weight-medium: 500;
  --pl-font-weight-semibold: 600;
  --pl-font-weight-bold: 700;
  --pl-line-height-tight: 1.2;
  --pl-line-height-normal: 1.5;

  /* Spacing */
  --pl-space-xs: 4px;
  --pl-space-sm: 8px;
  --pl-space-md: 12px;
  --pl-space-lg: 16px;
  --pl-space-xl: 24px;
  --pl-space-2xl: 32px;

  /* Border Radius */
  --pl-radius-sm: 6px;
  --pl-radius-md: 10px;
  --pl-radius-lg: 16px;
  --pl-radius-xl: 24px;
  --pl-radius-full: 9999px;

  /* Transitions */
  --pl-transition-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
  --pl-transition-normal: 250ms cubic-bezier(0.4, 0, 0.2, 1);
  --pl-transition-slow: 400ms cubic-bezier(0.4, 0, 0.2, 1);
  --pl-transition-spring: 500ms cubic-bezier(0.34, 1.56, 0.64, 1);
}
```

### 1.2 Window Constraints (Tauri)
```
Width: 800px, Height: 400px
No decorations, transparent background
Always-on-top, skip taskbar
```

## 2. Visual Language

### 2.1 Glassmorphism
Every surface uses the glass stack:
```css
.glass {
  background: var(--pl-glass-bg);
  backdrop-filter: var(--pl-glass-blur) var(--pl-glass-saturate);
  -webkit-backdrop-filter: var(--pl-glass-blur) var(--pl-glass-saturate);
  border: 1px solid var(--pl-glass-border);
}
```

### 2.2 Acrylic Depth Layers
| Layer | z-index | Use |
|-------|---------|-----|
| Background | 0 | Base overlay, body |
| Surface | 1 | Control bar, status bar |
| Card | 2 | Suggestion card, panels |
| Elevated | 3 | Modals, dropdowns |
| Overlay | 4 | Modal backdrops |

### 2.3 Animations
All animations use CSS custom properties for consistency:
```css
@keyframes pl-fade-in {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes pl-slide-up {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}

@keyframes pl-scale-in {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}

@keyframes pl-glow-pulse {
  0%, 100% { box-shadow: 0 0 8px var(--pl-accent-glow); }
  50% { box-shadow: 0 0 20px var(--pl-accent-glow), 0 0 40px var(--pl-accent-subtle); }
}

@keyframes pl-shimmer {
  0% { background-position: -200% center; }
  100% { background-position: 200% center; }
}
```

## 3. Component Design

### 3.1 Control Bar
- Position: top, full width
- Height: 36px, flex row
- Background: glass surface
- Border-bottom: 1px glass-border
- Buttons: 24x24px, transparent bg, icon color var(--pl-text-secondary)
- Hover: bg var(--pl-accent-subtle), color var(--pl-accent-primary)
- Active: scale(0.92) on click
- Status indicator: 8px dot, color based on state

### 3.2 Suggestion Card
- Position: center of main content area
- Background: glass card
- Border-left: 3px solid var(--pl-accent-primary)
- Border-radius: var(--pl-radius-lg)
- Padding: var(--pl-space-xl)
- Shadow: var(--pl-shadow-accent) when active
- Entry animation: pl-scale-in
- Text: accent color, font-size 1.4rem, font-weight 500

### 3.3 Interview Status Bar
- Full-width bar below controls
- Glass surface with border
- Badge pill: small rounded tag with accent color
- Timer: monospace, secondary color
- Fade in/out animation on active/inactive

### 3.4 Transcription Feed
- Bottom area, height 100px
- Glass background with top gradient mask
- Transcription items: 13px, secondary text
- Interviewer vs me: distinct styles (interviewer = primary + bold, me = dim + italic)
- Entry: pl-slide-up animation

### 3.5 Knowledge Panel
- Slide-in from left or top
- Collapsible sections with smooth height transitions
- Skill tags: small pills with accent border
- Header: drag handle + title + toggle

### 3.6 Modals
- Overlay: glass black (rgba(0,0,0,0.5) + blur)
- Modal body: elevated glass card
- Entry: pl-scale-in animation
- Close button: top-right, subtle

### 3.7 Minimal Mode
- Single floating dot (8px) in top-right
- Glow pulse animation when active
- Click to expand with pl-scale-in transition
- Hold position: same coordinates as the HUD's top-right corner

## 4. Layout Structure

```
┌──────────────────────────────────────────────────────────┐
│ [Control Bar — 36px]                                      │
│  Profile │ 🎧 │ 🎙️ │ ● │ 🔓 │ 🗑 │ 🔄 │ ✕ │ ●     │
├──────────────────────────────────────────────────────────┤
│ [Interview Status Bar — hidden when inactive]             │
│  🎙️ Interview Active │ Company Name │ 00:00             │
├──────────────────────────────────────────────────────────┤
│                                                           │
│                   [Suggestion Card]                       │
│              ┌─────────────────────────────┐              │
│              │  Main suggestion text       │              │
│              └─────────────────────────────┘              │
│                                                           │
│  [Knowledge Panel — collapsible from top/left]            │
│                                                           │
│  [Partial Transcription — subtle, centered]               │
│                                                           │
├──────────────────────────────────────────────────────────┤
│ [Transcription Feed — 100px]                              │
│  Interviewer: How many fingers are there...               │
│  Me: I believe the answer is five...                      │
└──────────────────────────────────────────────────────────┘
```

## 5. Interaction States

### Buttons
```
Default:   bg transparent, color var(--pl-text-secondary)
Hover:     bg var(--pl-accent-subtle), color var(--pl-accent-primary)
Active:    transform scale(0.92), bg var(--pl-accent-active)
Disabled:  opacity 0.4, pointer-events none
```

### Status Dot Colors
```
Recording:  var(--pl-status-active) + glow pulse
Idle:       var(--pl-status-idle)
Error:      var(--pl-status-error)
Processing: var(--pl-accent-primary) + spin
```

### Minimal Dot
```
Idle:   var(--pl-text-tertiary) + subtle glow
Active: var(--pl-accent-primary) + glow pulse animation
```
