import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands, type ErrorKind } from "../bindings";
import { Onboarding } from "./Onboarding";

vi.mock("../bindings", () => ({
  commands: { gitVersion: vi.fn(), addSource: vi.fn() },
}));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });
const err = (kind: ErrorKind, message: string) => ({ status: "error" as const, error: { kind, message } });

const source = {
  id: 1,
  url: "/repos/platform",
  branch: "main",
  local_path: "/data/sources/1",
  synced_commit: null,
  synced_at: null,
  approved_capability_hash: null,
};

function type(name: string, value: string) {
  const input = document.querySelector<HTMLInputElement>(`input[name=${name}]`)!;
  fireEvent.input(input, { target: { value } });
}

describe("Onboarding", () => {
  beforeEach(() => {
    vi.mocked(commands.gitVersion).mockReset().mockResolvedValue(ok("git version 2.50.0"));
    vi.mocked(commands.addSource).mockReset();
  });

  it("checks git before asking for the repo", async () => {
    render(() => <Onboarding onConnected={() => {}} />);
    expect((await screen.findByTestId("git-version")).textContent).toContain("git version 2.50.0");
    expect(document.querySelector("input[name=branch]")).toHaveProperty("value", "main");
  });

  it("pre-fills url and branch", async () => {
    render(() => <Onboarding onConnected={() => {}} initialUrl="git@gitlab.acme:platform/pharos.git" initialBranch="dev" />);
    await screen.findByTestId("git-version");
    expect(document.querySelector("input[name=url]")).toHaveProperty("value", "git@gitlab.acme:platform/pharos.git");
    expect(document.querySelector("input[name=branch]")).toHaveProperty("value", "dev");
  });

  it("explains a missing git and checks again", async () => {
    vi.mocked(commands.gitVersion).mockResolvedValueOnce(err("gitMissing", "git is not installed"));
    render(() => <Onboarding onConnected={() => {}} />);
    expect(await screen.findByText("git was not found")).toBeTruthy();
    expect(document.querySelector("form")).toBeNull();

    fireEvent.click(screen.getByText("Check again"));
    expect(await screen.findByTestId("git-version")).toBeTruthy();
    expect(commands.gitVersion).toHaveBeenCalledTimes(2);
  });

  it("connects only once a URL is entered", async () => {
    const onConnected = vi.fn();
    vi.mocked(commands.addSource).mockResolvedValue(ok(source));
    render(() => <Onboarding onConnected={onConnected} />);
    await screen.findByTestId("git-version");

    const connect = screen.getByText("Connect") as HTMLButtonElement;
    expect(connect.disabled).toBe(true);

    type("url", "/repos/platform");
    type("branch", "release");
    fireEvent.click(connect);

    await waitFor(() => expect(onConnected).toHaveBeenCalledWith(source));
    expect(commands.addSource).toHaveBeenCalledWith("/repos/platform", "release");
  });

  it("shows git's error and lets the user retry", async () => {
    vi.mocked(commands.addSource).mockResolvedValue(
      err("git", "`git clone` failed: fatal: repository '/nope' does not exist"),
    );
    render(() => <Onboarding onConnected={() => {}} />);
    await screen.findByTestId("git-version");

    type("url", "/nope");
    fireEvent.click(screen.getByText("Connect"));

    const alert = await screen.findByRole("alert");
    expect(alert.textContent).toContain("git could not clone the repository");
    expect(alert.textContent).toContain("repository '/nope' does not exist");
    expect((screen.getByText("Connect") as HTMLButtonElement).disabled).toBe(false);
  });
});
