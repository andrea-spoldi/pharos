import { useSearchParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, For, Index, Match, on, Show, Switch } from "solid-js";
import { commands, type InputView, type RepoSource, type SecretView } from "../bindings";
import { type IpcError, toIpcError, unwrap } from "../ipc";
import { Dialog } from "../components/Dialog";
import { shortCommit } from "../components/SyncReview";

function ago(unixSeconds: number): string {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - unixSeconds);
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

/**
 * Platform inputs (saved as you edit) and secrets (OS keyring only; values are never shown
 * again). `?focus=input:KEY|secret:NAME` focuses a field, e.g. from a Needs setup card.
 * `onChanged` reloads the dashboards so statuses follow without a sync.
 */
export function Settings(props: {
  source: RepoSource;
  onChanged: () => void;
  onDisconnected: (prev: { url: string; branch: string }) => void;
}) {
  const [settings, { refetch }] = createResource(() => unwrap(commands.getSettings()));
  const [params] = useSearchParams<{ focus?: string }>();
  const changed = async () => {
    props.onChanged();
    await refetch();
  };

  createEffect(
    on(
      () => settings.latest,
      (s) => {
        if (!s || !params.focus) return;
        const [kind, key] = params.focus.split(":");
        const field = document.querySelector<HTMLElement>(`#setting-${kind}-${CSS.escape(key ?? "")} :is(input, select)`);
        field?.scrollIntoView({ block: "center" });
        field?.focus();
      },
      { defer: false },
    ),
  );

  const src = () => settings.latest?.source ?? props.source;

  return (
    <section class="stack settings">
      <Show when={settings.error}>
        <div class="notice danger" role="alert">
          <strong>Could not load settings</strong>
          <pre>{String(settings.error?.message ?? settings.error)}</pre>
        </div>
      </Show>
      <Show when={settings.latest}>
        {(s) => (
          <>
            <section class="stack" aria-labelledby="settings-repo">
              <h2 id="settings-repo">Repository</h2>
              <p>
                {src().url} · {src().branch}
                <Show when={src().synced_commit} fallback={<> · Not synced yet</>}>
                  {(c) => (
                    <>
                      {" "}
                      · <code>{shortCommit(c())}</code>
                      <Show when={src().synced_at}>{(at) => <> · synced {ago(at())}</>}</Show>
                    </>
                  )}
                </Show>
              </p>
            </section>

            <section class="stack" aria-labelledby="settings-details">
              <h2 id="settings-details">Your details</h2>
              <Show when={s().platform} fallback={<p class="muted">Available after the first sync</p>}>
                {/* Index keeps each field mounted across refetches, so focus survives a save. */}
                <Index each={s().inputs} fallback={<p class="muted">The platform asks for no details.</p>}>
                  {(input) => <InputField input={input()} onChanged={changed} />}
                </Index>
              </Show>
            </section>

            <section class="stack" aria-labelledby="settings-secrets">
              <h2 id="settings-secrets">Secrets</h2>
              <Show when={s().platform} fallback={<p class="muted">Available after the first sync</p>}>
                <Index each={s().secrets} fallback={<p class="muted">The platform declares no secrets.</p>}>
                  {(secret) => <SecretRow secret={secret()} onChanged={changed} />}
                </Index>
              </Show>
            </section>

            <Show when={s().unused.length}>
              <section class="stack" aria-labelledby="settings-unused" data-testid="unused">
                <h2 id="settings-unused">No longer used</h2>
                <p class="muted">Stored for a platform or setting the repo no longer declares.</p>
                <ul class="plain">
                  <For each={s().unused}>
                    {(u) => (
                      <li class="row spread">
                        <span>
                          <code>{u.key}</code> · {u.platform}
                        </span>
                        <button
                          type="button"
                          onClick={async () => {
                            await unwrap(commands.removeStored(u.platform, u.kind, u.key));
                            await changed();
                          }}
                        >
                          Remove
                        </button>
                      </li>
                    )}
                  </For>
                </ul>
              </section>
            </Show>
          </>
        )}
      </Show>
    </section>
  );
}

