# ContextSphere Liquid Glass — Complete Implementation Blueprint

> **Goal:** Make ContextSphere feel like an app Apple could have designed using Liquid Glass.
> **Scope:** Refine and complete the existing implementation. macOS primary. Strict Apple material hierarchy. Full fluid motion system.

---

## Part I — Research Findings

### Apple Liquid Glass Summary

Announced at WWDC 2025, Liquid Glass is Apple's "digital meta-material" — the broadest design update since iOS 7. It combines optical glass properties (refraction, translucency, specular highlights) with fluid, context-aware behavior. It spans iOS 26, macOS Tahoe 26, watchOS 26, and visionOS 26.

**What makes it different from generic glassmorphism:**

| Generic Glassmorphism | Apple Liquid Glass |
|---|---|
| Static `backdrop-filter: blur()` | Real-time SVG displacement refraction |
| Decorative aesthetic | Functional depth hierarchy |
| Applied ad-hoc | System-enforced material rules |
| No context awareness | Content-adaptive tint/blur/opacity |
| Ignores accessibility | Honors Reduce Transparency, Increase Contrast, Reduce Motion |

### The Five Layers of Liquid Glass

1. **Highlights** — Specular light sources that respond to geometry and motion
2. **Shadows** — Dynamic depth that adapts to content behind the glass
3. **Illumination** — Interactive underglow feedback on touch/click
4. **Lensing/Refraction** — Edge light bending that creates depth (NOT just blur)
5. **Tint/Adaptation** — Dynamic color, opacity, and blur adjustment based on content

### Apple's Material Hierarchy Rules

**Glass belongs ONLY on the navigation layer:**
- Toolbars, tab bars, sidebars, floating action buttons
- Sheets, popovers, menus, action sheets, context menus
- Search bars, alerts (centered)

**Glass NEVER belongs on:**
- Content items (list rows, cards, table cells)
- Scrollable content areas
- Full-screen backgrounds
- Inner views of glass panels

**Never stack glass on glass** — glass cannot properly sample other glass.

**Tint only primary actions** — tinting everything reduces visual hierarchy.

### Apple's Motion Principles (WWDC 2018 — Designing Fluid Interfaces)

- **Response:** Kill latency. Feedback on pointer-down, not release.
- **Direct manipulation:** 1:1 tracking with pointer. Respect grab offset.
- **Interruptibility:** Every animation must be grabbable mid-flight. Start from current value, never target.
- **Springs over keyframes:** Mass/stiffness/damping replaced with damping ratio + response time.
- **Velocity handoff:** Animation continues at finger's exact release velocity.
- **Momentum projection:** Project resting position from velocity, don't snap from release point.
- **Spatial consistency:** Enter/exit along same path. Anchor to source.
- **Rubber-banding:** Progressive resistance at boundaries, not hard stops.

---

## Part II — What ContextSphere Already Has

### Implemented and Working

| Component | Status | Notes |
|---|---|---|
| SVG displacement refraction engine | ✅ Complete | `liquidGlass.ts` — 311 lines, chromatic aberration, maxArea guard |
| 7-tier CSS material system | ✅ Complete | chrome, surface, panel, well, control, nav, sheet, accent |
| GlassSurface component | ✅ Complete | Polymorphic, refraction lifecycle, material mapping |
| useLiquidGlass hook | ✅ Complete | Returns `LiquidGlassInstance | null`, proper lifecycle |
| Environmental canvas | ✅ Complete | 6 ambient gradients, 5 drifting orbs, grain, vignette |
| Accessibility | ✅ Complete | reduced-motion, reduced-transparency, increased-contrast |
| Light theme | ✅ Complete | `.light` class with adjusted glass tokens |
| Sidebar | ✅ Complete | 280px, `rounded-3xl`, chrome material, specular edge |
| Topbar | ✅ Complete | chrome material, specular edge, search field |
| Dialog system | ✅ Complete | sheet material |
| Button system | ✅ Complete | glass-accent (primary), glass-control (secondary) |
| GlassInput | ✅ Complete | glass-well material |
| SegmentedControl | ✅ Complete | glass-control material |

