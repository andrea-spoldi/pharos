import { useNavigate, useParams } from "@solidjs/router";
import {
  createEffect,
  createSignal,
  Index,
  Match,
  onCleanup,
  type Resource,
  Show,
  Switch,
} from "solid-js";
import { commands, type DashboardView } from "../bindings";
import { unwrap } from "../ipc";
import {
  AddWidgetDialog,
  DashboardToolbar,
  RemoveWidgetDialog,
  TitleDialog,
} from "../components/DashboardEditor";
import { narrow } from "../viewport";
import { WidgetFrame, type WidgetEdit } from "../components/WidgetFrame";
import {
  type Box,
  type DragController,
  type DragMode,
  type LayoutItem,
  resolveLayout,
} from "../components/grid-layout";

/** A leaf never shrinks below 1×1; a group keeps room for its members. */
const GRID_COLS = 12;
const MIN_SIZE = { leaf: { w: 1, h: 1 }, container: { w: 3, h: 2 } } as const;

// The drag coordinator is reached in tests through the grid element it is mounted on, keeping
// the gesture seam out of the component's props while leaving no trace in the rendered DOM.
const controllers = new WeakMap<Element, DragController>();
/** The move/resize coordinator mounted on a dashboard grid element, for tests to drive a gesture. */
export function dragControllerFor(
  grid: Element | null | undefined,
): DragController | undefined {
  return grid ? controllers.get(grid) : undefined;
}

/**
 * The current dashboard from `dashboards` (owned by Home, which also feeds the sidebar), in the
 * engine's order: the landing dashboard first. `#/d/:id` picks one; a missing or unknown id
 * goes to the first, which the engine orders `home` first (D-025). Every mutation calls
 * `refetch`, so the sidebar sees it too. `version` is the sync version; bumping it re-runs widgets.
 */
