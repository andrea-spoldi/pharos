import { render, screen } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { commands } from "./bindings";

vi.mock("./bindings", () => ({
  commands: { getSource: vi.fn(), gitVersion: vi.fn() },
}));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

const source = {
  id: 1,
  url: "git@gitlab.acme:platform/cockpit.git",
  branch: "main",
  local_path: "/data/sources/1",
  synced_commit: null,
  synced_at: null,
  approved_capability_hash: null,
};

describe("App", () => {
  beforeEach(() => {
    vi.mocked(commands.gitVersion).mockResolvedValue(ok("git version 2.50.0"));
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
