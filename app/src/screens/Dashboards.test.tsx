import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type DashboardView, type RepoSource, type SyncPlan } from "../bindings";
import { setSidebarMode } from "../sidebar";
import { setViewport } from "../test-utils";
import { dragControllerFor } from "./Dashboards";
import { Home } from "./Home";

window.scrollTo = vi.fn() as typeof window.scrollTo;

vi.mock("../bindings", () => ({
  commands: {
    pendingSync: vi.fn(),
    planSync: vi.fn(),
    discardSync: vi.fn(),
    applySync: vi.fn(),
    getPlatform: vi.fn(),
    listDashboards: vi.fn(),
    runPlacement: vi.fn(),
    listCatalog: vi.fn(),
    createDashboard: vi.fn(),
    renameDashboard: vi.fn(),
    deleteDashboard: vi.fn(),
    addWidget: vi.fn(),
    removeWidget: vi.fn(),
    setLanding: vi.fn(),
    createGroup: vi.fn(),
    renamePlacement: vi.fn(),
    setLayout: vi.fn(),
    customizeDashboard: vi.fn(),
  },
}));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

function dashboard(name: string, widgets: string[] = ["welcome"]): DashboardView {
  return {
    id: `p:${name}`,
    origin: "platform",
    editable: false,
    metadata: { name, title: name[0].toUpperCase() + name.slice(1), version: null, description: null },
    placements: widgets.map((w, i) => ({
      widget: w,
      container: false,
      title: null,
      x: 0,
      y: i * 2,
      w: 4,
      h: 2,
      manifest: null,
      status: { kind: "missing" },
      reads: null,
      id: null,
      children: [],
    })),
  };
}

const source: RepoSource = {
  id: 1,
  url: "/tmp/platform",
  branch: "main",
  local_path: "/data/sources/1",
  synced_commit: "a".repeat(40),
  synced_at: null,
  approved_capability_hash: "sha256:x",
};
const none = { added: [], removed: [], changed: [] };
const upToDate: SyncPlan = {
  from_commit: source.synced_commit,
  to_commit: source.synced_commit!,
  platform_changed: false,
  widgets: none,
  dashboards: none,
  diagnostics: [],
  capabilities: {},
  capability_hash: "sha256:x",
  capability_changes: {},
  needs_consent: false,
};

/** Renders through Home, which owns the list wiring, so these tests exercise the real thing. */
function renderAt(path: string) {
  const history = createMemoryHistory();
  history.set({ value: path });
  render(() => (
    <MemoryRouter history={history}>
      <Route
        path={["/", "/d/:id"]}
        component={() => <Home source={source} onSynced={() => {}} onDisconnected={() => {}} />}
      />
    </MemoryRouter>
  ));
  return history;
}

/** Lets the launch sync finish, so that a later Sync now is the only thing that re-runs widgets. */
function launchSynced() {
  vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
  return () => screen.findByTestId("up-to-date");
}

const rows = { rows: [{ metadata: { name: "payment-service" }, _source: "p.yaml" }], truncated: false };

/** A fresh list with one runnable file widget; new objects on every call, as after a sync. */
function runnableHome(): DashboardView {
  return {
    id: "p:home",
    origin: "platform",
    editable: false,
    metadata: { name: "home", title: "Home", version: null, description: null },
    placements: [
      {
        widget: "service-catalog",
        container: false,
        title: null,
        x: 0,
        y: 0,
        w: 8,
        h: 4,
        manifest: {
          apiVersion: "pharos.io/v1alpha1",
          kind: "Widget",
          metadata: { name: "service-catalog", title: "Service Catalog", version: null, description: null },
          spec: {
            source: { type: "file", path: "../../catalog/services/", glob: "*.yaml" },
            refresh: null,
            inputs: {},
            view: { type: "table", columns: [{ field: "metadata.name", label: "Service" }], content: null },
            permissions: { hosts: [] },
          },
        },
        status: { kind: "runnable" },
        reads: "catalog/services/*.yaml",
        id: null,
        children: [],
      },
    ],
  };
}

