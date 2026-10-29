# Slice 5 — Settings: inputs, secrets, repo connection (T-006) — design

Status: draft for approval · 2026-10-29 · session S-008

## Intent

Widgets need things only the developer knows: which team they're on, a personal GitLab
token. The platform repo declares them (`pharos.yaml` `spec.inputs` / `spec.secrets`), and
the dashboard already shows **Needs setup** when they're missing (D-024). This slice gives
the developer one place to provide them: a **Settings** screen opened from the header.
It never blocks onboarding.

Settings is also where you go when the first setup or a sync goes wrong. It shows the repo
connection and lets you disconnect and connect again, e.g. to fix a wrong URL or branch.

Decided by the user:
- Setup lives in **Settings**, not in an onboarding step (restart-plan §5 said onboarding,
  and this replaces that).
- Settings also holds the **repo connection**.
- The order of slices stays. The http executor is slice 6, so a widget whose setup is complete
  but whose source is http moves from *Needs setup* to *Not available yet*.

**Done when** (replaces §5's "token → renders"): set *team* and *GITLAB_TOKEN* in Settings,
and My Pipelines leaves *Needs setup* (it shows *Not available yet — http sources*).
Restart: both are still set. Remove the token: *Needs setup — Needs secret GITLAB_TOKEN*
again.

## Goals

1. A Settings screen with three sections: **Repository**, **Your details** (Platform
   inputs) and **Secrets**.
2. Inputs stored in SQLite. Secret values stored only in the OS keyring (macOS Keychain,
   Linux Secret Service, D-006), with only the names in SQLite.
3. The engine's `needsSetup` lists only what is still missing.
4. *Needs setup* cards link to the missing field in Settings.
5. Disconnect the repo, then connect again with the previous URL and branch pre-filled.

## Non-goals

- Using inputs and secrets at run time (templating `{{ user.X }}` and `{{ secrets.X }}` into
  requests). That arrives with the http and command executors (slices 6–7).
- Instance inputs (`{{ inputs.X }}` per placement). Placements set them in YAML, and user
  dashboards (slice 8) edit them.
- A setup step during onboarding, or a "setup missing" badge on the header.
- Multiple repos (D-002), environments or team scoping (slice 13).
- Revealing, copying or exporting a stored secret.

## Anti-goals

- A secret value is ever returned to the frontend, written to SQLite, logged, or included
  in an error message.
- Accepting a secret or input name the platform repo doesn't declare.
- Forgetting the developer's details just because they fixed the repo URL.

## Screen

```
┌ ACME Developer Portal ─────────────────────── [ Settings ] [ Sync now ] ┐
│ ← Back to dashboards                                                    │
│ Repository                                                              │
│   git@gitlab.acme:platform/pharos.git · main · abc1234 · synced 2m ago  │
│   [ Disconnect… ]                                                       │
│ Your details                                                            │
│   Your team *          [ payments-team      ▾ ]                         │
│ Secrets                                                                 │
│   GITLAB_TOKEN         Personal access token (read_api)                 │
│   Not set              [ •••••••••••• ] [ Save ]                        │
│   — once set: "Set" · [ Replace ] [ Remove ]                            │
└─────────────────────────────────────────────────────────────────────────┘
```

- **Route:** `#/settings`. A header **Settings** button opens it, and *Back to dashboards*
  returns to the last dashboard. Sync now, the changelog and the review stay in the header
  and main area exactly as today. A pending review still replaces the main area on both
  routes.
- **Before the first apply** there is no platform manifest. *Your details* and *Secrets*
  then say "Available after the first sync", and Repository is still shown, so a broken first
  setup can be fixed from here.
- **Repository:** URL, branch, the short commit and relative time of the last applied sync
  (or "Not synced yet"), and **Disconnect…**.
- **Your details:** one field per `spec.inputs` key, in key order, labelled with `label`
  (falling back to the key), and required ones marked `*`.
  - `type: string` is a text field.
  - `type: enum` is a select. Its choices come from static `values`, or from the `field`
    dot path of each YAML file in the `from` catalog directory, unique and sorted.
  - The field shows the `default` when the user hasn't set one. Saving an empty value
    clears it.
  - Changes save on blur/change (no Save button). A short "Saved" confirmation appears.
- **Secrets:** one row per `spec.secrets` entry, with its name and `description`. Status is
  *Not set* or *Set*.
  - *Not set*: a password field plus **Save**.
  - *Set*: **Replace** (shows the password field again) and **Remove** (confirmed with an
    in-app dialog, never `confirm()`).
  - After saving, the field is cleared and the value is never shown again.
- **Disconnect…** opens an in-app dialog:
  - text: "Disconnect from `<url>`? The app forgets the repo, its dashboards and your approval.
    You'll connect again from the start."
  - a checkbox **Also forget my details and secrets for `<platform title>`**, off by
    default
  - buttons **Disconnect** and **Cancel**

  After disconnecting, the app shows Onboarding with the previous URL and branch
  pre-filled.
- **Needs setup card:** gets an **Open Settings** button that navigates to
  `#/settings?focus=<input:KEY | secret:NAME>` (the first missing item). Settings scrolls
  to that field and focuses it.
- Back on the dashboards, statuses reflect the new settings without a sync. Any
  successful settings change bumps the same reload version the dashboards already listen to.

## Storage and engine

- **Keyed by platform, not by source:** inputs and secret names are stored under the
  Platform `metadata.name` (`pharos.yaml`). So disconnecting and reconnecting the same
  platform, for example with a corrected URL, keeps them. Only the "also forget" checkbox
  removes them (anti-goal 3).
- **Migration `002_settings.sql`** via `rusqlite_migration`:
  - `user_inputs (platform TEXT, key TEXT, value TEXT NOT NULL, PRIMARY KEY (platform, key))`
  - `secrets_meta (platform TEXT, name TEXT, set_at INTEGER NOT NULL, PRIMARY KEY (platform, name))`
- **Keyring:** service `io.pharos.desktop`, account `<platform>/<SECRET_NAME>`. This goes
  behind a small engine trait (`SecretStore`: `set`, `delete`; no `get` in this slice):
  - `Engine::open` uses the OS keyring.
  - Tests inject an in-memory store.
  - Use the `keyring` crate with its native macOS and Linux Secret Service backends.
  - The engine stays Tauri-free.
- **`set` writes to the keyring first, then `secrets_meta`.** A keyring failure leaves
  nothing recorded and returns an error that names the secret but never the value.
- **`delete`** removes the keyring entry, then the row. A keyring entry that is already
  missing counts as success.
- **The status rule changes:**
  - An input counts as **set** if the user has a value or the InputDef has a `default`.
  - A secret counts as set if `secrets_meta` has a row for it. The keyring isn't read to
    decide this. If the entry was deleted outside the app, slice 6's run reports it.
  - `needsSetup` lists only the inputs and secrets that aren't set. If nothing is missing,
    the status falls through to the next rule (notAvailable / static / runnable).
- **Validation** (engine; a frontend check is a convenience only):
  - The key or name must be declared in the applied platform manifest. Otherwise the
    error is `undeclared`.
  - An enum value must be one of the current choices.
  - A secret value must be non-empty after trimming. Store the trimmed value.
  - Before the first apply, all of these are refused (`noPlatform`).
- **Disconnect**, in this order, all-or-nothing on secrets (my call):
  1. With forget, delete that platform's keyring entries first. If any delete fails (other
     than "not found"), stop and return `keyring`. The repo stays connected and nothing else
     changes, so the user can retry or untick the checkbox. A secret the user believes is
     gone must never be left behind silently.
  2. With forget, delete the platform's `user_inputs` and `secrets_meta` rows.
  3. Drop any pending plan and delete the `sources` row (snapshot tables cascade).
  4. Remove the clone directory last. A failure here is not an error. The directory is
     app-owned, and `add_source` already replaces an existing destination.

