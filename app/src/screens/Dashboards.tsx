import { useNavigate, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, For, Index, Match, onCleanup, Show, Switch } from "solid-js";
import { commands, type DashboardView } from "../bindings";
import { unwrap } from "../ipc";
import { AddWidgetDialog, DashboardToolbar, RemoveWidgetDialog, TitleDialog } from "../components/DashboardEditor";
import { WidgetFrame, type WidgetEdit } from "../components/WidgetFrame";

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
  const [adding, setAdding] = createSignal<{ dashboardId: string; parentId: number }>();
  const [renaming, setRenaming] = createSignal<{ dashboardId: string; placementId: number; title: string }>();
  /** Compose callbacks for a user dashboard's placements; they reuse the dialogs below. */
  const editFor = (dashboardId: string, editable: boolean): WidgetEdit => ({
    editable,
    onAdd: (parentId) => setAdding({ dashboardId, parentId }),
    onRename: (placementId, title) => setRenaming({ dashboardId, placementId, title }),
    onRemove: (placementId, name) => setRemoving({ dashboardId, placementId, name }),
  });
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
            <p>No dashboards yet. Click + New dashboard to create one.</p>
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
            <AddWidgetDialog
              open={adding() !== undefined}
              dashboardId={adding()?.dashboardId ?? d().id}
              parentId={adding()?.parentId}
              onClose={() => setAdding(undefined)}
              onAdded={refetch}
            />
            <TitleDialog
              heading="Rename group"
              confirm="Rename"
              initial={renaming()?.title ?? ""}
              open={renaming() !== undefined}
              onClose={() => setRenaming(undefined)}
              onSubmit={async (t) => {
                const r = renaming();
                if (!r) return;
                await unwrap(commands.renamePlacement(r.dashboardId, r.placementId, t));
                await refetch();
              }}
            />
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
                      edit={editFor(d().id, d().editable)}
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
        {/* The star is decorative; the sr-only text puts the cue in the tab's accessible name. */}
        <span class="landing-mark" aria-hidden="true" title="Landing dashboard">
          {" "}
          ★
        </span>
        <span class="sr-only">{" (Landing dashboard)"}</span>
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
