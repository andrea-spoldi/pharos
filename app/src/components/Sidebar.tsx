import { For, type JSX, Show } from "solid-js";
import type { DashboardView } from "../bindings";
import { NewDashboardButton } from "./DashboardEditor";

const label = (d: DashboardView) => d.metadata.title ?? d.metadata.name;

/**
 * Left navigation: platform dashboards and the user's own, each in the engine's order (which
 * already puts `home` first among the platform ones). Items navigate through `onOpen`;
 * "+ New dashboard" closes *Mine* and hands the created dashboard to `onCreated`.
 */
export function Sidebar(props: {
  dashboards: DashboardView[];
  current?: string;
  landing?: string;
  onOpen: (id: string) => void;
  onCreated: (d: DashboardView) => void;
  children?: JSX.Element;
}) {
  const platform = () => props.dashboards.filter((d) => d.origin === "platform");
  const mine = () => props.dashboards.filter((d) => d.origin === "user");

  const item = (d: DashboardView) => (
    <li>
      <button
        type="button"
        class="nav-item"
        aria-current={d.id === props.current ? "page" : undefined}
        onClick={() => props.onOpen(d.id)}
      >
        <span class="nav-mono" aria-hidden="true">
          {label(d).charAt(0).toUpperCase()}
        </span>
        <span class="nav-label">{label(d)}</span>
        <Show when={props.dashboards.length > 1 && d.id === props.landing}>
          {/* The star is decorative; the sr-only text puts the cue in the item's accessible name. */}
          <span class="landing-mark" aria-hidden="true" title="Landing dashboard">
            ★
          </span>
          <span class="sr-only">{" (Landing dashboard)"}</span>
        </Show>
      </button>
    </li>
  );

  return (
    <aside class="sidebar">
      <Show when={platform().length}>
        <nav class="nav-group" aria-label="Platform dashboards">
          <h2 class="nav-heading">Platform</h2>
          <ul class="plain">
            <For each={platform()}>{item}</For>
          </ul>
        </nav>
      </Show>
      <nav class="nav-group" aria-label="Your dashboards">
        <h2 class="nav-heading">Mine</h2>
        <ul class="plain">
          <For each={mine()}>{item}</For>
          <li>
            <NewDashboardButton class="nav-item" onCreated={props.onCreated} />
          </li>
        </ul>
      </nav>
      {props.children}
    </aside>
  );
}
