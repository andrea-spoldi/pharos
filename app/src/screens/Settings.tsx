import { useSearchParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, For, Index, Match, on, Show, Switch } from "solid-js";
import { commands, type InputView, type PlatformRef, type RepoSource, type SecretView, type StoredRef } from "../bindings";
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
  // Keyed on the applied commit: a sync applied while Settings is open reloads it.
  const [settings, { refetch }] = createResource(
    () => ({ commit: props.source.synced_commit }),
    () => unwrap(commands.getSettings()),
  );
  const [params] = useSearchParams<{ focus?: string }>();
  const changed = async () => {
    props.onChanged();
    await refetch();
  };

  // Once per visit: refetches after each save must not pull focus back to the linked field.
  let focused = false;
  createEffect(
    on(
      () => settings.latest,
      (s) => {
        if (focused || !s || !params.focus) return;
        focused = true;
        const [kind, key] = params.focus.split(":");
        const field = document.querySelector<HTMLElement>(`#setting-${kind}-${CSS.escape(key ?? "")} :is(input, select)`);
        field?.scrollIntoView({ block: "center" });
        field?.focus();
      },
      { defer: false },
    ),
  );

  const src = () => props.source;
  const [disconnecting, setDisconnecting] = createSignal(false);

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
                <code>{src().url}</code> · <code>{src().branch}</code>
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
              <div class="row">
                <button type="button" onClick={() => setDisconnecting(true)}>
                  Disconnect…
                </button>
              </div>
              <DisconnectDialog
                open={disconnecting()}
                url={src().url}
                platform={s().platform}
                onClose={() => setDisconnecting(false)}
                onDone={() => props.onDisconnected({ url: src().url, branch: src().branch })}
              />
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
                  <For each={s().unused}>{(u) => <UnusedRow row={u} onChanged={changed} />}</For>
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
    // Leaving a field untouched must not pin the default as the user's own value.
    if (value.trim() === shown()) return;
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

  // The stale warning and save errors describe the control, so screen readers hear them on focus.
  const describedBy = () =>
    [props.input.stale && `${id()}-stale`, error() && `${id()}-error`].filter(Boolean).join(" ") || undefined;
  const invalid = () => props.input.stale || !!error() || undefined;

  return (
    <div class="field" id={id()}>
      <div class="row field-head">
        <label for={`${id()}-control`}>
          <span>
            {props.input.label ?? props.input.key}
            <Show when={props.input.required}>
              <span aria-hidden="true"> *</span>
            </Show>
          </span>
        </label>
        {/* Always mounted so the live region announces "Saved" when its text appears. */}
        <span class="muted" role="status">
          {saved() ? "Saved" : ""}
        </span>
      </div>
      <Switch>
        <Match when={props.input.type === "enum"}>
          <select
            id={`${id()}-control`}
            aria-required={props.input.required || undefined}
            aria-invalid={invalid()}
            aria-describedby={describedBy()}
            value={shown()}
            onChange={(e) => save(e.currentTarget.value)}
          >
            <option value="" />
            <For each={props.input.choices}>{(c) => <option value={c}>{c}</option>}</For>
          </select>
        </Match>
        <Match when={props.input.type === "string"}>
          <input
            id={`${id()}-control`}
            aria-required={props.input.required || undefined}
            aria-invalid={invalid()}
            aria-describedby={describedBy()}
            value={shown()}
            onBlur={(e) => save(e.currentTarget.value)}
          />
        </Match>
      </Switch>
      <Show when={props.input.stale}>
        <p class="warning" id={`${id()}-stale`}>
          {props.input.value} is no longer offered, pick another
        </p>
      </Show>
      <Show when={error()}>
        {(e) => (
          <p class="danger-text" id={`${id()}-error`}>
            {e().message}
          </p>
        )}
      </Show>
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
      <div class="row field-head">
        <code class="secret-name">{props.secret.name}</code>
        <span class={props.secret.set ? "ok" : "muted"}>{props.secret.set ? "Set" : "Not set"}</span>
      </div>
      <Show when={props.secret.description}>{(d) => <p class="muted">{d()}</p>}</Show>
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
            aria-invalid={!!error() || undefined}
            aria-describedby={error() ? `${id()}-error` : undefined}
            value={value()}
            onInput={(e) => setValue(e.currentTarget.value)}
          />
          <button type="submit" disabled={!value().trim()}>
            Save
          </button>
        </form>
      </Show>
      <Show when={error()}>
        {(e) => (
          <p class="danger-text" id={`${id()}-error`}>
            {e().message}
          </p>
        )}
      </Show>
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

/** Forgets the repo; the checkbox (off by default) also forgets this platform's details. */
function DisconnectDialog(props: {
  open: boolean;
  url: string;
  platform: PlatformRef | null;
  onClose: () => void;
  onDone: () => void;
}) {
  const [forget, setForget] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<IpcError>();
  const close = () => {
    setForget(false);
    setError(undefined);
    props.onClose();
  };
  const disconnect = async () => {
    setBusy(true);
    setError(undefined);
    try {
      await unwrap(commands.disconnect(forget()));
      props.onDone();
    } catch (err) {
      setError(toIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      title="Disconnect"
      open={props.open}
      onClose={close}
      actions={
        <>
          <button type="button" class="danger" onClick={disconnect} disabled={busy()}>
            Disconnect
          </button>
          <button type="button" onClick={close}>
            Cancel
          </button>
        </>
      }
    >
      <p>
        Disconnect from <code>{props.url}</code>? The app forgets the repo, its dashboards and your approval. You'll connect again
        from the start.
      </p>
      <Show when={props.platform}>
        {(p) => (
          <label class="row">
            <input type="checkbox" checked={forget()} onChange={(e) => setForget(e.currentTarget.checked)} />
            Also forget my details and secrets for {p().title ?? p().name}
          </label>
        )}
      </Show>
      <Show when={error()}>{(e) => <p class="danger-text">{e().message}</p>}</Show>
    </Dialog>
  );
}

function UnusedRow(props: { row: StoredRef; onChanged: () => Promise<void> }) {
  const [error, setError] = createSignal<IpcError>();
  const remove = async () => {
    setError(undefined);
    try {
      await unwrap(commands.removeStored(props.row.platform, props.row.kind, props.row.key));
      await props.onChanged();
    } catch (err) {
      setError(toIpcError(err));
    }
  };
  return (
    <li class="stack">
      <div class="row spread">
        <span>
          <code>{props.row.key}</code> · {props.row.platform}
        </span>
        <button type="button" onClick={remove}>
          Remove
        </button>
      </div>
      <Show when={error()}>{(e) => <p class="danger-text">{e().message}</p>}</Show>
    </li>
  );
}
