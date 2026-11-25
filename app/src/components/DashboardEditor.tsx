import { createEffect, createResource, createSignal, For, type JSX, Show } from "solid-js";
import { commands, type DashboardView } from "../bindings";
import { toIpcError, unwrap } from "../ipc";
import { Dialog } from "./Dialog";

/** Runs one mutating command; reports its error in the dialog instead of closing it. */
function useAction() {
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string>();
  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(undefined);
    try {
      await fn();
    } catch (err) {
      setError(toIpcError(err).message);
    } finally {
      setBusy(false);
    }
  };
  return { busy, error, setError, run };
}

const Failure = (props: { message?: string }) => (
  <Show when={props.message}>{(m) => <p class="danger-text" role="alert">{m()}</p>}</Show>
);

/** Asks for a title; used for both a new dashboard and a rename. */
export function TitleDialog(props: {
  heading: string;
  confirm: string;
  initial: string;
  open: boolean;
  onClose: () => void;
  onSubmit: (title: string) => Promise<void>;
}) {
  const [title, setTitle] = createSignal(props.initial);
  const action = useAction();
  createEffect(() => {
    if (props.open) {
      setTitle(props.initial);
      action.setError(undefined);
    }
  });
  const submit = (e?: Event) => {
    e?.preventDefault();
    const t = title().trim();
    if (!t) return;
    void action.run(async () => {
      await props.onSubmit(t);
      props.onClose();
    });
  };
  return (
    <Dialog
      title={props.heading}
      open={props.open}
      onClose={props.onClose}
      actions={
        <>
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button type="button" class="primary" disabled={action.busy() || !title().trim()} onClick={submit}>
            {props.confirm}
          </button>
        </>
      }
    >
      <form class="stack" onSubmit={submit}>
        <label>
          Title
          <input type="text" value={title()} onInput={(e) => setTitle(e.currentTarget.value)} autocomplete="off" />
        </label>
      </form>
      <Failure message={action.error()} />
    </Dialog>
  );
}

/** Cancel-first confirmation for a destructive action: Cancel is the first (and default) button. */
function ConfirmDialog(props: {
  heading: string;
  confirm: string;
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  children: JSX.Element;
}) {
  const action = useAction();
  createEffect(() => props.open && action.setError(undefined));
  return (
    <Dialog
      title={props.heading}
      open={props.open}
      onClose={props.onClose}
      actions={
        <>
          <button type="button" onClick={props.onClose}>
            Cancel
          </button>
          <button
            type="button"
            class="danger"
            disabled={action.busy()}
            onClick={() =>
              action.run(async () => {
                await props.onConfirm();
                props.onClose();
              })
            }
          >
            {props.confirm}
          </button>
        </>
      }
    >
      {props.children}
      <Failure message={action.error()} />
    </Dialog>
  );
}

