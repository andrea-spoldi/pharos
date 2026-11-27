import { createSignal } from "solid-js";

/**
 * Viewport width state shared by the shell and the dashboard drag guard. At or below this width
 * the sidebar gives way to the compact top bar and the dashboard grid collapses to one column.
 */
export const NARROW_BREAKPOINT = 900;

const isNarrow = () => typeof window !== "undefined" && window.innerWidth <= NARROW_BREAKPOINT;

const [narrow, setNarrow] = createSignal(isNarrow());

// One module-level listener for the app's lifetime.
if (typeof window !== "undefined") {
  window.addEventListener("resize", () => setNarrow(isNarrow()));
}

export { narrow };