### Values after a later sync changes the manifest (my calls)

- **An enum value that is no longer a choice** counts as **missing**: `needsSetup` lists it.
  The engine checks the value against the current choices on every status computation.
  Settings shows the select with a warning, "`<value>` is no longer offered, pick another",
  and keeps the stored value until the user changes it. Reading the catalog for choices
  reuses the file executor's confinement and file cap. That cost is accepted.
- **Stored rows the current manifest no longer declares** stay dormant: never used, never
  counted. This covers an input key or secret dropped from `pharos.yaml`, and all rows stored
  under a different platform `metadata.name` (a renamed platform). Settings lists them in
  a **No longer used** section with **Remove** on each. Removal is allowed for any stored row,
  declared or not, because it only ever deletes. Setting stays restricted to declared names.
  A renamed platform therefore starts with empty details. The old values are visible under
  *No longer used* and aren't migrated automatically.

## Commands (tauri-specta)

| Command | Args | Returns |
|---|---|---|
| `get_settings` | — | `SettingsView { source, platform: Option<{ name, title }>, inputs: [{ key, label, type, required, value, default, choices }], secrets: [{ name, description, set }], unused: [{ platform, kind, key }] }`. An input whose stored value isn't a current choice carries `stale: true` |
| `set_input` | `key, value: Option<String>` | `()` |
| `set_secret` | `name, value` | `()` |
| `remove_secret` | `name` | `()` |
| `remove_stored` | `platform, kind: input \| secret, key` | `()` — for *No longer used* rows |
| `disconnect` | `forget: bool` | `()` |

