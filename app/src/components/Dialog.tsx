import { Portal } from "solid-js/web";
import { createEffect, createUniqueId, type JSX, onCleanup, Show } from "solid-js";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * In-app modal (never `confirm()`): labelled by its title, Escape closes. Focus moves to the
 * dialog itself, not its first button, so Enter can't trigger a destructive action by accident.
 * Tab is trapped inside while open, and focus returns to whatever opened it on close.
 * Rendered through a Portal into document.body so no ancestor stacking context (the sticky
 * sidebar, a positioned widget) can paint over its backdrop.
 */
export function Dialog(props: {
  title: string;
  open: boolean;
  onClose: () => void;
  children: JSX.Element;
  actions: JSX.Element;
}) {
  const id = createUniqueId();
  let box: HTMLDivElement | undefined;

  createEffect(() => {
    if (!props.open) return;
    const trigger = document.activeElement as HTMLElement | null;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") return props.onClose();
      if (e.key !== "Tab" || !box) return;
      const items = [...box.querySelectorAll<HTMLElement>(FOCUSABLE)];
      // Nothing to tab to (or focus escaped the box): keep it on the dialog.
      if (items.length === 0) return e.preventDefault(), box.focus();
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (e.shiftKey && (active === first || active === box)) (e.preventDefault(), last.focus());
      else if (!e.shiftKey && active === last) (e.preventDefault(), first.focus());
    };
    document.addEventListener("keydown", onKey);
    box?.focus();
    onCleanup(() => {
      document.removeEventListener("keydown", onKey);
      // Best-effort: the trigger may have unmounted (e.g. a deleted dashboard's own button).
      trigger?.focus?.();
    });
  });

  return (
    <Show when={props.open}>
      <Portal>
        <div class="dialog-backdrop">
          <div class="dialog" role="dialog" aria-modal="true" aria-labelledby={id} tabindex="-1" ref={box}>
            <h2 id={id}>{props.title}</h2>
            {props.children}
            <div class="row dialog-actions">{props.actions}</div>
          </div>
        </div>
      </Portal>
    </Show>
  );
}
