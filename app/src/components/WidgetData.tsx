import { createResource, Match, Switch } from "solid-js";
import { Dynamic } from "solid-js/web";
import { commands, type Manifest, type WidgetSpec } from "../bindings";
import { unwrap, type IpcError } from "../ipc";
import { CardView, ListView, TableView } from "../views";

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

/** Runs a file widget from its stored manifest (only the name is sent) and renders its view. */
export function WidgetData(props: { widget: Manifest<WidgetSpec>; version: number }) {
  const [data] = createResource(
    () => [props.widget.metadata.name, props.version] as const,
    ([name]) => unwrap(commands.runWidget(name)),
  );
  const view = () => {
    const type = props.widget.spec.view.type;
    return type in views ? views[type as keyof typeof views] : undefined;
  };

  return (
    <Switch>
      <Match when={!view()}>
        <p class="muted">The {props.widget.spec.view.type} view is not available yet.</p>
      </Match>
      <Match when={data.error}>
        <div class="notice danger" role="alert">
          <strong>{errorTitle(data.error)}</strong>
          <pre>{String(data.error?.message ?? data.error)}</pre>
        </div>
      </Match>
      <Match when={data.latest}>
        {(d) => <Dynamic component={view()} columns={props.widget.spec.view.columns} data={d()} />}
      </Match>
    </Switch>
  );
}