export function Dashboards(props: {
  dashboards: Resource<DashboardView[]>;
  refetch: () => unknown;
  /** The landing dashboard's id, computed once by Home. */
  landing?: string;
  /** Opens a dashboard created by the toolbar (Customize) once it is listed; Home owns it. */
  onCreated: (d: DashboardView) => void;
  version: number;
  inputLabel?: (key: string) => string | null | undefined;
}) {
  const params = useParams<{ id?: string }>();
  const navigate = useNavigate();
  // One resource for the screen's lifetime: Home owns it and never swaps it.
  const dashboards = props.dashboards;
  const refetch = () => props.refetch();
  // Ids contain `:`, which navigation percent-encodes; compare the decoded form.
  const routeId = () =>
    params.id === undefined ? undefined : safeDecode(params.id);
  const current = () => dashboards.latest?.find((d) => d.id === routeId());
  const [removing, setRemoving] = createSignal<{
    dashboardId: string;
    placementId: number;
    name: string;
  }>();
  const [adding, setAdding] = createSignal<{
    dashboardId: string;
    parentId: number;
  }>();
  const [renaming, setRenaming] = createSignal<{
    dashboardId: string;
    placementId: number;
    title: string;
  }>();
  /** Compose callbacks for a user dashboard's placements; they reuse the dialogs below. */
  const editFor = (dashboardId: string, editable: boolean): WidgetEdit => ({
    editable,
    onAdd: (parentId) => setAdding({ dashboardId, parentId }),
    onRename: (placementId, title) =>
      setRenaming({ dashboardId, placementId, title }),
    onRemove: (placementId, name) =>
      setRemoving({ dashboardId, placementId, name }),
  });
  // View-first: a dashboard opens in view mode and an explicit Edit toggle turns editing on.
  // Ephemeral, per-dashboard; nothing persisted. We settle edit mode only when the *dashboard
  // itself* changes (open/switch), tracked by id — a re-sync hands back a new object for the same
  // id, and must not knock the user out of edit mode mid-compose. An empty editable dashboard
  // opens in edit mode, since its only content is the add-widget call to action.
  const [editing, setEditing] = createSignal(false);
  let settledFor: string | undefined;
  createEffect(() => {
    if (dashboards.error) return;
    const c = current();
    if (c?.id === settledFor) return;
    settledFor = c?.id;
    setEditing(!!(c?.editable && c.placements.length === 0));
  });
  // Drag/resize: a transient layout held while a gesture runs, reflowed by `resolveLayout` and
  // rendered live through each frame's grid-column/grid-row. The pixel→cell math lives in the
  // frames' pointer handlers; the coordinator below starts at a resolved target box, so the
  // reflow + persist seam is exercisable without jsdom pixel measurement. Nothing is persisted
  // until pointer-up, and Escape (or a narrow window) abandons the gesture.
  const [gesture, setGesture] = createSignal<{
    id: number;
    mode: DragMode;
    base: LayoutItem[];
    items: LayoutItem[];
  }>();
  const enabled = () => !narrow() && editing() && !!current()?.editable;
  const topLevel = () =>
    (current()?.placements ?? []).filter((p) => p.id !== null);
  const baseItems = (): LayoutItem[] =>
    topLevel().map((p) => ({
      id: p.id as number,
      x: p.x,
      y: p.y,
      w: p.w,
      h: p.h,
    }));

  const dragController: DragController = {
    enabled,
    active: () => gesture() !== undefined,
    isMoving: (id) => gesture()?.id === id,
    boxOf: (id) => gesture()?.items.find((i) => i.id === id),
    begin: (id, mode) => {
      if (!enabled()) return;
      const base = baseItems();
      setGesture({ id, mode, base, items: base });
    },
    onMove: (id, target) => {
      const g = gesture();
      if (!enabled() || !g || g.id !== id) return;
      let t: Box = {
        ...target,
        x: Math.max(0, target.x),
        y: Math.max(0, target.y),
      };
      if (g.mode === "resize") {
        const min = topLevel().find((p) => p.id === id)?.container
          ? MIN_SIZE.container
          : MIN_SIZE.leaf;
        // A resize only grows rightward and never past the grid edge (x + w <= GRID_COLS).
        const x = Math.min(t.x, GRID_COLS - min.w);
        t = {
          ...t,
          x,
          w: Math.max(min.w, Math.min(target.w, GRID_COLS - x)),
          h: Math.max(min.h, target.h),
        };
      }
      setGesture({ ...g, items: resolveLayout(g.base, id, t) });
    },
    commit: (id) => {
      const g = gesture();
      setGesture(undefined);
      const d = current();
      if (!enabled() || !g || g.id !== id || !d) return;
      const before = new Map(topLevel().map((p) => [p.id as number, p]));
      const changed = g.items
        .filter((i) => {
          const b = before.get(i.id);
          return (
            b && (b.x !== i.x || b.y !== i.y || b.w !== i.w || b.h !== i.h)
          );
        })
        .map((i) => ({ placementId: i.id, x: i.x, y: i.y, w: i.w, h: i.h }));
      if (changed.length === 0) return;
      void (async () => {
        try {
          await unwrap(commands.setLayout(d.id, changed));
        } catch {
          // Rejected: fall through to resync with the persisted layout.
        }
        await refetch();
      })();
    },
    cancel: () => setGesture(undefined),
  };
  // Escape abandons an in-flight gesture, leaving the stored layout untouched.
  const onDragKey = (e: KeyboardEvent) => {
    if (e.key === "Escape" && gesture()) {
      e.preventDefault();
      dragController.cancel();
    }
  };
  document.addEventListener("keydown", onDragKey);
  onCleanup(() => document.removeEventListener("keydown", onDragKey));

  createEffect(() => {
    if (dashboards.error) return;
    const list = dashboards.latest;
    if (list?.length && !current())
      navigate(`/d/${encodeURIComponent(list[0].id)}`, { replace: true });
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
          <header class="page-head">
            <h1>Dashboards</h1>
          </header>
          <div class="notice">
            <p>No dashboards yet. Click + New dashboard to create one.</p>
          </div>
        </section>
      </Match>
      <Match when={current()}>
        {(d) => (
          <section class="stack">
            <header class="row spread page-head">
              <div>
                <h1>{d().metadata.title ?? d().metadata.name}</h1>
                <p class="muted">
                  {d().origin === "platform"
                    ? "Platform dashboard · read-only"
                    : "Your dashboard"}
                </p>
              </div>
              <DashboardToolbar
                dashboard={d()}
                landing={d().id === props.landing}
                editing={editing()}
                onToggleEdit={() => setEditing((v) => !v)}
                onChanged={refetch}
                onCreated={props.onCreated}
                onDeleted={refetch}
              />
            </header>
            <RemoveWidgetDialog
              target={removing()}
              onClose={() => setRemoving(undefined)}
              onRemoved={refetch}
            />
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
                await unwrap(
                  commands.renamePlacement(r.dashboardId, r.placementId, t),
                );
                await refetch();
              }}
            />
            <Show
              when={d().placements.length}
              fallback={
                <div class="notice">
                  <Show
                    when={d().editable}
                    fallback={<p>This dashboard is empty.</p>}
                  >
                    <strong>Add your first widget</strong>
                    <p>Use Add widget to pick one from the platform catalog.</p>
                  </Show>
                </div>
              }
            >
              <div
                class="grid"
                classList={{ reflowing: dragController.active() }}
                ref={(el) => controllers.set(el, dragController)}
              >
                {/* Index, not For: a sync returns new objects, and keying by identity would
                    remount every widget and flash its skeleton. Each keeps its position in the
                    dashboard file, which is how the engine finds it to run. */}
                <Index
                  each={d()
                    .placements.map((placement, index) => ({
                      placement,
                      index,
                    }))
                    .sort(
                      (a, b) =>
                        a.placement.y - b.placement.y ||
                        a.placement.x - b.placement.x,
                    )}
                >
                  {(p) => (
                    <WidgetFrame
                      placement={p().placement}
                      slot={{ dashboardId: d().id, path: [p().index] }}
                      version={props.version}
                      inputLabel={props.inputLabel}
                      edit={editFor(d().id, d().editable && editing())}
                      drag={dragController}
                      onApproved={refetch}
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

/** Route ids contain `:`, which navigation percent-encodes; this gives back the stored id. */
export function safeDecode(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}
