import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render as renderUi, screen, waitFor } from "@solidjs/testing-library";
import type { JSX } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { commands } from "./bindings";

window.scrollTo = vi.fn() as typeof window.scrollTo;

function atPath(path: string) {
  const history = createMemoryHistory();
  history.set({ value: path });
  return history;
}

const render = (ui: () => JSX.Element, path = "/") =>
  renderUi(() => (
    <MemoryRouter history={atPath(path)}>
      <Route path={["/", "/d/:name", "/settings"]} component={ui} />
    </MemoryRouter>
  ));

vi.mock("./bindings", () => ({
  commands: { getSource: vi.fn(), gitVersion: vi.fn(), pendingSync: vi.fn(), planSync: vi.fn(), listDashboards: vi.fn(), getPlatform: vi.fn(), getSettings: vi.fn(), disconnect: vi.fn() },
}));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

const source = {
  // Synced, so Home shows the dashboards rather than planning a sync.
  id: 1,
  url: "git@gitlab.acme:platform/pharos.git",
  branch: "main",
  local_path: "/data/sources/1",
  synced_commit: "a".repeat(40),
  synced_at: 1_700_000_000,
  approved_capability_hash: "sha256:x",
};

describe("App", () => {
  beforeEach(() => {
    vi.mocked(commands.gitVersion).mockResolvedValue(ok("git version 2.50.0"));
    vi.mocked(commands.pendingSync).mockResolvedValue(ok(null));
    vi.mocked(commands.listDashboards).mockResolvedValue(ok([]));
    vi.mocked(commands.getPlatform).mockResolvedValue(ok(null));
    // Home syncs on launch; leave it in flight, the sync flow is covered in Home.test.tsx.
    vi.mocked(commands.planSync).mockReturnValue(new Promise(() => {}));
  });

  it("shows onboarding when no repo is connected", async () => {
    vi.mocked(commands.getSource).mockResolvedValue(ok(null));
    render(() => <App />);
    expect(await screen.findByText("Connect your platform repo")).toBeTruthy();
  });

  it("skips onboarding when a repo is connected", async () => {
    vi.mocked(commands.getSource).mockResolvedValue(ok(source));
    render(() => <App />);
    expect((await screen.findByTestId("source")).textContent).toContain(source.url);
    expect(screen.queryByText("Connect your platform repo")).toBeNull();
    expect(await screen.findByText("The platform repo defines no dashboards yet.")).toBeTruthy();
  });

  it("disconnect returns to a pre-filled onboarding", async () => {
    vi.mocked(commands.getSource).mockResolvedValue(ok(source));
    vi.mocked(commands.getSettings).mockResolvedValue(
      ok({ source, platform: { name: "acme-platform", title: null }, inputs: [], secrets: [], unused: [] }),
    );
    vi.mocked(commands.disconnect).mockResolvedValue(ok(null));
    render(() => <App />, "/settings");
    fireEvent.click(await screen.findByRole("button", { name: "Disconnect…" }));
    fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));
    expect(await screen.findByText("Connect your platform repo")).toBeTruthy();
    await waitFor(() => expect(document.querySelector("input[name=url]")).toHaveProperty("value", source.url));
  });

  it("reports unreadable local state", async () => {
    vi.mocked(commands.getSource).mockResolvedValue({
      status: "error",
      error: { kind: "internal", message: "database is locked" },
    });
    render(() => <App />);
    expect((await screen.findByRole("alert")).textContent).toContain("database is locked");
  });
});
