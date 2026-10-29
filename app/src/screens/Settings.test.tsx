import { createMemoryHistory, MemoryRouter, Route } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type RepoSource, type SettingsView } from "../bindings";
import { Settings } from "./Settings";

vi.mock("../bindings", () => ({
  commands: {
    getSettings: vi.fn(),
    setInput: vi.fn(),
    setSecret: vi.fn(),
    removeSecret: vi.fn(),
    removeStored: vi.fn(),
    disconnect: vi.fn(),
  },
}));

window.scrollTo = vi.fn() as typeof window.scrollTo;
Element.prototype.scrollIntoView = vi.fn();

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

const source: RepoSource = {
  id: 1,
  url: "git@gitlab.acme:platform/pharos.git",
  branch: "main",
  local_path: "/data/sources/1",
  synced_commit: "a".repeat(40),
  synced_at: Math.floor(Date.now() / 1000) - 120,
  approved_capability_hash: "sha256:x",
};

function view(over: Partial<SettingsView> = {}): SettingsView {
  return {
    source,
    platform: { name: "acme-platform", title: "ACME Developer Portal" },
    inputs: [
      {
        key: "team",
        label: "Your team",
        type: "enum",
        required: true,
        value: null,
        default: null,
        choices: ["payments-team", "platform-team"],
        stale: false,
      },
    ],
    secrets: [{ name: "GITLAB_TOKEN", description: "Personal access token (read_api)", set: false }],
    unused: [],
    ...over,
  };
}

function renderAt(path = "/settings", onChanged = vi.fn(), onDisconnected = vi.fn(), src: RepoSource = source) {
  const history = createMemoryHistory();
  history.set({ value: path });
  render(() => (
    <MemoryRouter history={history}>
      <Route
        path="/settings"
        component={() => <Settings source={src} onChanged={onChanged} onDisconnected={onDisconnected} />}
      />
    </MemoryRouter>
  ));
  return { history, onChanged, onDisconnected };
}

