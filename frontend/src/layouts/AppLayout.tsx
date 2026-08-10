import { Outlet } from "react-router-dom";
import { Sidebar } from "@/components/navigation/Sidebar";
import { Topbar } from "@/components/navigation/Topbar";

/**
 * The scene: a deep-space canvas with drifting light fields, floating
 * chrome (sidebar + toolbar), and the routed page content. Every glass
 * surface in the app frosts/refracts THIS background.
 */
export function AppLayout() {
  return (
    <div className="relative flex h-screen w-screen overflow-hidden bg-(--color-background) text-(--color-foreground)">
      {/* Level 0 — environmental canvas: light fields, orbs, grain, vignette. */}
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

      {/* Floating chrome — sidebar + toolbar hover above the canvas. */}
      <div className="relative z-10 flex min-h-0 min-w-0 flex-1 gap-3 p-3">
        <Sidebar />
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
          <Topbar />
          <main className="min-h-0 flex-1 overflow-y-auto rounded-xl animate-(--animate-fade-in)">
            <Outlet />
          </main>
        </div>
      </div>
    </div>
  );
}
