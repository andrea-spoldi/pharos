import { createSignal } from "solid-js";

/**
 * Sidebar presentation: "open" shows icon + label, "rail" shows icons only. Per-device, so it
 * lives in localStorage, not the engine. With no saved choice it follows the window width
 * (see defaultMode); once the user toggles, their choice wins.
 */
export type SidebarMode = "open" | "rail";

export const SIDEBAR_KEY = "pharos.sidebar";

/** Windows at least this wide default to the open sidebar; narrower ones to the rail. */
const OPEN_MIN_WIDTH = 1280;

export function defaultMode(width: number): SidebarMode {
  return width >= OPEN_MIN_WIDTH ? "open" : "rail";
}

function read(): SidebarMode {
  try {
    const v = localStorage.getItem(SIDEBAR_KEY);
    if (v === "open" || v === "rail") return v;
  } catch {
    // Storage can be unavailable (private mode, blocked). Fall back to the width default.
  }
  return defaultMode(window.innerWidth);
}

const [sidebarMode, setSignal] = createSignal<SidebarMode>(read());

export { sidebarMode };

export function setSidebarMode(m: SidebarMode): void {
  setSignal(m);
  try {
    localStorage.setItem(SIDEBAR_KEY, m);
  } catch {
    // A non-persisted choice still applies for this session.
  }
}

export function toggleSidebar(): void {
  setSidebarMode(sidebarMode() === "open" ? "rail" : "open");
}
