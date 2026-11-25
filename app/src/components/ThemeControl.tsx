import { For } from "solid-js";
import { setTheme, theme, THEME_LABELS, THEMES } from "../theme";

/**
 * Light / Dark / System segmented control. Native radios in a labelled group, so arrow keys
 * move and screen readers announce the group; the <label>s are styled as the segments.
 */
export function ThemeControl() {
  return (
    <fieldset class="segmented" aria-label="Theme">
      <For each={THEMES}>
        {(t) => (
          <label classList={{ selected: theme() === t }}>
            <input type="radio" name="theme" value={t} checked={theme() === t} onChange={() => setTheme(t)} />
            {THEME_LABELS[t]}
          </label>
        )}
      </For>
    </fieldset>
  );
}