### Gaps and Opportunities

| Gap | Impact | Priority |
|---|---|---|
| No spring-based fluid motion system | Missing Apple's core interaction philosophy | **Critical** |
| Pages don't use consistent glass hierarchy | Some pages use raw CSS classes, others use GlassSurface | **High** |
| No scroll edge effects | Missing the floating-chrome-over-content transition | **High** |
| No content-aware adaptation | Glass doesn't adapt to what's behind it | **Medium** |
| Missing materialize-on-enter animations | Glass surfaces fade in instead of materializing | **Medium** |
| No page transition system | Route changes are instant cuts | **Medium** |
| Dashboard hero panels use glass-surface | Content-adjacent panels with glass may violate strict rules | **Low** |
| No hover/interaction illumination | Glass doesn't light up on hover | **Low** |
| Inconsistent component usage patterns | Some files use raw CSS, others use GlassSurface | **Medium** |

---

## Part III — ContextSphere Design System

### Material Hierarchy (Strict Apple Rules)

```
Level 0 — Background:     Environmental canvas (orbs, gradients, grain)
Level 1 — Navigation:     Sidebar, Topbar (glass-chrome) ← GLASS LIVES HERE
Level 2 — Chrome:         Toolbars, search bar, floating controls (glass-chrome)
Level 3 — Sheets:         Dialogs, modals, context menus (glass-sheet)
Level 4 — Controls:       Buttons, segments, inputs (glass-control, glass-well, glass-accent)
Level 5 — Content:        Cards, lists, panels, tables (opaque — NO glass)
Level 6 — Text:           Labels, values, descriptions (on content layer)
```

**Rule:** Glass is Levels 1–4 only. Levels 5–6 are opaque content surfaces that scroll beneath the glass chrome.

### Color Tokens (Existing — No Changes Needed)

The existing color system is already Apple-inspired and correct:
- `--color-background: #05060a` (deep black with blue cast)
- `--color-accent: #0a84ff` (macOS system blue)
- `--color-foreground: #f5f5f7` (Apple label white)
- Light theme: warm-neutral canvas, frosted white glass

### Typography (Existing — Minor Refinements)

Current type system is Apple-aligned. Refinements:
- Page titles: `text-5xl font-semibold tracking-[-0.03em]` — tighten further for large display
- Body: `text-sm` with `letter-spacing: -0.011em` — correct
- Labels: `text-[11px] font-semibold uppercase tracking-[0.12em]` — correct
- Add `font-optical-sizing: auto` globally (already present via font features)

### Spacing & Geometry (Existing — No Changes Needed)

- `--radius-card: 0.875rem` (14px) — correct
- `--radius-control: 0.5rem` (8px) — correct
- `--radius-pill: 999px` — correct
- Sidebar gap: `gap-3` (0.75rem) — correct
- Main padding: `px-6 py-7 lg:px-8` — correct

### Spring Motion Tokens (New)

```css
/* Fluid motion system — Apple spring parameters */
@theme {
  --spring-default: cubic-bezier(0.32, 0.08, 0.24, 1);     /* critically damped */
  --spring-bounce: cubic-bezier(0.34, 1.56, 0.64, 1);      /* under-damped, momentum */
  --spring-gentle: cubic-bezier(0.25, 0.1, 0.25, 1);       /* gentle settle */
  --spring-snap: cubic-bezier(0.2, 0, 0, 1);               /* snappy, no overshoot */

  --duration-instant: 100ms;
  --duration-fast: 200ms;
  --duration-normal: 300ms;
  --duration-slow: 500ms;
  --duration-material: 400ms;  /* glass materialize */
}
```

---

## Part IV — Component & Material Specifications

### GlassSurface Refinements

**Current:** 7 materials with correct blur/saturation values. Missing: illumination, materialize animation, content-aware adaptation.

**Additions needed:**

1. **Illumination on hover/interaction:**
   - `glass-chrome` and `glass-sheet`: subtle underglow on hover (`box-shadow` additive)
   - `glass-control`: brightness increase on `:active` (0.5% → 1% white overlay)
   - `glass-accent`: scale pulse on press (0.97 → 1.0)

