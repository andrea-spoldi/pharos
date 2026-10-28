// SPIKE (throwaway): dashboard grid, switcher and shared widget states for T-005.
import { createMemo, createResource, createSignal, For, type JSX, Match, Show, Switch } from "solid-js";
import { Dynamic } from "solid-js/web";
import { commands, type Manifest, type Placement, type WidgetSpec } from "../bindings";
import { unwrap, type IpcError } from "../ipc";
import { CardView, ListView, TableView } from "../views";

const views = { table: TableView, list: ListView, card: CardView } as const;
const MAX_TABS = 5;

export function Dashboards(props: { version: number }) {
  const [data] = createResource(
    () => props.version,
    async () => {
      const [dashboards, widgets] = await Promise.all([
        unwrap(commands.listDashboards()),
        unwrap(commands.listWidgets()),
      ]);
      return { dashboards, widgets: new Map(widgets.map((w) => [w.metadata.name, w])) };
    },
  );
  const [selected, setSelected] = createSignal<string>();
  const [moreOpen, setMoreOpen] = createSignal(false);
  const current = createMemo(() => {
    const ds = data.latest?.dashboards ?? [];
    return (
      ds.find((d) => d.metadata.name === selected()) ??
      ds.find((d) => d.metadata.name === "home") ??
      ds[0]
    );
  });
  const tabs = () => data.latest?.dashboards.slice(0, MAX_TABS) ?? [];
  const more = () => data.latest?.dashboards.slice(MAX_TABS) ?? [];

  return (
    <Switch>
      <Match when={data.error}>
        <div class="notice danger" role="alert">
          <strong>Could not load dashboards</strong>
          <pre>{String(data.error?.message ?? data.error)}</pre>
        </div>
      </Match>
      <Match when={data.latest && !current()}>
        <div class="notice">
          <p>The platform repo defines no dashboards yet.</p>
        </div>
      </Match>
      <Match when={current()}>
        {(d) => (
          <section class="stack">
            <nav class="tabs" aria-label="Dashboards">
              <For each={tabs()}>
                {(t) => (
                  <button
                    type="button"
                    class="tab"
                    aria-current={t.metadata.name === d().metadata.name ? "page" : undefined}
                    onClick={() => setSelected(t.metadata.name)}
                  >
                    {t.metadata.title ?? t.metadata.name}
                  </button>
                )}
              </For>
              <Show when={more().length}>
                <div class="more">
                  <button type="button" class="tab" onClick={() => setMoreOpen((o) => !o)}>
                    More ▾
                  </button>
                  <Show when={moreOpen()}>
                    <ul class="menu">
                      <For each={more()}>
                        {(t) => (
                          <li>
                            <button
                              type="button"
                              class="link"
                              onClick={() => {
                                setSelected(t.metadata.name);
                                setMoreOpen(false);
                              }}
                            >
                              {t.metadata.title ?? t.metadata.name}
                            </button>
                          </li>
                        )}
                      </For>
                    </ul>
                  </Show>
                </div>
              </Show>
            </nav>
            <div class="grid">
              <For each={[...(d().spec.widgets ?? [])].sort((a, b) => a.y - b.y || a.x - b.x)}>
                {(p) => <WidgetCard placement={p} widget={data.latest?.widgets.get(p.widget)} version={props.version} />}
              </For>
            </div>
          </section>
        )}
      </Match>
    </Switch>
  );
}

function WidgetCard(props: { placement: Placement; widget?: Manifest<WidgetSpec>; version: number }) {
  const p = props.placement;
  const style = {
    "grid-column": `${p.x + 1} / span ${p.w}`,
    "grid-row": `${p.y + 1} / span ${p.h}`,
  };
  const title = () => props.widget?.metadata.title ?? p.widget;
  const missingInputs = () =>
    [...Object.values(p.inputs ?? {}).join(" ").matchAll(/\{\{\s*user\.(\w+)\s*\}\}/g)].map((m) => m[1]);

  return (
    <article class="widget" style={style}>
      <header class="widget-head">
        <strong>{title()}</strong>
      </header>
      <div class="widget-body">
        <Switch fallback={<Body widget={props.widget!} version={props.version} />}>
          <Match when={!props.widget}>
            <State tone="danger" title="Missing widget">
              This dashboard places <code>{p.widget}</code>, which the platform repo doesn't define.
            </State>
          </Match>
          <Match when={missingInputs().length}>
            <State tone="setup" title="Needs setup">
              This widget needs your <strong>{missingInputs().join(", ")}</strong>. Setup arrives in a later
              version.
            </State>
          </Match>
          <Match when={props.widget?.spec.source && props.widget.spec.source.type !== "file"}>
            <State title="Not available yet">
              {props.widget?.spec.source?.type} widgets run in a later version.
            </State>
          </Match>
        </Switch>
      </div>
    </article>
  );
}

function State(props: { tone?: "danger" | "setup"; title: string; children: JSX.Element }) {
  return (
    <div class={`state ${props.tone ?? ""}`} role={props.tone === "danger" ? "alert" : undefined}>
      <strong>{props.title}</strong>
      <p>{props.children}</p>
    </div>
  );
}

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

function Body(props: { widget: Manifest<WidgetSpec>; version: number }) {
  const spec = () => props.widget.spec;
  if (!spec().source) {
    return spec().view.type === "markdown" ? (
      <Markdown text={spec().view.content ?? ""} />
    ) : (
      <State title="Not available yet">The {spec().view.type} view is not available yet.</State>
    );
  }
  const view = () => views[spec().view.type as keyof typeof views];
  const [data] = createResource(
    () => [props.widget.metadata.name, props.version] as const,
    ([name]) => unwrap(commands.runWidget(name)),
  );
  return (
    <Switch>
      <Match when={!view()}>
        <State title="Not available yet">The {spec().view.type} view is not available yet.</State>
      </Match>
      <Match when={data.error}>
        <State tone="danger" title={errorTitle(data.error)}>
          <details>
            <summary>Details</summary>
            <pre>{String(data.error?.message ?? data.error)}</pre>
          </details>
        </State>
      </Match>
      <Match when={data.loading && !data.latest}>
        <div class="skeleton" aria-busy="true" aria-label="Loading">
          <span />
          <span />
          <span />
        </div>
      </Match>
      <Match when={data.latest}>
        {(d) =>
          d().rows.length ? (
            <Dynamic component={view()} columns={spec().view.columns} data={d()} />
          ) : (
            <State title="No data">
              Nothing found at <code>{spec().source?.type === "file" ? (spec().source as { path: string }).path : ""}</code>.
            </State>
          )
        }
      </Match>
    </Switch>
  );
}

/** Spike-grade markdown: headings and paragraphs only, rendered as text (no HTML). */
function Markdown(props: { text: string }) {
  const blocks = () =>
    props.text
      .trim()
      .split(/\n\s*\n|\n(?=#)|(?<=^#.*)\n/m)
      .filter(Boolean);
  return (
    <div class="markdown">
      <For each={blocks()}>
        {(b) => {
          const h = /^(#{1,3})\s+(.*)/.exec(b);
          return h ? <Dynamic component={`h${h[1].length + 2}`}>{h[2]}</Dynamic> : <p>{b}</p>;
        }}
      </For>
    </div>
  );
}
