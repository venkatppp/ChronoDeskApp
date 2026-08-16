# CHRONODESK — Apple Liquid Glass Implementation Handoff

> **Status:** PERMANENT HANDOFF. 
> **Last Commit:** `28daab7`
> **Stopping point:** Pre-implementation (Index.css and useLiquidGlass.ts were audited but reverted).
> **Goal:** High-fidelity Apple Liquid Glass implementation in ChronoDesk.

---

## 1. CORE PHILOSOPHY: APPLE LIQUID GLASS VS GENERIC GLASMORPHISM

### 1.1 The Definition
**Apple Liquid Glass** is a rendering technique, not just a set of CSS properties. It comprises:
*   **Geometric Refraction:** Not just `backdrop-filter: blur()`. It requires an SVG displacement map (`url(#lg-filter)`) to create physical "bulge" distortion at the rim.
*   **Atmospheric Environmental Canvas:** A deep-space canvas (Radial gradients, orbs, grain, vignette) behind the glass. The glass *must* have a world to frost and distort against to read as glass.
*   **Neutral Tint:** The glass pane itself is nearly neutral. Color comes from the environment.
*   **Specular Highlights:** Sharp, hairline bright reflections along the top edges.
*   **Depth:** Inset highlights and deep soft shadows to create physical dimension.

### 1.2 Architectural Foundation
*   **Functional Layer (Glass):** Nav, Toolbars, Sheets, Inputs.
*   **Content Layer (Calm):** Data Tables, Dashboards, Cards.
*   **Anti-Pattern:** Glass-on-glass (nested glass surfaces) creates double-blur/refraction costs and visual noise.

---

## 2. MATERIAL SYSTEM SPECIFICATION

| Material | CSS Utility | Blur | Specular/Rim Strategy | Use Case |
|---|---|---|---|---|
| **Chrome** | `.glass-chrome` | 32px | Sharp top, deep shadow, underlight | Sidebar, Header, Sheets |
| **Surface** | `.glass-surface` | 22px | Moderate specular edge | Hero panels, large cards |
| **Panel** | `.glass-panel` | 16px | Subtle edge | Secondary panels |
| **Well** | `.glass-well` | 12px | Inset interior | Inputs, search fields |
| **Control** | `.glass-control` | 12px | Focused highlight | Buttons, segments |
| **Sheet** | `.glass-sheet` | 40px | Strongest specular | Dialogs |
| **Accent** | `.glass-accent` | 14px | Strong top highlight | Primary action |

---

## 3. TECHNICAL IMPLEMENTATION REQUIREMENTS

### 3.1 `frontend/src/index.css` (Premium Dark Theme Tokens)
*   **Background:** `#05060a`
*   **Glass Tints (Refined):**
    *   `--glass-tint-top: rgba(255, 255, 255, 0.08)`
    *   `--glass-tint-mid: rgba(255, 255, 255, 0.035)`
    *   `--glass-tint-bottom: rgba(255, 255, 255, 0.015)`
    *   `--glass-tint-strong: rgba(20, 22, 30, 0.12)`
*   **Specular/Edge:**
    *   `--glass-specular: rgba(255, 255, 255, 0.55)`
    *   `--glass-edge-dark: rgba(0, 0, 0, 0.5)`
    *   `--glass-underlight: rgba(255, 255, 255, 0.06)`

### 3.2 `frontend/src/lib/liquidGlass.ts` (Optics)
*   **Default Refraction:** Increase `scale` to `-112` for chrome surfaces for more dramatic, Apple-like lensing.
*   **Chromatics:** Set `chroma` to `5` for clearer rim-fringe.
*   **Area Guard:** Maintain `420,000` px² threshold.

### 3.3 `frontend/src/hooks/useLiquidGlass.ts`
*   **Must return `LiquidGlassInstance | null`:** Fix implementation to return the instance for surface-level control of refraction state.

---

## 4. UI/UX REDESIGN SPECIFICATIONS

### 4.1 Sidebar Redesign
*   **Width:** Increase to `280px` (from `222px`).
*   **Radius:** Increase to `rounded-3xl` (from `rounded-2xl`).
*   **Visuals:** Add specular top edge (use `glass-chrome` utility).
*   **Typography:** Branding `text-[15px] font-semibold`, nav labels `text-[11px]`.

### 4.2 Topbar Redesign
*   **Specular:** Add a prominent specular top edge to `Topbar`.
*   **Search Field:** Upgrade to a more prominent `glass-chrome` or elevated `glass-well` treatment.

### 4.3 Dashboard Reorganization
*   **Hierarchy:** Predictive Intelligence and Priority Queue move from the right sidebar to the top of the main content area (right column).
*   **Material:** Predictive Intelligence and Priority Queue cards MUST use `glass-chrome` material.

---

## 5. VALIDATION & QA
1.  **Build Check:** `npm run build` must succeed without type errors.
2.  **Tauri Visual QA:** Inspect in Tauri window (Chromium) to verify SVG displacement rendering.
3.  **Cross-Browser:** Verify fallback (frosted blur) works correctly on non-Chromium fallback scenarios.
4.  **Motion:** Verify `prefers-reduced-motion: reduce` suppresses environmental orb drift and glass transitions.

---
**Status:** No source files currently modified. All changes reverted.
**Permanent Record:** Handed off to next session for implementation.