describe("Settings", () => {
  beforeEach(() => {
    for (const fn of Object.values(commands)) vi.mocked(fn).mockReset();
    vi.mocked(commands.getSettings).mockResolvedValue(ok(view()));
    vi.mocked(commands.setInput).mockResolvedValue(ok(null));
    vi.mocked(commands.setSecret).mockResolvedValue(ok(null));
    vi.mocked(commands.removeSecret).mockResolvedValue(ok(null));
    vi.mocked(commands.removeStored).mockResolvedValue(ok(null));
  });

  it("shows the three sections", async () => {
    renderAt();
    expect(await screen.findByRole("heading", { name: "Repository" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Your details" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Secrets" })).toBeTruthy();
    expect(document.body.textContent).toContain("git@gitlab.acme:platform/pharos.git · main · aaaaaaa");
  });

  it("before the first sync", async () => {
    const unsynced = { ...source, synced_commit: null, synced_at: null };
    vi.mocked(commands.getSettings).mockResolvedValue(ok(view({ platform: null, inputs: [], secrets: [], source: unsynced })));
    renderAt("/settings", vi.fn(), vi.fn(), unsynced);
    await screen.findByRole("heading", { name: "Repository" });
    expect(screen.getAllByText("Available after the first sync")).toHaveLength(2);
    expect(screen.getByText(/Not synced yet/)).toBeTruthy();
  });

  it("enum select lists the choices and saves on change", async () => {
    const { onChanged } = renderAt();
    const select = (await screen.findByLabelText(/Your team/)) as HTMLSelectElement;
    expect([...select.options].map((o) => o.value)).toEqual(["", "payments-team", "platform-team"]);
    fireEvent.change(select, { target: { value: "payments-team" } });
    await waitFor(() => expect(commands.setInput).toHaveBeenCalledWith("team", "payments-team"));
    expect(await screen.findByText("Saved")).toBeTruthy();
    expect(onChanged).toHaveBeenCalled();
  });

  it("stale value shows the warning", async () => {
    vi.mocked(commands.getSettings).mockResolvedValue(
      ok(view({ inputs: [{ ...view().inputs[0], value: "gone-team", stale: true }] })),
    );
    renderAt();
    expect(await screen.findByText(/gone-team is no longer offered, pick another/)).toBeTruthy();
  });

  it("secret flow", async () => {
    renderAt();
    const row = await screen.findByTestId("secret-GITLAB_TOKEN");
    expect(within(row).getByText("Not set")).toBeTruthy();
    const input = within(row).getByLabelText("GITLAB_TOKEN value") as HTMLInputElement;
    expect(input.type).toBe("password");
    fireEvent.input(input, { target: { value: "tok" } });
    vi.mocked(commands.getSettings).mockResolvedValue(
      ok(view({ secrets: [{ ...view().secrets[0], set: true }] })),
    );
    fireEvent.click(within(row).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(commands.setSecret).toHaveBeenCalledWith("GITLAB_TOKEN", "tok"));
    expect(await within(row).findByText("Set")).toBeTruthy();
    expect(within(row).queryByLabelText("GITLAB_TOKEN value")).toBeNull();

    fireEvent.click(within(row).getByRole("button", { name: "Remove" }));
    const dialog = screen.getByRole("dialog", { name: "Remove GITLAB_TOKEN?" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(commands.removeSecret).toHaveBeenCalledWith("GITLAB_TOKEN"));
  });

  it("clears the password field after saving", async () => {
    renderAt();
    const row = await screen.findByTestId("secret-GITLAB_TOKEN");
    const input = within(row).getByLabelText("GITLAB_TOKEN value") as HTMLInputElement;
    fireEvent.input(input, { target: { value: "tok" } });
    fireEvent.click(within(row).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(commands.setSecret).toHaveBeenCalled());
    await waitFor(() => expect(input.value).toBe(""));
  });

  it("no longer used rows are removable", async () => {
    vi.mocked(commands.getSettings).mockResolvedValue(
      ok(view({ unused: [{ platform: "acme-platform", kind: "secret", key: "OLD" }] })),
    );
    renderAt();
    const section = await screen.findByTestId("unused");
    expect(section.textContent).toContain("OLD · acme-platform");
    fireEvent.click(within(section).getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(commands.removeStored).toHaveBeenCalledWith("acme-platform", "secret", "OLD"));
  });

  it("focuses the field from the link", async () => {
    renderAt("/settings?focus=secret:GITLAB_TOKEN");
    const row = await screen.findByTestId("secret-GITLAB_TOKEN");
    await waitFor(() => expect(document.activeElement).toBe(within(row).getByLabelText("GITLAB_TOKEN value")));
  });

  it("disconnect dialog defaults to keeping details", async () => {
    vi.mocked(commands.disconnect).mockResolvedValue(ok(null));
    const { onDisconnected } = renderAt();
    fireEvent.click(await screen.findByRole("button", { name: "Disconnect…" }));
    const dialog = screen.getByRole("dialog", { name: "Disconnect" });
    expect(dialog.textContent).toContain(
      "Disconnect from git@gitlab.acme:platform/pharos.git? The app forgets the repo, its dashboards and your approval. You'll connect again from the start.",
    );
    const forget = within(dialog).getByLabelText(
      "Also forget my details and secrets for ACME Developer Portal",
    ) as HTMLInputElement;
    expect(forget.checked).toBe(false);
    fireEvent.click(within(dialog).getByRole("button", { name: "Disconnect" }));
    await waitFor(() => expect(commands.disconnect).toHaveBeenCalledWith(false));
    await waitFor(() => expect(onDisconnected).toHaveBeenCalledWith({ url: source.url, branch: "main" }));
  });

  it("disconnect can forget details", async () => {
    vi.mocked(commands.disconnect).mockResolvedValue(ok(null));
    renderAt();
    fireEvent.click(await screen.findByRole("button", { name: "Disconnect…" }));
    const dialog = screen.getByRole("dialog", { name: "Disconnect" });
    fireEvent.click(within(dialog).getByRole("checkbox"));
    fireEvent.click(within(dialog).getByRole("button", { name: "Disconnect" }));
    await waitFor(() => expect(commands.disconnect).toHaveBeenCalledWith(true));
  });

  it("a failed disconnect keeps the dialog open", async () => {
    vi.mocked(commands.disconnect).mockResolvedValue({
      status: "error",
      error: { kind: "keyring", message: "could not update secret `GITLAB_TOKEN` in the OS keyring: locked" },
    });
    const { onDisconnected } = renderAt();
    fireEvent.click(await screen.findByRole("button", { name: "Disconnect…" }));
    const dialog = screen.getByRole("dialog", { name: "Disconnect" });
    fireEvent.click(within(dialog).getByRole("button", { name: "Disconnect" }));
    expect(await within(dialog).findByText(/locked/)).toBeTruthy();
    expect(screen.getByRole("dialog", { name: "Disconnect" })).toBeTruthy();
    expect(onDisconnected).not.toHaveBeenCalled();
  });

  it("focuses the linked field only once", async () => {
    vi.mocked(commands.getSettings).mockImplementation(async () => ok(view()));
    renderAt("/settings?focus=input:team");
    const select = (await screen.findByLabelText(/Your team/)) as HTMLSelectElement;
    await waitFor(() => expect(document.activeElement).toBe(select));
    const secret = screen.getByLabelText("GITLAB_TOKEN value");
    secret.focus();
    fireEvent.change(select, { target: { value: "payments-team" } });
    await waitFor(() => expect(commands.getSettings).toHaveBeenCalledTimes(2));
    await new Promise((r) => setTimeout(r, 20));
    expect(document.activeElement).toBe(secret);
  });

  it("does not save an untouched default on blur", async () => {
    vi.mocked(commands.getSettings).mockResolvedValue(
      ok(
        view({
          inputs: [
            { key: "region", label: "Region", type: "string", required: false, value: null, default: "eu", choices: [], stale: false },
          ],
        }),
      ),
    );
    renderAt();
    const field = (await screen.findByLabelText("Region")) as HTMLInputElement;
    expect(field.value).toBe("eu");
    fireEvent.blur(field);
    await new Promise((r) => setTimeout(r, 20));
    expect(commands.setInput).not.toHaveBeenCalled();
    fireEvent.input(field, { target: { value: "us" } });
    fireEvent.blur(field);
    await waitFor(() => expect(commands.setInput).toHaveBeenCalledWith("region", "us"));
  });

  it("reloads after a sync applies a new commit", async () => {
    const [src, setSrc] = createSignal(source);
    const history = createMemoryHistory();
    history.set({ value: "/settings" });
    render(() => (
      <MemoryRouter history={history}>
        <Route path="/settings" component={() => <Settings source={src()} onChanged={() => {}} onDisconnected={() => {}} />} />
      </MemoryRouter>
    ));
    await screen.findByRole("heading", { name: "Repository" });
    setSrc({ ...source, synced_commit: "b".repeat(40) });
    await waitFor(() => expect(commands.getSettings).toHaveBeenCalledTimes(2));
    expect(document.body.textContent).toContain("bbbbbbb");
  });

  it("explains a failed removal of an unused row", async () => {
    vi.mocked(commands.getSettings).mockResolvedValue(
      ok(view({ unused: [{ platform: "acme-platform", kind: "secret", key: "OLD" }] })),
    );
    vi.mocked(commands.removeStored).mockResolvedValue({
      status: "error",
      error: { kind: "keyring", message: "could not update secret `OLD` in the OS keyring: locked" },
    });
    renderAt();
    const section = await screen.findByTestId("unused");
    fireEvent.click(within(section).getByRole("button", { name: "Remove" }));
    expect(await within(section).findByText(/locked/)).toBeTruthy();
  });
});
