import { AnimatePresence, motion } from "motion/react";
import { Outlet, useLocation } from "react-router-dom";
import { springs } from "@/lib/springs";
import { useReducedMotion } from "@/utils/motion";

/**
 * Route transitions for the app shell: pages materialize into the glass
 * scene instead of hard-cutting. A gentle spring on opacity + a short
 * rise; exit mirrors the path. Under prefers-reduced-motion this becomes
 * a plain opacity cross-fade (no movement).
 */
export function PageTransition() {
  const location = useLocation();
  const reduced = useReducedMotion();

  return (
    <AnimatePresence mode="wait" initial={false}>
      <motion.div
        key={location.pathname}
        className="h-full min-h-0"
        initial={{ opacity: 0, y: reduced ? 0 : 8 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: reduced ? 0 : -4 }}
        transition={springs.gentle}
      >
        <Outlet />
      </motion.div>
    </AnimatePresence>
  );
}