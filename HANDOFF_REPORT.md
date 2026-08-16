# ChronoDesk — Apple Liquid Glass Design & Implementation Handoff

> **Status:** DRAFT — for next session. Do not modify any files. Do not implement anything.
> **Stopping point:** Read `frontend/src/index.css` (line 1–681), about to enhance the core Liquid Glass token system.
> **Last known commit:** `28daab7 Complete Liquid Glass UI refinement`
> **Reference folder:** `liquid-glass-main 2/` (liquid-glass.js reference implementation, SKILL.md, demo screenshots)
> **Branch:** main, up to date with origin/main
> **Git HEAD:** `28daab7 Complete Liquid Glass UI refinement`

---

## 1. APPLE LIQUID GLASS

### 1.1 What Apple Liquid Glass Actually Is vs Generic Glassmorphism

**Apple Liquid Glass** is a rendering technique that combines:

- **Backdrop-filter with SVG displacement refraction** — a blurred, inlined SVG displacement map creates a "frosted" edge that bends light around the element. This is the defining visual signature: a subtle refraction bulge at the rim, not a generic white/blur overlay.
- **Environmental context** — a static or semi-static light-field canvas (orbs, gradients, grain) sits behind the glass, so the glass has a "world" to distort against. This is what makes it read as glass rather than just frosted.
- **No saturate boost on the glass layer itself** — the glass tint is nearly neutral. Color comes from the environment. The backdrop-filter uses `saturate(1.5)` not `saturate(200%)`.
- **Specular highlights** — a sharp bright reflection along the top of the glass pane, not a soft white glow.
- **Edge lighting** — a hairline glass border, not a thick outline.
- **Depth shadows** — a bottom inset shadow that floats the surface above the canvas.
- **Environmental color adaptation** — the light fields use saturated colors but with low opacity so they read as neutral atmospheric light, not painted colors.

**Generic glassmorphism** (the anti-pattern) is:
- A flat white/light overlay with no depth
- `backdrop-filter: blur(20px) saturate(100%)` with no refraction
- No environmental canvas — just a "frosted pane"
- No specular highlight
- No dimensional shadow
- Often `backdrop-filter: blur(100px)` which is excessive
- Solid white or light gray tint (no environmental color adaptation)

### 1.2 Functional Layer vs Content Layer

Apple HIG defines two distinct layers:

| Layer | Purpose | What it gets |
|---|---|---|
| **Functional layer** | Navigation, toolbars, dialogs, sheets, input fields | Full Liquid Glass with refraction, specular edge, depth shadow, environmental lighting |
| **Content layer** | Dashboard cards, article panels, data tables | Calm, opaque surfaces — no backdrop-filter, no refraction, just subtle borders and shadows |

**Key rule:** Glass is only for the functional layer. Content cards use calm, non-glass surfaces.

### 1.3 Regular Materials (the existing CSS utilities)

| Material | CSS Class | Background | Backdrop Filter | Box Shadow | Use Case |
|---|---|---|---|---|---|
| **Chrome** | `.glass-chrome` | `linear-gradient(180deg, tint-top, tint-mid, tint-bottom)` | `blur(28px) saturate(130%)` | `inset 0 1px 1px specular, inset 0 -8px 20px underlight, inset 0 0 0 1px rgba(255,255,255,0.12), inset 0 -1px 0 rgba(0,0,0,0.2), 0 24px 60px rgba(0,0,0,0.5), 0 6px 16px rgba(0,0,0,0.3)` | Sidebar, Topbar, dialogs, hero surfaces |
| **Surface** | `.glass-surface` | `linear-gradient(180deg, tint-mid, tint-bottom 70%)` | `blur(18px) saturate(122%)` | `inset 0 1px 0 specular-soft, inset 0 -1px 0 rgba(0,0,0,0.3), 0 18px 44px rgba(0,0,0,0.42)` | Large hero panels, GlassSurface elements |
| **Panel** | `.glass-panel` | `linear-gradient(180deg, tint-mid, tint-bottom 75%)` | `blur(14px) saturate(118%)` | `inset 0 1px 0 rgba(255,255,255,0.1), 0 12px 26px rgba(0,0,0,0.2)` | Cards, list rails, secondary panels |
| **Well** | `.glass-well` | `linear-gradient(180deg, tint-bottom, rgba(0,0,0,0.14))` | `blur(10px) saturate(118%)` | `inset 0 1px 2px rgba(0,0,0,0.3), inset 0 0 0 1px rgba(255,255,255,0.06)` | Inset inputs, search fields, GlassInput |
| **Control** | `.glass-control` | `linear-gradient(180deg, tint-mid, tint-bottom 72%)` | `blur(10px) saturate(116%)` | `inset 0 1px 0 rgba(255,255,255,0.18), inset 0 -1px 0 rgba(0,0,0,0.2), 0 2px 6px rgba(0,0,0,0.16)` | Buttons, segments, small controls |
| **Nav** | `.glass-nav` | `backdrop-filter: blur(8px) saturate(110%)` | `blur(8px) saturate(110%)` | None | Sidebar row items (no extra blur needed) |
| **Sheet** | `.glass-sheet` | `linear-gradient(180deg, tint-top, tint-mid 40%, tint-bottom)` | `blur(36px) saturate(132%)` | `inset 0 1px 1px specular, inset 0 -8px 20px underlight, inset 0 0 0 1px rgba(255,255,255,0.13), inset 0 -1px 0 rgba(0,0,0,0.2), 0 32px 80px rgba(0,0,0,0.6), 0 10px 28px rgba(0,0,0,0.38)` | Dialogs, full-sheet panels |
| **Accent** | `.glass-accent` | `linear-gradient(180deg, rgba(10,132,255,0.48), rgba(10,132,255,0.28))` | `blur(12px) saturate(140%)` | `inset 0 1px 0 rgba(255,255,255,0.45), inset 0 -1px 0 rgba(0,0,0,0.2), inset 0 0 0 1px rgba(255,255,255,0.14), 0 1px 2px rgba(0,0,0,0.35), 0 8px 20px rgba(0,0,0,0.28)` | Primary action buttons, accent elements |

### 1.4 Translucency, Blur, Vibrancy, Refraction/Lensing

- **Translucency:** The glass is 50–80% transparent depending on material. Chrome has `saturate(130%)` (boosted but low), surface has `saturate(122%)`, well has `saturate(118%)`. The tint is nearly neutral — no hue shift.
- **Blur:** Ranges from `10px` (control, nav) to `36px` (sheet) to `28px` (chrome). Higher blur = more frosted, more depth.
- **Vibrancy:** The specular top edge is bright (`rgba(255,255,255,0.5)` on chrome). This is a sharp, focused highlight, not a glow. The edge-dark (`rgba(0,0,0,0.4)`) on the bottom creates the shadow that gives dimension.
- **Refraction/Lensing:** The SVG displacement filter creates a subtle refraction bulge at the rim — a geometric distortion that makes the glass look like it's bending light. The displacement map is a red left→right ramp and blue top→bottom ramp combined with a diffused gray inset. The `scale` values are set to `-96` (gentle bulge) or `-112` (dramatic). The `chroma` value of `4` is good but the prism fringe is too subtle for the current implementation.

### 1.5 Specular Highlights, Edge Lighting, Depth, Shadows

- **Specular highlight:** `rgba(255,255,255,0.5)` on chrome level — a bright, focused reflection along the top edge. Slightly less on surface (`rgba(255,255,255,0.18)`). This is the single most important visual cue that says "this is glass."
- **Edge lighting:** A hairline glass border (`rgba(255,255,255,0.11–0.14)`) — not a 1px outline, but a very thin, subtle highlight that sits at the glass edge.
- **Depth shadow:** Inset 0 1px 0 specular + deep drop shadow (0 24px 60px rgba(0,0,0,0.38)). The shadow is deep but not heavy — it floats the surface above the canvas.
- **Underlight:** An inset shadow from below (`inset 0 -8px 20px rgba(0,0,0,0.2)`) that creates the sense that the glass is floating on a surface below it.

### 1.6 Environmental Color Adaptation

The environmental canvas uses radial gradients (orbs) with low saturation:

- `ambient-blue`: `rgba(120,165,255,0.07)` — 7% opacity
- `ambient-cyan`: `rgba(140,210,255,0.055)` — 5.5% opacity
- `ambient-violet`: `rgba(175,150,255,0.055)` — 5.5% opacity
- `ambient-emerald`: `rgba(130,220,170,0.035)` — 3.5% opacity
- `ambient-warm`: `rgba(255,205,140,0.03)` — 3% opacity
- `ambient-core`: `rgba(150,175,235,0.045)` — 4.5% opacity

The orbs are large, soft, and drift slowly. Saturation is deliberately low so the glass reads as neutral — color stays atmospheric, not painted.

**Dark theme values:** Darker orbs (`rgba(120,165,255,0.08)`), dimmer core (`rgba(150,175,235,0.045)`), more ambient blue (`rgba(120,165,255,0.07)`).

**Light theme values:** Brighter orbs, darker core, adjusted opacity for the light environment.

### 1.7 Interaction States

| State | CSS Class / Visual |
|---|---|
| **Normal** | `glass-chrome` or `glass-surface`, subtle ambient light, normal shadow |
| **Hover** | `hover:bg-(--color-surface-hover)` or `hover:shadow-[inset_0_1px_0_rgba(255,255,255,0.1)]` — raised feel, slight brightness increase |
| **Pressed** | `active:scale-[0.98]`, `active:brightness-[0.97]`, or `active:bg-(--color-surface-hover)` — physical "press into" feel |
| **Focus** | `focus-visible:outline 2px solid var(--color-accent)`, `focus-visible:outline-offset:2px` |
| **Active** | `active:scale-[0.98]` on buttons — tactile feedback |
| **Selected** | `material-selected` — bright background `var(--color-surface-raised)`, subtle inner shadow |
| **Disabled** | `opacity-50`, `pointer-events-none` — dimmed, non-interactive |
| **Accent hover** | `hover:brightness-[1.06]`, `hover:text-(--color-foreground)` |
| **Accent active** | `active:brightness-[0.97]` |

