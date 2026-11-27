import { useLocation, useNavigate, useParams } from "@solidjs/router";
import { createResource, createSignal, Match, onMount, Show, Switch } from "solid-js";
import { commands, type DashboardView, type RepoSource, type SyncPlan } from "../bindings";
import { type IpcError, toIpcError, unwrap } from "../ipc";
import { NarrowBar } from "../components/NarrowBar";
import { Sidebar } from "../components/Sidebar";
import { ChangeLine, hasAnyCapability, shortCommit, SyncReview } from "../components/SyncReview";
import { sidebarMode } from "../sidebar";
import { narrow } from "../viewport";
import { Dashboards, safeDecode } from "./Dashboards";
import { Settings } from "./Settings";

const errorCount = (plan: SyncPlan) => plan.diagnostics.filter((d) => d.severity === "error").length;

/** Widgets whose capability change has anything in `side`. */
const withCaps = (plan: SyncPlan, side: "added" | "removed") =>
  Object.entries(plan.capability_changes)
    .filter(([, change]) => hasAnyCapability(change[side]))
    .map(([name]) => name);

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
 * The sidebar names the platform (`pharos.yaml` title, else name, else "Pharos") and carries the sync status, Sync now and Settings.
 * Owns the dashboard list, shared by the sidebar and the dashboard screen: one resource, one refetch.
 */
export function Home(props: {
  source: RepoSource;
  onSynced: () => void;
  onDisconnected: (prev: { url: string; branch: string }) => void;
}) {
  const location = useLocation();
  const navigate = useNavigate();
  const params = useParams<{ id?: string }>();
  const onSettings = () => location.pathname === "/settings";
  const [plan, setPlan] = createSignal<SyncPlan | null>(null);
  const [busy, setBusy] = createSignal<string | null>(null);
  const [error, setError] = createSignal<IpcError>();
  // Result of the last sync: the applied plan (changelog), or that nothing changed.
  const [changelog, setChangelog] = createSignal<SyncPlan | null>(null);
  // Whether the changelog's plan was applied with approval; if not, widgets with new permissions wait.
  const [changelogApproved, setChangelogApproved] = createSignal(false);
  const [upToDate, setUpToDate] = createSignal(false);
  const [widgetsVersion, setWidgetsVersion] = createSignal(0);
  const neverSynced = () => props.source.synced_commit == null;
  const [platform] = createResource(
    () => widgetsVersion(),
    () => unwrap(commands.getPlatform()).catch(() => null),
  );
  const title = () => platform.latest?.metadata.title ?? platform.latest?.metadata.name ?? "Pharos";
  const inputLabel = (key: string) => platform.latest?.spec.inputs?.[key]?.label;
  const [dashboards, { refetch: refetchDashboards }] = createResource(
    () => widgetsVersion(),
    () => unwrap(commands.listDashboards()),
  );
  const dashboardList = () => (dashboards.error ? [] : (dashboards.latest ?? []));
  // The engine puts the landing dashboard first; computed once, for the sidebar and the page header.
  const landingId = () => dashboardList()[0]?.id;
  const openDashboard = (id: string) => navigate(`/d/${encodeURIComponent(id)}`);
  // A new dashboard is opened once it is listed, else the unknown-id fallback would bounce to the first.
  const onCreated = async (view: DashboardView) => {
    await refetchDashboards();
    openDashboard(view.id);
  };

  // Sync progress or result, shown in the sidebar's always-mounted live region.
  const status = () => busy() ?? (upToDate() ? <span data-testid="up-to-date">Already up to date.</span> : null);

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

  const applied = (p: SyncPlan, approve: boolean) => {
    setPlan(null);
    setChangelog(p);
    setChangelogApproved(approve);
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
        applied(next, false);
      }
    });

  const apply = (approve: boolean) => {
    const p = plan();
    if (!p) return;
    return run("Applying…", async () => {
      await unwrap(commands.applySync(approve));
      applied(p, approve);
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
    <div class="app-shell" classList={{ rail: sidebarMode() === "rail" }}>
      <Show
        when={!narrow()}
        fallback={
          <NarrowBar
            dashboards={dashboardList()}
            current={params.id === undefined ? undefined : safeDecode(params.id)}
            landing={landingId()}
            onOpen={openDashboard}
            onCreated={onCreated}
            title={title()}
            status={status()}
            onSync={!neverSynced() && !plan() ? sync : undefined}
            syncing={!!busy()}
            onSettings={() => navigate("/settings")}
            settingsCurrent={onSettings()}
          />
        }
      >
        <Sidebar
          dashboards={dashboardList()}
          current={params.id === undefined ? undefined : safeDecode(params.id)}
          landing={landingId()}
          onOpen={openDashboard}
          onCreated={onCreated}
          title={title()}
          source={props.source}
          status={status()}
          onSync={!neverSynced() && !plan() ? sync : undefined}
          syncing={!!busy()}
          onSettings={() => navigate("/settings")}
          settingsCurrent={onSettings()}
          neverSynced={neverSynced()}
        />
      </Show>
      <main class="main">
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
                <Show when={withCaps(p(), "removed").length > 0}>
                  <li>
                    <strong>Permissions removed:</strong> {withCaps(p(), "removed").join(", ")}
                  </li>
                </Show>
                <Show when={!changelogApproved() && withCaps(p(), "added").length > 0}>
                  <li>
                    <strong>Waiting for approval:</strong> {withCaps(p(), "added").join(", ")}
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
            <Settings
              source={props.source}
              onChanged={() => setWidgetsVersion((v) => v + 1)}
              onDisconnected={props.onDisconnected}
            />
          </Match>
          <Match when={!neverSynced()}>
            <Dashboards
              dashboards={dashboards}
              refetch={refetchDashboards}
              landing={landingId()}
              onCreated={onCreated}
              version={widgetsVersion()}
              inputLabel={inputLabel}
            />
          </Match>
          <Match when={!busy() && !error()}>
            <header class="page-head">
              <h1>{title()}</h1>
            </header>
            <div class="notice">
              <p>Nothing from the platform repo is applied yet.</p>
              <button type="button" class="primary" onClick={sync}>
                Review the repo
              </button>
            </div>
          </Match>
        </Switch>
      </main>
    </div>
  );
}
