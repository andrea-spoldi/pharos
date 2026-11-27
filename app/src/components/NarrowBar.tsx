import {
  createSignal,
  For,
  type JSX,
  onCleanup,
  onMount,
  Show,
} from "solid-js";
import type { DashboardView } from "../bindings";
import { NewDashboardButton } from "./DashboardEditor";
import { PharosMark } from "./PharosMark";

const label = (d: DashboardView) => d.metadata.title ?? d.metadata.name;

/**
 * Compact top bar below the sidebar breakpoint: the mark and the current dashboard's title as a
 * button that opens the dashboard menu (both groups, then Sync now and Settings). Escape,
 * a selection or a click outside close it; Escape and selection return focus to the button.
 * Owns the sync status live region while the sidebar is not rendered (always mounted).
 */
export function NarrowBar(props: {
  dashboards: DashboardView[];
  current?: string;
  landing?: string;
  onOpen: (id: string) => void;
  onCreated: (d: DashboardView) => void;
  /** Platform title, shown when no dashboard is current (Settings, or before the first sync). */
  title: string;
  status: JSX.Element;
  /** Absent while Sync now is not offered. */
  onSync?: () => void;
  syncing: boolean;
  onSettings: () => void;
  settingsCurrent?: boolean;
}) {
  const [open, setOpen] = createSignal(false);
  let root: HTMLDivElement | undefined;
  let trigger: HTMLButtonElement | undefined;

  const platform = () =>
    props.dashboards.filter((d) => d.origin === "platform");
  const mine = () => props.dashboards.filter((d) => d.origin === "user");
  const shown = () =>
    (props.current ?? props.landing) &&
    props.dashboards.find((d) => d.id === (props.current ?? props.landing));
  const heading = () =>
    props.settingsCurrent
      ? "Settings"
      : shown()
        ? label(shown() as DashboardView)
        : props.title;

  const close = (refocus: boolean) => {
    setOpen(false);
    if (refocus) trigger?.focus();
  };
  const choose = (action: () => void) => () => {
    close(true);
    action();
  };

  onMount(() => {
    const onDown = (e: MouseEvent) => {
      // The new-dashboard dialog is portalled out of the menu; clicks in it are not "outside".
      if (
        open() &&
        root &&
        e.target instanceof Node &&
        !root.contains(e.target) &&
        !(e.target instanceof Element && e.target.closest(".dialog-backdrop"))
      )
        close(false);
    };
    document.addEventListener("mousedown", onDown);
    onCleanup(() => document.removeEventListener("mousedown", onDown));
  });

  const item = (d: DashboardView) => (
    <li>
      <button
        type="button"
        class="menu-item"
        aria-current={d.id === props.current ? "page" : undefined}
        onClick={choose(() => props.onOpen(d.id))}
      >
        {label(d)}
      </button>
    </li>
  );

  return (
    <header class="narrow-bar">
      <div
        class="narrow-menu"
        ref={root}
        onKeyDown={(e) => {
          if (e.key === "Escape" && open()) {
            e.stopPropagation();
            close(true);
          }
        }}
      >
        <button
          type="button"
          class="narrow-trigger"
          ref={trigger}
          aria-expanded={open()}
          onClick={() => setOpen(!open())}
        >
          <PharosMark class="brand-mark" />
          <span>{heading()}</span>
          <span aria-hidden="true"> ▾</span>
        </button>
        <Show when={open()}>
          <div class="menu">
            <Show when={platform().length}>
              <h2 class="nav-heading">Platform</h2>
              <ul class="plain">
                <For each={platform()}>{item}</For>
              </ul>
            </Show>
            <h2 class="nav-heading">Mine</h2>
            <ul class="plain">
              <For each={mine()}>{item}</For>
              <li>
                <NewDashboardButton
                  class="menu-item"
                  onCreated={(d) => {
                    close(false);
                    props.onCreated(d);
                  }}
                />
              </li>
            </ul>
            <ul class="plain">
              <Show when={props.onSync}>
                {(sync) => (
                  <li>
                    <button
                      type="button"
                      class="menu-item"
                      disabled={props.syncing}
                      onClick={choose(sync())}
                    >
                      {props.syncing ? "Syncing…" : "Sync now"}
                    </button>
                  </li>
                )}
              </Show>
              <li>
                <button
                  type="button"
                  class="menu-item"
                  aria-current={props.settingsCurrent ? "page" : undefined}
                  onClick={choose(props.onSettings)}
                >
                  Settings
                </button>
              </li>
            </ul>
          </div>
        </Show>
      </div>
      {/* Always mounted: a live region only announces text that changes inside it. */}
      <p class="muted sync-status" role="status">
        {props.status}
      </p>
    </header>
  );
}
