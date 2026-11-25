import { createSignal } from "solid-js";

/**
 * Appearance preference. "system" follows the OS (`prefers-color-scheme`); "light"/"dark"
 * force a theme regardless. Applied as `data-theme` on <html>, which the CSS overrides read.
 * Per-device, so it lives in localStorage, not the engine. Applied before first paint
 * (see initTheme) so there's no theme flash.
 */
export type Theme = "light" | "dark" | "system";

export const THEMES: Theme[] = ["light", "dark", "system"];
export const THEME_LABELS: Record<Theme, string> = { light: "Light", dark: "Dark", system: "System" };

const KEY = "pharos.theme";

function read(): Theme {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    // Storage can be unavailable (private mode, blocked). Fall back to following the OS.
  }
  return "system";
}

function apply(t: Theme): void {
  document.documentElement.dataset.theme = t;
}

const [theme, setSignal] = createSignal<Theme>(read());

/** Call once at startup, before render, so the first paint already has the chosen theme. */
export function initTheme(): void {
  apply(theme());
}

export { theme };

export function setTheme(t: Theme): void {
  setSignal(t);
  apply(t);
  try {
    localStorage.setItem(KEY, t);
  } catch {
    // A non-persisted choice still applies for this session.
  }
}