2. **Materialize-on-enter animation:**
   - Blur: start at 0px, animate to target blur
   - Scale: start at 0.98, animate to 1.0
   - Opacity: start at 0, animate to 1
   - Duration: 400ms with `--spring-default` easing
   - Apply via CSS `@keyframes` class

3. **Content-aware shadow adaptation:**
   - Shadows increase opacity when background is busy (dynamic via CSS)
   - Shadows decrease when background is plain
   - Implementation: CSS custom property updated by parent context

### Sidebar Specifications

**Current state:** Correct material, width, radius, specular edge.

**Refinements:**
- Add scroll edge effect at bottom of nav list (gradient fade)
- Nav rows: `glass-nav` material on hover/active only (not always)
- Active nav row: subtle `glass-control` background with left accent bar
- Brand section: slight elevation above nav items
- Health status indicator: pulse animation using spring motion

### Topbar Specifications

**Current state:** Correct material, height, radius, specular edge.

**Refinements:**
- Search field: focus state with `glass-well` illumination
- Route title: spring-animated text transitions on route change
- Theme toggle: rotate animation with spring physics
- Add scroll edge effect when content scrolls beneath

### Dashboard Specifications

**Current state:** Uses `GlassSurface material="surface"` for hero panels.

**Refinements under strict Apple rules:**
- Hero panels (Continue Working, Start New): transition to `content-card` (opaque) — these are content, not chrome
- Keep `GlassSurface` only on actual navigation elements within dashboard
- Dashboard metric cards: use `content-card` with subtle shadow hierarchy
- Activity feed: opaque content, no glass
- Briefing banner: opaque content with accent color

### Dialog Specifications

**Current state:** Uses `glass-sheet` material correctly.

**Refinements:**
- Add dimming scrim behind dialogs (already present)
- Spring-animated entrance (scale from 0.95, opacity from 0)
- Spring-animated exit (reverse)
- Scrim opacity: 60% black with blur
- Dialog origin: spring from trigger element position (where possible)

### Button Specifications

**Current state:** `glass-accent` (primary), `glass-control` (secondary).

**Refinements:**
- Primary (`glass-accent`): scale 0.97 on press, spring back on release
- Secondary (`glass-control`): brightness increase on press
- Ghost: opacity change on hover, no glass
- All buttons: `transition: transform 100ms ease-out` on `:active`
- Icon buttons: same glass treatment as text buttons

### Input Specifications

**Current state:** `GlassInput` uses `glass-well`.

**Refinements:**
- Focus state: `glass-well` with illumination underglow
- Error state: red tint overlay
- Disabled state: reduced opacity, no blur

---

## Part V — Where and How Glass Should Be Introduced

### The Layer Map

```
┌─────────────────────────────────────────────────┐
│  Level 0: Environmental Canvas                  │
│  (orbs, gradients, grain, vignette)             │
│                                                  │
│  ┌──────────┐  ┌────────────────────────────┐   │
│  │ Level 1  │  │ Level 2: Topbar            │   │
│  │ Sidebar  │  │ (glass-chrome)              │   │
│  │(chrome)  │  │                             │   │
│  │          │  ├────────────────────────────┤   │
│  │          │  │ Level 5: Content            │   │
│  │          │  │ (opaque cards, lists)       │   │
│  │          │  │ Scrolls UNDER chrome        │   │
│  │          │  │                             │   │
│  └──────────┘  └────────────────────────────┘   │
│                                                  │
│  Level 3: Dialogs/Sheets (glass-sheet)          │
│  Level 4: Controls (glass-control/well/accent)  │
└─────────────────────────────────────────────────┘
```

### Per-Page Glass Mapping

| Page | Glass Elements | Content Layer |
|---|---|---|
| **Dashboard** | None (content page) | All cards, feeds, metrics |
| **Workspaces** | Topbar search, sidebar | Workspace cards, file tree |
| **Timeline** | Sidebar | Timeline entries, filters |
| **Knowledge Graph** | Sidebar | Graph SVG, detail panel |
| **Search** | Sidebar, search field | Results list |
| **Learning** | Sidebar | Learning cards, preferences |
| **Memory** | Sidebar | Memory snapshots |
| **Performance** | Sidebar | Charts, diagnostics |
| **Recovery** | Sidebar | Health cards, history |
| **Maintenance** | Sidebar | Backup panels, integrity |
| **Settings** | Sidebar | Settings cards |

