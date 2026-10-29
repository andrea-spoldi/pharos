import type { Nodes } from "mdast";
import { fromMarkdown } from "mdast-util-from-markdown";
import { For, type JSX } from "solid-js";
import { Dynamic } from "solid-js/web";

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
