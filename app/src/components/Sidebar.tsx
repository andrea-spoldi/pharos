import { createEffect, createSignal, For, type JSX, onCleanup, onMount, Show } from "solid-js";
import { Portal } from "solid-js/web";
import type { DashboardView, RepoSource } from "../bindings";
import { NewDashboardButton } from "./DashboardEditor";
import { sidebarMode, toggleSidebar } from "../sidebar";
import { PharosMark } from "./PharosMark";
import { shortCommit } from "./SyncReview";

const label = (d: DashboardView) => d.metadata.title ?? d.metadata.name;

/** `owner/repo` from the last two path segments of a git URL, `.git` stripped; the whole URL if it has fewer. */
export function repoName(url: string): string {
  const parts = url
    .replace(/\/+$/, "")
    .replace(/\.git$/, "")
    .split(/[/:]/)
    .filter(Boolean);
  return parts.length >= 2 ? parts.slice(-2).join("/") : url;
}

/** The shortcut must not steal `\` from a text field or fire behind an open dialog. */
function shortcutBlocked(e: KeyboardEvent): boolean {
  const t = e.target;
  if (t instanceof Element && t.closest("input, textarea, select, [contenteditable]")) return true;
  return document.querySelector('dialog[open], [role="dialog"]') !== null;
}

/**
 * Left navigation: platform dashboards and the user's own, each in the engine's order (which
 * already puts `home` first among the platform ones). Items navigate through `onOpen`;
 * "+ New dashboard" closes *Mine* and hands the created dashboard to `onCreated`.
 * The brand and source sit above; the sync status (always mounted, it is a live region), Sync now
 * and Settings sit in the footer. Before the first sync there are no dashboards to list.
 * In rail mode only icons are visible: every label stays in the accessibility tree (`.sr-only`)
 * and a tooltip names the item on hover and keyboard focus. Cmd/Ctrl+\ toggles rail and open.
 */
export function Sidebar(props: {
  dashboards: DashboardView[];
  current?: string;
  landing?: string;
  onOpen: (id: string) => void;
  onCreated: (d: DashboardView) => void;
  title: string;
  source: RepoSource;
  /** Sync progress or result; Home owns the text, the live region lives here. */
  status: JSX.Element;
  /** Absent while Sync now is not offered (never synced, or a plan is waiting for review). */
  onSync?: () => void;
  syncing: boolean;
  onSettings: () => void;
  settingsCurrent: boolean;
  neverSynced?: boolean;
  children?: JSX.Element;
}) {
  const platform = () => props.dashboards.filter((d) => d.origin === "platform");
  const mine = () => props.dashboards.filter((d) => d.origin === "user");
  const rail = () => sidebarMode() === "rail";

  // One fixed-position tooltip for the whole rail: the column scrolls, so a positioned child would clip.
  const [tip, setTip] = createSignal<{ text: string; top: number; left: number }>();
  const showTip = (e: Event) => {
    if (!rail()) return;
    const el = e.target instanceof Element ? e.target.closest(".nav-item, .sync-icon, .sidebar-toggle") : null;
    if (!el) return setTip(undefined);
    const text =
      el.getAttribute("aria-label") ?? el.querySelector(".nav-label")?.textContent ?? el.textContent ?? "";
    const r = el.getBoundingClientRect();
    setTip({ text: text.trim(), top: r.top + r.height / 2, left: r.right + 8 });
  };

  // A mode change (the shortcut) while hovering must not leave a stale tooltip behind.
  createEffect(() => {
    if (!rail()) setTip(undefined);
  });

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "\\" && !shortcutBlocked(e)) {
        e.preventDefault();
        toggleSidebar();
      }
    };
    document.addEventListener("keydown", onKey);
    onCleanup(() => document.removeEventListener("keydown", onKey));
  });

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
        <span class="nav-label" classList={{ "sr-only": rail() }}>
          {label(d)}
        </span>
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
    <aside
      class="sidebar"
      classList={{ rail: rail() }}
      onMouseOver={showTip}
      onFocusIn={showTip}
      onMouseLeave={() => setTip(undefined)}
      onFocusOut={() => setTip(undefined)}
    >
      <div class="sidebar-top">
        <p class="brand" data-testid="brand">
          <PharosMark class="brand-mark" />
          <span classList={{ "sr-only": rail() }}>{props.title}</span>
        </p>
        <p class="muted nav-source" classList={{ "sr-only": rail() }} data-testid="source" title={props.source.url}>
          <span aria-hidden="true">
            {repoName(props.source.url)} · {props.source.branch}
          </span>
          <span class="sr-only">
            {props.source.url} · {props.source.branch}
          </span>
        </p>
        <button
          type="button"
          class="sidebar-toggle"
          aria-expanded={!rail()}
          aria-label={rail() ? "Expand sidebar" : "Collapse sidebar"}
          onClick={toggleSidebar}
        >
          <span aria-hidden="true">{rail() ? "»" : "«"}</span>
        </button>
      </div>
      <Show when={!props.neverSynced}>
        <Show when={platform().length}>
          <nav class="nav-group" aria-label="Platform dashboards">
            <h2 class="nav-heading" classList={{ "sr-only": rail() }}>
              Platform
            </h2>
            <ul class="plain">
              <For each={platform()}>{item}</For>
            </ul>
          </nav>
        </Show>
        <nav class="nav-group" aria-label="Your dashboards">
          <h2 class="nav-heading" classList={{ "sr-only": rail() }}>
            Mine
          </h2>
          <ul class="plain">
            <For each={mine()}>{item}</For>
            <li>
              <NewDashboardButton class="nav-item" onCreated={props.onCreated} />
            </li>
          </ul>
        </nav>
      </Show>
      {props.children}
      <div class="sidebar-foot">
        {/* Always mounted: a live region only announces text that changes inside it. */}
        <p class="muted sync-status" classList={{ "sr-only": rail() }} role="status">
          {props.status}
        </p>
        <Show when={props.source.synced_commit}>
          {(c) => (
            <p class="muted" classList={{ "sr-only": rail() }}>
              <code>{shortCommit(c())}</code>
            </p>
          )}
        </Show>
        <Show when={props.onSync}>
          {(sync) => (
            <button
              type="button"
              classList={{ "nav-item": rail(), "sync-icon": rail() }}
              aria-label={rail() ? (props.syncing ? "Syncing…" : "Sync now") : undefined}
              onClick={sync()}
              disabled={props.syncing}
            >
              <Show when={rail()} fallback={props.syncing ? "Syncing…" : "Sync now"}>
                <span class="nav-mono" aria-hidden="true">
                  ↻
                </span>
              </Show>
            </button>
          )}
        </Show>
        <button
          type="button"
          class="nav-item"
          aria-current={props.settingsCurrent ? "page" : undefined}
          onClick={props.onSettings}
        >
          <span class="nav-mono" aria-hidden="true">
            ⚙
          </span>
          <span class="nav-label" classList={{ "sr-only": rail() }}>
            Settings
          </span>
        </button>
      </div>
      <Show when={tip()}>
        {(t) => (
          <Portal>
            <div class="rail-tip" role="tooltip" style={{ top: `${t().top}px`, left: `${t().left}px` }}>
              {t().text}
            </div>
          </Portal>
        )}
      </Show>
    </aside>
  );
}
