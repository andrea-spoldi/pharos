import { createResource, For, Match, Switch } from "solid-js";
import { commands, type WidgetSpec } from "../bindings";
import { unwrap } from "../ipc";

function sourceSummary(spec: WidgetSpec): string {
  const s = spec.source;
  if (!s) return "static";
  switch (s.type) {
    case "file":
      return `file · ${s.path}`;
    case "command":
      return `command · ${s.executable}`;
    case "http":
      return `http · ${s.url}`;
  }
}

/** Widgets of the last applied sync. They run from slice 3 on. Bump `version` to reload. */
export function WidgetList(props: { version: number }) {
  const [widgets] = createResource(
    () => props.version,
    () => unwrap(commands.listWidgets()),
  );

  return (
    <section class="stack" aria-labelledby="widgets-title">
      <h2 id="widgets-title">Widgets</h2>
      <Switch>
        <Match when={widgets.error}>
          <div class="notice danger" role="alert">
            <strong>Could not load widgets</strong>
            <pre>{String(widgets.error?.message ?? widgets.error)}</pre>
          </div>
        </Match>
        <Match when={widgets.latest}>
          <ul class="cards" data-testid="widgets">
            <For each={widgets.latest} fallback={<li class="muted">The platform repo defines no widgets.</li>}>
              {(w) => (
                <li class="card">
                  <strong>{w.metadata.title ?? w.metadata.name}</strong>
                  {w.metadata.description && <p>{w.metadata.description}</p>}
                  <p class="muted">
                    <code>{w.metadata.name}</code> · {w.spec.view.type} view · {sourceSummary(w.spec)}
                  </p>
                </li>
              )}
            </For>
          </ul>
        </Match>
      </Switch>
    </section>
  );
}
