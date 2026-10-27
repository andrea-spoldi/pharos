import { createResource, Show } from "solid-js";
import { commands } from "./bindings";

export function App() {
  const [pong] = createResource(() => commands.ping());

  return (
    <main class="app">
      <h1>Local IDP</h1>
      <Show when={pong()} fallback={<p>Connecting…</p>}>
        <p data-testid="ping">{pong()}</p>
      </Show>
    </main>
  );
}
