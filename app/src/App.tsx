import { createResource, Match, Switch } from "solid-js";
import { commands } from "./bindings";
import { unwrap } from "./ipc";
import { Home } from "./screens/Home";
import { Onboarding } from "./screens/Onboarding";

export function App() {
  const [source, { mutate, refetch }] = createResource(() => unwrap(commands.getSource()));

  return (
    <Switch>
      <Match when={source.error}>
        <main class="screen">
          <div class="notice danger" role="alert">
            <strong>Could not read the app's local state</strong>
            <pre>{String(source.error?.message ?? source.error)}</pre>
          </div>
        </main>
      </Match>
      {/* Only the first load blanks the screen; a refetch after a sync keeps Home mounted. */}
      <Match when={source.state === "pending"}>{null}</Match>
      <Match when={source() === null}>
        <Onboarding onConnected={(s) => mutate(s)} />
      </Match>
      <Match when={source()}>{(s) => <Home source={s()} onSynced={refetch} />}</Match>
    </Switch>
  );
}
