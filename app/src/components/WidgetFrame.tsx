import { createResource, type JSX, Match, Switch } from "solid-js";
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
 * The engine decides the status (D-024); only runnable placements run, by widget name.
 */
export function WidgetFrame(props: { placement: PlacementView; version: number }) {
  const p = () => props.placement;
  const style = () => ({
    "grid-column": `${p().x + 1} / span ${p().w}`,
    "grid-row": `${p().y + 1} / span ${p().h}`,
  });
  const status = () => p().status;

  return (
    <article class="widget" style={style()}>
      <header class="widget-head">
        <strong>{p().manifest?.metadata.title ?? p().widget}</strong>
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
                  ...s().inputs.map((i) => (
                    <>
                      Needs your <strong>{i}</strong>
                    </>
                  )),
                  ...s().secrets.map((name) => (
                    <>
                      Needs secret <strong>{name}</strong>
                    </>
                  )),
                ].flatMap((el, i) => (i ? [" · ", el] : [el]))}
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
            {(m) => <Run widget={m()} reads={p().reads} version={props.version} />}
          </Match>
        </Switch>
      </div>
    </article>
  );
}

/** Keyed on the sync version; a re-run keeps the previous rows on screen (no skeleton flash). */
function Run(props: { widget: Manifest<WidgetSpec>; reads: string | null; version: number }) {
  // A string key, so a new manifest object with the same name and version doesn't re-run.
  const [data] = createResource(
    () => `${props.version}:${props.widget.metadata.name}`,
    (key) => unwrap(commands.runWidget(key.slice(key.indexOf(":") + 1))),
  );
  const view = () => views[props.widget.spec.view.type as keyof typeof views];

  return (
    <Switch>
      <Match when={data.error as IpcError | undefined}>
        {(err) => (
          <State tone="danger" title={errorTitle(err())}>
            <details>
              <summary>Details</summary>
              <pre>{String(err().message ?? err())}</pre>
            </details>
          </State>
        )}
      </Match>
      <Match when={data.latest}>
        {(d) => (
          <Switch>
            <Match when={d().rows.length === 0}>
              <State title="No data">
                No data in <code>{props.reads}</code>.
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
