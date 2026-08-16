import type { Transition } from "motion";

/**
 * Fluid motion presets — Apple-style spring parameters, one vocabulary
 * for the whole application. Every interactive animation goes through
 * these; no ad-hoc transition values on glass elements.
 *
 * Springs are used only where interruptibility or physical feel actually
 * matters (sheets, menus, buttons, page transitions). Ambient or
 * decorative effects stay in CSS.
 */
export const springs = {
  /** Critically damped — default UI transitions, no overshoot. */
  default: { type: "spring", damping: 28, stiffness: 380, mass: 0.9 } satisfies Transition,

  /** Under-damped — momentum-driven interactions, slight bounce. */
  bounce: { type: "spring", damping: 18, stiffness: 320, mass: 0.85 } satisfies Transition,

  /** Gentle settle — large surfaces, minimal movement. */
  gentle: { type: "spring", damping: 32, stiffness: 240, mass: 1 } satisfies Transition,

  /** Snappy — small controls, instant response. */
  snap: { type: "spring", damping: 34, stiffness: 620, mass: 0.7 } satisfies Transition,

  /** Material — glass materialize/dematerialize (sheets, menus). */
  material: { type: "spring", damping: 26, stiffness: 300, mass: 0.95 } satisfies Transition,

  /** Overlay — scrim/backdrop fades (opacity only). */
  overlay: { type: "spring", damping: 30, stiffness: 400, mass: 1 } satisfies Transition,
} as const;

export type SpringPreset = keyof typeof springs;