**Key insight:** Under strict Apple rules, most pages are content pages. Glass lives in the persistent chrome (Sidebar + Topbar) and floating controls (Dialogs, context menus, search overlays).

---

## Part VI — Technical Implementation Approach

### Technology Stack

- **Motion library:** `motion` (Framer Motion v11+) — already compatible with React 19
- **Spring physics:** Built into `motion` — `type: "spring"` with `damping`, `stiffness`, `mass`
- **Glass optics:** Existing `liquidGlass.ts` SVG engine — no changes needed
- **CSS:** Tailwind CSS v4 with `@theme` tokens — extend for motion tokens
- **React:** React 19 — `useId`, `useTransition`, `useDeferredValue` for route transitions

### Architecture Decisions

1. **Keep GlassSurface as the single glass primitive** — all glass goes through it
2. **Add a `<Fluid>` wrapper component** — applies spring animations to children
3. **Add a `<PageTransition>` component** — handles route transitions with glass materialize
4. **Add a `useScrollEdge` hook** — detects scroll position for edge effects
5. **Add a `useInteraction` hook** — provides hover/press illumination feedback

### File Organization

```
frontend/src/
├── components/
│   ├── ui/
│   │   ├── GlassSurface.tsx          # MODIFY — add illumination, materialize
│   │   ├── Fluid.tsx                 # NEW — spring animation wrapper
│   │   ├── PageTransition.tsx        # NEW — route transition with glass
│   │   ├── ScrollEdge.tsx            # NEW — scroll edge fade effect
│   │   ├── Button.tsx                # MODIFY — add spring press feedback
│   │   ├── Dialog.tsx                # MODIFY — add spring entrance/exit
│   │   └── GlassInput.tsx            # MODIFY — add focus illumination
│   └── navigation/
│       ├── Sidebar.tsx               # MODIFY — add scroll edge, hover illumination
│       └── Topbar.tsx                # MODIFY — add scroll edge, title transition
├── hooks/
│   ├── useLiquidGlass.ts            # KEEP — no changes
│   ├── useScrollEdge.ts             # NEW — scroll position detection
│   └── useInteraction.ts            # NEW — hover/press state
├── lib/
│   ├── liquidGlass.ts               # KEEP — no changes
│   └── springs.ts                   # NEW — spring preset configurations
├── index.css                        # MODIFY — add motion tokens, materialize keyframes
└── App.tsx                          # MODIFY — wrap routes in PageTransition
```

---

## Part VII — Step-by-Step Implementation Phases

### Phase 1: Motion Foundation (Days 1–2)

**Goal:** Establish the spring motion system that everything else builds on.

**Files to modify:**
- `frontend/src/index.css` — Add motion tokens, materialize keyframes
- `frontend/package.json` — Add `motion` dependency

**Files to create:**
- `frontend/src/lib/spring.ts` — Spring preset configurations

**Tasks:**
1. Install `motion` package
2. Add CSS motion tokens to `@theme` block in `index.css`
3. Create `spring.ts` with presets: `default`, `bounce`, `gentle`, `snap`, `material`
4. Add `@keyframes glass-materialize` (blur 0→target, scale 0.98→1, opacity 0→1)
5. Add `@keyframes glass-dematerialize` (reverse)
6. Add `@keyframes glass-illuminate` (underglow pulse)
7. Test: verify spring presets work with `motion` components

### Phase 2: Fluid Components (Days 3–5)

**Goal:** Create the motion wrapper components that give everything spring behavior.

**Files to create:**
- `frontend/src/components/ui/Fluid.tsx` — Spring animation wrapper
- `frontend/src/components/ui/ScrollEdge.tsx` — Scroll edge fade effect
- `frontend/src/hooks/useScrollEdge.ts` — Scroll position detection
- `frontend/src/hooks/useInteraction.ts` — Hover/press state management

