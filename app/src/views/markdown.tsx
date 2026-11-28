import type { Nodes } from "mdast";
import { fromMarkdown } from "mdast-util-from-markdown";
import { For, Show, type JSX } from "solid-js";
import { Dynamic } from "solid-js/web";
import type { ViewProps } from ".";
import { display, field, type Row } from "./fields";

/** Rendered markdown is capped so a huge document cannot stall the UI. */
export const MAX_MARKDOWN_BYTES = 256 * 1024;

/** The first `maxBytes` of UTF-8 of `text`, cut on a character boundary. */
export function capText(text: string, maxBytes: number): { text: string; cut: boolean } {
  const bytes = new TextEncoder().encode(text);
  if (bytes.length <= maxBytes) return { text, cut: false };
  let end = maxBytes;
  while (end > 0 && (bytes[end] & 0xc0) === 0x80) end--;
  return { text: new TextDecoder().decode(bytes.subarray(0, end)), cut: true };
}

/** Markdown fetched by the widget's source: the `value` field, or the one column the manifest names. */
export function MarkdownSourceView(props: ViewProps) {
  const key = () => (props.columns?.length === 1 ? props.columns[0].field : "value");
  const doc = (row: Row) => {
    const v = field(row, key());
    return capText(typeof v === "string" ? v : display(v), MAX_MARKDOWN_BYTES);
  };
  return (
    <For each={props.data.rows}>
      {(row) => (
        <>
          <MarkdownView content={doc(row).text} />
          <Show when={doc(row).cut}>
            <p class="muted">Only the first 256 KB is shown.</p>
          </Show>
        </>
      )}
    </For>
  );
}

/**
 * Renders markdown from the repo as Solid elements, never as HTML: raw HTML shows as text,
 * links are not clickable (no opener yet) and images show their alt text (the CSP blocks them).
 */
export function MarkdownView(props: { content: string }) {
  return (
    <div class="markdown">
      <Children nodes={fromMarkdown(props.content).children} />
    </div>
  );
}

function Children(props: { nodes: Nodes[] }) {
  return <For each={props.nodes}>{(n) => <Node node={n} />}</For>;
}

function Node(props: { node: Nodes }): JSX.Element {
  const n = props.node;
  const kids = () => ("children" in n ? <Children nodes={n.children as Nodes[]} /> : null);
  switch (n.type) {
    case "heading":
      return <Dynamic component={`h${Math.min(n.depth + 2, 6)}`}>{kids()}</Dynamic>;
    case "paragraph":
      return <p>{kids()}</p>;
    case "emphasis":
      return <em>{kids()}</em>;
    case "strong":
      return <strong>{kids()}</strong>;
    case "inlineCode":
      return <code>{n.value}</code>;
    case "code":
      return (
        <pre>
          <code>{n.value}</code>
        </pre>
      );
    case "list":
      return <Dynamic component={n.ordered ? "ol" : "ul"}>{kids()}</Dynamic>;
    case "listItem":
      return <li>{kids()}</li>;
    case "blockquote":
      return <blockquote>{kids()}</blockquote>;
    case "thematicBreak":
      return <hr />;
    case "break":
      return <br />;
    case "link":
      return (
        <>
          {kids()} <span class="muted">({n.url})</span>
        </>
      );
    case "image":
      return <>{n.alt ?? ""}</>;
    case "text":
    case "html":
      return <>{n.value}</>;
    default:
      return kids();
  }
}