/** Picks a catalog widget and adds it to a dashboard, at the top level or inside a group. */
export function AddWidgetDialog(props: {
  open: boolean;
  dashboardId: string;
  /** The group to add into; omitted or null for the dashboard itself. */
  parentId?: number | null;
  onClose: () => void;
  onAdded: () => void;
}) {
  // Read when opened, so a sync that changed the catalog is reflected.
  const [catalog] = createResource(
    () => props.open || undefined,
    () => unwrap(commands.listCatalog()),
  );
  const action = useAction();
  createEffect(() => props.open && action.setError(undefined));
  return (
    <Dialog
      title="Add widget"
      open={props.open}
      onClose={props.onClose}
      actions={
        <button type="button" onClick={props.onClose}>
          Cancel
        </button>
      }
    >
      <Show when={!catalog.error} fallback={<Failure message={String(catalog.error?.message ?? catalog.error)} />}>
        <Show when={catalog.latest} fallback={<p>Loading widgets…</p>}>
          {(entries) => (
            <Show when={entries().length} fallback={<p>The platform repo defines no widgets.</p>}>
              <ul class="menu catalog-list">
                <For each={entries()}>
                  {(w) => (
                    <li>
                      <button
                        type="button"
                        class="menu-item"
                        disabled={action.busy()}
                        onClick={() =>
                          action.run(async () => {
                            await unwrap(commands.addWidget(props.dashboardId, w.name, props.parentId ?? null));
                            props.onAdded();
                            props.onClose();
                          })
                        }
                      >
                        <strong>{w.title ?? w.name}</strong> <code>{w.name}</code>
                        <Show when={w.container}> · holds other widgets</Show>
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
          )}
        </Show>
      </Show>
      <Failure message={action.error()} />
    </Dialog>
  );
}

const title = (d: DashboardView) => d.metadata.title ?? d.metadata.name;

type Open = "new" | "group" | "add" | "rename" | "delete" | undefined;

/**
 * The compose toolbar. "+ New dashboard" is always there; the rest act on `dashboard` and are
 * shown only when it is the user's own (`editable`). Each mutation calls `onChanged` so the
 * screen refetches from the engine rather than patching its own copy.
 */
export function DashboardToolbar(props: {
  dashboard?: DashboardView;
  /** True when this dashboard is the one the app opens on. */
  landing: boolean;
  onChanged: () => void;
  onCreated: (created: DashboardView) => void;
  onDeleted: () => void;
}) {
  const [open, setOpen] = createSignal<Open>();
  const close = () => setOpen(undefined);
  const action = useAction();
  const editable = () => (props.dashboard?.editable ? props.dashboard : undefined);

  return (
    <div class="row toolbar">
      <button type="button" onClick={() => setOpen("new")}>
        + New dashboard
      </button>
      <Show when={props.dashboard && !editable() && !props.landing ? props.dashboard : undefined}>
        {(d) => (
          <button
            type="button"
            disabled={action.busy()}
            title="Make an editable copy of this dashboard"
            onClick={() =>
              action.run(async () => {
                props.onCreated(await unwrap(commands.customizeDashboard(d().id)));
              })
            }
          >
            Customize
          </button>
        )}
      </Show>
      <Show when={editable()}>
        {(d) => (
          <>
            <button type="button" onClick={() => setOpen("add")}>
              Add widget
            </button>
            <button type="button" onClick={() => setOpen("group")}>
              + New group
            </button>
            <button type="button" onClick={() => setOpen("rename")}>
              Rename
            </button>
            <button
              type="button"
              disabled={action.busy()}
              onClick={() =>
                action.run(async () => {
                  await unwrap(commands.setLanding(props.landing ? null : d().id));
                  props.onChanged();
                })
              }
            >
              {props.landing ? "Clear landing" : "Set as landing"}
            </button>
            <button type="button" class="danger" onClick={() => setOpen("delete")}>
              Delete
            </button>
          </>
        )}
      </Show>
      <Failure message={action.error()} />

      <TitleDialog
        heading="New dashboard"
        confirm="Create"
        initial=""
        open={open() === "new"}
        onClose={close}
        onSubmit={async (t) => {
          const created = await unwrap(commands.createDashboard(t));
          props.onCreated(created);
        }}
      />
      <Show when={editable()}>
        {(d) => (
          <>
            <AddWidgetDialog open={open() === "add"} dashboardId={d().id} onClose={close} onAdded={props.onChanged} />
            <TitleDialog
              heading="New group"
              confirm="Create"
              initial=""
              open={open() === "group"}
              onClose={close}
              onSubmit={async (t) => {
                await unwrap(commands.createGroup(d().id, t, null));
                props.onChanged();
              }}
            />
            <TitleDialog
              heading="Rename dashboard"
              confirm="Rename"
              initial={title(d())}
              open={open() === "rename"}
              onClose={close}
              onSubmit={async (t) => {
                await unwrap(commands.renameDashboard(d().id, t));
                props.onChanged();
              }}
            />
            <ConfirmDialog
              heading="Delete dashboard"
              confirm="Delete"
              open={open() === "delete"}
              onClose={close}
              onConfirm={async () => {
                await unwrap(commands.deleteDashboard(d().id));
                props.onDeleted();
              }}
            >
              <p>
                Delete <strong>{title(d())}</strong> and the widgets placed on it? The widgets themselves stay in the catalog.
              </p>
            </ConfirmDialog>
          </>
        )}
      </Show>
    </div>
  );
}

/** Confirmation for removing one placement from a user dashboard. */
export function RemoveWidgetDialog(props: {
  target?: { dashboardId: string; placementId: number; name: string };
  onClose: () => void;
  onRemoved: () => void;
}) {
  return (
    <ConfirmDialog
      heading="Remove widget"
      confirm="Remove"
      open={props.target !== undefined}
      onClose={props.onClose}
      onConfirm={async () => {
        const t = props.target;
        if (!t) return;
        await unwrap(commands.removeWidget(t.dashboardId, t.placementId));
        props.onRemoved();
      }}
    >
      <p>
        Remove <strong>{props.target?.name}</strong> from this dashboard?
      </p>
    </ConfirmDialog>
  );
}