// A landing item's accessible name carries a "(Landing dashboard)" suffix, so match the title prefix.
const tab = (name: string) => screen.getByRole("button", { name: new RegExp(`^${name}\\b`) });

// Home runs a launch sync around every screen; it never finishes unless a test asks, so widgets
// run once per list load.
beforeEach(() => {
  setSidebarMode("open"); // jsdom is 1024 wide, which defaults to the rail
  vi.mocked(commands.pendingSync).mockReset().mockResolvedValue(ok(null));
  vi.mocked(commands.planSync).mockReset().mockReturnValue(new Promise(() => {}));
  vi.mocked(commands.discardSync).mockReset().mockResolvedValue(ok(null));
  vi.mocked(commands.getPlatform).mockReset().mockResolvedValue(ok(null));
});

describe("Dashboards", () => {
  beforeEach(() => {
    vi.mocked(commands.listDashboards).mockReset().mockResolvedValue(ok([dashboard("home"), dashboard("ops", ["k8s"])]));
  });

  it("redirects to the first dashboard", async () => {
    const history = renderAt("/");
    await waitFor(() => expect(history.get()).toBe("/d/p%3Ahome"));
    expect(tab("Home").getAttribute("aria-current")).toBe("page");
    expect(tab("Ops").getAttribute("aria-current")).toBeNull();
  });

  it("redirects an unknown dashboard", async () => {
    const history = renderAt("/d/p:nope");
    await waitFor(() => expect(history.get()).toBe("/d/p%3Ahome"));
  });

  it("switches_dashboards_from_the_sidebar", async () => {
    const history = renderAt("/d/p:home");
    await screen.findByText("welcome", { selector: "header *" });
    fireEvent.click(tab("Ops"));
    await waitFor(() => expect(history.get()).toBe("/d/p%3Aops"));
    expect(await screen.findByText("k8s", { selector: "header *" })).toBeTruthy();
    expect(screen.queryByText("welcome", { selector: "header *" })).toBeNull();
  });

  it("no_more_menu_with_nine_dashboards", async () => {
    const names = ["home", "a", "b", "c", "d", "e", "f", "g", "h"];
    vi.mocked(commands.listDashboards).mockResolvedValue(ok(names.map((n) => dashboard(n))));
    const history = renderAt("/d/p:home");
    const nav = await screen.findByRole("navigation", { name: "Platform dashboards" });
    await waitFor(() => expect(within(nav).getAllByRole("button")).toHaveLength(9));
    expect(screen.queryByRole("button", { name: "More ▾" })).toBeNull();
    fireEvent.click(within(nav).getByRole("button", { name: "H" }));
    await waitFor(() => expect(history.get()).toBe("/d/p%3Ah"));
    expect(tab("H").getAttribute("aria-current")).toBe("page");
  });

  it("shows no dashboard items when there are none", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([]));
    renderAt("/");
    expect(await screen.findByText("No dashboards yet. Click + New dashboard to create one.")).toBeTruthy();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Dashboards");
    expect(screen.queryByRole("navigation", { name: "Platform dashboards" })).toBeNull();
    const mine = screen.getByRole("navigation", { name: "Your dashboards" });
    expect(within(mine).getAllByRole("button").map((b) => b.textContent)).toEqual(["+ New dashboard"]);
  });

  it("shows an empty dashboard notice", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home", [])]));
    renderAt("/d/p:home");
    expect(await screen.findByText("This dashboard is empty.")).toBeTruthy();
  });

  it("falls back when the current dashboard disappears", async () => {
    const synced = launchSynced();
    const history = renderAt("/d/p:ops");
    await synced();
    await screen.findByText("k8s", { selector: "header *" });
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home")]));
    fireEvent.click(screen.getByRole("button", { name: "Sync now" }));
    await waitFor(() => expect(history.get()).toBe("/d/p%3Ahome"));
    expect(await screen.findByText("welcome", { selector: "header *" })).toBeTruthy();
  });

  it("runs each widget by its position in the dashboard file, not on screen", async () => {
    const home = runnableHome();
    const [first] = home.placements;
    // Listed second but placed first on the grid.
    home.placements = [{ ...first, widget: "lower", y: 4 }, first];
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([home]));
    vi.mocked(commands.runPlacement).mockReset().mockResolvedValue(ok(rows));
    renderAt("/d/p:home");
    await waitFor(() => expect(commands.runPlacement).toHaveBeenCalledTimes(2));
    expect(vi.mocked(commands.runPlacement).mock.calls).toEqual([
      ["p:home", [1]],
      ["p:home", [0]],
    ]);
  });

  it("keeps widget data visible across a re-sync", async () => {
    vi.mocked(commands.listDashboards).mockImplementation(async () => ok([runnableHome()]));
    vi.mocked(commands.runPlacement).mockReset().mockResolvedValue(ok(rows));
    const synced = launchSynced();
    renderAt("/d/p:home");
    await synced();
    await screen.findByText("payment-service");
    await waitFor(() => expect(commands.listDashboards).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(commands.runPlacement).toHaveBeenCalledTimes(2));

    vi.mocked(commands.runPlacement).mockReturnValue(new Promise(() => {}));
    fireEvent.click(screen.getByRole("button", { name: "Sync now" }));
    await waitFor(() => expect(commands.listDashboards).toHaveBeenCalledTimes(3));
    await waitFor(() => expect(commands.runPlacement).toHaveBeenCalledTimes(3));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.getByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
    expect(commands.runPlacement).toHaveBeenCalledTimes(3);
  });

  it("page_header_shows_title_and_origin_line", async () => {
    const mine: DashboardView = { ...dashboard("mine", []), id: "u:1", origin: "user", editable: true };
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), mine]));
    const history = renderAt("/d/p:home");
    const head = async (title: string) => {
      const h1 = await screen.findByRole("heading", { level: 1, name: title });
      return h1.closest("header.page-head") as HTMLElement;
    };
    expect((await head("Home")).textContent).toContain("Platform dashboard · read-only");

    history.set({ value: "/d/u:1" });
    const userHead = await head("Mine");
    expect(userHead.textContent).toContain("Your dashboard");
    expect(userHead.textContent).not.toContain("read-only");
  });

  it("reports a failed load", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue({
      status: "error",
      error: { kind: "internal", message: "database is locked" },
    });
    renderAt("/");
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Could not load dashboards");
    expect(alert.textContent).toContain("database is locked");
  });
});

