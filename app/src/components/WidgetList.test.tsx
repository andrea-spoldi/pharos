import { render, screen, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type FileData, type Manifest, type WidgetSpec } from "../bindings";
import { WidgetList } from "./WidgetList";

vi.mock("../bindings", () => ({
  commands: { listWidgets: vi.fn(), runWidget: vi.fn() },
}));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

type Widget = Manifest<WidgetSpec>;

function widget(name: string, source: WidgetSpec["source"], view: WidgetSpec["view"]): Widget {
  return {
    apiVersion: "cockpit.dev/v1alpha1",
    kind: "Widget",
    metadata: { name, title: null, version: null, description: null },
    spec: { source, refresh: null, inputs: {}, view, permissions: { hosts: [] } },
  };
}

const catalog = widget(
  "service-catalog",
  { type: "file", path: "../../catalog/services/", glob: "*.yaml" },
  { type: "table", columns: [{ field: "metadata.name", label: "Service" }], content: null },
);
const kubectl = widget(
  "k8s-context",
  { type: "command", executable: "kubectl", args: [], timeout: null },
  { type: "raw", columns: [], content: null },
);
const rows: FileData = { rows: [{ metadata: { name: "payment-service" }, _source: "p.yaml" }], truncated: false };

describe("WidgetList", () => {
  beforeEach(() => {
    vi.mocked(commands.listWidgets).mockReset().mockResolvedValue(ok([catalog, kubectl]));
    vi.mocked(commands.runWidget).mockReset().mockResolvedValue(ok(rows));
  });

  it("runs file widgets by name and renders their view", async () => {
    render(() => <WidgetList version={0} />);
    const table = await screen.findByRole("table");
    expect(table.textContent).toContain("Service");
    expect(table.textContent).toContain("payment-service");
    // Only the name crosses to the backend; other source types don't run yet.
    expect(commands.runWidget).toHaveBeenCalledTimes(1);
    expect(commands.runWidget).toHaveBeenCalledWith("service-catalog");
  });

  it("explains a run the backend refused", async () => {
    vi.mocked(commands.runWidget).mockResolvedValue({
      status: "error",
      error: { kind: "permissionDenied", message: "`catalog/services/evil.yaml`: path leaves the repository root through a symlink" },
    });
    render(() => <WidgetList version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Access denied");
    expect(alert.textContent).toContain("evil.yaml");
  });

  it("reports unreadable data", async () => {
    vi.mocked(commands.runWidget).mockResolvedValue({
      status: "error",
      error: { kind: "dataError", message: "cannot parse `catalog/services/bad.yaml`" },
    });
    render(() => <WidgetList version={0} />);
    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("Could not read the data");
    expect(alert.textContent).toContain("bad.yaml");
  });

  it("names views that don't render yet", async () => {
    vi.mocked(commands.listWidgets).mockResolvedValue(
      ok([{ ...catalog, spec: { ...catalog.spec, view: { type: "kv" as const, columns: [], content: null } } }]),
    );
    render(() => <WidgetList version={0} />);
    expect(await screen.findByText("The kv view is not available yet.")).toBeTruthy();
  });

  it("runs again after a sync", async () => {
    const [version, setVersion] = createSignal(0);
    render(() => <WidgetList version={version()} />);
    await screen.findByRole("table");
    setVersion(1);
    await waitFor(() => expect(commands.runWidget).toHaveBeenCalledTimes(2));
  });
});
