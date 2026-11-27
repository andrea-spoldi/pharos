import { For, type JSX, Show } from "solid-js";
import type { DashboardView, RepoSource } from "../bindings";
import { NewDashboardButton } from "./DashboardEditor";
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

/**
 * Left navigation: platform dashboards and the user's own, each in the engine's order (which
 * already puts `home` first among the platform ones). Items navigate through `onOpen`;
 * "+ New dashboard" closes *Mine* and hands the created dashboard to `onCreated`.
 * The brand and source sit above; the sync status (always mounted, it is a live region), Sync now
 * and Settings sit in the footer. Before the first sync there are no dashboards to list.
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
      <div class="sidebar-top">
        <p class="brand" data-testid="brand">
          <PharosMark class="brand-mark" />
          {props.title}
        </p>
        <p class="muted nav-source" data-testid="source" title={props.source.url}>
          <span aria-hidden="true">
            {repoName(props.source.url)} · {props.source.branch}
          </span>
          <span class="sr-only">
            {props.source.url} · {props.source.branch}
          </span>
        </p>
      </div>
      <Show when={!props.neverSynced}>
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
      </Show>
      {props.children}
      <div class="sidebar-foot">
        {/* Always mounted: a live region only announces text that changes inside it. */}
        <p class="muted sync-status" role="status">
          {props.status}
        </p>
        <Show when={props.source.synced_commit}>
          {(c) => (
            <p class="muted">
              <code>{shortCommit(c())}</code>
            </p>
          )}
        </Show>
        <Show when={props.onSync}>
          {(sync) => (
            <button type="button" onClick={sync()} disabled={props.syncing}>
              {props.syncing ? "Syncing…" : "Sync now"}
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
          <span class="nav-label">Settings</span>
        </button>
      </div>
    </aside>
  );
}