`set_secret` is the only command that carries a secret value into the backend.
CLAUDE.md's "never accept secret names from the frontend" is about *execution*: running
still resolves only manifest-declared secrets. Here the frontend names which declared
secret it is *setting*, and the engine checks the name against the manifest. Record this
as a decision.

New `ErrorKind`s: `undeclared`, `invalidValue`, `noPlatform`, `keyring`.

## Constraints

- CLAUDE.md security rules, especially: secret values only in the OS keyring, names only in
  SQLite; strict CSP; in-app dialogs only.
- Engine and manifest crates stay Tauri-free. New types derive `specta` behind the feature
  (D-016). Bindings are regenerated by `cargo test`.
- Plain CSS with the existing tokens.

## Tests and demo

- **Engine:**
  - set/clear an input
  - an enum rejects a value outside its choices; choices come from `catalog/teams/`
  - an undeclared key or name is refused
  - a secret goes to the store, the meta row exists, and the value is never in SQLite
    (scan the db file bytes for the value)
  - remove
  - `needsSetup` shrinks as items are set, and a `default` counts as set
  - disconnect without forget keeps inputs/secrets, and reconnecting the same platform
    shows them set
  - disconnect with forget deletes rows and store entries
  - disconnect with forget whose store delete fails keeps the repo, rows and entries
    (returns `keyring`)
  - a stored team removed from `catalog/teams/` by a sync → `needsSetup` again, and
    `stale: true`
  - a secret dropped from `pharos.yaml` → listed in `unused` and removable via
    `remove_stored` (store entry deleted too)
  - a keyring failure records nothing
- **Frontend:**
  - Settings renders the sections, and "Available after the first sync" before apply
  - an enum select lists the choices; changing it calls `setInput`
  - the secret flow: Not set → Save → Set; Replace; Remove via the dialog
  - the password field is cleared after save
  - Disconnect dialog: the forget checkbox defaults off and is passed through; afterwards
    Onboarding is pre-filled
  - a Needs setup card links to `#/settings?focus=…`, and the field gets focus
- **Demo `docs/demos/05-settings.md`:** the "done when" above, plus disconnect → reconnect
  with a pre-filled URL keeping *team*, and disconnect with forget clearing it. Include a
  check that the token is in Keychain Access (macOS) under `io.pharos.desktop`.

## Left to the builder

- Component/file names (suggested `app/src/screens/Settings.tsx`, a small
  `components/Dialog.tsx` reusable for slice 8).
- How the reload version is shared between Settings and Dashboards (context or lifted
  signal).
- Exact copy beyond what's quoted, and layout within the tokens.
- `keyring` crate version and feature flags, as long as it uses the native backends and
  no plain-file fallback.
