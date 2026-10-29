import { createResource, createSignal, Match, Show, Switch } from "solid-js";
import { commands, type ErrorKind, type RepoSource } from "../bindings";
import { type IpcError, toIpcError, unwrap } from "../ipc";

const ERROR_TITLES: Partial<Record<ErrorKind, string>> = {
  invalidBranch: "That branch name isn't valid",
  git: "git could not clone the repository",
  gitTimedOut: "git took too long and was stopped",
  sourceExists: "A platform repo is already connected",
};

const GIT_HINT =
  "The message above is git's own. Check the URL and branch, and that you can clone it from a terminal. " +
  "For an SSH URL, connect to the host once from a terminal so its host key is known.";

/** `initialUrl` / `initialBranch` pre-fill the form, e.g. after disconnecting to fix the URL. */
export function Onboarding(props: {
  onConnected: (source: RepoSource) => void;
  initialUrl?: string;
  initialBranch?: string;
}) {
  const [git, { refetch }] = createResource(() => unwrap(commands.gitVersion()));
  // Reading an errored resource throws, so gate on its state instead.
  const gitFound = () => git.state === "ready";
  const [url, setUrl] = createSignal(props.initialUrl ?? "");
  const [branch, setBranch] = createSignal(props.initialBranch ?? "main");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<IpcError>();

  const submit = async (e: SubmitEvent) => {
    e.preventDefault();
    if (busy()) return;
    setBusy(true);
    setError(undefined);
    try {
      props.onConnected(await unwrap(commands.addSource(url(), branch())));
    } catch (err) {
      setError(toIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <main class="screen onboarding">
      <header>
        <h1>Connect your platform repo</h1>
        <p class="muted">
          Pharos shows the widgets and dashboards your platform team keeps in a git repo. It
          clones the repo into its own data folder and never writes back to it.
        </p>
      </header>

      <section class="git-check" aria-live="polite">
        <Switch>
          <Match when={git.loading}>
            <p class="muted">Checking for git…</p>
          </Match>
          <Match when={git.error}>
            <div class="notice danger" role="alert">
              <strong>git was not found</strong>
              <p>
                Pharos uses the git on your PATH, with your SSH keys and credential helpers.
                Install git (on macOS: <code>xcode-select --install</code>), then check again.
              </p>
              <button type="button" onClick={refetch}>
                Check again
              </button>
            </div>
          </Match>
          <Match when={gitFound()}>
            <p class="ok" data-testid="git-version">
              ✓ {git()}
            </p>
          </Match>
        </Switch>
      </section>

      <Show when={gitFound()}>
        <form class="stack" onSubmit={submit}>
          <label>
            <span>Repository URL or local path</span>
            <input
              name="url"
              value={url()}
              onInput={(e) => setUrl(e.currentTarget.value)}
              placeholder="git@gitlab.acme:platform/pharos.git or ~/src/platform-repo"
              autofocus
              autocomplete="off"
              autocapitalize="off"
              spellcheck={false}
              disabled={busy()}
            />
          </label>
          <label>
            <span>Branch</span>
            <input
              name="branch"
              value={branch()}
              onInput={(e) => setBranch(e.currentTarget.value)}
              autocomplete="off"
              autocapitalize="off"
              spellcheck={false}
              disabled={busy()}
            />
          </label>
          <div class="row">
            <button type="submit" class="primary" disabled={busy() || !url().trim() || !branch().trim()}>
              {busy() ? "Cloning…" : "Connect"}
            </button>
            <Show when={busy()}>
              <span class="muted" role="status">
                Cloning the repo. This can take a minute.
              </span>
            </Show>
          </div>
        </form>
      </Show>

      <Show when={error()}>
        {(err) => (
          <div class="notice danger" role="alert">
            <strong>{ERROR_TITLES[err().kind] ?? "Something went wrong"}</strong>
            <pre>{err().message}</pre>
            <Show when={err().kind === "git" || err().kind === "gitTimedOut"}>
              <p class="muted">{GIT_HINT}</p>
            </Show>
          </div>
        )}
      </Show>
    </main>
  );
}