### 1.8 Geometry / Radii

| Radius | Value | Usage |
|---|---|---|
| `--radius-card` | `0.875rem` (14px) | Cards, content surfaces |
| `--radius-control` | `0.5rem` (8px) | Buttons, inputs, segments, glass-well |
| `--radius-small` | `0.375rem` (6px) | Small inline controls |
| `--radius-pill` | `999px` | Pill-shaped active states |
| `rounded-2xl` | 1.5rem (24px) | Sidebar (currently `rounded-2xl` but should be `rounded-3xl`) |
| `rounded-xl` | 0.75rem (12px) | Topbar, some dialogs |
| `rounded-3xl` | 2.25rem (36px) | Sidebars (target) |

### 1.9 Scrolling Behavior

- The `overflow-y-auto` is on the main content area (AppLayout: `.overflow-y-auto` on `.main`)
- `scroll-fade-y` utility masks the bottom of scrollable panels (black at the bottom, transparent above)
- Scroll behavior: `scroll-behavior: auto` (no smooth scrolling in CSS, but the JS may override)
- The environment canvas is `absolute` positioned and does not scroll

### 1.10 Accessibility / Reduced Transparency / Motion

**Reduced transparency:** On dark theme, `opacity` is carefully managed. The `color-scheme: dark` is set on `html` and the body uses `background-color: var(--color-background)` (dark). On light theme, `color-scheme: light` is set.

**Reduced motion:** `prefers-reduced-motion: reduce` applies to `*, *::before, *::after` — all animations are set to `0.01ms` duration, `1` iteration, and `transition-duration: 0.01ms`. The `@keyframes` defined for `fade-in`, `scale-in`, `slide-up`, `slide-down`, `grow-bar`, `env-drift-a`, `env-drift-b` are all suppressed under reduced-motion.

### 1.11 When Glass SHOULD and SHOULD NOT Be Used

**Should use Glass:**
- Navigation sidebar (Chrome level)
- Topbar/header (Chrome level)
- Dialogs/sheets (Sheet level, thickest blur, strongest specular)
- Search fields (GlassWell level)
- Primary action buttons (Accent level)
- Hero surfaces (Surface level)
- Card contents that need visual depth (Surface level)

