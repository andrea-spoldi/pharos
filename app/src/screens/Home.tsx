import { createSignal, Match, onMount, Show, Switch } from "solid-js";
import { commands, type RepoSource, type SyncPlan } from "../bindings";
import { type IpcError, toIpcError, unwrap } from "../ipc";
import { shortCommit, SyncReview } from "../components/SyncReview";
import { WidgetList } from "../components/WidgetList";

/**
 * Connected state. A plan waiting in the engine is reviewed first; a repo that was never
 * applied is planned straight away, so the first launch after onboarding lands on consent.
 */
export function Home(props: { source: RepoSource; onSynced: () => void }) {
  const [plan, setPlan] = createSignal<SyncPlan | null>(null);
  const [busy, setBusy] = createSignal<string | null>(null);
  const [error, setError] = createSignal<IpcError>();
  const neverSynced = () => props.source.synced_commit == null;

  const run = async (label: string, step: () => Promise<void>) => {
    setBusy(label);
    setError(undefined);
    try {
      await step();
    } catch (err) {
      setError(toIpcError(err));
    } finally {
      setBusy(null);
    }
  };

  const review = () =>
    run("Fetching and checking the repo…", async () => {
      setPlan(await unwrap(commands.planSync()));
    });

  const apply = (consent: boolean) =>
    run("Applying…", async () => {
      await unwrap(commands.applySync(consent));
      setPlan(null);
      props.onSynced();
    });

  const discard = () =>
    run("Discarding…", async () => {
      await unwrap(commands.discardSync());
      setPlan(null);
    });

  onMount(() =>
    run("Loading…", async () => {
      const pending = await unwrap(commands.pendingSync());
      if (pending) setPlan(pending);
      else if (neverSynced()) setPlan(await unwrap(commands.planSync()));
    }),
  );

  return (
    <main class="screen">
      <header>
        <h1>Local IDP</h1>
        <p class="muted" data-testid="source">
          {props.source.url} · {props.source.branch}
          <Show when={props.source.synced_commit}>{(c) => <> · <code>{shortCommit(c())}</code></>}</Show>
        </p>
      </header>

      <Show when={busy()}>
        <p class="muted" role="status">
          {busy()}
        </p>
      </Show>

      <Show when={error()}>
        {(err) => (
          <div class="notice danger" role="alert">
            <strong>{err().kind === "stalePlan" ? "The repo changed since this review" : "Sync failed"}</strong>
            <pre>{err().message}</pre>
            <button type="button" onClick={review} disabled={!!busy()}>
              Review again
            </button>
          </div>
        )}
      </Show>

      <Switch>
        <Match when={plan()}>
          {(p) => (
            <SyncReview
              plan={p()}
              firstSync={neverSynced()}
              busy={!!busy()}
              onApply={apply}
              onDiscard={discard}
              onReview={review}
            />
          )}
        </Match>
        <Match when={busy() || error()}>{null}</Match>
        <Match when={neverSynced()}>
          <div class="notice">
            <p>Nothing from the platform repo is applied yet.</p>
            <button type="button" class="primary" onClick={review}>
              Review the repo
            </button>
          </div>
        </Match>
        <Match when={true}>
          <WidgetList />
        </Match>
      </Switch>
    </main>
  );
}
