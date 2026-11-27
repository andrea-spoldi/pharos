import { useNavigate } from "@solidjs/router";
import { createResource, createSignal, Index, type JSX, Match, Show, Switch } from "solid-js";
import { Dynamic } from "solid-js/web";
import { commands, type Capabilities, type Manifest, type PlacementView, type WidgetSpec } from "../bindings";
import { type Box, type DragController, type DragMode } from "./grid-layout";
import { toIpcError, unwrap, type IpcError } from "../ipc";
import { CardView, ListView, RawView, TableView } from "../views";
import { MarkdownView } from "../views/markdown";
import { CapabilityList } from "./SyncReview";

const views = { table: TableView, list: ListView, card: CardView, raw: RawView } as const;

function errorTitle(err: IpcError): string {
  switch (err.kind) {
    case "permissionDenied":
      return "Access denied";
    case "dataError":
      return "Could not read the data";
    case "network":
      return "Could not reach the service";
    case "needsSetup":
      return "Needs setup";
    case "executableNotFound":
      return "Program not found";
    case "commandFailed":
      return "The command failed";
    default:
      return "Could not run this widget";
  }
}

function State(props: { tone?: "danger" | "setup"; title: string; children: JSX.Element }) {
  return (
    <div class={`state ${props.tone ?? ""}`} role={props.tone === "danger" ? "alert" : undefined}>
      <strong>{props.title}</strong>
      <div>{props.children}</div>
    </div>
  );
}

/** What the user may do to a placement on their own dashboard; absent on a read-only one. */
export type WidgetEdit = {
  editable: boolean;
  /** Open the add-widget picker, targeting this group. */
  onAdd: (groupId: number) => void;
  onRename: (placementId: number, title: string) => void;
  onRemove: (placementId: number, name: string) => void;
};

/**
 * One dashboard placement: a card placed on the 12-column grid, showing exactly one state.
 * The engine decides the status (D-024); only runnable placements run, by their slot: the
 * dashboard and the placement's path in it (D-031).
 */