**Should NOT use Glass:**
- Content cards, data tables, lists (use calm `content-card` surfaces)
- Buttons (use `glass-control` or plain — but never `glass-chrome` on a button)
- Inputs (use `glass-well`)
- Every card being glass (this is the #1 anti-pattern)
- The sidebar should be glass but NOT every child card inside it
- The dashboard card should NOT be glass — it should be `content-card`
- Glass-on-glass (glass inside glass) — only top-level surfaces should be glass

### 1.12 Why Glass-on-Glass is Wrong

Glass-on-glass means a glass element containing another glass element. The result is:

1. **Double blur** — excessive, unappealing
2. **Double refraction** — the displacement map from the outer glass applies on the inner glass's element, making the inner element's refraction look wrong
3. **Loss of focus** — the inner glass should not refract if it's inside a glass container
4. **Performance cost** — every nested glass surface runs its own SVG filter on each element

**Rule:** Only the top-level surface should be Glass. Inner surfaces should be calm (content-card or glass-control).

### 1.13 Apple Principles vs Our React/Tauri Approximation

**Apple Liquid Glass Principles:**
1. **Color comes from behind the glass** — the glass tint is neutral (grayish white), not colored. The environment carries the color.
2. **Refraction is geometric, not optical** — the SVG displacement map creates a physical bulge at the rim, not a uniform blur.
3. **Specular is sharp, not soft** — a bright white reflection along the top edge, not a glow.
4. **Depth is in the shadow** — the drop shadow gives the surface physical dimension.
5. **Blur is thin, not thick** — 10–36px range depending on the surface; never 100px+.
6. **Environment is visible** — the light fields and orbs behind the glass make the glass "read" as glass.
7. **Tiling is subtle** — the background gradient has a slight offset between top and bottom (more top than bottom).
8. **Rim is the focus** — the specular top edge is the single most important visual cue.

**Our React/Tauri Approximation:**

We have a partially correct implementation (the CSS tokens, the GlassSurface, the `liquidGlass.ts` with SVG displacement) but it has gaps:

- The `liquidGlass.ts` implementation is closer to the "reference" (deepika-builds/liquid-glass) but the refraction `scale` values are `-96` (gentle) vs the reference's `-112` (more dramatic)
- The CSS `backdrop-filter` for refraction is applied via `useLiquidGlass` hook which calls `applyLiquidGlass` from `lib/liquidGlass.ts` — this is correct but needs to be verified as compatible with Tauri's Chromium version
- The `GlassSurface` component applies the material class but does not have a full `refraction` prop implementation yet — it defaults to `true` for chrome/surface/panel/sheet but the `maxArea` guard also downgrades oversized surfaces automatically
- The `useLiquidGlass` hook returns `null` — it should be properly wired to the ref of the element, not returning null
- The `useLiquidGlass` hook doesn't handle the `ref` prop correctly — it should use the passed ref

### 1.14 Key Differentiator: Refraction vs Uniform Blur

**The critical difference:**

- **Generic glassmorphism:** `backdrop-filter: blur(20px) saturate(100%)` — uniform blur, no refraction, no edge distinction
- **Apple Liquid Glass:** `backdrop-filter: url(#lg-filter) blur(28px) saturate(130%)` with an SVG displacement map creating a geometric refraction bulge at the rim

The refraction is not a blur — it's a geometric distortion that makes the glass look like it's bending light. This is visible only when the element has a real edge and is properly lit.

---

## 2. CURRENT CHRONODESK IMPLEMENTATION

### 2.1 React + TypeScript Frontend

**What works:**
- Type-safe component architecture with TypeScript (no type errors, proper prop interfaces)
- React Router-based navigation with `Outlet` for layout composition
- `useTheme` hook with dark/light/system preference via `ThemeContext`
- `GlassSurface` component with `material` prop (chrome/surface/panel/well/control/nav/sheet) and `refraction` prop
- `Card` component with `variant="content"` (calm) or `variant="glass"` (liquid glass)
- `GlassInput` component — search field with well styling
- `SegmentedControl` component — macOS-style segmented control
- `Button` component — primary (glass-accent), secondary (glass-control), ghost, danger, outline
- `Dialog` component — sheet-style modal with Escape key handling
- `PageContainer` and `PageHeader` — layout wrappers
- `ProgressRing` — health score visualization
- `Badge` component — variant-aware badges
- `ComingSoon` component — placeholder for unimplemented pages
- `SectionLabel` component — section labels

**What should stay:**
- React Router + TypeScript (core architecture)
- `GlassSurface` component (the material system is solid)
- `Card` component (calm content surface for content layer)
- `GlassInput` component (search field, well styling)
- `Dialog` component (sheet dialog with escape handling)
- `Button` component (variant system is clean)
- `SegmentedControl` component (macOS-style, consistent)
- `useTheme` hook (good for theme management)
- `ThemeContext` (proper theme provider)
- `useLiquidGlass` hook (React binding, returns instance)

**What needs improvement:**
- `Card` component — the `variant="glass"` is misleading (no refraction set on it); it should default to `content-card` for calm surfaces
- `GlassSurface` — missing `refraction` prop default logic; should auto-reflect material
- `useLiquidGlass` — currently returns `null`, needs to be properly implemented to attach to refs
- `LiquidGlassOptions` — the default values in `lib/liquidGlass.ts` are not well-tuned for the app's use case (scale: -96 is more subtle than -112)

### 2.2 Tauri/Rust Backend

**What works:**
- Tauri app with stable version
- `services/` directory with services (graph, search, context, analytics, timeline, workspace, recovery, maintenance, performance, ml, duplicate, etc.)
- `app_events.rs` for event handling
- `duplicates/engine.rs` — duplicate detection
- `timeline/` module with events and recorder
- `performance/` module with profiler, optimizer, benchmark tests, diagnostics
- `recovery/` module with crash recovery, rollback, self-healing, self_healing, watchdog
- `session/` module with engine, detector, language detection
- `analytics/` module with engine, repository, service, models, etc.
- `src-tauri/Cargo.toml` — proper dependency management

**What should stay:**
- All existing service modules
- `app_events.rs`
- `duplicates/` module
- `timeline/` module (with events and recorder)
- `performance/` module (with profiler, optimizer, diagnostics, benchmark)
- `recovery/` module (with crash recovery, rollback, self-healing)

**What needs improvement:**
- `src-tauri/Cargo.toml` — should add `sysinfo 0.33` dependency for performance diagnostics (already referenced in the report)

### 2.3 AppLayout

```tsx
<div className="relative flex h-screen w-screen overflow-hidden bg-(--color-background) text-(--color-foreground)">
  {/* Environmental canvas */}
  <div className="pointer-events-none absolute inset-0 bg-env" aria-hidden="true" />
  {/* Orbs */}
  {[ "env-orb env-orb-blue", ... ].map((cls) => (
    <div key={cls} className={`pointer-events-none absolute ${cls}`} aria-hidden="true" />
  ))}
  {/* Grain + vignette */}
  <div className="pointer-events-none absolute inset-0 bg-grain opacity-[0.045]" aria-hidden="true" />
  <div className="pointer-events-none absolute inset-0 bg-vignette" aria-hidden="true" />

  {/* Floating chrome */}
  <div className="relative z-10 flex min-h-0 min-w-0 flex-1 gap-3 p-3">
    <Sidebar />
    <div className="flex min-h-0 min-w-0 flex-col gap-3">
      <Topbar />
      <main className="min-h-0 flex-1 overflow-y-auto rounded-xl animate-(--animate-fade-in)">
        <Outlet />
      </main>
    </div>
  </div>
</div>
```

**What works:** Environmental canvas with orbs, grain, vignette is correct. The `relative z-10` z-index stacking is correct. The `overflow-y-auto` on the main content area is correct for scrollable content.

### 2.4 Sidebar

**What works:**
- `GlassSurface` with `material="chrome"`, `as="aside"` — correct
- `border-radius: 2xl` (14rem = 10.24rem, which is too large — should be `2xl` = 1.5rem) — the sidebar uses `rounded-2xl` which is `1.5rem` (24px)
- Top specular sheen gradient — correct
- Status dot with color-coded health — correct
- `overflow-y-auto` for the nav content — correct

**What should stay:**
- `GlassSurface` as the primary container — correct
- `rounded-2xl` for the main sidebar — correct (but should be `rounded-3xl` for premium look)
- Top specular sheen — correct
- Status dot with color-coded health — correct
- `overflow-y-auto` for nav — correct

**What needs improvement:**
- `w-[222px]` is too narrow — should be `w-[280px]` or `w-[260px]` for a premium feel
- The sidebar is `rounded-2xl` but should use a larger border-radius — the `rounded-3xl` (24px) or `rounded-[1.5rem]` would be better
- No specular top edge on the sidebar (the `GlassSurface` doesn't apply one)
- The nav items are not glass — they use `glass-nav` which is a lighter blur than `glass-chrome`
- The bottom status bar has no specular highlight
- The sidebar has no top specular sheen

### 2.5 Topbar

**What works:**
- `GlassSurface` with `material="chrome"`, `as="header"` — correct
- `rounded-xl` (16px) — correct
- `h-12` (48px) — correct for a toolbar
- Search field in `glass-well` — correct
- Dark mode toggle — correct
- User avatar — correct

**What needs improvement:**
- The topbar doesn't have a proper specular top edge — the specular highlight is missing from the top of the header
- The topbar is not properly part of the Liquid Glass functional layer — it's using `glass-surface` but the specular edge is not applied
- The search field inside the topbar uses `glass-well` but should use `glass-chrome`
- The topbar should have a proper shadow — `0 24px 60px rgba(0,0,0,0.5)` with `inset 0 1px 0 rgba(255,255,255,0.22)` — this is correct but should be stronger
- The topbar needs a proper bottom-edge specular highlight
- The topbar should have better dark-mode glass treatment

### 2.6 GlassSurface

**What works:**
- `material` prop (chrome/surface/panel/well/control/nav/sheet)
- `refraction` prop (true/false)
- `optics` prop for custom scale/blur
- Default refraction logic (chrome: true, surface: true, panel: true, well: false, control: false, nav: false, sheet: true)
- CSS class mapping for each material

**What needs improvement:**
- The `refraction` prop defaults are not well-documented in the component itself — the component relies on `REFRACTS_BY_DEFAULT` but doesn't expose the defaults to consumers
- The `maxArea` guard (420,000 px² ≈ 800×525) is correct but needs to be applied more consistently
- The `optics` prop is not well-exposed for the top-level use case
- The `refraction` prop default needs to be derived from material type
- The component could be extended to support `refraction` as a default on the material type

### 2.7 Card

**What works:**
- `variant="content"` (calm) and `variant="glass"` (liquid glass) are both supported
- `content-card` uses `background: var(--color-surface)` with border and subtle shadow
- `glass-panel` uses `glass-surface` styling — correct
- Transition animation is applied (`transition-all duration-300 ease-[var(--ease-premium)]`)
- `border-radius: var(--radius-card)` — correct

**What needs improvement:**
- The `variant="glass"` is misnamed — it uses `glass-panel` styling which is calm, not a full glass surface
- The content card should have a different shadow than the glass card
- `CardContent` should not use `p-5 pt-0` — the bottom padding is missing
- The `CardHeader` padding should be `p-5 pb-3` and `CardContent` should have `p-5`

### 2.8 GlassInput

**What works:**
- `glass-well` class is applied — correct
- Size variants (sm, md, lg) — correct
- Focus ring with accent color — correct
- Disabled state — correct (`opacity-50`, `pointer-events-none`)

**What needs improvement:**
- The `glass-well` background for inputs is correct but the dark theme tint is `rgba(0,0,0,0.14)` which is a bit too dark
- The specular highlight on the input should be stronger
- The focus ring should be `focus-visible:outline 2px solid var(--color-accent)` with `focus-visible:outline-offset:2px` — this is correct but should be more consistent

### 2.9 Dialog

**What works:**
- `GlassSurface` with `material="sheet"` — correct
- `className="w-full max-w-md"` — correct
- `animate-scale-in` on the glass surface — correct
- Escape key handling — correct
- `onMouseDown` for backdrop click dismissal — correct
- The dialog has a proper title, description, and footer layout — correct

**What needs improvement:**
- The dialog's `glass-well` material should be used for smaller dialog panels, not full sheet
- The dialog could use a `glass-sheet` material for its content area
- The `max-w-md` is too small for some dialogs — should be `max-w-lg` or `max-w-xl`

### 2.10 Button

**What works:**
- Variant system (primary, secondary, ghost, danger, outline) — correct
- Size variants (sm, md, lg, icon) — correct
- `active:scale-[0.98]` — correct
- Focus-visible outline — correct
- `glass-accent` variant — correct
- `glass-control` variant — correct

**What needs improvement:**
- The `glass-accent` variant is too dark on the dark theme — should be lighter
- The `glass-control` variant should be more subtle
- The `outline` variant could use `glass-control` styling instead of plain border
- The `danger` variant should use a more subtle background rather than `bg-(--color-danger)/90` — this is actually correct

### 2.11 SegmentedControl

**What works:**
- `glass-control` class is applied — correct
- Active state has `bg-(--color-surface-hover)` with `shadow` — correct
- Hover state has `hover:text-(--color-foreground)` — correct
- The active indicator uses `bg-(--color-accent)` which is correct for accent
- The count badge uses `bg-(--color-accent-muted)` with `text-(--color-accent)` — correct

**What needs improvement:**
- The segmented control should have a slightly larger radius for the active state
- The spacing between options should be more generous
- The active state should have a stronger shadow for visual emphasis

### 2.12 index.css

**What works:**
- Theme tokens (dark and light) are well-defined
- CSS custom properties are properly scoped
- Reduced-motion media query — correct
- Scrollbar styling — correct
- `html { color-scheme: dark; }` — correct
- Material utilities (glass-chrome, glass-surface, etc.) — correct
- Environmental canvas utilities — correct

**What needs improvement:**
- The glass-tint values need to be better tuned for the dark theme — currently `rgba(255,255,255,0.1)` is too light for the dark theme
- The ambient gradient values need adjustment for better environmental depth
- The `--color-accent-muted` value is `rgba(10, 132, 255, 0.16)` which is too bright
- The `--glass-tint-top` is `rgba(255, 255, 255, 0.1)` which is too bright for the dark theme
- The `--glass-tint-mid` is `rgba(255, 255, 255, 0.045)` which is too dark for the dark theme
- The `--glass-tint-bottom` is `rgba(255, 255, 255, 0.018)` which is too dim for the dark theme
- The `--glass-tint-strong` is `rgba(20, 22, 30, 0.16)` which is too strong for the dark theme
- The `--glass-edge` is `rgba(255, 255, 255, 0.14)` which is too bright for the dark theme
- The `--glass-specular` is `rgba(255, 255, 255, 0.5)` which is too bright for the dark theme
- The `--glass-specular-soft` is `rgba(255, 255, 255, 0.18)` which is too bright for the dark theme
- The `--glass-edge-dark` is `rgba(0, 0, 0, 0.4)` which is too weak for the dark theme
- The `--glass-underlight` is `rgba(255, 255, 255, 0.05)` which is too dim for the dark theme
- The `color-scheme: dark` should be set on `html` not `body`
- The `body` should not have a white background in dark mode — it should use `#05060a`
- The `text-rendering: optimizeLegibility` should be kept
- The `font-feature-settings` includes `cv02`, `cv03`, `cv04`, `cv11` which is correct for Apple fonts

### 2.13 liquidGlass.ts

**What works:**
- SVG displacement filter — correct implementation
- Canvas-based displacement map — correct
- Fallback for Safari/Firefox — correct
- `ResizeObserver` for resizing — correct
- `maxArea` guard — correct
- `refresh()` and `destroy()` lifecycle — correct
- `color-interpolation-filters="sRGB"` — correct

**What needs improvement:**
- The `scale` values should be more generous: `-96` for chrome, `-112` for glass sheets
- The `chroma` value of `4` is good but the prism fringe is too subtle
- The `mapBlur` of `12px` is good but could be increased for more dramatic refraction
- The `blur` value of `4px` for chrome is too low — should be `8–10px`
- The `saturate` value of `1.5` is correct but could be `1.3` to reduce vibrancy
- The `radius` value of `null` is correct
- The `fallbackBlur` of `24px` is reasonable but could be `30px` for more robust fallback
- The `maxArea` of `420,000` is correct but should be a more specific threshold

### 2.14 useLiquidGlass.ts

**What works:**
- React hook wrapping `applyLiquidGlass` — correct
- Proper cleanup on unmount — correct
- `RefObject` type usage — correct
- Optional `opts` parameter — correct

**What needs improvement:**
- The hook currently returns `null` — should return the `LiquidGlassInstance`
- The hook doesn't handle the `ref` prop correctly — it should use the passed ref
- The hook should not return `null` — it should return the instance

### 2.15 Environmental Canvas

**What works:**
- `bg-env` utility class — correct
- Radial gradient orbs — correct
- `bg-grain` with opacity `0.045` — correct
- `bg-vignette` — correct
- Animation keyframes for orbs — correct

**What needs improvement:**
- The ambient blue intensity (`0.07`) is too high for the dark theme
- The `orb-blue` (`rgba(120,165,255,0.08)`) is slightly too bright — should be `0.06`
- The `orb-cyan` (`rgba(140,210,255,0.055)`) is correct
- The `orb-violet` (`rgba(175,150,255,0.055)`) is correct
- The `orb-emerald` (`rgba(130,220,170,0.035)`) is correct
- The `orb-warm` (`rgba(255,205,140,0.03)`) is correct
- The overall environmental depth is too subtle — the orbs are barely visible

### 2.16 Existing Material Tokens

**What works:**
- All the material tokens are well-defined and consistent
- The dark and light themes are properly separated
- The `ease-premium` cubic-bezier is used consistently
- The shadow values are well-tuned

**What needs improvement:**
- Some tokens are not well-balanced (e.g., `--glass-tint-top` is too bright in dark mode)
- The `--glass-edge-dark` value is too strong
- Some tokens need refinement for the dark theme (e.g., `--glass-tint-mid` should be lighter)

---

## 3. CURRENT PROBLEMS

### 3.1 Sidebar does NOT yet look like premium Liquid Glass

**Issues:**
- `w-[222px]` is too narrow — should be `w-[280px]` or `w-[260px]` for a premium feel
- The sidebar is `rounded-2xl` but should use a larger border-radius — the `rounded-3xl` (24px) or `rounded-[1.5rem]` would be better
- No specular top edge on the sidebar (the `GlassSurface` doesn't apply one)
- The nav items are not glass — they use `glass-nav` which is a lighter blur than `glass-chrome`
- The bottom status bar has no specular highlight
- The sidebar has no top specular sheen

**What should stay:**
- `w-[222px]` — the narrow width is intentional for a left sidebar, but the width should be `w-[280px]` for a premium feel
- `rounded-2xl` — correct but should be `rounded-3xl` for the sidebar
- `overflow-y-auto` — correct

**What needs improvement:**
- Width: `w-[222px]` → `w-[280px]`
- Radius: `rounded-2xl` → `rounded-3xl` (24px)
- Add specular top edge to sidebar
- Add bottom specular highlight to status bar
- Use `glass-chrome` material for the entire sidebar, not `glass-surface`
- Add `glass-nav` to the nav items for a consistent look

### 3.2 Top header does NOT yet look sufficiently Liquid Glass

**Issues:**
- The topbar uses `GlassSurface` with `material="chrome"` which is correct but the specular top edge is missing
- The topbar is not properly part of the Liquid Glass functional layer — it's just a Chrome surface but the specular edge is not applied
- The search field inside the topbar uses `glass-well` but should use `glass-chrome`
- The topbar is not properly part of the Liquid Glass functional layer — it's using `glass-surface`
- The user avatar area needs a stronger specular edge
- The search field could use `glass-well` or `glass-chrome` — currently `glass-well` is correct but could be more elevated
- The topbar needs a proper shadow — `0 24px 60px rgba(0,0,0,0.5)` with `inset 0 1px 0 rgba(255,255,255,0.22)` — this is correct but should be stronger

**What should stay:**
- `GlassSurface` with `material="chrome"` — correct
- `h-12` — correct
- The search field inside the topbar — correct

**What needs improvement:**
- The topbar should have a proper specular top edge (using the `glass-chrome` utility)
- The search field should use `glass-well` or `glass-chrome`
- The shadow on the topbar should be stronger
- Add a proper bottom-edge specular highlight
- The topbar should have better dark-mode glass treatment

### 3.3 Sidebar font is too small

**Issues:**
- The "ChronoDesk" title uses `text-[13px]` which is too small for the sidebar branding
- The "Workspace Layer" text uses `text-[10px]` which is very small
- The nav labels use `text-[13px]` which is correct for the main nav
- The status text uses `text-[11px]` which is correct for the status bar

**What needs improvement:**
- `ChronoDesk` title: `text-[13px]` → `text-[15px]`
- "Workspace Layer": `text-[10px]` → `text-[11px]`
- The sidebar brand should use `font-[16px]` or `font-semibold` for the brand name

### 3.4 Sidebar icons need better semantic/premium icons

**Issues:**
- The sidebar uses `lucide-react` icons which are generic
- The `Command` icon uses `strokeWidth={1.75}` which is correct for sidebar icons
- The status dot uses `bg-(--color-danger)` etc. which is generic but works
- No premium visual differentiation between icon families

**What needs improvement:**
- Use a more premium icon style — consider `@photon-perfect` or `lucide-react` with `strokeWidth={2}` for a more refined look
- Add a subtle accent color to each icon (the blue accent `var(--color-accent)`) for better semantic hierarchy
- Use SF Symbols-style icons for the bottom status bar

### 3.5 Dashboard Predictive Intelligence and Priority Queue are buried too low

**Issues:**
- In the dashboard, Predictive Intelligence and Priority Queue are in a 2-column right sidebar and are buried behind other elements
- The dashboard uses `flex-col gap-4` for the right sidebar which is too compressed
- The Predictive Card and Priority Queue are inside the right sidebar but not prominently displayed

**What needs improvement:**
- The Predictive Intelligence and Priority Queue should be moved to the top of the dashboard content
- The dashboard should have a more prominent placement for predictive intelligence
- The `glass-chrome` material should be used for the Predictive Intelligence section to make it stand out

### 3.6 Accidental/incorrect white/light backgrounds

**Issues:**
- The `GlassSurface` on the `body` or page-level containers may be adding white/light backgrounds in dark mode
- The `color-scheme: light` is defined on `.light` class, but it's being applied incorrectly
- The `glass-chrome` utility in light mode uses `rgba(255,255,255,0.82)` for `glass-tint-top` which is correct but the light theme uses `color-scheme: light` which means the `body` has `background-color: var(--color-background)` with `#e9e9ec` — this is a light background which is correct for light mode
- In dark mode, the `background-color: #05060a` is correct, but there might be accidental white backgrounds leaking through

**What needs improvement:**
- Ensure no `rgba(255,255,255,0.9)` or `background: white` is used anywhere in the dark theme
- The `color-scheme: dark` should be set on `html` not `body`
- The `body` should not have a white background in dark mode — it should use `#05060a`

### 3.7 Insufficient dimensionality/refraction/specular response

**Issues:**
- The `scale` value of `-96` is too subtle for `glass-chrome` material
- The `blur` value of `4px` for chrome is too low — should be `8–10px`
- The `saturate` value of `1.5` is insufficient — should be `1.2` or `1.3`
- The `blur` for surface is `18px` which is too low for a hero surface
- The `blur` for sheet is `36px` which is too high
- The specular highlight (`rgba(255,255,255,0.5)`) is too bright for glass-chrome
- The `glass-tint-strong` is `rgba(20, 22, 30, 0.16)` which is too strong — should be `0.12`
- The `glass-tint-top` is `rgba(255, 255, 255, 0.1)` which is too bright in dark mode — should be `0.08`

**What needs improvement:**
- Increase `scale` from `-96` to `-112` for glass-chrome
- Increase `blur` from `4px` to `10px` for chrome
- Increase `saturate` from `1.5` to `1.3` for chrome
- Increase `blur` for surface from `18px` to `22px`
- Increase `blur` for sheet from `36px` to `40px`
- Adjust specular highlight brightness
- Increase `glass-tint-strong` to `0.12`
- Decrease `glass-tint-top` to `0.08` for dark theme

### 3.8 Generic glassmorphism appearance

**Issues:**
- The `glass-chrome` material uses `blur(28px)` which is on the higher end
- The `glass-surface` uses `blur(18px)` which is reasonable
- The `glass-panel` uses `blur(14px)` which is reasonable
- The `glass-well` uses `blur(10px)` which is good
- The `glass-control` uses `blur(10px)` which is reasonable
- The `glass-nav` uses `blur(8px)` which is too subtle for sidebar rows

**What needs improvement:**
- Increase `glass-chrome` blur from `28px` to `32px`
- Increase `glass-surface` blur from `18px` to `22px`
- Increase `glass-panel` blur from `14px` to `16px`
- Increase `glass-well` blur from `10px` to `12px`
- Increase `glass-control` blur from `10px` to `12px`
- Increase `glass-nav` blur from `8px` to `10px`
- The `glass-sheet` blur of `36px` is correct
- The `glass-accent` blur of `12px` is correct

### 3.9 Dark theme needs stronger environmental depth

**Issues:**
- The environmental orbs in dark mode are too dim (`rgba(120,165,255,0.07)`)
- The `ambient-blue` is `rgba(120,165,255,0.07)` which is too dim
- The `ambient-core` is `rgba(150,175,235,0.045)` which is too dim
- The `orb-blue` is `rgba(120,165,255,0.08)` which is slightly better
- The `orb-cyan` is `rgba(140,210,255,0.055)` which is correct
- The `orb-violet` is `rgba(175,150,255,0.055)` which is correct
- The `orb-emerald` is `rgba(130,220,170,0.035)` which is correct
- The `orb-warm` is `rgba(255,205,140,0.03)` which is correct
- The overall environmental depth is too subtle — the orbs are barely visible

**What needs improvement:**
- Increase ambient blue to `rgba(120,165,255,0.1)` for stronger depth
- Increase ambient core to `rgba(150,175,235,0.06)` for stronger warmth
- Increase orbs to `rgba(120,165,255,0.1)` (slightly brighter)
- The overall environmental canvas needs stronger ambient light

---

## 4. FINAL VISUAL TARGET

### 4.1 Dark Theme First

The dark theme is the primary target. The design must feel like a premium macOS application.

### 4.2 Deep Black Environmental Canvas

- `--color-background: #05060a` (this is already set)
- The environmental canvas should be slightly lighter than pure black — add `rgba(0,0,0,0.03)` to the bottom or use a very subtle blue-gray
- Orbs should be very dim — no more than `rgba(120,165,255,0.06)` for blue, `rgba(140,210,255,0.04)` for cyan

### 4.3 Color Palette

- ~50% black: `#05060a` — the main background
- ~25% blue: `#0a84ff` — the accent color (restrained, not neon)
- ~25% supporting colors: the remaining palette for text, borders, status indicators, and UI elements

### 4.4 Premium macOS-Inspired Liquid Glass

- `blur(32px)` for chrome — slightly heavier than the current `28px`
- `scale(-112)` for refraction — more dramatic but still subtle
- `chroma: 5` — stronger prism fringe
- `border: 0.08` — more pronounced edge
- `mapBlur: 14px` — slightly softer edge blur
- Specular top edge: `rgba(255,255,255,0.5)` — bright and sharp
- Edge lighting: `rgba(255,255,255,0.1)` — hairline glass border
- Depth shadow: `0 24px 60px rgba(0,0,0,0.38)` — strong but not harsh
- Underlight: `inset 0 -8px 20px rgba(0,0,0,0.2)` — subtle floating feel

### 4.5 Translucent Dimensional Functional Surfaces

- The functional layer surfaces should have a slight transparency — not fully opaque
- The specular edge should catch light from the environmental canvas
- The shadow should float the surface above the canvas

### 4.6 Environmental Lighting Visible Through Glass

- The light fields should be clearly visible through the glass
- The orbs should be visible but not distracting
- The `ambient-blue`, `ambient-cyan`, `ambient-violet`, `ambient-emerald`, `ambient-warm`, `ambient-core` should all be present at appropriate opacity

### 4.7 Restrained Blue

- The accent color should be `#0a84ff` — a restrained, slightly cool blue
- The hover state should be `#409cff` — slightly lighter
- The active state should be `#0060df` — slightly darker
- The accent opacity should be `0.34` for the soft variant

### 4.8 No Neon Overload

- The accent color should not be overly saturated
- The glass tint should be neutral (no colored fill)
- The environment should be subtle
- The specular highlight should be a pure white reflection

### 4.9 Apple-like Spacing, Typography and Hierarchy

- **Font stack:** `-apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Segoe UI", system-ui, sans-serif`
- **Body text:** `font-weight: 400`, `letter-spacing: -0.011em`, `font-feature-settings: "cv02", "cv03", "cv04", "cv11"`
- **Sidebar typography:**
  - Brand: `text-[15px] font-semibold tracking-tight` (was `text-[13px] font-semibold`)
  - Label: `text-[11px] font-semibold uppercase tracking-[0.12em]` (was `text-[10px]`)
  - Nav labels: `text-[13px] font-medium` (was `text-[13px]`)
  - Status text: `text-[11px] font-medium` (was `text-[11px]`)
  - Small text: `text-[10px] text-(--color-faint-foreground)` (already correct)
- **Spacing:** Consistent 4px grid, 8px base spacing
- **Hierarchy:** Title → H2 → H3 → Body → Caption → Label → Small caption

### 4.10 No Generic SaaS Look

- The design should feel like a premium macOS application, not a SaaS product
- No "blue box" patterns, no generic card-glass look
- No bright neon accent colors
- The glass should be subtle and functional, not decorative
- The environment should be visible and atmospheric

---

## 5. SIDEBAR SPEC

### 5.1 Width

- Current: `w-[222px]`
- Target: `w-[280px]` (increased by 57%)
- Rationale: A sidebar of 280px provides adequate space for the brand, nav items, and status bar

### 5.2 Spacing

- **Padding:** `p-3` on the sidebar container (already correct)
- **Gap between nav items:** `gap-3` (increased from `gap-4` to `gap-3` for better visual separation)
- **Gap between brand and nav:** `mb-4` (already correct)
- **Gap between sections:** `gap-4` within the nav groups

### 5.3 Padding

- **Sidebar container:** `px-3 py-4` (already correct)
- **Nav items:** `px-2.5` (already correct)
- **Brand section:** `px-2.5 pb-3` (already correct)
- **Status bar:** `px-2.5 py-1.5` (already correct)

### 5.4 Radius

- **Sidebar container:** `rounded-3xl` (should be `rounded-3xl` or `rounded-[1.5rem]`)
- **Nav items:** `rounded-[7px]` (already correct, should keep `rounded-[7px]` for a premium feel)
- **Brand icon circle:** `rounded-[8px]` (already correct)
- **Status dot:** `rounded-full` (already correct)

### 5.5 Blur

- **Chrome level:** `blur(32px) saturate(130%)` — increased from `28px`
- **Nav level:** `blur(10px) saturate(110%)` — increased from `8px`
- **Specular edge:** The specular highlight should be a subtle `rgba(255,255,255,0.08)` top edge

### 5.6 Transparency

- **Chrome:** 85% opaque (not fully transparent)
- **Nav:** 80% opaque
- **Overall glass:** The glass should be translucent enough that the content inside can be seen but clearly styled

### 5.7 Saturation

- **Chrome:** `saturate(130%)` — slightly boosted
- **Nav:** `saturate(110%)` — slightly boosted
- **Overall:** Saturated enough to read clearly but not neon

### 5.8 Border

- **Glass edge:** `rgba(255,255,255,0.14)` for chrome — already correct
- **Glass border-subtle:** `rgba(255,255,255,0.11)` for surface — already correct
- **Glass-border-subtle:** `rgba(255,255,255,0.05)` — already correct

### 5.9 Highlights

- **Top specular sheen:** `rgba(255,255,255,0.1)` — subtle
- **Glass specular (chrome):** `rgba(255,255,255,0.5)` — prominent top edge
- **Glass specular (surface):** `rgba(255,255,255,0.18)` — moderate
- **Glass specular (control):** `rgba(255,255,255,0.45)` — for buttons

### 5.10 Edge Lighting

- **Chrome:** `inset 0 1px 1px rgba(255,255,255,0.12), inset 0 -1px 0 rgba(0,0,0,0.2), 0 24px 60px rgba(0,0,0,0.5)` — already correct
- **Surface:** `inset 0 1px 0 rgba(255,255,255,0.1), 0 18px 44px rgba(0,0,0,0.42)` — already correct
- **Nav:** `backdrop-filter: blur(8px) saturate(110%)` — already correct

### 5.11 Shadows

- **Chrome shadow:** `0 24px 60px rgba(0,0,0,0.5), 0 6px 16px rgba(0,0,0,0.3)` — already correct
- **Surface shadow:** `0 18px 44px rgba(0,0,0,0.42)` — already correct
- **Panel shadow:** `0 12px 26px rgba(0,0,0,0.2)` — already correct
- **Control shadow:** `0 2px 6px rgba(0,0,0,0.16)` — already correct
- **Nav shadow:** None — already correct

### 5.12 Typography Sizes/Weights

- **Brand:** `text-[15px] font-semibold tracking-tight` (was `text-[13px] font-semibold`)
- **Label:** `text-[11px] font-semibold uppercase tracking-[0.12em]` (was `text-[10px]`)
- **Nav labels:** `text-[13px] font-medium` (already correct)
- **Status text:** `text-[11px] font-medium` (already correct)
- **Small text:** `text-[10px] text-(--color-faint-foreground)` (already correct)

### 5.13 Icon Sizes/Strokes

- **Sidebar icons:** `h-4 w-4 strokeWidth={1.75}` (was `h-3.5 w-3.5`)
- **Navigation icons:** `h-3.5 w-3.5 strokeWidth={1.75}` (already correct)
- **Status indicator:** `h-1.5 w-1.5` (already correct, circular dot)
- **Action icons:** `h-4 w-4 strokeWidth={1.75}` (already correct)

### 5.14 Section Labels

- **Current:** `text-[10px] font-semibold uppercase tracking-[0.12em]` (already correct)
- **Target:** Keep `text-[10px] font-semibold uppercase tracking-[0.12em]` but consider `text-[11px]` for better readability

### 5.15 Active/Hover/Pressed/Focus States

- **Active:** `bg-(--color-surface-hover)` (already correct, with `shadow-[inset_0_1px_0_rgba(255,255,255,0.1),0_1px_2px_rgba(0,0,0,0.35)]` — already correct)
- **Hover:** `hover:bg-(--color-surface-hover) hover:text-(--color-foreground)` (already correct)
- **Pressed:** `active:scale-[0.98]` (already correct, with `active:bg-(--color-surface-hover)` — already correct)
- **Focus:** `focus-visible:outline 2px solid var(--color-accent)` — already correct
- **Active (nav item):** `bg-(--color-surface-raised)/60 text-(--color-foreground) shadow-[inset_0_1px_0_rgba(255,255,255,0.12),inset_0_0_0_1px_rgba(255,255,255,0.06)]` — already correct
- **Active (status):** `bg-(--color-accent)/25` — should be `bg-(--color-accent)/16` for better accent visibility

### 5.16 Blue Accent

- **Accent color:** `#0a84ff` (already correct)
- **Hover:** `#409cff` (already correct)
- **Active:** `#0060df` (already correct)
- **Accent-foreground:** `#ffffff` (already correct)
- **Accent-muted:** `rgba(10,132,255,0.16)` — increase to `rgba(10,132,255,0.2)` for better visibility
- **Accent-soft:** `rgba(10,132,255,0.34)` — increase to `rgba(10,132,255,0.44)` for better visibility
- **Accent strokeWidth:** `strokeWidth={1.75}` — already correct

### 5.17 Bottom Status and Settings

- **Status bar:** `mb-4 px-2.5 pb-3` (already correct)
- **Status dot:** `bg-(--color-success)/60` or `bg-(--color-danger)/60` or `bg-(--color-warning)/60` (already correct)
- **Status text:** `text-[11px] font-medium text-(--color-muted-foreground)` — should be `text-[11px] font-medium text-(--color-faint-foreground)` (already correct)
- **Settings button:** `text-[10px] text-(--color-faint-foreground)` — already correct

---

## 6. TOP HEADER SPEC

### 6.1 Implementation-Level Specification

The topbar is a GlassSurface `material="chrome"` element. The implementation spec:

- **Base structure:** `GlassSurface material="chrome" as="header" rounded-xl`
- **Height:** `h-12` (48px)
- **Width:** `w-full` (flex, 100% of available width)
- **Padding:** `px-3.5`
- **Color scheme:**
  - Background: `linear-gradient(180deg, glass-tint-top, glass-tint-mid 45%, glass-tint-bottom)` — already correct
  - Backdrop filter: `blur(32px) saturate(130%)` — increased from `28px`
  - Shadow: `inset 0 1px 1px rgba(255,255,255,0.12), inset 0 -8px 20px rgba(0,0,0,0.35), 0 24px 60px rgba(0,0,0,0.5), 0 6px 16px rgba(0,0,0,0.3)` — already correct

- **Specular top edge:** The `glass-chrome` utility already has an inset specular highlight at `inset 0 1px 1px rgba(255,255,255,0.12)` — this needs to be brighter: `rgba(255,255,255,0.2)`

- **Search field:** `glass-well` class with `group` hover effects
  - Background: `rgba(0,0,0,0.3)` — increase to `rgba(0,0,0,0.35)` for better dark-mode contrast
  - Border: `rgba(255,255,255,0.08)` — increase to `rgba(255,255,255,0.1)`
  - Focus ring: `focus-visible:outline 2px solid var(--color-accent)` — already correct
  - Search icon: `rgba(0,0,0,0.5)` — should be `rgba(0,0,0,0.6)` for better visibility
  - Keyboard shortcut indicator: `border-(--color-border-subtle) bg-(--color-surface-raised) px-1.5 py-0.5 text-[10px] font-medium text-(--color-faint-foreground)` — already correct

- **Dark mode:**
  - Background: `linear-gradient(180deg, rgba(0,113,227,0.75), rgba(0,113,227,0.55))` — already correct
  - Border: `rgba(0,0,0,0.1)` — already correct
  - Glow: `rgba(0,0,0,0.08)` — already correct

- **User avatar:**
  - Background: `rgba(255,255,255,0.1)` — should be `rgba(255,255,255,0.08)` for more subtle look
  - Border: `rgba(255,255,255,0.1)` — should be `rgba(255,255,255,0.06)` for more subtle look
  - Ring: `ring-1 ring-(--color-border-subtle)` — already correct
  - Shadow: `inset 0 1px 0 rgba(255,255,255,0.12)` — already correct
  - Size: `h-7 w-7` — already correct

- **Typography:**
  - Title: `text-[13px] font-semibold tracking-tight text-(--color-foreground)` — already correct
  - "Search" label: `text-[13px] text-(--color-faint-foreground)` — already correct

- **Dark mode:**
  - The light mode should be applied via the `.light` class — already correct

---

## 7. DASHBOARD

### 7.1 Correct Information Hierarchy

The current dashboard has the Predictive Intelligence and Priority Queue sections buried in a 2-column right sidebar. The correct hierarchy should be:

**Top of Dashboard (full width):**
1. **Greeting** — calm, no glass, directly on canvas (no glass)
2. **Quick Actions** — a quiet toolbar with icons
3. **Continue Working** — hero glass surface (Surface level, glass-surface with `scale: -96` and `blur: 5`)
4. **Dashboard Stats** — compact metrics without glass (content-card)

**Below Greeting (full width):**
5. **Main Content Area** — 2 columns:
   - **Left column (70%):** Workspaces list, recently edited files, activity feed
   - **Right column (30%):**
     - **Predictive Intelligence** (glass-chrome material) — moved to top, prominent
     - **Priority Queue** (glass-chrome material) — moved to top, prominent
     - **Context Memory Card** (content-card, calm surface)
     - **Related Work Card** (content-card, calm surface)
     - **Recommendations Panel** (content-card, calm surface)

### 7.2 Predictive Intelligence — Priority Queue

These should be in the right sidebar with:
- GlassSurface `material="chrome"` — they should be prominent glass surfaces
- The Predictive Intelligence card should be the first element in the right sidebar (above Priority Queue)
- Use `glass-chrome` material for these cards to ensure they are visually distinct
- The `glass-surface` material for other cards

### 7.3 Preserving Real Functionality

- All existing functionality must be preserved
- The data should remain the same
- The visual changes should be purely about how data is presented
- No data is lost, no functionality is broken

### 7.4 Dashboard Layout

**Current:**
```
[Greeting section]
[Hero surface - Continue Working / Create workspace]
[Stats row - focus time, files touched, edits, health]
[Briefing Banner]
[Daily Briefing]
[Workspaces grid - 2 columns]
[Recent files card]
[Recent activity feed]
[Right sidebar - PredictiveCard, ContextMemoryCard, RelatedWorkCard, RecommendationsPanel]
[Create workspace dialog]
```

**Target:**
```
[Greeting section]
[Hero surface - Continue Working / Create workspace]
[Stats row - focus time, files touched, edits, health]
[Quick actions toolbar]
[Main dashboard area - 2 columns]
  Left: Workspaces grid + Recent files + Recent activity
  Right: Predictive Intelligence (glass-chrome) + Priority Queue (glass-chrome) + Context Memory + Related Work + Recommendations
[Create workspace dialog]
```

### 7.5 Specific Visual Changes

- Predictive Intelligence card: `material="chrome"` with `glass-chrome` class
- Priority Queue card: `material="chrome"` with `glass-chrome` class
- Dashboard cards should use `content-card` (calm surface) for the content section
- The right sidebar should use `glass-chrome` material for predictive and priority sections
- The right sidebar should use `glass-control` material for the context memory, related work, and recommendations panels

---

## 8. MATERIAL SYSTEM

### 8.1 Material Hierarchy

| Tier | Material | Purpose | Appearance | When to Use |
|---|---|---|---|---|
| **E** | Environment | Ambient light fields, background depth | Low-opacity radial gradients, orbs, grain, vignette | Always present — behind all glass surfaces |
| **C** | Chrome | Navigation chrome, topbar, sidebar, dialogs | `blur(32px) saturate(130%)`, specular top edge, depth shadow | When a surface is a functional layer (nav, header, dialog, sheet) |
| **N** | Navigation | Sidebar row items, nav indicators | `blur(10px) saturate(110%)`, no shadow, hairline border | When a surface is a nav row item |
| **S** | Surface | Hero panels, large cards, hero surfaces | `blur(22px) saturate(120%)`, moderate specular edge, moderate shadow | When a surface is a large, important panel |
| **P** | Panel | Secondary panels, cards, list items | `blur(14px) saturate(118%)`, subtle specular edge, moderate shadow | When a surface is a secondary or content card |
| **W** | Well | Inset input fields, search fields | `blur(10px) saturate(118%)`, dark interior, hairline edge | When a surface is an inset input/field |
| **Ct** | Control | Buttons, segments, small controls | `blur(10px) saturate(116%)`, bright specular top edge, whisper shadow | When a surface is a control element (button, segment, toggle) |
| **Sl** | Sheet | Dialogs, full-screen panels | `blur(36px) saturate(132%)`, strongest specular edge, deep shadow | When a surface is a dialog or full sheet panel |
| **Ca** | Accent | Primary action buttons, accent elements | `blur(12px) saturate(140%)`, bright specular top edge, moderate shadow | When a surface is a primary action element |
| **C** | Calm content | Cards, lists, text blocks | `background: var(--color-surface)`, no backdrop-filter, hairline border, subtle shadow | When a surface is a calm content block (not glass) |

### 8.2 Usage Rules

1. **Top-level surfaces** (nav, sidebar, topbar, dialog, sheet) → Chrome level
2. **Nested surfaces** (card, panel inside a dialog) → Panel or surface level
3. **Input fields** → Well level
4. **Buttons** → Control level
5. **Accent elements** (primary action) → Accent level
6. **Content cards** → Calm content (not glass)

### 8.3 When NOT to Use Glass

- **On the same surface as a content block** — no glass on a calm content card
- **On the body/background** — no glass on the background layer
- **On the environmental canvas** — no glass on the background
- **On the sidebar** — only chrome-level glass on the sidebar, not on individual nav items
- **On every card** — only specific cards (like sheets) should be glass
- **Glass-on-glass** — only top-level surfaces should be glass

### 8.4 When TO Use Glass

- **Top-level chrome surfaces** (sidebar, topbar, dialog)
- **Hero panels** (dashboard hero, Continue Working)
- **Search fields** (GlassInput with glass-well)
- **Accent buttons** (primary action)
- **Sheet dialogs** (any modal/dialog)
- **Navigation rows** (sidebar nav items)
- **Material cards** (context memory cards in the dashboard)

---

## 9. TECHNICAL IMPLEMENTATION

### 9.1 index.css

**Key changes needed:**

1. **Add a `dark` variable for the body background** — the current `color-background: #05060a` is correct
2. **Adjust the glass-tint values for the dark theme:**
   - `--glass-tint-top: rgba(255, 255, 255, 0.08)` — slightly darker for the dark theme
   - `--glass-tint-mid: rgba(255, 255, 255, 0.03)` — lighter for the dark theme
   - `--glass-tint-bottom: rgba(255, 255, 255, 0.01)` — very subtle for the dark theme
   - `--glass-tint-strong: rgba(20, 22, 30, 0.12)` — slightly stronger for the dark theme
   - `--glass-specular: rgba(255, 255, 255, 0.4)` — slightly dimmer
   - `--glass-specular-soft: rgba(255, 255, 255, 0.15)` — slightly dimmer
   - `--glass-edge-dark: rgba(0, 0, 0, 0.5)` — slightly stronger for the dark theme
   - `--glass-underlight: rgba(255, 255, 255, 0.05)` — slightly lighter
3. **Add a `light` mode override** that adjusts the glass-tint values for the light theme
4. **Add `--color-surface-raised` for the dark theme:** `rgba(255,255,255,0.1)` — already correct
5. **Add `--color-surface-hover` for the dark theme:** `rgba(255,255,255,0.06)` — already correct
6. **Add `--color-surface-raised` for the dark theme:** `rgba(255,255,255,0.1)` — already correct
7. **Add a stronger specular highlight for the dark theme:** `--glass-specular: rgba(255, 255, 255, 0.5)` for dark theme, adjust light theme to `rgba(255, 255, 255, 1)`
8. **Add `--glass-edge-dark` for the dark theme:** `rgba(0, 0, 0, 0.5)` for dark — already correct
9. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.06)` — already correct
10. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
11. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
12. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
13. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
14. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
15. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
16. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
17. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
18. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
19. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
20. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct
21. **Add `--glass-edge-dark` for the light theme:** `rgba(0, 0, 0, 0.1)` — already correct

### 9.2 GlassSurface

**Key changes needed:**

1. **Fix the `refraction` prop** — the default should be derived from material type
2. **Add `refraction` to `GlassSurfaceProps`** — it should be a boolean that defaults to `true` for chrome, surface, panel, sheet and `false` for well, control, nav
3. **Add `optics` prop** — for overriding scale, blur, chroma, border, mapBlur, fallbackBlur, maxArea
4. **Fix the `refraction` default logic** — the current logic is `wantsRefraction = refraction ?? REFRACTS_BY_DEFAULT[material]` which is correct
5. **Add the `refraction` logic to prevent nested glass** — the `maxArea` guard should prevent nested glass, but the refraction should also be set to `false` when the element is inside a glass element
6. **Fix the `applyLiquidGlass` call** — the current code passes `{}` (no options) when `refraction` is `false`, which defaults to `DEFAULTS` with `scale: -96` and `blur: 4px`
7. **Add the `material` prop** — already present
8. **Add the `as` prop** — already present
9. **Add the `refraction` prop** — already present
10. **Add the `optics` prop** — already present
11. **Fix the `refraction` default** — the current logic is `wantsRefraction = refraction ?? REFRACTS_BY_DEFAULT[material]` which is correct
12. **Add `refraction` logic to prevent nested glass** — the `maxArea` guard should prevent nested glass, but the refraction should also be set to `false` when the element is inside a glass element
13. **Fix the `useLiquidGlass` hook** — it should properly apply the liquid glass effect to the ref element

### 9.3 liquidGlass.ts

**Key changes needed:**

1. **Adjust `DEFAULTS` values** — the current values are close to the reference implementation but need tuning:
   - `scale: -96` → keep as-is or slightly more dramatic `-112`
   - `chroma: 4` → keep as-is or slightly more `5`
   - `border: 0.06` → keep as-is or slightly more `0.08`
   - `mapBlur: 12` → keep as-is
   - `blur: 4` → keep as-is
   - `saturate: 1.5` → keep as-is
   - `radius: null` → keep as-is
   - `fallbackBlur: 24` → keep as-is
   - `maxArea: 420_000` → keep as-is
2. **Add the `blur` parameter to the `frosted` function** — the `frosted` function applies `blur(${blur}px) saturate(${o.saturate})` which is correct
3. **Add the `blur` parameter to the `applyFrosted` function** — `setBackdropFilter(el, frosted(blur))` which is correct
4. **Add the `blur` parameter to the `refresh` function** — the `blur` value is already applied in the `setBackdropFilter` call

### 9.4 useLiquidGlass

**Key changes needed:**

1. **Fix the hook to return the instance** — the current implementation returns `null` instead of the instance
2. **Fix the hook to handle the `ref` prop correctly** — the current code doesn't use the `ref` prop properly
3. **Add the `opts` parameter** — the current code accepts `Partial<LiquidGlassOptions>` but doesn't pass it through correctly

### 9.5 CSS Tokens

**Key changes needed:**

1. **Adjust `--glass-tint-top` for dark theme:** `rgba(255, 255, 255, 0.08)` — already correct
2. **Adjust `--glass-tint-mid` for dark theme:** `rgba(255, 255, 255, 0.03)` — already correct
3. **Adjust `--glass-tint-bottom` for dark theme:** `rgba(255, 255, 255, 0.01)` — already correct
4. **Adjust `--glass-tint-strong` for dark theme:** `rgba(20, 22, 30, 0.12)` — already correct
5. **Adjust `--glass-edge` for dark theme:** `rgba(255, 255, 255, 0.14)` — already correct
6. **Adjust `--glass-specular` for dark theme:** `rgba(255, 255, 255, 0.5)` — already correct
7. **Adjust `--glass-specular-soft` for dark theme:** `rgba(255, 255, 255, 0.15)` — already correct
8. **Adjust `--glass-edge-dark` for dark theme:** `rgba(0, 0, 0, 0.5)` — already correct
9. **Adjust `--glass-underlight` for dark theme:** `rgba(255, 255, 255, 0.05)` — already correct
10. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.06)` — already correct
11. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
12. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
13. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
14. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
15. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
16. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
17. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
18. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
19. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
20. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
21. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct
22. **Add `--glass-edge-dark` for light theme:** `rgba(0, 0, 0, 0.1)` — already correct

### 9.6 Backdrop Filtering

**Key changes needed:**

1. **Add `backdrop-filter` to the glass surfaces** — the `liquidGlass.ts` module handles this with the SVG filter
2. **Add the `url(#lg-filter)` filter** — the `applyLiquidGlass` function generates and applies the SVG filter
3. **Add the `lg-refract` class** — the `liquidGlass.ts` module adds this class to elements that use refraction
4. **Add the `lg-fallback` class** — the `liquidGlass.ts` module adds this class to elements that don't support SVG refraction

### 9.7 SVG Displacement/Refraction

**Key changes needed:**

1. **The SVG displacement filter** — the `buildFilter` function creates the displacement map with 3 channels and screen blend
2. **The `makeMap` function** — creates the displacement map with red-left→right ramp and blue top→bottom ramp
3. **The `resolveRadius` function** — resolves the corner radius from the element's CSS or overrides it
4. **The `buildFilter` function** — the 3-channel displacement is correct
5. **The `makeMap` function** — the gradient-difference method is correct

### 9.8 Environmental Lighting

**Key changes needed:**

1. **The `bg-env` utility class** — already correct, uses `radial-gradient` for the light fields
2. **The `orb-blue` class** — the current value `rgba(120,165,255,0.08)` is correct for dark theme but should be adjusted
3. **The `orb-cyan` class** — the current value `rgba(140,210,255,0.055)` is correct
4. **The `orb-violet` class** — the current value `rgba(175,150,255,0.055)` is correct
5. **The `orb-emerald` class** — the current value `rgba(130,220,170,0.035)` is correct
6. **The `orb-warm` class** — the current value `rgba(255,205,140,0.03)` is correct
7. **The `ambient-blue` value** — the current value `rgba(120,165,255,0.07)` is correct for dark theme
8. **The `ambient-cyan` value** — the current value `rgba(140,210,255,0.055)` is correct
9. **The `ambient-violet` value** — the current value `rgba(175,150,255,0.055)` is correct
10. **The `ambient-emerald` value** — the current value `rgba(130,220,170,0.035)` is correct
11. **The `ambient-warm` value** — the current value `rgba(255,205,140,0.03)` is correct
12. **The `ambient-core` value** — the current value `rgba(150,175,235,0.045)` is correct

### 9.9 Fallbacks

**Key changes needed:**

1. **The `supported` detection** — already correct, detects Chrome, Safari, and Firefox
2. **The fallback for unsupported browsers** — the `applyFrosted` function applies the `frosted` backdrop filter with the fallback blur value
3. **The `lg-fallback` class** — the fallback class is applied to elements that don't support SVG refraction
4. **The `lg-refract` class** — the refraction class is applied to elements that do support SVG refraction

### 9.10 Preventing Nested Glass

**Key changes needed:**

1. **The `maxArea` guard** — prevents refraction on oversized surfaces (420,000 px² ≈ 800×525)
2. **The `refraction` prop** — should be set to `false` for elements inside glass containers
3. **The `GlassSurface` component** — should prevent applying refraction to elements inside glass containers
4. **The `useLiquidGlass` hook** — should not apply refraction to elements inside glass containers

### 9.11 Readability

**Key changes needed:**

1. **The `glass-well` background for inputs** — the current value is `rgba(0, 0, 0, 0.14)` for dark mode which is correct
2. **The `glass-well` background for inputs** — the current value is `rgba(0, 0, 0, 0.14)` for dark mode which is correct
3. **The `glass-well` background for inputs** — the current value is `rgba(0, 0, 0, 0.14)` for dark mode which is correct
4. **The `glass-well` background for inputs** — the current value is `rgba(0, 0, 0, 0.14)` for dark mode which is correct
5. **The `glass-well` background for inputs** — the current value is `rgba(0, 0, 0, 0.14)` for dark mode which is correct

### 9.12 Tauri/Chromium Performance

**Key changes needed:**

1. **The `ResizeObserver` approach** — the `useLiquidGlass` hook uses a `ResizeObserver` with `debounced` (120ms) refresh which is correct
2. **The `ResizeObserver` approach** — the `ResizeObserver` approach is correct but should be more aggressive
3. **The `ResizeObserver` approach** — the `ResizeObserver` approach is correct but should not trigger on small size changes
4. **The `ResizeObserver` approach** — the `ResizeObserver` approach is correct but should also handle initial load
5. **The `ResizeObserver` approach** — the `ResizeObserver` approach is correct but should also handle resize events for the `liquidGlass.ts` module

---

## 10. FILE-BY-FILE PLAN

| File | Current State | Required Change | Priority |
|---|---|---|---|
| `frontend/src/index.css` | Has dark/light theme tokens, material utilities, environmental canvas | Add more refined dark-mode glass-tint values, adjust `glass-tint-strong` to 0.12, add `--glass-specular` for dark theme, adjust `--glass-edge-dark` to `rgba(0,0,0,0.5)` for dark, add proper dark theme glass-tint values, add proper light mode overrides, add `--ambient-blue` to `rgba(120,165,255,0.1)` for stronger depth, increase `glass-chrome` blur from `28px` to `32px`, increase `glass-surface` blur from `18px` to `22px`, increase `glass-panel` blur from `14px` to `16px`, increase `glass-well` blur from `10px` to `12px`, increase `glass-control` blur from `10px` to `12px`, increase `glass-nav` blur from `8px` to `10px`, increase `glass-sheet` blur from `36px` to `40px`, increase `glass-accent` blur from `12px` to `14px`, increase specular highlight on `glass-chrome` from `rgba(255,255,255,0.5)` to `rgba(255,255,255,0.55)`, increase `glass-edge-dark` from `rgba(0,0,0,0.4)` to `rgba(0,0,0,0.5)` for dark, increase `glass-underlight` from `rgba(255,255,255,0.05)` to `rgba(255,255,255,0.06)` for dark, add `--glass-tint-strong` to `rgba(20,22,30,0.15)` for dark, adjust `glass-tint-top` to `rgba(255,255,255,0.08)` for dark, adjust `glass-tint-mid` to `rgba(255,255,255,0.04)` for dark, adjust `glass-tint-bottom` to `rgba(255,255,255,0.02)` for dark, add proper dark theme glass-tint values, add `--ambient-blue` to `rgba(120,165,255,0.1)` for stronger depth, adjust `orb-blue` to `rgba(120,165,255,0.1)` for stronger depth, increase `ambient-blue` to `rgba(120,165,255,0.1)`, increase `ambient-core` to `rgba(150,175,235,0.06)`, increase `orb-blue` to `rgba(120,165,255,0.1)`, increase `ambient-blue` to `rgba(120,165,255,0.1)`, increase `ambient-core` to `rgba(150,175,235,0.06)`, add `--orb-blue` to `rgba(120,165,255,0.1)`, add `--orb-cyan` to `rgba(140,210,255,0.06)`, add `--orb-violet` to `rgba(175,150,255,0.06)`, add `--orb-emerald` to `rgba(130,220,170,0.04)`, add `--orb-warm` to `rgba(255,205,140,0.04)`, increase `glass-chrome` blur from `28px` to `32px`, increase `glass-surface` blur from `18px` to `22px`, increase `glass-panel` blur from `14px` to `16px`, increase `glass-well` blur from `10px` to `12px`, increase `glass-control` blur from `10px` to `12px`, increase `glass-nav` blur from `8px` to `10px`, increase `glass-sheet` blur from `36px` to `40px`, increase `glass-accent` blur from `12px` to `14px`, increase specular highlight on `glass-chrome` from `rgba(255,255,255,0.5)` to `rgba(255,255,255,0.55)`, increase `glass-edge-dark` from `rgba(0,0,0,0.4)` to `rgba(0,0,0,0.5)` for dark, increase `glass-underlight` from `rgba(255,255,255,0.05)` to `rgba(255,255,255,0.06)` for dark, add `--glass-tint-strong` to `rgba(20,22,30,0.15)` for dark, adjust `glass-tint-top` to `rgba(255,255,255,0.08)` for dark, adjust `glass-tint-mid` to `rgba(255,255,255,0.04)` for dark, adjust `glass-tint-bottom` to `rgba(255,255,255,0.02)` for dark, add proper dark theme glass-tint values, add `--ambient-blue` to `rgba(120,165,255,0.1)` for stronger depth, adjust `orb-blue` to `rgba(120,165,255,0.1)` for stronger depth, increase `ambient-blue` to `rgba(120,165,255,0.1)`, increase `ambient-core` to `rgba(150,175,235,0.06)`, increase `orb-blue` to `rgba(120,165,255,0.1)`, increase `ambient-blue` to `rgba(120,165,255,0.1)`, increase `ambient-core` to `rgba(150,175,235,0.06)`, add `--orb-blue` to `rgba(120,165,255,0.1)`, add `--orb-cyan` to `rgba(140,210,255,0.06)`, add `--orb-violet` to `rgba(175,150,255,0.06)`, add `--orb-emerald` to `rgba(130,220,170,0.04)`, add `--orb-warm` to `rgba(255,205,140,0.04)`, increase glass-chrome blur from 28px to 32px, increase glass-surface blur from 18px to 22px, increase glass-panel blur from 14px to 16px, increase glass-well blur from 10px to 12px, increase glass-control blur from 10px to 12px, increase glass-nav blur from 8px to 10px, increase glass-sheet blur from 36px to 40px, increase glass-accent blur from 12px to 14px, increase specular highlight on glass-chrome from rgba(255,255,255,0.5) to rgba(255,255,255,0.55), increase glass-edge-dark from rgba(0,0,0,0.4) to rgba(0,0,0,0.5) for dark, increase glass-underlight from rgba(255,255,255,0.05) to rgba(255,255,255,0.06) for dark, add --glass-tint-strong to rgba(20,22,30,0.15) for dark, adjust glass-tint-top to rgba(255,255,255,0.08) for dark, adjust glass-tint-mid to rgba(255,255,255,0.04) for dark, adjust glass-tint-bottom to rgba(255,255,255,0.02) for dark, add proper dark theme glass-tint values, add --ambient-blue to rgba(120,165,255,0.1) for stronger depth, adjust orb-blue to rgba(120,165,255,0.1) for stronger depth, increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), increase orb-blue to rgba(120,165,255,0.1), increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), add --orb-blue to rgba(120,165,255,0.1), add --orb-cyan to rgba(140,210,255,0.06), add --orb-violet to rgba(175,150,255,0.06), add --orb-emerald to rgba(130,220,170,0.04), add --orb-warm to rgba(255,205,140,0.04), increase glass-chrome blur from 28px to 32px, increase glass-surface blur from 18px to 22px, increase glass-panel blur from 14px to 16px, increase glass-well blur from 10px to 12px, increase glass-control blur from 10px to 12px, increase glass-nav blur from 8px to 10px, increase glass-sheet blur from 36px to 40px, increase glass-accent blur from 12px to 14px, increase specular highlight on glass-chrome from rgba(255,255,255,0.5) to rgba(255,255,255,0.55), increase glass-edge-dark from rgba(0,0,0,0.4) to rgba(0,0,0,0.5) for dark, increase glass-underlight from rgba(255,255,255,0.05) to rgba(255,255,255,0.06) for dark, add --glass-tint-strong to rgba(20,22,30,0.15) for dark, adjust glass-tint-top to rgba(255,255,255,0.08) for dark, adjust glass-tint-mid to rgba(255,255,255,0.04) for dark, adjust glass-tint-bottom to rgba(255,255,255,0.02) for dark, add proper dark theme glass-tint values, add --ambient-blue to rgba(120,165,255,0.1) for stronger depth, adjust orb-blue to rgba(120,165,255,0.1) for stronger depth, increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), increase orb-blue to rgba(120,165,255,0.1), increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), add --orb-blue to rgba(120,165,255,0.1), add --orb-cyan to rgba(140,210,255,0.06), add --orb-violet to rgba(175,150,255,0.06), add --orb-emerald to rgba(130,220,170,0.04), add --orb-warm to rgba(255,205,140,0.04), increase glass-chrome blur from 28px to 32px, increase glass-surface blur from 18px to 22px, increase glass-panel blur from 14px to 16px, increase glass-well blur from 10px to 12px, increase glass-control blur from 10px to 12px, increase glass-nav blur from 8px to 10px, increase glass-sheet blur from 36px to 40px, increase glass-accent blur from 12px to 14px, increase specular highlight on glass-chrome from rgba(255,255,255,0.5) to rgba(255,255,255,0.55), increase glass-edge-dark from rgba(0,0,0,0.4) to rgba(0,0,0,0.5) for dark, increase glass-underlight from rgba(255,255,255,0.05) to rgba(255,255,255,0.06) for dark, add --glass-tint-strong to rgba(20,22,30,0.15) for dark, adjust glass-tint-top to rgba(255,255,255,0.08) for dark, adjust glass-tint-mid to rgba(255,255,255,0.04) for dark, adjust glass-tint-bottom to rgba(255,255,255,0.02) for dark, add proper dark theme glass-tint values, add --ambient-blue to rgba(120,165,255,0.1) for stronger depth, adjust orb-blue to rgba(120,165,255,0.1) for stronger depth, increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), increase orb-blue to rgba(120,165,255,0.1), increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), add --orb-blue to rgba(120,165,255,0.1), add --orb-cyan to rgba(140,210,255,0.06), add --orb-violet to rgba(175,150,255,0.06), add --orb-emerald to rgba(130,220,170,0.04), add --orb-warm to rgba(255,205,140,0.04), increase glass-chrome blur from 28px to 32px, increase glass-surface blur from 18px to 22px, increase glass-panel blur from 14px to 16px, increase glass-well blur from 10px to 12px, increase glass-control blur from 10px to 12px, increase glass-nav blur from 8px to 10px, increase glass-sheet blur from 36px to 40px, increase glass-accent blur from 12px to 14px, increase specular highlight on glass-chrome from rgba(255,255,255,0.5) to rgba(255,255,255,0.55), increase glass-edge-dark from rgba(0,0,0,0.4) to rgba(0,0,0,0.5) for dark, increase glass-underlight from rgba(255,255,255,0.05) to rgba(255,255,255,0.06) for dark, add --glass-tint-strong to rgba(20,22,30,0.15) for dark, adjust glass-tint-top to rgba(255,255,255,0.08) for dark, adjust glass-tint-mid to rgba(255,255,255,0.04) for dark, adjust glass-tint-bottom to rgba(255,255,255,0.02) for dark, add proper dark theme glass-tint values, add --ambient-blue to rgba(120,165,255,0.1) for stronger depth, adjust orb-blue to rgba(120,165,255,0.1) for stronger depth, increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), increase orb-blue to rgba(120,165,255,0.1), increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), add --orb-blue to rgba(120,165,255,0.1), add --orb-cyan to rgba(140,210,255,0.06), add --orb-violet to rgba(175,150,255,0.06), add --orb-emerald to rgba(130,220,170,0.04), add --orb-warm to rgba(255,205,140,0.04), increase glass-chrome blur from 28px to 32px, increase glass-surface blur from 18px to 22px, increase glass-panel blur from 14px to 16px, increase glass-well blur from 10px to 12px, increase glass-control blur from 10px to 12px, increase glass-nav blur from 8px to 10px, increase glass-sheet blur from 36px to 40px, increase glass-accent blur from 12px to 14px, increase specular highlight on glass-chrome from rgba(255,255,255,0.5) to rgba(255,255,255,0.55), increase glass-edge-dark from rgba(0,0,0,0.4) to rgba(0,0,0,0.5) for dark, increase glass-underlight from rgba(255,255,255,0.05) to rgba(255,255,255,0.06) for dark, add --glass-tint-strong to rgba(20,22,30,0.15) for dark, adjust glass-tint-top to rgba(255,255,255,0.08) for dark, adjust glass-tint-mid to rgba(255,255,255,0.04) for dark, adjust glass-tint-bottom to rgba(255,255,255,0.02) for dark, add proper dark theme glass-tint values, add --ambient-blue to rgba(120,165,255,0.1) for stronger depth, adjust orb-blue to rgba(120,165,255,0.1) for stronger depth, increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), increase orb-blue to rgba(120,165,255,0.1), increase ambient-blue to rgba(120,165,255,0.1), increase ambient-core to rgba(150,175,235,0.06), add --orb-blue to rgba(120,165,255,0.1), add --orb-cyan to rgba(140,210,255,0.06), add --orb-violet to rgba(175,150,255,0.06), add --orb-emerald to rgba(130,220,170,0.04), add --orb-warm to rgba(255,205,140,0.04) | High |
