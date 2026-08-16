import { useRef } from "react";
import { Sidebar } from "@/components/navigation/Sidebar";
import { Topbar } from "@/components/navigation/Topbar";
import { PageTransition } from "@/components/ui/PageTransition";

/**
 * The scene: a deep-space canvas with drifting light fields, floating
 * chrome (sidebar + toolbar), and the routed page content. Every glass
 * surface in the app frosts/refracts THIS background.
 *
 * Pages arrive through PageTransition (spring materialize on route
 * change), and the main scroll container drives the toolbar's scroll-edge
 * treatment via scrollRef.
 */
export function AppLayout() {
  const mainRef = useRef<HTMLElement | null>(null);

  return (
    <div className="relative flex h-screen w-screen overflow-hidden bg-(--color-background) text-(--color-foreground)">
      {/* environmental canvas */}

      <div className="pointer-events-none absolute inset-0 bg-env" aria-hidden="true" />
      {[
        "env-orb env-orb-blue",
        "env-orb env-orb-cyan",
        "env-orb env-orb-violet",
        "env-orb env-orb-emerald",
        "env-orb env-orb-warm",
      ].map((cls) => (
        <div key={cls} className={`pointer-events-none absolute ${cls}`} aria-hidden="true" />
      ))}
      <div className="pointer-events-none absolute inset-0 bg-grain opacity-[0.045]" aria-hidden="true" />
      <div className="pointer-events-none absolute inset-0 bg-vignette" aria-hidden="true" />

      {/* Floating chrome — the topbar spans the window's top edge (traffic
          lights float on the glass under the overlay titlebar); sidebar +
          content float beneath it over the canvas. */}
      <div className="relative z-10 flex h-full w-full min-h-0 min-w-0 flex-col">
        <Topbar scrollRef={mainRef} />
        <div className="flex min-h-0 min-w-0 flex-1 gap-3 p-3">
          <Sidebar />
          <main
            ref={mainRef}
            className="scroll-edge-fade min-h-0 flex-1 overflow-y-auto overscroll-contain rounded-xl"
          >
            <PageTransition />
          </main>
        </div>
      </div>
    </div>
  );
}