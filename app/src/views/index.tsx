import { For, Show, type JSX } from "solid-js";
import type { Column, WidgetData } from "../bindings";
import { columnsFor, display, field, MAX_FILES } from "./fields";

export type ViewProps = { columns?: Column[]; data: WidgetData };

/** Truncation note shared by every view; the widget frame owns the empty state. */
function Frame(props: { data: WidgetData; children: JSX.Element }) {
  return (
    <div class="view">
      {props.children}
      <Show when={props.data.truncated}>
        <p class="muted">Only the first {MAX_FILES} files are shown.</p>
      </Show>
    </div>
  );
}

export function TableView(props: ViewProps) {
  const cols = () => columnsFor(props.columns, props.data.rows);
  return (
    <Frame data={props.data}>
      <div class="table-scroll">
        <table>
          <thead>
            <tr>
              <For each={cols()}>{(c) => <th scope="col">{c.label}</th>}</For>
            </tr>
          </thead>
          <tbody>
            <For each={props.data.rows}>
              {(row) => (
                <tr>
                  <For each={cols()}>{(c) => <td>{display(field(row, c.field))}</td>}</For>
                </tr>
              )}
            </For>
          </tbody>
        </table>
      </div>
    </Frame>
  );
}

export function ListView(props: ViewProps) {
  const cols = () => columnsFor(props.columns, props.data.rows);
  return (
    <Frame data={props.data}>
      <ul class="plain">
        <For each={props.data.rows}>
          {(row) => {
            const [first, ...rest] = cols();
            return (
              <li>
                <strong>{first && display(field(row, first.field))}</strong>
                <For each={rest}>
                  {(c) => (
                    <span class="muted">
                      {" · "}
                      {c.label}: {display(field(row, c.field))}
                    </span>
                  )}
                </For>
              </li>
            );
          }}
        </For>
      </ul>
    </Frame>
  );
}

export function CardView(props: ViewProps) {
  const cols = () => columnsFor(props.columns, props.data.rows);
  return (
    <Frame data={props.data}>
      <div class="card-grid">
        <For each={props.data.rows}>
          {(row) => {
            const [first, ...rest] = cols();
            return (
              <article class="card">
                <h3>{first && display(field(row, first.field))}</h3>
                <dl class="fields">
                  <For each={rest}>
                    {(c) => (
                      <>
                        <dt>{c.label}</dt>
                        <dd>{display(field(row, c.field))}</dd>
                      </>
                    )}
                  </For>
                </dl>
              </article>
            );
          }}
        </For>
      </div>
    </Frame>
  );
}

/**
 * Command output as text: a single `{ value }` row (plain stdout) is shown as is, anything
 * else (JSON output, file or http rows) pretty-printed.
 */
export function RawView(props: ViewProps) {
  const text = () => {
    const rows = props.data.rows;
    const only = rows.length === 1 ? rows[0] : undefined;
    if (only && Object.keys(only).length === 1 && typeof only.value === "string") return only.value;
    return JSON.stringify(rows.length === 1 ? rows[0] : rows, null, 2);
  };
  return (
    <Frame data={props.data}>
      <pre class="raw">{text()}</pre>
    </Frame>
  );
}