function userDashboard(id: number, title: string, widgets: string[] = []): DashboardView {
  const d = dashboard(title.toLowerCase(), widgets);
  return {
    ...d,
    id: `u:${id}`,
    origin: "user",
    editable: true,
    placements: d.placements.map((p, i) => ({ ...p, id: 10 + i })),
  };
}

describe("Dashboards compose", () => {
  beforeEach(() => {
    vi.mocked(commands.listDashboards).mockReset().mockResolvedValue(ok([dashboard("home")]));
    vi.mocked(commands.listCatalog)
      .mockReset()
      .mockResolvedValue(
        ok([
          { name: "welcome", title: "Welcome", container: false },
          { name: "pair", title: null, container: true },
        ]),
      );
    for (const c of ["createDashboard", "renameDashboard", "deleteDashboard", "addWidget", "removeWidget", "setLanding"] as const) {
      vi.mocked(commands[c]).mockReset();
    }
  });

  it("creates_a_user_dashboard", async () => {
    const mine = userDashboard(1, "Mine");
    vi.mocked(commands.createDashboard).mockResolvedValue(ok(mine));
    const history = renderAt("/d/p:home");
    await screen.findByRole("button", { name: "Home" });
    fireEvent.click(screen.getByRole("button", { name: "+ New dashboard" }));
    const dialog = within(screen.getByRole("dialog"));
    fireEvent.input(dialog.getByLabelText("Title"), { target: { value: "Mine" } });
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), mine]));
    fireEvent.click(dialog.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(commands.createDashboard).toHaveBeenCalledWith("Mine"));
    await waitFor(() => expect(history.get()).toBe("/d/u%3A1"));
    expect(await screen.findByRole("button", { name: "Mine" })).toBeTruthy();
    expect(tab("Mine").getAttribute("aria-current")).toBe("page");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("adds_and_removes_a_widget", async () => {
    const empty = userDashboard(1, "Mine");
    const one = userDashboard(1, "Mine", ["welcome"]);
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), empty]));
    vi.mocked(commands.addWidget).mockResolvedValue(ok(one));
    vi.mocked(commands.removeWidget).mockResolvedValue(ok(empty));
    renderAt("/d/u:1");
    await screen.findByText("Add your first widget");

    fireEvent.click(screen.getByRole("button", { name: "Add widget" }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByRole("button", { name: /Welcome/ })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: /pair/ })).toBeTruthy();
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), one]));
    fireEvent.click(within(dialog).getByRole("button", { name: /Welcome/ }));
    await waitFor(() => expect(commands.addWidget).toHaveBeenCalledWith("u:1", "welcome", null));
    expect(await screen.findByText("welcome", { selector: "header *" })).toBeTruthy();

    fireEvent.click(await screen.findByRole("button", { name: "Remove welcome" }));
    const confirm = within(screen.getByRole("dialog"));
    const buttons = confirm.getAllByRole("button").map((b) => b.textContent);
    expect(buttons).toEqual(["Cancel", "Remove"]);
    expect(commands.removeWidget).not.toHaveBeenCalled();
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), empty]));
    fireEvent.click(confirm.getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(commands.removeWidget).toHaveBeenCalledWith("u:1", 10));
    expect(await screen.findByText("Add your first widget")).toBeTruthy();
  });

  it("empty_user_dashboard_shows_add_cta", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), userDashboard(1, "Mine")]));
    renderAt("/d/u:1");
    expect(await screen.findByText("Add your first widget")).toBeTruthy();
  });

  it("set_as_landing_toggles", async () => {
    const mine = userDashboard(1, "Mine");
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), mine]));
    vi.mocked(commands.setLanding).mockResolvedValue(ok(null));
    renderAt("/d/u:1");
    await screen.findByText("Add your first widget");
    // Until one is chosen, the app opens on the first (platform) dashboard.
    expect(tab("Home").contains(screen.getByText(/\(Landing dashboard\)/))).toBe(true);
    // The cue is in the accessibility tree: it is part of the tab's accessible name.
    expect(screen.getByRole("button", { name: /^Home\s*\(Landing dashboard\)$/ })).toBeTruthy();

    // The engine moves the landing dashboard to the front of the list.
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([mine, dashboard("home")]));
    fireEvent.click(screen.getByRole("button", { name: "Set as landing" }));
    await waitFor(() => expect(commands.setLanding).toHaveBeenCalledWith("u:1"));
    await waitFor(() => expect(tab("Mine").contains(screen.queryByText(/\(Landing dashboard\)/))).toBe(true));
    expect(tab("Home").querySelector(".landing-mark, .sr-only")).toBeNull();

    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), mine]));
    fireEvent.click(screen.getByRole("button", { name: "Clear landing" }));
    await waitFor(() => expect(commands.setLanding).toHaveBeenLastCalledWith(null));
    await waitFor(() => expect(tab("Home").contains(screen.queryByText(/\(Landing dashboard\)/))).toBe(true));
    expect(screen.getByRole("button", { name: /^Home\s*\(Landing dashboard\)$/ })).toBeTruthy();
  });

  it("renames and deletes after confirmation", async () => {
    const mine = userDashboard(1, "Mine");
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), mine]));
    vi.mocked(commands.renameDashboard).mockResolvedValue(ok(null));
    vi.mocked(commands.deleteDashboard).mockResolvedValue(ok(null));
    const history = renderAt("/d/u:1");
    await screen.findByText("Add your first widget");

    fireEvent.click(screen.getByRole("button", { name: "Rename" }));
    let dialog = within(screen.getByRole("dialog"));
    fireEvent.input(dialog.getByLabelText("Title"), { target: { value: "Renamed" } });
    fireEvent.click(dialog.getByRole("button", { name: "Rename" }));
    await waitFor(() => expect(commands.renameDashboard).toHaveBeenCalledWith("u:1", "Renamed"));

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    dialog = within(screen.getByRole("dialog"));
    expect(dialog.getAllByRole("button").map((b) => b.textContent)).toEqual(["Cancel", "Delete"]);
    expect(commands.deleteDashboard).not.toHaveBeenCalled();
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home")]));
    fireEvent.click(dialog.getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(commands.deleteDashboard).toHaveBeenCalledWith("u:1"));
    await waitFor(() => expect(history.get()).toBe("/d/p%3Ahome"));
  });

  it("shows no edit affordances on a platform dashboard", async () => {
    renderAt("/d/p:home");
    await screen.findByRole("button", { name: "Home" });
    expect(screen.getByRole("button", { name: "+ New dashboard" })).toBeTruthy();
    for (const name of ["Add widget", "Rename", "Delete", "Set as landing", "Clear landing"]) {
      expect(screen.queryByRole("button", { name })).toBeNull();
    }
    expect(screen.queryByRole("button", { name: /^Remove/ })).toBeNull();
    expect(screen.queryByText("Add your first widget")).toBeNull();
  });

  it("shows + New group on an editable dashboard and creates a top-level group", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), userDashboard(1, "Mine")]));
    vi.mocked(commands.createGroup).mockResolvedValue(ok(userDashboard(1, "Mine")));
    renderAt("/d/u:1");
    fireEvent.click(await screen.findByRole("button", { name: "+ New group" }));
    const dialog = within(screen.getByRole("dialog"));
    fireEvent.input(dialog.getByLabelText("Title"), { target: { value: "Ops" } });
    fireEvent.click(dialog.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(commands.createGroup).toHaveBeenCalledWith("u:1", "Ops", null));
  });

  it("customize_is_hidden_on_the_landing_dashboard", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), dashboard("ops", ["k8s"])]));
    renderAt("/d/p:home");
    await screen.findByRole("button", { name: /^Home\b/ });
    expect(screen.queryByRole("button", { name: "Customize" })).toBeNull();
  });

  it("customize_shows_on_a_non_landing_platform_dashboard", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), dashboard("ops", ["k8s"])]));
    renderAt("/d/p:ops");
    expect(await screen.findByRole("button", { name: "Customize" })).toBeTruthy();
  });

  it("customizes a non-landing platform dashboard and opens the copy", async () => {
    const copy = userDashboard(2, "Ops copy");
    vi.mocked(commands.customizeDashboard).mockResolvedValue(ok(copy));
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), dashboard("ops", ["k8s"])]));
    const history = renderAt("/d/p:ops");
    await screen.findByRole("button", { name: /^Ops\b/ });
    expect(screen.queryByRole("button", { name: "+ New group" })).toBeNull();
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), dashboard("ops", ["k8s"]), copy]));
    fireEvent.click(screen.getByRole("button", { name: "Customize" }));
    await waitFor(() => expect(commands.customizeDashboard).toHaveBeenCalledWith("p:ops"));
    await waitFor(() => expect(history.get()).toBe("/d/u%3A2"));
  });

  it("adds a widget into a group via its own Add widget control", async () => {
    const mine = userDashboard(1, "Mine");
    mine.placements = [{ ...dashboard("x").placements[0], widget: "group", container: true, title: "Ops", id: 5 }];
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), mine]));
    vi.mocked(commands.addWidget).mockResolvedValue(ok(mine));
    renderAt("/d/u:1");
    // Non-empty dashboards open in view mode; enter edit to reach group controls.
    fireEvent.click(await screen.findByRole("button", { name: "Edit" }));
    fireEvent.click(await screen.findByRole("button", { name: "Add widget to Ops" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(await within(dialog).findByRole("button", { name: /Welcome/ }));
    await waitFor(() => expect(commands.addWidget).toHaveBeenCalledWith("u:1", "welcome", 5));
  });

  it("editable_dashboard_opens_in_view_mode", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), userDashboard(1, "Mine", ["welcome"])]));
    renderAt("/d/u:1");
    expect(await screen.findByRole("button", { name: "Edit" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Add widget" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Delete" })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Remove/ })).toBeNull();
  });

  it("clicking_edit_reveals_the_controls", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), userDashboard(1, "Mine", ["welcome"])]));
    renderAt("/d/u:1");
    fireEvent.click(await screen.findByRole("button", { name: "Edit" }));
    expect(screen.getByRole("button", { name: "Add widget" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Delete" })).toBeTruthy();
    expect(screen.getByRole("button", { name: /^Remove/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Done" })).toBeTruthy();
  });

  it("empty_editable_dashboard_opens_in_edit_mode", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), userDashboard(1, "Mine")]));
    renderAt("/d/u:1");
    expect(await screen.findByRole("button", { name: "Add widget" })).toBeTruthy();
    // An empty personal dashboard is always in edit mode — nothing to view, so no toggle.
    expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Done" })).toBeNull();
  });

  it("the_edit_toggle_appears_once_the_dashboard_has_a_widget", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home"), userDashboard(1, "Mine", ["welcome"])]));
    renderAt("/d/u:1");
    expect(await screen.findByRole("button", { name: "Edit" })).toBeTruthy();
  });

  it("switching_dashboards_resets_to_view_mode", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(
      ok([dashboard("home"), userDashboard(1, "One", ["welcome"]), userDashboard(2, "Two", ["welcome"])]),
    );
    renderAt("/d/u:1");
    fireEvent.click(await screen.findByRole("button", { name: "Edit" }));
    expect(screen.getByRole("button", { name: "Add widget" })).toBeTruthy();
    fireEvent.click(tab("Two"));
    await waitFor(() => expect(screen.getByRole("button", { name: "Edit" })).toBeTruthy());
    expect(screen.queryByRole("button", { name: "Add widget" })).toBeNull();
  });

  it("platform_dashboard_has_no_edit_toggle", async () => {
    renderAt("/d/p:home");
    await screen.findByRole("button", { name: /^Home\b/ });
    expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Done" })).toBeNull();
  });
});