function InputField(props: { input: InputView; onChanged: () => Promise<void> }) {
  const [saved, setSaved] = createSignal(false);
  const [error, setError] = createSignal<IpcError>();
  const id = () => `setting-input-${props.input.key}`;
  const shown = () => props.input.value ?? props.input.default ?? "";

  const save = async (value: string) => {
    setError(undefined);
    setSaved(false);
    try {
      await unwrap(commands.setInput(props.input.key, value.trim() || null));
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
      await props.onChanged();
    } catch (err) {
      setError(toIpcError(err));
    }
  };

  return (
    <div class="field" id={id()}>
      <label for={`${id()}-control`}>
        {props.input.label ?? props.input.key}
        {props.input.required ? " *" : ""}
      </label>
      <Switch>
        <Match when={props.input.type === "enum"}>
          <select
            id={`${id()}-control`}
            value={shown()}
            onChange={(e) => save(e.currentTarget.value)}
          >
            <option value="" />
            <For each={props.input.choices}>{(c) => <option value={c}>{c}</option>}</For>
          </select>
        </Match>
        <Match when={props.input.type === "string"}>
          <input id={`${id()}-control`} value={shown()} onBlur={(e) => save(e.currentTarget.value)} />
        </Match>
      </Switch>
      <Show when={props.input.stale}>
        <p class="warning">{props.input.value} is no longer offered, pick another</p>
      </Show>
      <Show when={saved()}>
        <span class="muted" role="status">
          Saved
        </span>
      </Show>
      <Show when={error()}>{(e) => <p class="danger-text">{e().message}</p>}</Show>
    </div>
  );
}

function SecretRow(props: { secret: SecretView; onChanged: () => Promise<void> }) {
  const [editing, setEditing] = createSignal(false);
  const [value, setValue] = createSignal("");
  const [confirming, setConfirming] = createSignal(false);
  const [error, setError] = createSignal<IpcError>();
  const id = () => `setting-secret-${props.secret.name}`;

  const attempt = async (step: () => Promise<void>) => {
    setError(undefined);
    try {
      await step();
    } catch (err) {
      setError(toIpcError(err));
    }
  };
  const save = () =>
    attempt(async () => {
      await unwrap(commands.setSecret(props.secret.name, value()));
      setValue("");
      setEditing(false);
      await props.onChanged();
    });
  const remove = () =>
    attempt(async () => {
      setConfirming(false);
      await unwrap(commands.removeSecret(props.secret.name));
      await props.onChanged();
    });

  return (
    <div class="field" id={id()} data-testid={`secret-${props.secret.name}`}>
      <strong>
        <code>{props.secret.name}</code>
      </strong>
      <Show when={props.secret.description}>{(d) => <span class="muted"> {d()}</span>}</Show>
      <p>{props.secret.set ? "Set" : "Not set"}</p>
      <Show
        when={!props.secret.set || editing()}
        fallback={
          <div class="row">
            <button type="button" onClick={() => setEditing(true)}>
              Replace
            </button>
            <button type="button" onClick={() => setConfirming(true)}>
              Remove
            </button>
          </div>
        }
      >
        <form
          class="row"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <input
            type="password"
            autocomplete="off"
            aria-label={`${props.secret.name} value`}
            value={value()}
            onInput={(e) => setValue(e.currentTarget.value)}
          />
          <button type="submit" disabled={!value().trim()}>
            Save
          </button>
        </form>
      </Show>
      <Show when={error()}>{(e) => <p class="danger-text">{e().message}</p>}</Show>
      <Dialog
        title={`Remove ${props.secret.name}?`}
        open={confirming()}
        onClose={() => setConfirming(false)}
        actions={
          <>
            <button type="button" class="danger" onClick={remove}>
              Remove
            </button>
            <button type="button" onClick={() => setConfirming(false)}>
              Cancel
            </button>
          </>
        }
      >
        <p>The value is deleted from your OS keyring. Widgets that need it will ask for setup again.</p>
      </Dialog>
    </div>
  );
}
