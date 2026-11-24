import { useNavigate, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, For, Index, Match, onCleanup, Show, Switch } from "solid-js";
import { commands, type DashboardView } from "../bindings";
import { unwrap } from "../ipc";
import { DashboardToolbar, RemoveWidgetDialog } from "../components/DashboardEditor";
import { WidgetFrame } from "../components/WidgetFrame";

const MAX_TABS = 5;

const label = (d: DashboardView) => d.metadata.title ?? d.metadata.name;

/**
 * Platform dashboards (read-only) and the user's own (composable), in the engine's order:
 * the landing dashboard first. `#/d/:id` picks one; a missing
 * or unknown id goes to the first, which the engine orders `home` first (D-025).
 * Bump `version` after a sync to reload.
 */
export function Dashboards(props: { version: number; inputLabel?: (key: string) => string | null | undefined }) {
  const params = useParams<{ id?: string }>();
  const navigate = useNavigate();
  const [dashboards, { refetch }] = createResource(
    () => props.version,
    () => unwrap(commands.listDashboards()),
  );
  // Ids contain `:`, which navigation percent-encodes; compare the decoded form.
  const routeId = () => (params.id === undefined ? undefined : safeDecode(params.id));
  const current = () => dashboards.latest?.find((d) => d.id === routeId());
  const open = (id: string) => navigate(`/d/${encodeURIComponent(id)}`);
  const [removing, setRemoving] = createSignal<{ dashboardId: string; placementId: number; name: string }>();
  // The engine puts the landing dashboard first, and the app opens on the first one.
  const landingId = () => dashboards.latest?.[0]?.id;
  // Mutations re-read the list from the engine; a new dashboard is opened once it is listed,
  // else the unknown-id fallback below would bounce to the first.
  const created = async (view: DashboardView) => {
    await refetch();
    open(view.id);
  };

  createEffect(() => {
    if (dashboards.error) return;
    const list = dashboards.latest;
    if (list?.length && !current()) navigate(`/d/${encodeURIComponent(list[0].id)}`, { replace: true });
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
        <section class="stack">
          <DashboardToolbar landing={false} onChanged={refetch} onCreated={created} onDeleted={refetch} />
          <div class="notice">
            <p>The platform repo defines no dashboards yet.</p>
          </div>
        </section>
      </Match>
      <Match when={current()}>
        {(d) => (
          <section class="stack">
            <Tabs dashboards={dashboards.latest ?? []} current={d().id} landing={landingId()} onOpen={open} />
            <DashboardToolbar
              dashboard={d()}
              landing={d().id === landingId()}
              onChanged={refetch}
              onCreated={created}
              onDeleted={refetch}
            />
            <RemoveWidgetDialog target={removing()} onClose={() => setRemoving(undefined)} onRemoved={refetch} />
            <Show
              when={d().placements.length}
              fallback={
                <div class="notice">
                  <Show when={d().editable} fallback={<p>This dashboard is empty.</p>}>
                    <strong>Add your first widget</strong>
                    <p>Use Add widget to pick one from the platform catalog.</p>
                  </Show>
                </div>
              }
            >
              <div class="grid">
                {/* Index, not For: a sync returns new objects, and keying by identity would
                    remount every widget and flash its skeleton. Each keeps its position in the
                    dashboard file, which is how the engine finds it to run. */}
                <Index
                  each={d()
                    .placements.map((placement, index) => ({ placement, index }))
                    .sort((a, b) => a.placement.y - b.placement.y || a.placement.x - b.placement.x)}
                >
                  {(p) => (
                    <WidgetFrame
                      placement={p().placement}
                      slot={{ dashboardId: d().id, path: [p().index] }}
                      version={props.version}
                      inputLabel={props.inputLabel}
                      onRemove={
                        d().editable && p().placement.id !== null
                          ? () =>
                              setRemoving({
                                dashboardId: d().id,
                                placementId: p().placement.id as number,
                                name: p().placement.manifest?.metadata.title ?? p().placement.widget,
                              })
                          : undefined
                      }
                    />
                  )}
                </Index>
              </div>
            </Show>
          </section>
        )}
      </Match>
    </Switch>
  );
}

function Tabs(props: { dashboards: DashboardView[]; current: string; landing?: string; onOpen: (id: string) => void }) {
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
      aria-current={d.id === props.current ? "page" : undefined}
      onClick={() => props.onOpen(d.id)}
    >
      {label(d)}
      <Show when={props.dashboards.length > 1 && d.id === props.landing}>
        {/* Hidden from the accessible name so a tab keeps its plain title. */}
        <span class="landing-mark" aria-hidden="true" aria-label="Landing dashboard" title="Landing dashboard">
          {" "}
          ★
        </span>
      </Show>
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
            classList={{ active: props.dashboards.slice(MAX_TABS).some((d) => d.id === props.current) }}
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
                      aria-current={d.id === props.current ? "page" : undefined}
                      onClick={() => {
                        setMenuOpen(false);
                        props.onOpen(d.id);
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

function safeDecode(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}
