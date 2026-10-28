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
});
