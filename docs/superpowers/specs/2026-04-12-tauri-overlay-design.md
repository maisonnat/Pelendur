# Design Spec: Pelendur Tauri Invisible Overlay

**Date:** 2026-04-12
**Status:** Draft
**Complexity:** Medium
**Topic:** UI/UX Migration from CLI to Tauri Invisible Overlay

## 1. Problem Statement
The current Pelendur prototype runs in a terminal (CLI). For a real interview context, this is problematic:
- **Visibility:** You have to switch windows to read suggestions, breaking eye contact with the interviewer.
- **Stealth:** A terminal window is easily detectable if you accidentally share your full screen.
- **UX:** Reading dense terminal text in real-time is stressful.

## 2. Approach & Goals
We will migrate the UI to **Tauri (Rust + WebView)** to create an **Invisible Overlay**.

### Key Goals:
- **Invisibility:** The window should be transparent and ignored by screen-sharing software (Zoom/Meet/Teams).
- **Heads-up Display (HUD):** Text should appear floating near your camera or over the interviewer's face so you never move your eyes.
- **Stealth Mode:** Disguise the process name and hide it from the taskbar.
- **Minimal Latency:** Keep the current Rust backend performance while updating the frontend via Tauri events.

## 3. Architecture

### Backend (Rust/Tauri)
- **Audio Thread:** Continues running the `cpal` + `wasapi` loop.
- **STT/LLM Pipeline:** Stays in Rust for maximum speed.
- **Tauri Events:** When a response is ready, Rust emits a `new-suggestion` event to the frontend.
- **Stealth Module:** Win32 API calls to set `WS_EX_LAYERED` and `WS_EX_TRANSPARENT` flags to ensure the window is "click-through" and "share-proof".

### Frontend (React/Vanilla CSS)
- **Minimalist HUD:** High-contrast, easy-to-read typography.
- **Dynamic Opacity:** Fade in when a message arrives, fade out after reading.
- **Positioning:** Fixed at the top-center of the screen.

## 4. Approach Comparison

| Feature | Approach A: Simple Tauri Window | Approach B: True Stealth Overlay (Recommended) |
|---------|---------------------------------|------------------------------------------------|
| **Visibility** | Normal Window (visible in share) | Click-through & Invisible to Capture |
| **UX** | Standard Desktop App | Floating HUD |
| **Complexity** | Low | Medium (Requires Windows API crates) |
| **Risk** | High (Interviewer sees it) | Low (Stealth) |

**Recommendation:** **Approach B**. It follows the success patterns of Natively and Meetily identified in our research.

## 5. Implementation Phases (Post-MVP)
1. **Phase 1:** Tauri Scaffold & Window Transparency.
2. **Phase 2:** Connect Rust STT/LLM events to Frontend.
3. **Phase 3:** Windows Stealth Optimizations (Hide from capture).
4. **Phase 4:** Visual Polish (Animations, Themes).

## 6. Questions for the User
1. **Positioning:** Do you prefer the text floating at the **top** (near the webcam) or as a **sidebar**?
2. **Interaction:** Should the overlay be "click-through" (you can't click it, it's just for reading) or should it have small buttons for "Regenerate" or "Clear"?