**Files to modify:**
- `frontend/src/components/ui/Button.tsx` — Add spring press feedback
- `frontend/src/components/ui/GlassSurface.tsx` — Add illumination, materialize

**Tasks:**
1. Create `Fluid.tsx` — wraps any child with spring-animated `motion.div`
   - Props: `animate` (visibility toggle), `spring` (preset name), `className`
   - Uses `AnimatePresence` for mount/unmount transitions
   - Applies spring physics from `spring.ts` presets
2. Create `ScrollEdge.tsx` — gradient fade at scroll boundary
   - Renders a gradient mask (transparent → background) at top/bottom
   - Controlled by `useScrollEdge` hook
3. Create `useScrollEdge` hook — observes scroll container, returns `{ atTop, atBottom, scrollY }`
4. Create `useInteraction` hook — returns `{ isHovered, isPressed, interactionProps }`
   - Attaches pointerenter/pointerleave/pointerdown/pointerup
   - Returns spread props for any element
5. Modify `Button.tsx`:
   - Primary (`glass-accent`): `motion.button` with `whileTap={{ scale: 0.97 }}`, `transition: { type: "spring", damping: 1.0, stiffness: 400 }`
   - Secondary (`glass-control`): `whileTap={{ brightness: 1.1 }}` via CSS
6. Modify `GlassSurface.tsx`:
   - Add `materialize` prop (default: false) — when true, applies `glass-materialize` animation on mount
   - Add `illuminate` prop (default: false) — when true, applies underglow on hover
   - Add `className` passthrough for animation classes

### Phase 3: Navigation Polish (Days 6–8)

**Goal:** Make Sidebar and Topbar feel like Apple's Liquid Glass chrome.

**Files to modify:**
- `frontend/src/components/navigation/Sidebar.tsx`
- `frontend/src/components/navigation/Topbar.tsx`
- `frontend/src/layouts/AppLayout.tsx`

**Tasks:**
1. Sidebar refinements:
   - Add `ScrollEdge` at bottom of nav list (gradient fade before health status)
   - Nav rows: apply `glass-nav` background on hover/active only (currently always visible)
   - Active nav row: add left accent bar (`w-[3px] bg-accent rounded-full`)
   - Brand section: slight vertical separator from nav items
   - Wrap sidebar content in `motion.nav` for entrance animation
2. Topbar refinements:
   - Add `ScrollEdge` at bottom (gradient fade where content meets topbar)
   - Route title: wrap in `AnimatePresence` + `motion.span` with `key={route}` for spring text swap
   - Search field: focus state with `glass-well` + subtle blue border glow
   - Theme toggle: `motion.button` with `whileTap={{ rotate: 180 }}` spring
3. AppLayout refinements:
   - Main content area: add `overflow-y-auto overscroll-contain` (already present)
   - Ensure sidebar and topbar have `position: sticky` or proper z-index layering
   - Add `contain: strict` to main content for GPU isolation

### Phase 4: Dialog & Sheet System (Days 9–10)

**Goal:** Make dialogs feel like Apple's spring-from-trigger sheets.

**Files to modify:**
- `frontend/src/components/ui/Dialog.tsx`

**Tasks:**
1. Replace CSS fade transition with spring animation:
   - Entrance: `motion.div` with `initial={{ opacity: 0, scale: 0.95, y: 10 }}` → `animate={{ opacity: 1, scale: 1, y: 0 }}`
   - Exit: reverse with `exit={{ opacity: 0, scale: 0.95, y: 10 }}`
   - Spring: `{ type: "spring", damping: 1.0, stiffness: 300 }`
2. Scrim: `motion.div` with `initial={{ opacity: 0 }}` → `animate={{ opacity: 0.6 }}`
3. Dialog origin: set `transformOrigin` to center (or trigger element if accessible)
4. Ensure `AnimatePresence` wraps conditional rendering
5. Add keyboard dismiss (Escape key) with spring exit

### Phase 5: Page Transitions (Days 11–12)

**Goal:** Route changes feel like Apple's materialize/dematerialize transitions.