export function WidgetFrame(props: {
  placement: PlacementView;
  slot: { dashboardId: string; path: number[] };
  version: number;
  /** The platform's label for an input key ("Your team"), if it declares one. */
  inputLabel?: (key: string) => string | null | undefined;
  /** Compose affordances, threaded down so a group's members can be removed too. */
  edit?: WidgetEdit;
  /** The dashboard's move/resize coordinator; present only on top-level frames. */
  drag?: DragController;
  /** A blocked widget was approved; the dashboards reload to run it. */
  onApproved: () => void;
}) {
  const p = () => props.placement;
  const name = () => p().manifest?.metadata.title ?? p().widget;
  // While a gesture runs the frame reads its box from the transient layout, so it (and every card
  // the reflow nudged) animates live; otherwise it sits at its stored geometry.
  const box = (): Box => {
    const id = p().id;
    return (id !== null && props.drag?.boxOf(id)) || p();
  };
  const style = () => ({
    "grid-column": `${box().x + 1} / span ${box().w}`,
    "grid-row": `${box().y + 1} / span ${box().h}`,
  });
  const status = () => p().status;
  /** The heading: a user group's own title, else the widget's. */
  const heading = () => p().title ?? name();
  /** Only the user's own placements carry an id; platform and manifest-expanded ones do not. */
  const editing = () => {
    const e = props.edit;
    const id = p().id;
    return e?.editable && id !== null ? { e, id } : undefined;
  };
  const remove = () => {
    const ed = editing();
    return ed ? () => ed.e.onRemove(ed.id, heading()) : undefined;
  };
  // Move/resize grips: only a top-level, editable placement at a wide-enough window carries them.
  const grips = () => {
    const d = props.drag;
    const id = p().id;
    return d && id !== null && d.enabled() ? { d, id } : undefined;
  };
  // Translate a pointer gesture into a target box in grid cells and feed the coordinator. The
  // cell size is measured from the live grid (12 columns, 80px rows, a --space-3 gap); jsdom
  // reports zeros, which is why the coordinator itself is what the tests drive.
  const startGesture = (mode: DragMode) => (e: PointerEvent) => {
    const g = grips();
    if (!g) return;
    e.preventDefault();
    e.stopPropagation();
    const gridEl = (e.currentTarget as HTMLElement).closest(".grid") as HTMLElement | null;
    const rect = gridEl?.getBoundingClientRect();
    const gap = 16;
    const cols = 12;
    const cellW = ((rect?.width ?? 0) - gap * (cols - 1)) / cols;
    const rowH = 80;
    const start = { x: e.clientX, y: e.clientY };
    const base = { x: p().x, y: p().y, w: p().w, h: p().h };
    g.d.begin(g.id, mode);
    const onMove = (ev: PointerEvent) => {
      const dx = Math.round((ev.clientX - start.x) / (cellW + gap));
      const dy = Math.round((ev.clientY - start.y) / (rowH + gap));
      const target =
        mode === "move"
          ? { ...base, x: base.x + dx, y: base.y + dy }
          : { ...base, w: base.w + dx, h: base.h + dy };
      g.d.onMove(g.id, target);
    };
    const stop = (commit: boolean) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancel);
      commit ? g.d.commit(g.id) : g.d.cancel();
    };
    const onUp = () => stop(true);
    const onCancel = () => stop(false);
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
  };
  // The header is the move surface, but its own buttons (Rename/Remove) must still click through.
  const onHeaderPointerDown = (e: PointerEvent) => {
    if (!grips() || (e.target as HTMLElement).closest("button")) return;
    startGesture("move")(e);
  };
  const navigate = useNavigate();
  /** Settings, focused on the first missing input, else the first missing secret. */
  const openSettings = (s: { inputs: string[]; secrets: string[] }) => {
    const focus = s.inputs.length ? `input:${s.inputs[0]}` : `secret:${s.secrets[0]}`;
    navigate(`/settings?focus=${encodeURIComponent(focus)}`);
  };

  return (
    <article
      class="widget"
      classList={{ "widget-group": p().container, dragging: p().id !== null && !!props.drag?.isMoving(p().id as number) }}
      style={style()}
    >
      <header
        class="widget-head"
        classList={{ "widget-head--grab": !!grips() }}
        data-move-handle={grips() ? "" : undefined}
        onPointerDown={onHeaderPointerDown}
      >
        <strong>{p().title ?? p().manifest?.metadata.title ?? <code>{p().widget}</code>}</strong>
        <span class="widget-actions">
          <Show when={p().container && editing()}>
            {(ed) => (
              <button
                type="button"
                class="link"
                aria-label={`Rename ${heading()}`}
                onClick={() => ed().e.onRename(ed().id, heading())}
              >
                Rename
              </button>
            )}
          </Show>
          <Show when={remove()}>
            {(rm) => (
              <button type="button" class="link" aria-label={`Remove ${heading()}`} onClick={() => rm()()}>
                Remove
              </button>
            )}
          </Show>
        </span>
      </header>
      <div class="widget-body">
        <Switch>
          <Match when={p().container}>
            {/* A container, even an empty one: its parts, each run by this placement's path plus its index. */}
            <section class="grid subgrid" aria-label={heading()}>
              <Index each={p().children}>
                {(child, i) => (
                  <WidgetFrame
                    placement={child()}
                    slot={{ dashboardId: props.slot.dashboardId, path: [...props.slot.path, i] }}
                    version={props.version}
                    inputLabel={props.inputLabel}
                    edit={props.edit}
                    onApproved={props.onApproved}
                  />
                )}
              </Index>
            </section>
            <Show when={editing()}>
              {(ed) => (
                <div class="group-add">
                  <Show when={p().children.length === 0}>
                    <span class="muted">Nothing in this group yet.</span>
                  </Show>
                  <button type="button" aria-label={`Add widget to ${heading()}`} onClick={() => ed().e.onAdd(ed().id)}>
                    Add widget
                  </button>
                </div>
              )}
            </Show>
          </Match>
          <Match when={status().kind === "missing"}>
            <State tone="danger" title="Missing widget">
              This dashboard places <code>{p().widget}</code>, which the platform repo doesn't define.
            </State>
          </Match>
          <Match when={status().kind === "needsApproval" && (status() as { added: Capabilities })}>
            {(s) => <NeedsApproval widget={p().widget} added={s().added} onApproved={props.onApproved} />}
          </Match>
          <Match when={status().kind === "needsSetup" && (status() as { inputs: string[]; secrets: string[] })}>
            {(s) => (
              <State tone="setup" title="Needs setup">
                {[
                  ...s().inputs.map((i) => {
                    const label = props.inputLabel?.(i);
                    return label ? (
                      <>
                        Needs <span class="term">{label}</span>
                      </>
                    ) : (
                      <>
                        Needs your <code>{i}</code>
                      </>
                    );
                  }),
                  ...s().secrets.map((name) => (
                    <>
                      Needs secret <code>{name}</code>
                    </>
                  )),
                ].flatMap((el, i) => (i ? [" · ", el] : [el]))}
                <div>
                  <button type="button" onClick={() => openSettings(s())}>
                    Open Settings
                  </button>
                </div>
              </State>
            )}
          </Match>
          <Match when={status().kind === "notAvailable" && (status() as { feature: string })}>
            {(s) => <State title="Not available yet">{s().feature} arrive in a later version.</State>}
          </Match>
          <Match when={(status().kind === "static" || status().kind === "runnable") && p().manifest?.spec.view === null}>
            <State title="No view">This widget doesn't say how to show anything.</State>
          </Match>
          <Match when={status().kind === "static" && p().manifest}>
            {(m) => <MarkdownView content={m().spec.view?.content ?? ""} />}
          </Match>
          <Match when={status().kind === "runnable" && p().manifest}>
            {(m) => (
              <Run
                widget={m()}
                slot={props.slot}
                reads={p().reads}
                version={props.version}
                onSettings={(focus) => navigate(focus ? `/settings?focus=${encodeURIComponent(focus)}` : "/settings")}
              />
            )}
          </Match>
        </Switch>
      </div>
      <Show when={grips()}>
        <button
          type="button"
          class="resize-grip"
          aria-label={`Resize ${heading()}`}
          onPointerDown={startGesture("resize")}
        />
      </Show>
    </article>
  );
}

