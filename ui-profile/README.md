# Pelendur Profile Management Window

React + Tailwind + Vite build system for the profile management interface.

## Setup

This project uses:
- React 19 with TypeScript
- Tailwind CSS v4 (PostCSS plugin)
- Vite for fast build and dev server

## Installation

```bash
npm install
```

## Development

```bash
npm run dev
```

## Build

```bash
npm run build
```

The build outputs to `dist/` directory with:
- JavaScript bundle: ~192KB (gzipped: ~60KB)
- CSS: ~0.22KB (gzipped: ~0.18KB)
- Total: <500KB ✓

## Design Theme

Matches the existing HUD glassmorphism design:
- Dark background: `rgba(10, 10, 10, 0.95)`
- Accent color: `#ffd700` (gold)
- Card background: `rgba(30, 30, 30, 0.85)` with backdrop-blur
- Text: white main, dimmed secondary text

## Tauri Integration

This build will be served by Tauri in a separate window (Task 6 configuration).
- Window will be configured in `src-tauri/tauri.conf.json`
- IPC communication with HUD window via Tauri events

## Performance Targets

- Bundle size: <500KB ✓ (currently ~192KB)
- Load time: <2s (target)
- Knowledge graph rendering: 200+ nodes at 60fps (target)


You can also install [eslint-plugin-react-x](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-x) and [eslint-plugin-react-dom](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-dom) for React-specific lint rules:

```js
// eslint.config.js
import reactX from 'eslint-plugin-react-x'
import reactDom from 'eslint-plugin-react-dom'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...
      // Enable lint rules for React
      reactX.configs['recommended-typescript'],
      // Enable lint rules for React DOM
      reactDom.configs.recommended,
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```
