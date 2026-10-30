import { useNavigate, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, For, Index, Match, onCleanup, Show, Switch } from "solid-js";
import { commands, type DashboardView } from "../bindings";
import { unwrap } from "../ipc";
import { WidgetFrame } from "../components/WidgetFrame";

const MAX_TABS = 5;

const label = (d: DashboardView) => d.metadata.title ?? d.metadata.name;

/**
 * Platform dashboards of the last applied sync, read-only. `#/d/:name` picks one; a missing
 * or unknown name goes to the first, which the engine orders `home` first (D-025).
 * Bump `version` after a sync to reload.
 */
export function Dashboards(props: { version: number }) {
  const params = useParams<{ name?: string }>();
  const navigate = useNavigate();
  const [dashboards, { refetch }] = createResource(
    () => props.version,
    () => unwrap(commands.listDashboards()),
  );
  const current = () => dashboards.latest?.find((d) => d.metadata.name === params.name);
  const open = (name: string) => navigate(`/d/${encodeURIComponent(name)}`);

  createEffect(() => {
    if (dashboards.error) return;
    const list = dashboards.latest;
    if (list?.length && !current()) navigate(`/d/${encodeURIComponent(list[0].metadata.name)}`, { replace: true });
  });

  return (
    <Switch>
      <Match when={dashboards.error}>
        <div class="notice danger" role="alert">
          <strong>Could not load dashboards</strong>
          <pre>{String(dashboards.error?.message ?? dashboards.error)}</pre>
          <button type="button" onClick={() => refetch()}>
            Try again
          </button>
        </div>
      </Match>
      <Match when={dashboards.latest?.length === 0}>
        <div class="notice">
          <p>The platform repo defines no dashboards yet.</p>
        </div>
      </Match>
      <Match when={current()}>
        {(d) => (
          <section class="stack">
            <Tabs dashboards={dashboards.latest ?? []} current={d().metadata.name} onOpen={open} />
            <Show
              when={d().placements.length}
              fallback={
                <div class="notice">
                  <p>This dashboard is empty.</p>
                </div>
              }
            >
              <div class="grid">
                {/* Index, not For: a sync returns new objects, and keying by identity would
                    remount every widget and flash its skeleton. */}
                <Index each={[...d().placements].sort((a, b) => a.y - b.y || a.x - b.x)}>
                  {(p) => <WidgetFrame placement={p()} version={props.version} />}
                </Index>
              </div>
            </Show>
          </section>
        )}
      </Match>
    </Switch>
  );
}

function Tabs(props: { dashboards: DashboardView[]; current: string; onOpen: (name: string) => void }) {
  const [menuOpen, setMenuOpen] = createSignal(false);
  let more: HTMLDivElement | undefined;

  const onKey = (e: KeyboardEvent) => e.key === "Escape" && setMenuOpen(false);
  const onClick = (e: MouseEvent) => more && !more.contains(e.target as Node) && setMenuOpen(false);
  document.addEventListener("keydown", onKey);
  document.addEventListener("click", onClick);
  onCleanup(() => {
    document.removeEventListener("keydown", onKey);
    document.removeEventListener("click", onClick);
  });

  const tab = (d: DashboardView) => (
    <button
      type="button"
      class="tab"
      aria-current={d.metadata.name === props.current ? "page" : undefined}
      onClick={() => props.onOpen(d.metadata.name)}
    >
      {label(d)}
    </button>
  );

  return (
    <nav class="tabs" aria-label="Dashboards">
      <For each={props.dashboards.slice(0, MAX_TABS)}>{tab}</For>
      <Show when={props.dashboards.length > MAX_TABS}>
        <div class="more" ref={more}>
          <button
            type="button"
            class="tab"
            classList={{ active: props.dashboards.slice(MAX_TABS).some((d) => d.metadata.name === props.current) }}
            aria-expanded={menuOpen()}
            onClick={() => setMenuOpen((o) => !o)}
          >
            More ▾
          </button>
          <Show when={menuOpen()}>
            <ul class="menu">
              <For each={props.dashboards.slice(MAX_TABS)}>
                {(d) => (
                  <li>
                    <button
                      type="button"
                      class="menu-item"
                      aria-current={d.metadata.name === props.current ? "page" : undefined}
                      onClick={() => {
                        setMenuOpen(false);
                        props.onOpen(d.metadata.name);
                      }}
                    >
                      {label(d)}
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </div>
      </Show>
    </nav>
  );
}
