# Pharos — Restart Plan (Tauri 2 + SolidJS)

> Written 2026-10-27 after reviewing `developer-cockpit` (MVP + specs 001–003).
> Purpose: seed a fresh repo. Carry over concepts and lessons, not code.

---

## 1. What we're building

**A desktop "local IDP" that a platform engineering team distributes to developers.**

The platform team owns one Git repo (the **platform repo**) containing everything:
widget definitions, dashboards, and catalog data (services, teams, runbooks) as YAML.
Developers install the app, point it at the platform repo (or get a build that is
pre-pointed), and get a ready-made Pharos. Staying current = the app pulls the repo.

```
 Platform team                          Developer laptop
 ─────────────                          ─────────────────────────────────────
 platform repo (git)  ── git pull ──▶   Pharos app
  ├─ pharos.yaml                         ├─ clones repo into app data
  ├─ widgets/*/widget.yaml                ├─ validates + shows what changed
  ├─ dashboards/*.yaml                    ├─ asks consent for new capabilities
  └─ catalog/**/*.yaml                    ├─ renders platform dashboards
                                          ├─ prompts for personal inputs/secrets
 CI: `pharos lint` on every PR           └─ executes widgets locally (file/cmd/http)
```

Key properties:

- **Git is the control plane and the data plane.** No server. Catalog = YAML in the repo.
- **Local execution.** Widgets run on the developer's machine with *their* credentials
  (kubectl context, GitLab token in keyring, local CLIs).
- **Platform-curated, user-personalised.** Platform ships defaults; users add inputs,
  secrets, and their own dashboards on top. Platform updates never wipe user state.
- **Declarative only.** Widgets are YAML (data source + view). No custom JS/WASM in v1.

### Non-goals (v1)

WASM runtime, widget marketplace, multiple platform repos, personal/user-added widgets,
custom widget UI bundles, catalog entity pages, AI widget generation, auto file-watching,
background periodic sync, Windows.

---

## 2. What the current prototype taught us

### Worth keeping (concepts)
- Manifest shape `apiVersion / kind / metadata / spec` with `runtime` + `view`.
- `file` runtime with glob → flat array + `_source` — this *is* the catalog primitive.
- Sync = plan → review → apply; identity by `metadata.name`; dashboards survive updates.
- Path canonicalisation + allow-list for filesystem reads (`evaluator.rs` test is good).
- Secrets in OS keyring, only names in SQLite, resolved server-side.

### Problems to design out

