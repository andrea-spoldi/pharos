import { createEffect, createUniqueId, type JSX, onCleanup, Show } from "solid-js";

/**
 * In-app modal (never `confirm()`): labelled by its title, Escape closes. Focus moves to the
 * dialog itself, not its first button, so Enter can't trigger a destructive action by accident.
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
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && props.onClose();
    document.addEventListener("keydown", onKey);
    onCleanup(() => document.removeEventListener("keydown", onKey));
    box?.focus();
  });

  return (
    <Show when={props.open}>
      <div class="dialog-backdrop">
        <div class="dialog" role="dialog" aria-modal="true" aria-labelledby={id} tabindex="-1" ref={box}>
          <h2 id={id}>{props.title}</h2>
          {props.children}
          <div class="row dialog-actions">{props.actions}</div>
        </div>
      </div>
    </Show>
  );
}
