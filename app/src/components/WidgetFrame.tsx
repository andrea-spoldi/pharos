import { useNavigate } from "@solidjs/router";
import { createResource, type JSX, Match, Show, Switch } from "solid-js";
import { Dynamic } from "solid-js/web";
import { commands, type Manifest, type PlacementView, type WidgetSpec } from "../bindings";
import { unwrap, type IpcError } from "../ipc";
import { CardView, ListView, TableView } from "../views";
import { MarkdownView } from "../views/markdown";

const views = { table: TableView, list: ListView, card: CardView } as const;

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

/**
 * One dashboard placement: a card placed on the 12-column grid, showing exactly one state.
 * The engine decides the status (D-024); only runnable placements run, by their slot: the
 * dashboard and the placement's position in it (D-031).
 */
export function WidgetFrame(props: {
  placement: PlacementView;
  slot: { dashboard: string; index: number };
  version: number;
  /** The platform's label for an input key ("Your team"), if it declares one. */
  inputLabel?: (key: string) => string | null | undefined;
}) {
  const p = () => props.placement;
  const style = () => ({
    "grid-column": `${p().x + 1} / span ${p().w}`,
    "grid-row": `${p().y + 1} / span ${p().h}`,
  });
  const status = () => p().status;
  const navigate = useNavigate();
  /** Settings, focused on the first missing input, else the first missing secret. */
  const openSettings = (s: { inputs: string[]; secrets: string[] }) => {
    const focus = s.inputs.length ? `input:${s.inputs[0]}` : `secret:${s.secrets[0]}`;
    navigate(`/settings?focus=${encodeURIComponent(focus)}`);
  };

  return (
    <article class="widget" style={style()}>
      <header class="widget-head">
        <strong>{p().manifest?.metadata.title ?? <code>{p().widget}</code>}</strong>
      </header>
      <div class="widget-body">
        <Switch>
          <Match when={status().kind === "missing"}>
            <State tone="danger" title="Missing widget">
              This dashboard places <code>{p().widget}</code>, which the platform repo doesn't define.
            </State>
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
          <Match when={status().kind === "static" && p().manifest}>
            {(m) => <MarkdownView content={m().spec.view.content ?? ""} />}
          </Match>
          <Match when={status().kind === "runnable" && p().manifest}>
            {(m) => (
              <Run
                widget={m()}
                slot={props.slot}
                reads={p().reads}
                version={props.version}
                onSettings={() => navigate("/settings")}
              />
            )}
          </Match>
        </Switch>
      </div>
    </article>
  );
}

/** Keyed on the sync version; a re-run keeps the previous rows on screen (no skeleton flash). */
function Run(props: {
  widget: Manifest<WidgetSpec>;
  slot: { dashboard: string; index: number };
  reads: string | null;
  version: number;
  onSettings: () => void;
}) {
  // A string key, so a new manifest object for the same slot and version doesn't re-run.
  const [data] = createResource(
    () => JSON.stringify([props.version, props.slot.dashboard, props.slot.index]),
    (key) => {
      const [, dashboard, index] = JSON.parse(key) as [number, string, number];
      return unwrap(commands.runPlacement(dashboard, index));
    },
  );
  // A refused token is fixed in Settings; a file outside the approval is not.
  const refused = (err: IpcError) => err.kind === "permissionDenied" && props.widget.spec.source?.type === "http";
  const view = () => views[props.widget.spec.view.type as keyof typeof views];

  return (
    <Switch>
      <Match when={data.error as IpcError | undefined}>
        {(err) => (
          <State tone="danger" title={errorTitle(err())}>
            <Show when={refused(err())}>
              <p>The service refused the request. Check the secrets this widget uses.</p>
            </Show>
            <details>
              <summary>Details</summary>
              <pre>{String(err().message ?? err())}</pre>
            </details>
            <Show when={refused(err())}>
              <div>
                <button type="button" onClick={props.onSettings}>
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
              <Dynamic component={view()} columns={props.widget.spec.view.columns} data={d()} />
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
