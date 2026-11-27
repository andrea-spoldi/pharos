import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render as renderUi, screen, waitFor, within } from "@solidjs/testing-library";
import type { JSX } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type DashboardView, type RepoSource, type SyncPlan } from "../bindings";
import { setSidebarMode } from "../sidebar";
import { Home } from "./Home";

window.scrollTo = vi.fn() as typeof window.scrollTo;

/** Home lives inside the router (`#/d/:id`), as in index.tsx. */
let history = createMemoryHistory();
const render = (ui: () => JSX.Element) => {
  history = createMemoryHistory();
  return renderUi(() => (
    <MemoryRouter history={history}>
      <Route path={["/", "/d/:id", "/settings"]} component={ui} />
    </MemoryRouter>
  ));
};

vi.mock("../bindings", () => ({
  commands: {
    pendingSync: vi.fn(),
    planSync: vi.fn(),
    applySync: vi.fn(),
    discardSync: vi.fn(),
    listDashboards: vi.fn(),
    getPlatform: vi.fn(),
    runPlacement: vi.fn(),
    getSettings: vi.fn(),
    getSearchPath: vi.fn(),
    createDashboard: vi.fn(),
    renameDashboard: vi.fn(),
    deleteDashboard: vi.fn(),
  },
}));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

const fresh: RepoSource = {
  id: 1,
  url: "/tmp/platform",
  branch: "main",
  local_path: "/data/sources/1",
  synced_commit: null,
  synced_at: null,
  approved_capability_hash: null,
};
const synced: RepoSource = { ...fresh, synced_commit: "a".repeat(40), approved_capability_hash: "sha256:x" };

const none = { added: [], removed: [], changed: [] };
const firstPlan: SyncPlan = {
  from_commit: null,
  to_commit: "b".repeat(40),
  platform_changed: true,
  widgets: { ...none, added: ["k8s-context", "my-pipelines"] },
  dashboards: { ...none, added: ["home"] },
  diagnostics: [],
  capabilities: {
    "k8s-context": { executables: ["kubectl"] },
    "my-pipelines": { hosts: ["https://gitlab.com"] },
  },
  capability_hash: "sha256:y",
  capability_changes: {
    "k8s-context": { added: { executables: ["kubectl"] }, removed: {} },
    "my-pipelines": { added: { hosts: ["https://gitlab.com"] }, removed: {} },
  },
  needs_consent: true,
};

const home: DashboardView = {
  id: "p:home",
  origin: "platform",
  editable: false,
  metadata: { name: "home", title: "Home", version: null, description: null },
  placements: [
    {
      widget: "k8s-context",
      container: false,
      title: null,
      x: 0,
      y: 0,
      w: 4,
      h: 2,
      manifest: {
        apiVersion: "pharos.io/v1alpha1",
        kind: "Widget",
        metadata: { name: "k8s-context", title: "Kubernetes Context", version: null, description: null },
        spec: {
          source: { type: "command", executable: "kubectl", args: [], timeout: null },
          refresh: null,
          inputs: {},
          view: { type: "kv", columns: [], content: null },
          permissions: { hosts: [] },
        },
      },
      status: { kind: "notAvailable", feature: "command sources" },
      reads: null,
      id: null,
      children: [],
    },
  ],
};
const platform = {
  apiVersion: "pharos.io/v1alpha1",
  kind: "Platform",
  metadata: { name: "acme-platform", title: "ACME Developer Portal", version: null, description: null },
  spec: { inputs: {}, secrets: [], widgets: "widgets/", dashboards: "dashboards/" },
};

const sidebar = () => within(screen.getByRole("complementary"));