**Files to create:**
- `frontend/src/components/ui/PageTransition.tsx`

**Files to modify:**
- `frontend/src/App.tsx`

**Tasks:**
1. Create `PageTransition.tsx`:
   - Wraps `<Outlet />` in `AnimatePresence` with `mode="wait"`
   - Each page wrapped in `motion.div` with `key={location.pathname}`
   - Entrance: `initial={{ opacity: 0, y: 8 }}` → `animate={{ opacity: 1, y: 0 }}`
   - Exit: `exit={{ opacity: 0, y: -4 }}`
   - Spring: `{ type: "spring", damping: 1.0, stiffness: 300 }`
   - Duration: ~300ms (fast enough to feel instant, slow enough to register)
2. Modify `App.tsx`:
   - Wrap routes with `PageTransition`
   - Use `useLocation` for key propagation
   - Ensure `HashRouter` compatibility

### Phase 6: Content Layer Audit (Days 13–14)

**Goal:** Ensure all content-layer components are properly opaque, not glass.

**Files to audit and potentially modify:**
- `frontend/src/components/ui/Card.tsx` — verify `content-card` default
- `frontend/src/features/dashboard/DashboardView.tsx` — remove glass from hero panels
- `frontend/src/features/dashboard/components/*.tsx` — audit all dashboard sub-components
- `frontend/src/pages/*.tsx` — audit all 12 pages for glass misuse

**Tasks:**
1. Audit every page for glass-on-content violations
2. Replace any `GlassSurface material="surface"` on content panels with `content-card`
3. Ensure `Card` component defaults to opaque content layer
4. Add `content-card` CSS class with proper shadow hierarchy:
   ```css
   .content-card {
     background: rgba(255, 255, 255, 0.03);
     border: 1px solid rgba(255, 255, 255, 0.06);
     box-shadow: 0 14px 36px rgba(0, 0, 0, 0.38);
   }
   ```
5. Verify no nested glass surfaces exist anywhere

### Phase 7: Performance & Accessibility Validation (Days 15–16)

**Goal:** Ensure glass + motion doesn't hurt performance or accessibility.

**Tasks:**
1. Performance audit:
   - Count simultaneous glass surfaces on each page (max 4 recommended)
   - Verify `maxArea: 420,000` guard triggers for oversized surfaces
   - Profile GPU usage with glass + motion enabled
   - Test on macOS with Apple Silicon and Intel
   - Verify `contain: strict` on glass containers
   - Check `will-change: transform` on animated elements
2. Accessibility audit:
   - Test `prefers-reduced-motion: reduce` — all springs become opacity cross-fades
   - Test `prefers-reduced-transparency: reduce` — glass becomes opaque
   - Test `prefers-contrast: more` — near-solid surfaces with defined borders
   - Verify all text over glass passes WCAG AA (4.5:1 for normal, 3:1 for large)
   - Test with VoiceOver — all glass elements must be accessible
   - Test Dynamic Type scaling
3. Cross-platform fallback:
   - Verify Chromium: full SVG refraction
   - Verify Safari: frosted blur fallback (browser detection in `liquidGlass.ts`)
   - Verify Firefox: frosted blur fallback

### Phase 8: Polish & Consistency (Days 17–18)

**Goal:** Final pass for Apple-level craft.

**Tasks:**
1. Verify all spring animations use consistent presets (no ad-hoc durations)
2. Verify all glass materials follow the hierarchy (no glass on content)
3. Verify specular edges are consistent across sidebar and topbar
4. Verify environmental canvas orbs are subtle (low saturation, slow drift)
5. Verify film grain is barely visible (4.5% opacity)
6. Verify light theme glass looks correct (frosted white, adjusted tokens)
7. Run full test suite: `npm test`
8. Run lint: `npm run lint`
9. Run build: `npm run build`
10. Test on actual macOS hardware

---

## Part VIII — Performance & Accessibility Considerations

### Performance Budget

