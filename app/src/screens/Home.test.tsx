import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type RepoSource, type SyncPlan } from "../bindings";
import { Home } from "./Home";

vi.mock("../bindings", () => ({
  commands: {
    pendingSync: vi.fn(),
    planSync: vi.fn(),
    applySync: vi.fn(),
    discardSync: vi.fn(),
    listWidgets: vi.fn(),
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
  needs_consent: true,
};

const widget = {
  apiVersion: "cockpit.dev/v1alpha1",
  kind: "Widget",
  metadata: { name: "k8s-context", title: "Kubernetes Context", version: null, description: null },
  spec: {
    source: { type: "command" as const, executable: "kubectl", args: [], timeout: null },
    refresh: null,
    inputs: {},
    view: { type: "kv" as const, columns: [], content: null },
    permissions: { hosts: [] },
  },
};

describe("Home", () => {
  beforeEach(() => {
    for (const fn of Object.values(commands)) vi.mocked(fn).mockReset();
    vi.mocked(commands.pendingSync).mockResolvedValue(ok(null));
    vi.mocked(commands.planSync).mockResolvedValue(ok(firstPlan));
    vi.mocked(commands.applySync).mockResolvedValue(ok(null));
    vi.mocked(commands.discardSync).mockResolvedValue(ok(null));
    vi.mocked(commands.listWidgets).mockResolvedValue(ok([widget]));
  });

  it("plans the first sync and asks consent for capabilities by widget", async () => {
    const onSynced = vi.fn();
    render(() => <Home source={fresh} onSynced={onSynced} />);

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
    render(() => <Home source={fresh} onSynced={() => {}} />);
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
    render(() => <Home source={fresh} onSynced={() => {}} />);

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
    render(() => <Home source={fresh} onSynced={() => {}} />);
    fireEvent.click(await screen.findByText("Approve and apply"));

    expect(await screen.findByText("The repo changed since this review")).toBeTruthy();
    fireEvent.click(screen.getAllByText("Review again")[0]);
    await waitFor(() => expect(commands.planSync).toHaveBeenCalledTimes(2));
  });

  it("discards back to an empty state when nothing was applied", async () => {
    render(() => <Home source={fresh} onSynced={() => {}} />);
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
      needs_consent: false,
    };
    const update: SyncPlan = {
      ...upToDate,
      to_commit: "c".repeat(40),
      widgets: { ...none, changed: ["my-pipelines"] },
    };

    it("syncs on launch while the widgets stay listed", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok(upToDate));
      render(() => <Home source={synced} onSynced={() => {}} />);

      const list = await screen.findByTestId("widgets");
      expect(list.textContent).toContain("Kubernetes Context");
      expect(list.textContent).toContain("command · kubectl");
      expect(await screen.findByTestId("up-to-date")).toBeTruthy();
      expect(commands.planSync).toHaveBeenCalledTimes(1);
      // The no-op plan is dropped so it is not offered for review later.
      expect(commands.discardSync).toHaveBeenCalled();
      expect(commands.applySync).not.toHaveBeenCalled();
    });

    it("applies unchanged capabilities without asking and shows the changelog", async () => {
      vi.mocked(commands.planSync).mockResolvedValueOnce(ok(upToDate)).mockResolvedValueOnce(ok(update));
      const onSynced = vi.fn();
      render(() => <Home source={synced} onSynced={onSynced} />);
      await screen.findByTestId("up-to-date");

      fireEvent.click(screen.getByText("Sync now"));
      const changelog = await screen.findByTestId("changelog");
      expect(changelog.textContent).toContain("Synced to ccccccc");
      expect(changelog.textContent).toContain("changed my-pipelines");
      expect(commands.applySync).toHaveBeenCalledWith(false);
      expect(onSynced).toHaveBeenCalled();
      await waitFor(() => expect(commands.listWidgets).toHaveBeenCalledTimes(2));
    });

    it("asks again when capabilities changed", async () => {
      vi.mocked(commands.planSync).mockResolvedValue(ok({ ...update, needs_consent: true }));
      render(() => <Home source={synced} onSynced={() => {}} />);

      expect(await screen.findByText("Review changes")).toBeTruthy();
      expect(screen.getByText("Approve and apply")).toBeTruthy();
      expect(commands.applySync).not.toHaveBeenCalled();
      expect(screen.queryByText("Sync now")).toBeNull();
    });

    it("keeps the widgets when the sync fails", async () => {
      vi.mocked(commands.planSync).mockResolvedValue({
        status: "error",
        error: { kind: "git", message: "`git fetch --quiet --prune origin main` failed: offline" },
      });
      render(() => <Home source={synced} onSynced={() => {}} />);

      expect((await screen.findByRole("alert")).textContent).toContain("offline");
      expect(await screen.findByTestId("widgets")).toBeTruthy();
      expect(screen.getByText("Try again")).toBeTruthy();
    });
  });
});