describe("Home", () => {
  beforeEach(() => {
    setSidebarMode("open"); // jsdom is 1024 wide, which defaults to the rail
    for (const fn of Object.values(commands)) vi.mocked(fn).mockReset();
    vi.mocked(commands.pendingSync).mockResolvedValue(ok(null));
    vi.mocked(commands.planSync).mockResolvedValue(ok(firstPlan));
    vi.mocked(commands.applySync).mockResolvedValue(ok(null));
    vi.mocked(commands.discardSync).mockResolvedValue(ok(null));
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([home]));
    vi.mocked(commands.getPlatform).mockResolvedValue(ok(platform));
  });

  it("plans the first sync and asks consent for capabilities by widget", async () => {
    const onSynced = vi.fn();
    render(() => <Home source={fresh} onSynced={onSynced} onDisconnected={() => {}} />);

    const caps = await screen.findByTestId("capabilities");
    expect(caps.textContent).toContain("k8s-context");
    expect(caps.textContent).toContain("Run kubectl");
    expect(caps.textContent).toContain("Call https://gitlab.com");
    expect(screen.getByText(/added k8s-context, my-pipelines/)).toBeTruthy();

    fireEvent.click(screen.getByText("Approve and apply"));
    await waitFor(() => expect(onSynced).toHaveBeenCalled());
    expect(commands.applySync).toHaveBeenCalledWith(true);
  });

  it("resumes a plan already waiting in the engine", async () => {
    vi.mocked(commands.pendingSync).mockResolvedValue(ok(firstPlan));
    render(() => <Home source={fresh} onSynced={() => {}} onDisconnected={() => {}} />);
    await screen.findByTestId("capabilities");
    expect(commands.planSync).not.toHaveBeenCalled();
  });

  it("blocks apply on lint errors", async () => {
    vi.mocked(commands.planSync).mockResolvedValue(
      ok({
        ...firstPlan,
        diagnostics: [
          {
            severity: "error",
            code: "path-escape",
            file: "widgets/x/widget.yaml",
            path: "spec.source.path",
            message: "path escapes the repository root",
          },
        ],
      }),
    );
    render(() => <Home source={fresh} onSynced={() => {}} onDisconnected={() => {}} />);

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("1 lint error: this version can't be applied");
    expect(alert.textContent).toContain("widgets/x/widget.yaml spec.source.path");
    expect(screen.queryByText("Approve and apply")).toBeNull();
    expect(screen.queryByTestId("capabilities")).toBeNull();
  });

  it("offers a new review when the clone moved under the plan", async () => {
    vi.mocked(commands.applySync).mockResolvedValue({
      status: "error",
      error: { kind: "stalePlan", message: "the local clone changed after the sync was planned" },
    });
    render(() => <Home source={fresh} onSynced={() => {}} onDisconnected={() => {}} />);
    fireEvent.click(await screen.findByText("Approve and apply"));

    expect(await screen.findByText("The repo changed since this review")).toBeTruthy();
    fireEvent.click(screen.getAllByText("Review again")[0]);
    await waitFor(() => expect(commands.planSync).toHaveBeenCalledTimes(2));
  });

  it("discards back to an empty state when nothing was applied", async () => {
    render(() => <Home source={fresh} onSynced={() => {}} onDisconnected={() => {}} />);
    fireEvent.click(await screen.findByText("Discard"));
    expect(await screen.findByText("Nothing from the platform repo is applied yet.")).toBeTruthy();
    expect(commands.discardSync).toHaveBeenCalled();
  });

  describe("once synced", () => {
    const upToDate: SyncPlan = {
      ...firstPlan,
      from_commit: synced.synced_commit,
      to_commit: synced.synced_commit!,
      platform_changed: false,
      widgets: none,
      dashboards: none,
      capability_changes: {},
      needs_consent: false,
    };
    const update: SyncPlan = {
      ...upToDate,
      to_commit: "c".repeat(40),
      widgets: { ...none, changed: ["my-pipelines"] },
    };

    it("syncs on launch while the dashboard stays shown", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);

      expect(await screen.findByText("Kubernetes Context")).toBeTruthy();
      expect(screen.getByText("command sources arrive in a later version.")).toBeTruthy();
      expect(await screen.findByTestId("up-to-date")).toBeTruthy();
      expect(commands.planSync).toHaveBeenCalledTimes(1);
      // The no-op plan is dropped so it is not offered for review later.
      expect(commands.discardSync).toHaveBeenCalled();
      expect(commands.applySync).not.toHaveBeenCalled();
      // Nothing changed, but the widgets reload so http data is fresh.
      await waitFor(() => expect(commands.listDashboards).toHaveBeenCalledTimes(2));
    });

    it("applies unchanged capabilities without asking and shows the changelog", async () => {
      vi.mocked(commands.planSync).mockResolvedValueOnce(ok(upToDate)).mockResolvedValueOnce(ok(update));
      const onSynced = vi.fn();
      render(() => <Home source={synced} onSynced={onSynced} onDisconnected={() => {}} />);
      await screen.findByTestId("up-to-date");

      fireEvent.click(screen.getByText("Sync now"));
      const changelog = await screen.findByTestId("changelog");
      expect(changelog.textContent).toContain("Synced to ccccccc");
      expect(changelog.textContent).toContain("changed my-pipelines");
      expect(commands.applySync).toHaveBeenCalledWith(false);
      expect(onSynced).toHaveBeenCalled();
      // Initial load, the up-to-date launch sync, then the applied update.
      await waitFor(() => expect(commands.listDashboards).toHaveBeenCalledTimes(3));
    });

    it("asks again when capabilities changed", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(
        ok({
          ...update,
          capability_changes: { "status-page": { added: { hosts: ["https://status.acme.dev"] }, removed: {} } },
          needs_consent: true,
        }),
      );
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);

      expect(await screen.findByText("Review changes")).toBeTruthy();
      expect(screen.getByText("Approve and apply")).toBeTruthy();
      expect(commands.applySync).not.toHaveBeenCalled();
      expect(screen.queryByText("Sync now")).toBeNull();
    });

    it("lists widgets waiting for approval after applying without approval", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(
        ok({
          ...update,
          capability_changes: {
            "status-page": { added: { hosts: ["https://status.acme.dev"] }, removed: {} },
            legacy: { added: {}, removed: { executables: ["kubectl"] } },
          },
          needs_consent: true,
        }),
      );
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      fireEvent.click(await screen.findByText("Apply, keep these blocked"));

      const changelog = await screen.findByTestId("changelog");
      expect(commands.applySync).toHaveBeenCalledWith(false);
      expect(changelog.textContent).toContain("Waiting for approval: status-page");
      expect(changelog.textContent).toContain("Permissions removed: legacy");
    });

    it("does not list waiting widgets after approving", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(
        ok({
          ...update,
          capability_changes: { "status-page": { added: { hosts: ["https://status.acme.dev"] }, removed: {} } },
          needs_consent: true,
        }),
      );
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      fireEvent.click(await screen.findByText("Approve and apply"));

      const changelog = await screen.findByTestId("changelog");
      expect(commands.applySync).toHaveBeenCalledWith(true);
      expect(changelog.textContent).not.toContain("Waiting for approval");
    });

    it("keeps the dashboard when the sync fails", async () => {
      vi.mocked(commands.planSync).mockResolvedValue({
        status: "error",
        error: { kind: "git", message: "`git fetch --quiet --prune origin main` failed: offline" },
      });
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);

      expect((await screen.findByRole("alert")).textContent).toContain("offline");
      expect(await screen.findByRole("navigation", { name: "Platform dashboards" })).toBeTruthy();
      expect(screen.getByText("Try again")).toBeTruthy();
    });

    it("settings_has_aria_current_and_no_back_link", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      vi.mocked(commands.getSettings).mockResolvedValue(
        ok({ source: synced, platform: null, inputs: [], secrets: [], unused: [] }),
      );
      vi.mocked(commands.getSearchPath).mockResolvedValue(ok([]));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      await screen.findByText("Kubernetes Context");
      await waitFor(() => expect(history.get()).toBe("/d/p%3Ahome"));

      fireEvent.click(sidebar().getByRole("button", { name: "Settings" }));
      expect(await screen.findByRole("heading", { name: "Repository" })).toBeTruthy();
      expect(history.get()).toBe("/settings");
      expect(screen.queryByText("Kubernetes Context")).toBeNull();
      expect(sidebar().getByRole("button", { name: "Settings" }).getAttribute("aria-current")).toBe("page");
      expect(screen.queryByRole("button", { name: /Back to dashboards/ })).toBeNull();

      fireEvent.click(sidebar().getByRole("button", { name: "Home" }));
      await waitFor(() => expect(history.get()).toBe("/d/p%3Ahome"));
      expect(await screen.findByText("Kubernetes Context")).toBeTruthy();
    });

    it("sidebar_refreshes_after_create_rename_delete", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      const mine: DashboardView = { ...home, id: "u:1", origin: "user", editable: true, placements: [] };
      const titled = (title: string): DashboardView => ({
        ...mine,
        metadata: { ...mine.metadata, name: "mine", title },
      });
      vi.mocked(commands.createDashboard).mockResolvedValue(ok(titled("Mine")));
      vi.mocked(commands.renameDashboard).mockResolvedValue(ok(null));
      vi.mocked(commands.deleteDashboard).mockResolvedValue(ok(null));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      await screen.findByTestId("up-to-date");
      await waitFor(() => expect(commands.listDashboards).toHaveBeenCalledTimes(2));
      // The same landmark element throughout: the sidebar updates in place, it is not remounted.
      const nav = screen.getByRole("navigation", { name: "Your dashboards" });
      const submit = (button: string, value?: string) => {
        const dialog = within(screen.getByRole("dialog"));
        if (value !== undefined) fireEvent.input(dialog.getByLabelText("Title"), { target: { value } });
        fireEvent.click(dialog.getByRole("button", { name: button }));
      };

      vi.mocked(commands.listDashboards).mockResolvedValue(ok([home, titled("Mine")]));
      fireEvent.click(within(nav).getByRole("button", { name: "+ New dashboard" }));
      submit("Create", "Mine");
      expect(await within(nav).findByRole("button", { name: "Mine" })).toBeTruthy();
      await waitFor(() => expect(history.get()).toBe("/d/u%3A1"));

      vi.mocked(commands.listDashboards).mockResolvedValue(ok([home, titled("Renamed")]));
      fireEvent.click(await screen.findByRole("button", { name: "Rename" }));
      submit("Rename", "Renamed");
      expect(await within(nav).findByRole("button", { name: "Renamed" })).toBeTruthy();
      expect(within(nav).queryByRole("button", { name: "Mine" })).toBeNull();

      vi.mocked(commands.listDashboards).mockResolvedValue(ok([home]));
      fireEvent.click(screen.getByRole("button", { name: "Delete" }));
      submit("Delete");
      await waitFor(() => expect(within(nav).queryByRole("button", { name: "Renamed" })).toBeNull());
      expect(screen.getByRole("navigation", { name: "Your dashboards" })).toBe(nav);
      await waitFor(() => expect(history.get()).toBe("/d/p%3Ahome"));
    });

    it("sync_status_region_is_mounted_before_sync", () => {
      vi.mocked(commands.planSync).mockReturnValue(new Promise(() => {}));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      // Present on the first render, inside the sidebar footer: a live region announces only changes.
      const region = sidebar().getByRole("status");
      expect(region.classList.contains("sync-status")).toBe(true);
    });

    it("sync_now_is_disabled_and_reads_syncing_while_in_flight", async () => {
      vi.mocked(commands.planSync).mockReturnValue(new Promise(() => {}));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      const button = (await sidebar().findByRole("button", { name: "Syncing…" })) as HTMLButtonElement;
      expect(button.disabled).toBe(true);
      expect(sidebar().queryByRole("button", { name: "Sync now" })).toBeNull();
    });

    it("footer_shows_sync_status_commit_and_sync_now", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      expect((await sidebar().findByTestId("up-to-date")).closest("[role=status]")).toBe(sidebar().getByRole("status"));
      expect(sidebar().getByText("aaaaaaa")).toBeTruthy();
      expect(sidebar().getByRole("button", { name: "Sync now" })).toBeTruthy();
    });

    it("sidebar_refreshes_after_sync", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      const other: DashboardView = {
        ...home,
        id: "p:ops",
        metadata: { name: "ops", title: "Ops", version: null, description: null },
      };
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      await screen.findByTestId("up-to-date");
      expect(sidebar().queryByRole("button", { name: "Ops" })).toBeNull();

      vi.mocked(commands.listDashboards).mockResolvedValue(ok([home, other]));
      fireEvent.click(sidebar().getByRole("button", { name: "Sync now" }));
      expect(await sidebar().findByRole("button", { name: "Ops" })).toBeTruthy();
    });

    it("shows_the_source_as_owner_repo_and_branch", async () => {
      const remote = { ...synced, url: "https://github.com/acme/platform.git" };
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      render(() => <Home source={remote} onSynced={() => {}} onDisconnected={() => {}} />);
      const line = await screen.findByTestId("source");
      expect(line.textContent).toContain("acme/platform · main");
      expect(line.getAttribute("title")).toBe("https://github.com/acme/platform.git");
      expect(sidebar().getByText(/https:\/\/github.com\/acme\/platform.git/)).toBeTruthy();
    });

    it("never_synced_sidebar_has_no_dashboard_sections", async () => {
      render(() => <Home source={fresh} onSynced={() => {}} onDisconnected={() => {}} />);
      await screen.findByTestId("capabilities");
      expect(screen.getByTestId("brand").textContent).toBe("ACME Developer Portal");
      expect(screen.queryByRole("navigation", { name: "Platform dashboards" })).toBeNull();
      expect(screen.queryByRole("navigation", { name: "Your dashboards" })).toBeNull();
      expect(sidebar().getByRole("status")).toBeTruthy();
      expect(sidebar().getByRole("button", { name: "Settings" })).toBeTruthy();
      expect(sidebar().queryByRole("button", { name: "Sync now" })).toBeNull();
    });

    it("shows the platform title", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      await waitFor(() => expect(screen.getByTestId("brand").textContent).toBe("ACME Developer Portal"));
    });

    it("falls back to Pharos", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      vi.mocked(commands.getPlatform).mockResolvedValue(ok(null));
      render(() => <Home source={synced} onSynced={() => {}} onDisconnected={() => {}} />);
      await screen.findByTestId("up-to-date");
      expect(screen.getByTestId("brand").textContent).toBe("Pharos");
    });
  });
});