/** A leaf the platform repo changed to ask for more: it stays stopped until the user approves. */
function NeedsApproval(props: { widget: string; added: Capabilities; onApproved: () => void }) {
  const [error, setError] = createSignal<string>();
  const [busy, setBusy] = createSignal(false);
  const [note, setNote] = createSignal<string>();
  const [manualText, setManualText] = createSignal<string>();
  const report = async () => {
    setBusy(true);
    setError(undefined);
    setNote(undefined);
    setManualText(undefined);
    try {
      const r = await unwrap(commands.reportWidget(props.widget));
      if (r.opened) {
        setNote("Opened in your browser — send it from there.");
        return;
      }
      try {
        await navigator.clipboard.writeText(r.text);
        setNote("Copied — paste it to your platform team.");
      } catch {
        setManualText(r.text);
      }
    } catch (e) {
      setError(toIpcError(e).message);
    } finally {
      setBusy(false);
    }
  };
  const approve = async () => {
    setBusy(true);
    setError(undefined);
    try {
      await unwrap(commands.approveWidget(props.widget));
      props.onApproved();
    } catch (e) {
      setError(toIpcError(e).message);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div class="state setup">
      <strong>Needs approval</strong>
      <div>
        <p>This widget now wants to:</p>
        <dl class="approval-caps">
          <CapabilityList caps={props.added} />
        </dl>
        <Show when={error()}>
          {(msg) => (
            <p class="error" role="alert">
              {msg()}
            </p>
          )}
        </Show>
        <div class="row">
          <button type="button" disabled={busy()} onClick={approve}>
            Approve
          </button>
          <button type="button" disabled={busy()} onClick={report}>
            Report to platform team
          </button>
        </div>
        <Show when={note()}>
          {(msg) => (
            <p role="status">{msg()}</p>
          )}
        </Show>
        <Show when={manualText()}>
          {(text) => (
            <div>
              <p role="status">Copy this and send it to your platform team:</p>
              <textarea readOnly rows={8} value={text()} />
            </div>
          )}
        </Show>
      </div>
    </div>
  );
}

/** Keyed on the sync version; a re-run keeps the previous rows on screen (no skeleton flash). */
function Run(props: {
  widget: Manifest<WidgetSpec>;
  slot: { dashboardId: string; path: number[] };
  reads: string | null;
  version: number;
  onSettings: (focus?: string) => void;
}) {
  // A string key, so a new manifest object for the same slot and version doesn't re-run.
  const [data] = createResource(
    () => JSON.stringify([props.version, props.slot.dashboardId, props.slot.path]),
    (key) => {
      const [, dashboardId, path] = JSON.parse(key) as [number, string, number[]];
      return unwrap(commands.runPlacement(dashboardId, path));
    },
  );
  // A refused token is fixed in Settings; a file outside the approval is not.
  const refused = (err: IpcError) => err.kind === "permissionDenied" && props.widget.spec.source?.type === "http";
  const view = () => views[props.widget.spec.view?.type as keyof typeof views];

  return (
    <Switch>
      <Match when={data.error as IpcError | undefined}>
        {(err) => (
          <State tone="danger" title={errorTitle(err())}>
            <Show when={refused(err())}>
              <p>The service refused the request. Check the secrets this widget uses.</p>
            </Show>
            <Show when={err().kind === "executableNotFound"}>
              <p>Add the folder that contains it to the program search path in Settings.</p>
            </Show>
            <Show
              when={err().kind === "commandFailed"}
              fallback={
                <details>
                  <summary>Details</summary>
                  <pre>{String(err().message ?? err())}</pre>
                </details>
              }
            >
              <pre>{err().message}</pre>
            </Show>
            <Show when={refused(err()) || err().kind === "executableNotFound"}>
              <div>
                <button
                  type="button"
                  onClick={() => props.onSettings(err().kind === "executableNotFound" ? "search-path" : undefined)}
                >
                  Open Settings
                </button>
              </div>
            </Show>
          </State>
        )}
      </Match>
      <Match when={data.latest}>
        {(d) => (
          <Switch>
            <Match when={d().rows.length === 0}>
              <State title="No data">
                <Show when={props.reads} fallback="The source returned no rows.">
                  {(reads) => (
                    <>
                      No data in <code>{reads()}</code>.
                    </>
                  )}
                </Show>
              </State>
            </Match>
            <Match when={d()}>
              <Dynamic component={view()} columns={props.widget.spec.view?.columns ?? []} data={d()} />
            </Match>
          </Switch>
        )}
      </Match>
      <Match when={data.loading}>
        <div class="skeleton" aria-busy="true" aria-label="Loading">
          <span />
          <span />
          <span />
        </div>
      </Match>
    </Switch>
  );
}