| # | Problem in prototype | Root cause | New design |
|---|---|---|---|
| 1 | IPC arg mismatches (`widget_id` vs `widgetId`) silently broke calls (Scenario 2 needed fixes) | Hand-written untyped `invoke()` wrapper | Generate TS bindings from Rust with **tauri-specta** |
| 2 | Frontend sends URL, headers, `secret_names`, executable, args, wasm path to backend | Runtimes take execution details from the UI | Backend executes from the **stored manifest** only: `run_instance(instance_id)` |
| 3 | Any widget can request any secret | `secret_names` not checked against manifest | Secrets must be declared in manifest; backend resolves only declared ones |
| 4 | `wasm_execute` has no permission check | Spike left in the product | Drop WASM |
| 5 | No way to put an already-installed widget on a dashboard; only install→configure→add | Library modelled as a wizard, not a catalog | Library = browse + "Add to dashboard" any time |
| 6 | Synced widgets can't be placed; platform can't ship dashboards | Dashboards are user-only | `kind: Dashboard` in platform repo |
| 7 | Grid is static CSS; drag/resize dep unused; overlapping placement | Layout never finished | Explicit layout model; auto-place; drag/resize as a later slice |
| 8 | `prompt()` for dashboard name (unreliable in Tauri webview) | Shortcut | In-app dialogs only |
| 9 | Every widget instance calls `listWidgets()`; scheduler is a hook-named plain fn inside `useEffect` | Ad-hoc data fetching | Central Solid store + `createResource`; one scheduler in backend or a single frontend ticker |
| 10 | Per-widget permission approval meaningless (all approved at once) | Trust modelled per widget | Trust at **source** level: approve platform repo capabilities, re-consent only on diff |
| 11 | "Sync from folder" — user manages git themselves | Folder picker, no git | App clones/pulls the repo (shell out to system `git`, reuses user's SSH/credential helper) |
| 12 | Migrations = re-run `CREATE IF NOT EXISTS` files | No versioning | `rusqlite_migration` with `user_version` |
| 13 | `csp: null` | Default | Strict CSP from day one |
| 14 | Specs/tasks drifted from code; all tasks ✅ but flows broken | Manual verification only | Every slice ends with an automated check + a demo script |

---

## 3. Domain model

### Platform repo layout (contract with platform teams)

```
platform-repo/
├─ pharos.yaml              # kind: Platform — name, branding, required inputs/secrets
├─ widgets/
│  ├─ service-catalog/widget.yaml
│  ├─ my-pipelines/widget.yaml
│  └─ k8s-context/widget.yaml
├─ dashboards/
│  └─ home.yaml              # kind: Dashboard — default layout
└─ catalog/
   ├─ services/*.yaml
   └─ teams/*.yaml
```

### Kinds

```yaml
# pharos.yaml
apiVersion: pharos.io/v1alpha1
kind: Platform
metadata: { name: acme-platform, title: ACME Developer Portal }
spec:
  inputs:                      # asked once per user, available to all widgets as {{ user.X }}
    team: { type: string, required: true, label: "Your team" }
  secrets:
    - { name: GITLAB_TOKEN, description: "Personal access token (read_api)" }
  widgets: widgets/            # discovery roots (defaults shown)
  dashboards: dashboards/
```

```yaml
# widgets/service-catalog/widget.yaml
apiVersion: pharos.io/v1alpha1
kind: Widget
metadata: { name: service-catalog, title: Service Catalog, version: 0.1.0 }
spec:
  source:
    type: file                 # file | command | http   (static = view-only, no source)
    path: ../../catalog/services/   # relative to widget dir, must stay inside the repo
    glob: "*.yaml"
  refresh: 5m
  view:
    type: table
    columns: [ { field: name }, { field: owner }, { field: tier } ]
```

```yaml
# dashboards/home.yaml
apiVersion: pharos.io/v1alpha1
kind: Dashboard
metadata: { name: home, title: Home }
spec:
  widgets:
    - { widget: service-catalog, x: 0, y: 0, w: 8, h: 4 }
    - { widget: my-pipelines,    x: 8, y: 0, w: 4, h: 4, inputs: { project: "{{ user.team }}" } }
```

Changes vs prototype manifest:
- `runtime` + per-runtime blocks → single `spec.source` with `type` (one place to look).
- Permissions **derived** from `source` (host from URL, executable from command, path from file)
  instead of hand-declared and drift-prone. Optional explicit `permissions` only to widen
  (e.g. multiple hosts).
- File paths are confined to the platform repo root (not just the widget dir) so widgets
  can read shared `catalog/`.
- `{{ user.X }}` for platform-level inputs, `{{ inputs.X }}` for instance inputs,
  `{{ secrets.X }}` only in http headers/url and only if declared.

### Local state (SQLite)

| Table | Purpose |
|---|---|
| `sources` | platform repo: url, local path, branch, last synced commit, approved capability hash |
| `widgets` | snapshot of each manifest at the synced commit (source_id, name, manifest_json) |
| `dashboards` | id, origin (`platform:<name>` or `user`), title |
| `placements` | dashboard_id, widget name, layout, instance inputs |
| `user_inputs` | key/value for Platform `spec.inputs` |
| `secrets_meta` | names only; values in keyring |
| `settings` | key/value |

Platform dashboards are stored as read-only templates; "Customize" forks them into a user
dashboard. Re-sync updates templates, never forks.

### Trust / permissions

1. On first sync, show a **capability summary** for the whole repo
   (hosts, executables, paths outside repo — grouped by widget). One consent.
2. Store a hash of the approved capability set.
3. On later syncs, if capabilities are unchanged → apply silently and show a changelog
   toast. If changed → show only the **diff** for consent; until approved, affected
   widgets show "Needs approval" and don't execute.
4. Backend enforces at execution time against the approved set (defence in depth).

---

## 4. Architecture

```
src-tauri/                 Tauri shell, commands (thin), app state
crates/
  pharos-manifest/        kinds, parsing, validation, permission derivation (no Tauri dep)
  pharos-engine/          source sync (git), executors (file/command/http), templating,
                           permission enforcement, scheduler; SQLite repos
  pharos-cli/             `pharos lint <repo>` and `pharos run <widget>` for platform CI
app/                       SolidJS + Vite + TS
  src/bindings.ts          generated by tauri-specta — never hand-edit
  src/stores/              Solid stores (sources, dashboards, widgets, run results)
  src/views/               table / list / card / kv / raw / markdown renderers
  src/screens/             Onboarding, Home(dashboards), Catalog(library), Settings
```

Decisions:
- **Typed IPC**: `tauri-specta` + `specta` derive on all command args/results.
- **Execution in backend**: frontend calls `run_instance(id)` / subscribes to
  `instance-result` events. Backend owns scheduling (one tokio task per refresh interval),
  so results keep updating regardless of which screen is open.
- **Git**: shell out to system `git` (inherits SSH agent / credential helpers; avoids
  libgit2 auth pain). Clone into `$APP_DATA/sources/<id>`; `fetch` + `reset --hard` on sync
  (the local clone is app-owned, never edited).
- **Engine crate is Tauri-free** → unit-testable and reused by the CLI.
- **Frontend**: SolidJS, `@solidjs/router`, plain CSS with design tokens (light/dark).
  No component library initially. Grid: CSS grid from layout data; drag/resize slice
  later (evaluate `gridstack` — framework-agnostic).
- **Distribution**: build can embed a default `platform.json`
  (`{ "url": "git@gitlab.acme:platform/pharos.git", "branch": "main" }`) so onboarding
  is one click for platform-distributed builds; unbranded builds ask for the URL.

---

## 5. Delivery plan — thin vertical slices

Each slice ends with: automated tests green + a 3–5 step demo in `docs/demos/<slice>.md`
run in the real app. No slice is "done" on checkboxes alone.

| # | Slice | Size | Done when |
|---|---|---|---|
| 0 | **Scaffold**: Tauri 2 + Solid + Vite + tauri-specta, strict CSP, CI (`cargo test`, `clippy`, `tsc`, `vitest`), one `ping` command via generated bindings | S | App window shows "pong" from Rust; CI green |
| 1 | **Manifest crate + `pharos lint`**: Platform/Widget/Dashboard kinds, validation with structured errors (file, path, message), permission derivation. Example repo in `examples/platform-repo/` | M | `pharos lint examples/platform-repo` passes; broken fixtures produce precise errors (snapshot tests) |
| 2 | **Source onboarding + sync**: first-run screen (URL or local path), clone, lint, capability consent, store snapshot; "Sync now" with changelog | M | Fresh install → enter repo → consent → widgets listed. Edit repo, sync → changelog shows diff |
| 3 | **File executor + table/list/card views**: `run_instance`, repo-confined paths, glob, `_source` | M | Service catalog renders from `catalog/services/*.yaml`; traversal test denies `../../etc` |
| 4 | **Platform dashboards**: render `kind: Dashboard` read-only; dashboard switcher; empty/loading/error/needs-setup states designed once and reused | M | Home dashboard from repo appears after sync with zero user clicks |
| 5 | **User inputs + secrets**: Platform `spec.inputs` form on onboarding; keyring secrets screen; "Setup required" widget state links to the missing input/secret | M | Widget needing `GITLAB_TOKEN` shows setup CTA → add token → renders |
| 6 | **HTTP executor** (declared secrets only, host enforcement) | S | GitLab pipelines widget works; undeclared secret reference fails lint |
| 7 | **Command executor** (executable + args, no shell, timeout, stderr/exit code) | S | `kubectl config current-context` widget works; unapproved executable denied |
| 8 | **User dashboards**: create/rename/delete (in-app dialogs), "Customize" fork of platform dashboard, add any widget from Catalog screen, auto-placement | M | Fork Home, add a widget, restart app → layout persists; re-sync doesn't touch fork |
| 9 | **Capability diff on re-sync** | S | Add a host to a widget in repo → sync → only that diff is asked; widget blocked until approved |
| 10 | **Layout editing**: drag/resize | M | Resize/move persists across restart |
| 11 | **Command palette**: jump to dashboard/widget, sync, refresh all | S | ⌘K → type widget name → focuses it |
| 12 | **Pre-configured build**: embedded `platform.json`, app name/icon override | S | Build with config → first launch skips URL entry |
| 13 | **Environments + team scoping** (§6.1): env switcher, `{{ env.X }}`, team picker from catalog, row `filter` | M | Switch dev→prod → pipelines widget hits the prod URL; team picker narrows the service catalog |

Slices 0–5 are the "it works for the IDP story" milestone. 6–7 bring back the prototype's
runtimes. 8+ is personalisation.

---

## 6. Decisions (2026-10-27)

| # | Question | Decision | Consequence for the plan |
|---|---|---|---|
| D1 | Git auth | System `git` is a prerequisite, already installed and authenticated | Shell out to `git`; no credential handling in the app. Onboarding checks `git --version` and reports clone errors verbatim (auth failures included) |
| D2 | One repo or many | **One platform repo in v1.** Stages and teams are modelled *inside* that repo, not as extra repos | `sources` table keeps a `source_id` column so multi-repo is a later migration, not a rewrite. See §6.1 |
| D3 | Personal widgets | **Not in v1.** Platform-only, to validate the model | No local-folder source, no per-widget install flow. Catalog screen = browse platform widgets + add to dashboard |
| D4 | Sync cadence | On launch + on demand ("Sync now") | No background timer. Launch sync is non-blocking: show last snapshot immediately, apply changes when the fetch completes |
| D5 | Catalog model | Plain YAML, **Kubernetes-style** (`apiVersion/kind/metadata/spec`) for catalog entries too; no entity pages in v1 | View `columns[].field` supports dot paths (`metadata.name`, `spec.owner`). `pharos lint` checks catalog files are well-formed k8s-style YAML but does not enforce a schema per kind |
| D6 | OS support | macOS + Linux | Keyring: macOS Keychain + Secret Service. CI matrix: `macos-latest`, `ubuntu-latest`. No Windows paths/quirks |

### 6.1 Stages and teams within one repo (proposal for D2)

Instead of one repo per stage or team, let `pharos.yaml` declare **environments**, and let
`user.team` (a Platform input) select team-relevant data:

```yaml
# pharos.yaml
spec:
  environments:              # user picks the active one from a switcher in the top bar
    - { name: dev,     vars: { gitlabUrl: https://gitlab.dev.acme,  argocd: https://argo.dev.acme } }
    - { name: staging, vars: { gitlabUrl: https://gitlab.acme,      argocd: https://argo.stg.acme } }
    - { name: prod,    vars: { gitlabUrl: https://gitlab.acme,      argocd: https://argo.acme } }
  inputs:
    team: { type: enum, from: catalog/teams/, field: metadata.name }   # team picker fed by the catalog
```

Widgets reference `{{ env.argocd }}` and `{{ user.team }}`; dashboards can filter rows with
`filter: { spec.owner: "{{ user.team }}" }`. One repo, one consent, and switching stage
re-runs the widgets. Permission derivation expands over **all** environments, so consent
covers every stage at once.

If a real need for separate repos appears (e.g. a team that owns its own widget set), that
becomes the multi-source epic: the platform repo stays primary, and extra repos are
declared in `pharos.yaml` (`spec.includes`). The platform team stays in control, and users
still don't add sources themselves.

---

## 7. Carry-over checklist for the new repo

- [ ] Copy `examples/widgets/file-catalog/catalog/*.yaml` as seed catalog data
- [ ] Port `check_filesystem_read_allowed` tests (adapted to repo-root confinement)
- [ ] Port glob merge + `_source` behaviour and the 500-file cap
- [ ] Lean `CLAUDE.md` (<50 lines): stack, commands, "bindings.ts is generated", security rules
- [ ] `SESSIONS.md` (session planner state) — avoid `TASKS.md` vs `tasks.md` clash on macOS