/** A user dashboard with a top-level leaf, a top-level group, and a child inside the group. */
function layoutDashboard(): DashboardView {
  const leaf = { ...dashboard("x").placements[0], widget: "welcome", id: 10, x: 0, y: 0, w: 4, h: 2 };
  const child = { ...dashboard("x").placements[0], widget: "k8s", id: 21, x: 0, y: 0, w: 4, h: 2 };
  const group = {
    ...dashboard("x").placements[0],
    widget: "group",
    container: true,
    title: "Ops",
    id: 20,
    x: 4,
    y: 0,
    w: 4,
    h: 2,
    children: [child],
  };
  return { ...userDashboard(1, "Mine"), placements: [leaf, group] };
}

const topGrid = () => document.querySelector(".grid:not(.subgrid)") as HTMLElement;
const article = (text: string) => screen.getByText(text, { selector: "header *" }).closest("article") as HTMLElement;

describe("Dashboards drag/resize", () => {
  beforeEach(() => {
    setViewport(1200);
    vi.mocked(commands.listDashboards).mockReset().mockResolvedValue(ok([layoutDashboard()]));
    vi.mocked(commands.setLayout).mockReset().mockResolvedValue(ok(layoutDashboard()));
  });

  it("move_resize_grips_only_in_edit_mode_on_top_level", async () => {
    renderAt("/d/u:1");
    // View mode: no grips anywhere.
    await screen.findByText("welcome", { selector: "header *" });
    expect(document.querySelector("[data-move-handle]")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Resize/ })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    // The top-level leaf shows both a move handle and a resize grip.
    const leaf = article("welcome");
    expect(leaf.querySelector("[data-move-handle]")).toBeTruthy();
    expect(within(leaf).getByRole("button", { name: "Resize welcome" })).toBeTruthy();
    // A group's child (inside the sub-grid) shows neither.
    const child = article("k8s");
    expect(child.querySelector("[data-move-handle]")).toBeNull();
    expect(within(child).queryByRole("button", { name: /^Resize/ })).toBeNull();
  });

  it("grips_absent_on_a_platform_dashboard", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home", ["welcome"])]));
    renderAt("/d/p:home");
    await screen.findByText("welcome", { selector: "header *" });
    expect(document.querySelector("[data-move-handle]")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Resize/ })).toBeNull();
    expect(screen.queryByRole("button", { name: "Edit" })).toBeNull();
  });

  it("a_completed_drag_commits_via_setLayout", async () => {
    renderAt("/d/u:1");
    await screen.findByText("welcome", { selector: "header *" });
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const drag = dragControllerFor(topGrid());
    expect(drag).toBeTruthy();

    // Move the leaf (id 10) into the empty right of the grid — no collision, one changed card.
    drag!.begin(10, "move");
    drag!.onMove(10, { x: 8, y: 0, w: 4, h: 2 });
    drag!.commit(10);
    await waitFor(() => expect(commands.setLayout).toHaveBeenCalledTimes(1));
    expect(vi.mocked(commands.setLayout).mock.calls[0]).toEqual([
      "u:1",
      [{ placementId: 10, x: 8, y: 0, w: 4, h: 2 }],
    ]);

    // A gesture that changes nothing does not persist.
    drag!.begin(10, "move");
    drag!.commit(10);
    expect(commands.setLayout).toHaveBeenCalledTimes(1);
  });

  it("resize_is_clamped_to_the_grid_width", async () => {
    renderAt("/d/u:1");
    await screen.findByText("welcome", { selector: "header *" });
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const drag = dragControllerFor(topGrid())!;

    drag.begin(10, "resize");
    drag.onMove(10, { x: 8, y: 0, w: 8, h: 2 });
    const box = drag.boxOf(10)!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.w).toBeLessThanOrEqual(12);
    // The card grows rightward only, up to the grid edge; it does not slide left.
    expect(box).toMatchObject({ x: 8, w: 4 });
    drag.commit(10);
    for (const call of vi.mocked(commands.setLayout).mock.calls) {
      for (const p of call[1]) expect(p.x + p.w).toBeLessThanOrEqual(12);
    }
  });

  it("escape_cancels_without_persisting", async () => {
    renderAt("/d/u:1");
    await screen.findByText("welcome", { selector: "header *" });
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const drag = dragControllerFor(topGrid())!;

    drag.begin(10, "move");
    drag.onMove(10, { x: 8, y: 0, w: 4, h: 2 });
    // The live preview moves the card.
    expect(article("welcome").style.getPropertyValue("grid-column")).toBe("9 / span 4");

    fireEvent.keyDown(document, { key: "Escape" });
    expect(commands.setLayout).not.toHaveBeenCalled();
    // Geometry reverts to the stored placement.
    expect(article("welcome").style.getPropertyValue("grid-column")).toBe("1 / span 4");
  });

  it("handlers_inert_below_the_single_column_breakpoint", async () => {
    setViewport(800);
    renderAt("/d/u:1");
    await screen.findByText("welcome", { selector: "header *" });
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const drag = dragControllerFor(topGrid())!;

    drag.begin(10, "move");
    drag.onMove(10, { x: 8, y: 0, w: 4, h: 2 });
    drag.commit(10);
    expect(commands.setLayout).not.toHaveBeenCalled();
    // The gesture never took hold, so geometry is unchanged.
    expect(article("welcome").style.getPropertyValue("grid-column")).toBe("1 / span 4");
  });
});
