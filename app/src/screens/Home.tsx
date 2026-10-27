import type { RepoSource } from "../bindings";

/** Shell for the connected state; sync review and the widget list land here next. */
export function Home(props: { source: RepoSource }) {
  return (
    <main class="screen">
      <header>
        <h1>Local IDP</h1>
        <p class="muted" data-testid="source">
          {props.source.url} · {props.source.branch}
        </p>
      </header>
    </main>
  );
}
