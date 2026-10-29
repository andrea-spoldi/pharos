import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type DashboardView } from "../bindings";
import { Dashboards } from "./Dashboards";

window.scrollTo = vi.fn() as typeof window.scrollTo;

vi.mock("../bindings", () => ({ commands: { listDashboards: vi.fn(), runWidget: vi.fn() } }));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

function dashboard(name: string, widgets: string[] = ["welcome"]): DashboardView {
  return {
    metadata: { name, title: name[0].toUpperCase() + name.slice(1), version: null, description: null },
    placements: widgets.map((w, i) => ({
      widget: w,
      x: 0,
      y: i * 2,
      w: 4,
      h: 2,
      manifest: null,
      status: { kind: "missing" },
      reads: null,
    })),
  };
}

function renderAt(path: string, version: () => number = () => 0) {
  const history = createMemoryHistory();
  history.set({ value: path });
  render(() => (
    <MemoryRouter history={history}>
      <Route path={["/", "/d/:name"]} component={() => <Dashboards version={version()} />} />
    </MemoryRouter>
  ));
  return history;
}

const rows = { rows: [{ metadata: { name: "payment-service" }, _source: "p.yaml" }], truncated: false };

/** A fresh list with one runnable file widget; new objects on every call, as after a sync. */
function runnableHome(): DashboardView {
  return {
    metadata: { name: "home", title: "Home", version: null, description: null },
    placements: [
      {
        widget: "service-catalog",
        x: 0,
        y: 0,
        w: 8,
        h: 4,
        manifest: {
          apiVersion: "cockpit.dev/v1alpha1",
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
      },
    ],
  };
}

const tab = (name: string) => screen.getByRole("button", { name });

describe("Dashboards", () => {
  beforeEach(() => {
    vi.mocked(commands.listDashboards).mockReset().mockResolvedValue(ok([dashboard("home"), dashboard("ops", ["k8s"])]));
  });

  it("redirects to the first dashboard", async () => {
    const history = renderAt("/");
    await waitFor(() => expect(history.get()).toBe("/d/home"));
    expect(tab("Home").getAttribute("aria-current")).toBe("page");
    expect(tab("Ops").getAttribute("aria-current")).toBeNull();
  });

  it("redirects an unknown dashboard", async () => {
    const history = renderAt("/d/nope");
    await waitFor(() => expect(history.get()).toBe("/d/home"));
  });

  it("switches dashboards from the tabs", async () => {
    const history = renderAt("/d/home");
    await screen.findByText("welcome", { selector: "header *" });
    fireEvent.click(tab("Ops"));
    await waitFor(() => expect(history.get()).toBe("/d/ops"));
    expect(await screen.findByText("k8s", { selector: "header *" })).toBeTruthy();
    expect(screen.queryByText("welcome", { selector: "header *" })).toBeNull();
  });

  it("more menu closes on escape and selection", async () => {
    const names = ["home", "a", "b", "c", "d", "e", "f"];
    vi.mocked(commands.listDashboards).mockResolvedValue(ok(names.map((n) => dashboard(n))));
    const history = renderAt("/d/home");
    await screen.findByRole("button", { name: "More ▾" });
    expect(screen.getByRole("navigation", { name: "Dashboards" }).querySelectorAll(".tab").length).toBe(6);
    expect(screen.queryByRole("button", { name: "E" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "More ▾" }));
    expect(screen.getByRole("button", { name: "E" })).toBeTruthy();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByRole("button", { name: "E" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "More ▾" }));
    fireEvent.click(screen.getByRole("button", { name: "E" }));
    await waitFor(() => expect(history.get()).toBe("/d/e"));
    expect(screen.queryByRole("button", { name: "F" })).toBeNull();
  });

  it("shows no tabs when there are no dashboards", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([]));
    renderAt("/");
    expect(await screen.findByText("The platform repo defines no dashboards yet.")).toBeTruthy();
    expect(screen.queryByRole("navigation")).toBeNull();
  });

  it("shows an empty dashboard notice", async () => {
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home", [])]));
    renderAt("/d/home");
    expect(await screen.findByText("This dashboard is empty.")).toBeTruthy();
  });

  it("falls back when the current dashboard disappears", async () => {
    const [version, setVersion] = createSignal(0);
    const history = renderAt("/d/ops", version);
    await screen.findByText("k8s", { selector: "header *" });
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([dashboard("home")]));
    setVersion(1);
    await waitFor(() => expect(history.get()).toBe("/d/home"));
    expect(await screen.findByText("welcome", { selector: "header *" })).toBeTruthy();
  });

  it("keeps widget data visible across a re-sync", async () => {
    vi.mocked(commands.listDashboards).mockImplementation(async () => ok([runnableHome()]));
    vi.mocked(commands.runWidget).mockReset().mockResolvedValue(ok(rows));
    const [version, setVersion] = createSignal(0);
    renderAt("/d/home", version);
    await screen.findByText("payment-service");

    vi.mocked(commands.runWidget).mockReturnValue(new Promise(() => {}));
    setVersion(1);
    await waitFor(() => expect(commands.listDashboards).toHaveBeenCalledTimes(2));
    await waitFor(() => expect(commands.runWidget).toHaveBeenCalledTimes(2));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.getByText("payment-service")).toBeTruthy();
    expect(screen.queryByLabelText("Loading")).toBeNull();
    expect(commands.runWidget).toHaveBeenCalledTimes(2);
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

  it("marks More as active when the current dashboard is in the menu", async () => {
    const names = ["home", "a", "b", "c", "d", "e", "f"];
    vi.mocked(commands.listDashboards).mockResolvedValue(ok(names.map((n) => dashboard(n))));
    renderAt("/d/e");
    const more = await screen.findByRole("button", { name: "More ▾" });
    expect(more.classList.contains("active")).toBe(true);
    expect(tab("Home").getAttribute("aria-current")).toBeNull();
  });
});