| Metric | Target | Current |
|---|---|---|
| Max simultaneous glass surfaces | 4 | Varies per page |
| Blur radius (sidebar/topbar) | 32px | 32px ✅ |
| Blur radius (dialogs) | 40px | 40px ✅ |
| Max element area for refraction | 420,000 px² | 420,000 px² ✅ |
| Spring animation frame budget | < 2ms per frame | Depends on element count |
| Orb animation count | 5 | 5 ✅ |
| Orb animation speed | 90–130s cycle | 90–130s ✅ |

### GPU Optimization Rules

1. Only animate `transform` and `opacity` — never animate `width`, `height`, `top`, `left`, `blur`
2. Apply `will-change: transform` to elements before animation starts
3. Use `contain: strict` on glass containers to isolate filter nodes
4. Remove `will-change` after animation completes
5. Limit glass surfaces to 4 per viewport
6. Use `content-visibility: auto` for off-screen content

### Accessibility Checklist

| Requirement | Implementation |
|---|---|
| `prefers-reduced-motion: reduce` | Springs → opacity cross-fades, orbs → static, no parallax |
| `prefers-reduced-transparency: reduce` | Glass → `rgba(14,16,24,0.96)` near-opaque fill |
| `prefers-contrast: more` | Near-solid surfaces with defined contrasting borders |
| WCAG AA text contrast | 4.5:1 minimum for normal text, 3:1 for large text |
| Dynamic Type | All spacing in `rem`/`em`, glass containers scale with text |
| VoiceOver | All glass elements must have accessible labels and roles |
| Keyboard navigation | All glass controls must be focusable and operable |
| No focus traps | Dialogs trap focus, but glass chrome does not |

---

## Part IX — Exact Files to Modify

### Critical Path (Must Modify)

| File | Changes |
|---|---|
| `frontend/package.json` | Add `motion` dependency |
| `frontend/src/index.css` | Add motion tokens, materialize keyframes, content-card refinement |
| `frontend/src/components/ui/GlassSurface.tsx` | Add `materialize`, `illuminate`, `className` props |
| `frontend/src/components/ui/Button.tsx` | Add `motion.button` with spring press feedback |
| `frontend/src/components/ui/Dialog.tsx` | Replace CSS transition with spring entrance/exit |
| `frontend/src/components/navigation/Sidebar.tsx` | Add scroll edge, hover illumination, active accent |
| `frontend/src/components/navigation/Topbar.tsx` | Add scroll edge, title transition, search focus |
| `frontend/src/layouts/AppLayout.tsx` | Add `contain: strict` to main, verify z-index |
| `frontend/src/App.tsx` | Add `PageTransition` wrapper |

### New Files (Must Create)

| File | Purpose |
|---|---|
| `frontend/src/lib/spring.ts` | Spring preset configurations |
| `frontend/src/components/ui/Fluid.tsx` | Spring animation wrapper component |
| `frontend/src/components/ui/PageTransition.tsx` | Route transition with glass materialize |
| `frontend/src/components/ui/ScrollEdge.tsx` | Scroll edge fade effect |
| `frontend/src/hooks/useScrollEdge.ts` | Scroll position detection hook |
| `frontend/src/hooks/useInteraction.ts` | Hover/press state management hook |

### Audit-Only (Verify Correctness)

| File | Verify |
|---|---|
| `frontend/src/components/ui/Card.tsx` | Defaults to `content-card`, not glass |
| `frontend/src/features/dashboard/DashboardView.tsx` | No glass on content panels |
| `frontend/src/features/dashboard/components/*.tsx` | No glass misuse |
| `frontend/src/pages/*.tsx` | All 12 pages follow material hierarchy |
| `frontend/src/components/ui/GlassInput.tsx` | Focus illumination correct |
| `frontend/src/components/ui/SegmentedControl.tsx` | glass-control only |

---

## Part X — Rules for Consistency

### The Ten Commandments of ContextSphere Liquid Glass

1. **Glass is chrome, not content.** Sidebar, topbar, dialogs, controls — never cards, lists, or panels.

2. **One glass layer, never stacked.** If a parent is glass, children must be opaque content.

3. **Tint only primary actions.** `glass-accent` for the one main action per context. Everything else is `glass-control`.

