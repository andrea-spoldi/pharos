import { useLocation, useNavigate } from "@solidjs/router";
import { createEffect, createResource, createSignal, Match, onMount, Show, Switch } from "solid-js";
import { commands, type RepoSource, type SyncPlan } from "../bindings";
import { type IpcError, toIpcError, unwrap } from "../ipc";
import { ChangeLine, shortCommit, SyncReview } from "../components/SyncReview";
import { Dashboards } from "./Dashboards";
import { Settings } from "./Settings";

const errorCount = (plan: SyncPlan) => plan.diagnostics.filter((d) => d.severity === "error").length;

/** Mirrors `SyncPlan::is_up_to_date` in the engine. */
const isUpToDate = (plan: SyncPlan) =>
  plan.from_commit === plan.to_commit &&
  !plan.platform_changed &&
  !plan.needs_consent &&
  [plan.widgets, plan.dashboards].every((c) => !c.added.length && !c.changed.length && !c.removed.length);

/**
 * Connected state. Syncs on launch and on demand (D-004), never blocking the widget list.
 * A plan with unchanged capabilities and no lint errors is applied without asking and
 * reported as a changelog (restart-plan §3); anything else is reviewed first.
 * The header names the platform (`pharos.yaml` title, else name, else "Pharos").
 */
export function Home(props: {
  source: RepoSource;
  onSynced: () => void;
  onDisconnected: (prev: { url: string; branch: string }) => void;
}) {
  const location = useLocation();
  const navigate = useNavigate();
  const onSettings = () => location.pathname === "/settings";
  // Where "Back to dashboards" returns to.
  const [lastDashboard, setLastDashboard] = createSignal("/");
  createEffect(() => {
    if (location.pathname.startsWith("/d/")) setLastDashboard(location.pathname);
  });
  const [plan, setPlan] = createSignal<SyncPlan | null>(null);
  const [busy, setBusy] = createSignal<string | null>(null);
  const [error, setError] = createSignal<IpcError>();
  // Result of the last sync: the applied plan (changelog), or that nothing changed.
  const [changelog, setChangelog] = createSignal<SyncPlan | null>(null);
  const [upToDate, setUpToDate] = createSignal(false);
  const [widgetsVersion, setWidgetsVersion] = createSignal(0);
  const neverSynced = () => props.source.synced_commit == null;
  const [platform] = createResource(
    () => widgetsVersion(),
    () => unwrap(commands.getPlatform()).catch(() => null),
  );
  const title = () => platform.latest?.metadata.title ?? platform.latest?.metadata.name ?? "Pharos";
  const inputLabel = (key: string) => platform.latest?.spec.inputs?.[key]?.label;

  const run = async (label: string, step: () => Promise<void>) => {
    setBusy(label);
    setError(undefined);
    setChangelog(null);
    setUpToDate(false);
    try {
      await step();
    } catch (err) {
      setError(toIpcError(err));
    } finally {
      setBusy(null);
    }
  };

  const applied = (p: SyncPlan) => {
    setPlan(null);
    setChangelog(p);
    setWidgetsVersion((v) => v + 1);
    props.onSynced();
  };

  const sync = () =>
    run("Fetching and checking the repo…", async () => {
      const next = await unwrap(commands.planSync());
      if (neverSynced() || next.needs_consent || errorCount(next) > 0) {
        setPlan(next);
      } else if (isUpToDate(next)) {
        await unwrap(commands.discardSync());
        setUpToDate(true);
        // Nothing to apply, but Sync now still re-runs the widgets: it is how http data
        // refreshes until widgets refresh on their own.
        setWidgetsVersion((v) => v + 1);
      } else {
        await unwrap(commands.applySync(false));
        applied(next);
      }
    });

  const apply = (consent: boolean) => {
    const p = plan();
    if (!p) return;
    return run("Applying…", async () => {
      await unwrap(commands.applySync(consent));
      applied(p);
    });
  };

  const discard = () =>
    run("Discarding…", async () => {
      await unwrap(commands.discardSync());
      setPlan(null);
    });

  onMount(async () => {
    try {
      const pending = await unwrap(commands.pendingSync());
      if (pending) return setPlan(pending);
    } catch (err) {
      return setError(toIpcError(err));
    }
    await sync();
  });

  return (
    <main class="screen wide">
      <header class="row spread topbar">
        <div>
          <h1>{title()}</h1>
          <p class="muted" data-testid="source">
            <code>{props.source.url}</code> · <code>{props.source.branch}</code>
            <Show when={props.source.synced_commit}>
              {(c) => (
                <>
                  {" "}
                  · <code>{shortCommit(c())}</code>
                </>
              )}
            </Show>
          </p>
        </div>
        <div class="row">
          {/* Sync progress sits beside the actions, so a launch sync never shifts the dashboard.
              Always mounted: a live region only announces text that changes inside it. */}
          <p class="muted sync-status" role="status">
            {busy() ?? (upToDate() ? <span data-testid="up-to-date">Already up to date.</span> : null)}
          </p>
          <Show when={!onSettings()}>
            <button type="button" onClick={() => navigate("/settings")}>
              Settings
            </button>
          </Show>
          <Show when={!neverSynced() && !plan()}>
            <button type="button" onClick={sync} disabled={!!busy()}>
              {busy() ? "Syncing…" : "Sync now"}
            </button>
          </Show>
        </div>
      </header>

      <Show when={changelog()}>
        {(p) => (
          <div class="notice" role="status" data-testid="changelog">
            <strong>
              Synced to <code>{shortCommit(p().to_commit)}</code>
            </strong>
            <ul class="plain changelog">
              <ChangeLine label="Widgets" changes={p().widgets} />
              <ChangeLine label="Dashboards" changes={p().dashboards} />
              <Show when={p().platform_changed}>
                <li>
                  <strong>Platform settings</strong> changed
                </li>
              </Show>
            </ul>
            <button type="button" class="link" onClick={() => setChangelog(null)}>
              Dismiss
            </button>
          </div>
        )}
      </Show>

      <Show when={error()}>
        {(err) => (
          <div class="notice danger" role="alert">
            <strong>{err().kind === "stalePlan" ? "The repo changed since this review" : "Sync failed"}</strong>
            <pre>{err().message}</pre>
            <button type="button" onClick={sync} disabled={!!busy()}>
              {err().kind === "stalePlan" ? "Review again" : "Try again"}
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
              onReview={sync}
            />
          )}
        </Match>
        <Match when={onSettings()}>
          <div class="stack">
            <button type="button" class="link back" onClick={() => navigate(lastDashboard())}>
              ← Back to dashboards
            </button>
            <Settings
              source={props.source}
              onChanged={() => setWidgetsVersion((v) => v + 1)}
              onDisconnected={props.onDisconnected}
            />
          </div>
        </Match>
        <Match when={!neverSynced()}>
          <Dashboards version={widgetsVersion()} inputLabel={inputLabel} />
        </Match>
        <Match when={!busy() && !error()}>
          <div class="notice">
            <p>Nothing from the platform repo is applied yet.</p>
            <button type="button" class="primary" onClick={sync}>
              Review the repo
            </button>
          </div>
        </Match>
      </Switch>
    </main>
  );
}