4. **Springs, not keyframes.** Every animation uses spring physics. CSS `@keyframes` only for infinite ambient effects (orbs).

5. **Interrupt everything.** Every spring animation must be interruptible. No locked transitions.

6. **Materialize, don't fade.** Glass surfaces arrive with blur + scale + opacity together. Never opacity-only fade.

7. **Honor accessibility.** `prefers-reduced-motion`, `prefers-reduced-transparency`, `prefers-contrast` — all three must work.

8. **Consistent presets.** Use `spring.ts` presets. No ad-hoc `transition` values on glass elements.

9. **Scroll edge, not hard border.** Where content meets glass chrome, use gradient fade. Never 1px borders.

10. **Profile regularly.** Max 4 glass surfaces per viewport. Test on Intel Macs. Check GPU usage.

### Naming Conventions

- Glass materials: always via `GlassSurface` component with `material` prop
- Spring animations: always via `motion.*` components with preset from `spring.ts`
- CSS classes: use Tailwind utilities, not raw CSS for new components
- Component props: `materialize`, `illuminate`, `spring` (not `animate`, `blur`, `glass`)

### Testing Requirements

- Every new component must pass `npm test`
- Every modified component must pass `npm run lint`
- Build must succeed: `npm run build`
- Manual test: reduce-motion, reduce-transparency, increase-contrast
- Manual test: resize window to minimum (980x640) — no glass overflow
- Manual test: route through all 12 pages — consistent glass hierarchy

---

## Appendix A — Spring Preset Reference

```typescript
// frontend/src/lib/spring.ts

export const springs = {
  /** Critically damped — default UI transitions, no overshoot */
  default: { type: "spring" as const, damping: 1.0, stiffness: 300 },

  /** Under-damped — momentum-driven interactions, slight bounce */
  bounce: { type: "spring" as const, damping: 0.8, stiffness: 300 },

  /** Gentle settle — large surface transitions, minimal movement */
  gentle: { type: "spring" as const, damping: 1.0, stiffness: 200 },

  /** Snappy — small controls, instant response */
  snap: { type: "spring" as const, damping: 1.0, stiffness: 500 },

  /** Material — glass materialize/dematerialize */
  material: { type: "spring" as const, damping: 1.0, stiffness: 250 },
} as const;
```

## Appendix B — CSS Motion Tokens

```css
/* Add to @theme block in index.css */

/* Motion */
--spring-default: cubic-bezier(0.32, 0.08, 0.24, 1);
--spring-bounce: cubic-bezier(0.34, 1.56, 0.64, 1);
--spring-gentle: cubic-bezier(0.25, 0.1, 0.25, 1);
--spring-snap: cubic-bezier(0.2, 0, 0, 1);

--duration-instant: 100ms;
--duration-fast: 200ms;
--duration-normal: 300ms;
--duration-slow: 500ms;
--duration-material: 400ms;
```

## Appendix C — Glass Materialize Keyframes

```css
/* Add to index.css */

@keyframes glass-materialize {
  0% {
    opacity: 0;
    filter: blur(0px);
    transform: scale(0.98);
  }
  100% {
    opacity: 1;
    filter: blur(var(--glass-blur, 22px));
    transform: scale(1);
  }
}

@keyframes glass-dematerialize {
  0% {
    opacity: 1;
    filter: blur(var(--glass-blur, 22px));
    transform: scale(1);
  }
  100% {
    opacity: 0;
    filter: blur(0px);
    transform: scale(0.98);
  }
}

@keyframes glass-illuminate {
  0% { box-shadow: inset 0 0 0 0 rgba(255,255,255,0); }
  50% { box-shadow: inset 0 0 20px 0 rgba(255,255,255,0.03); }
  100% { box-shadow: inset 0 0 0 0 rgba(255,255,255,0); }
}
```

---

*This blueprint was produced by analyzing Apple's WWDC 2025 Liquid Glass sessions (219, 310, 323), the updated Human Interface Guidelines for iOS 26/macOS Tahoe, the existing ContextSphere codebase (696-line material system, 311-line SVG refraction engine, 14 UI primitives, 12 route pages), and the apple-design skill's fluid interface principles.*
