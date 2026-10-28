# Sessions — Local IDP (developer-cockpit-v2)

> Session-planner state. Named `SESSIONS.md` (not `TASKS.md`) to avoid a case clash
> with `tasks.md` on macOS — see `docs/restart-plan.md` §7.
> Source of the backlog: `docs/restart-plan.md` §5 (slices). JSON below is canonical.

## Current session
None. Last closed: **S-005** — Slice 3 file executor + views (T-004). Next up: T-005.

## Backlog (milestone: slices 0–5 = "works for the IDP story")
| ID | Slice | Title | Size | Status |
|---|---|---|---|---|
| T-001 | 0 | Scaffold + typed IPC + strict CSP + CI | S | done |
| T-002 | 1 | Manifest crate + `cockpit lint` | M | done |
| T-003a | 2 | Engine: clone/sync via git, changelog, consent, snapshot | M | done |
| T-003b | 2 | UI: Tauri commands, onboarding, consent screen, Sync now | M | done |
| T-004 | 3 | File executor + table/list/card views | M | done |
| T-015 | — | Lint: refuse catalog files that symlink out of the repo | S | pending |
| T-016 | — | Decide: block widget runs while a sync is pending review | S | pending |
| T-005 | 4 | Platform dashboards (read-only) | M | pending |
| T-006 | 5 | User inputs + secrets | M | pending |
| T-007 | 6 | HTTP executor | S | pending |
| T-008 | 7 | Command executor | S | pending |
| T-009 | 8 | User dashboards + Customize fork | M | pending |
| T-010 | 9 | Capability diff on re-sync | S | pending |
| T-011 | 10 | Layout drag/resize | M | pending |
| T-012 | 11 | Command palette | S | pending |
| T-013 | 12 | Pre-configured build | S | pending |
| T-014 | 13 | Environments + team scoping | M | pending |

```json
{
  "project": "developer-cockpit-v2",
  "updated": "2026-10-28",
  "current_session": null,
  "backlog": [
    {"id": "T-001", "title": "Scaffold: Tauri 2 + Solid + Vite + tauri-specta, strict CSP, CI, ping command", "description": "Done when app window shows 'pong' from Rust via generated bindings and CI is green. Also: git init, lean CLAUDE.md commands section.", "size": "S", "priority": 1, "status": "done", "tags": ["slice-0", "scaffold"]},
    {"id": "T-002", "title": "Manifest crate + cockpit lint", "description": "Platform/Widget/Dashboard kinds, structured validation errors, permission derivation, examples/platform-repo, snapshot tests for broken fixtures", "size": "M", "priority": 2, "status": "done", "tags": ["slice-1", "manifest", "cli"]},
    {"id": "T-003a", "title": "Engine: source clone/sync, changelog, consent, snapshot", "description": "crates/cockpit-engine: rusqlite_migration schema, system git clone/fetch/reset, plan -> consent -> apply/discard held in the engine, tests against temp git repos", "size": "M", "priority": 3, "status": "done", "tags": ["slice-2", "git", "consent", "engine"]},
    {"id": "T-003b", "title": "UI: source onboarding + sync screens", "description": "Tauri commands via tauri-specta, first-run screen (URL/path + branch, git check), capability consent, widget list, Sync now + changelog. Demo docs/demos/02-sync.md", "size": "M", "priority": 4, "status": "done", "tags": ["slice-2", "ui", "consent"]},
    {"id": "T-004", "title": "File executor + table/list/card views", "description": "run_instance, repo-confined paths, glob, _source, 500-file cap, traversal test", "size": "M", "priority": 4, "status": "done", "tags": ["slice-3", "engine", "views"]},
    {"id": "T-005", "title": "Platform dashboards", "description": "Render kind: Dashboard read-only, switcher, shared empty/loading/error/needs-setup states", "size": "M", "priority": 5, "status": "pending", "tags": ["slice-4", "ui"]},
    {"id": "T-006", "title": "User inputs + secrets", "description": "Platform spec.inputs form, keyring secrets screen, setup-required widget state", "size": "M", "priority": 6, "status": "pending", "tags": ["slice-5", "secrets"]},
    {"id": "T-007", "title": "HTTP executor", "description": "Declared secrets only, host enforcement", "size": "S", "priority": 7, "status": "pending", "tags": ["slice-6", "engine"]},
    {"id": "T-008", "title": "Command executor", "description": "Executable + args, no shell, timeout, stderr/exit code", "size": "S", "priority": 8, "status": "pending", "tags": ["slice-7", "engine"]},
    {"id": "T-009", "title": "User dashboards", "description": "Create/rename/delete via in-app dialogs, Customize fork, add from Catalog, auto-placement", "size": "M", "priority": 9, "status": "pending", "tags": ["slice-8", "ui"]},
    {"id": "T-010", "title": "Capability diff on re-sync", "description": null, "size": "S", "priority": 10, "status": "pending", "tags": ["slice-9", "consent"]},
    {"id": "T-011", "title": "Layout editing: drag/resize", "description": "Evaluate gridstack", "size": "M", "priority": 11, "status": "pending", "tags": ["slice-10", "ui"]},
    {"id": "T-012", "title": "Command palette", "description": null, "size": "S", "priority": 12, "status": "pending", "tags": ["slice-11", "ui"]},
    {"id": "T-013", "title": "Pre-configured build", "description": "Embedded platform.json, app name/icon override", "size": "S", "priority": 13, "status": "pending", "tags": ["slice-12", "distribution"]},
    {"id": "T-014", "title": "Environments + team scoping", "description": "Env switcher, {{ env.X }}, team picker from catalog, row filter (restart-plan §6.1)", "size": "M", "priority": 14, "status": "pending", "tags": ["slice-13"]},
    {"id": "T-015", "title": "Lint: refuse catalog files that symlink out of the repo", "description": "cockpit lint currently follows catalog symlinks outside the repo and reads them (found in S-005). The run-time file executor already denies them; lint should report them so the sync is blocked up front", "size": "S", "priority": 15, "status": "pending", "tags": ["lint", "security"]},
    {"id": "T-016", "title": "Decide: block widget runs while a sync is pending review", "description": "While a plan awaits review the clone is at the planned commit, so run_widget reads newer data files with the old manifest and approved paths (path approval still enforced). Deferred by user in S-005", "size": "S", "priority": 16, "status": "pending", "tags": ["engine", "consent"]}
  ],
  "decisions": [
    {"id": "D-001", "date": "2026-10-27", "decision": "Shell out to system git; no credential handling in app", "rationale": "Inherits user's SSH agent / credential helpers (restart-plan D1)", "supersedes": null},
    {"id": "D-002", "date": "2026-10-27", "decision": "One platform repo in v1; stages/teams modelled inside it", "rationale": "Keep source_id column so multi-repo is a migration later (D2, §6.1)", "supersedes": null},
    {"id": "D-003", "date": "2026-10-27", "decision": "No personal widgets in v1", "rationale": "Validate platform-only model first (D3)", "supersedes": null},
    {"id": "D-004", "date": "2026-10-27", "decision": "Sync on launch + on demand only, non-blocking", "rationale": "No background timer (D4)", "supersedes": null},
    {"id": "D-005", "date": "2026-10-27", "decision": "Catalog = k8s-style YAML, dot-path columns, no per-kind schema", "rationale": "D5", "supersedes": null},
    {"id": "D-006", "date": "2026-10-27", "decision": "macOS + Linux only", "rationale": "Keychain + Secret Service; CI matrix macos-latest/ubuntu-latest (D6)", "supersedes": null},
    {"id": "D-007", "date": "2026-10-27", "decision": "Session state lives in SESSIONS.md, not TASKS.md", "rationale": "Avoid TASKS.md vs tasks.md case clash on macOS (restart-plan §7)", "supersedes": null},
    {"id": "D-008", "date": "2026-10-27", "decision": "Pin tauri-specta and specta at =2.0.0-rc.26; bindings exported by a cargo test, CI fails on git diff of app/src/bindings.ts", "rationale": "RC crates break between versions; drift check replaces manual verification (restart-plan problem #1, #14)", "supersedes": null},
    {"id": "D-009", "date": "2026-10-27", "decision": "Separate devCsp allows 'unsafe-inline' styles and the Vite websocket; production csp is 'self' + ipc only", "rationale": "Vite dev server injects style tags and uses HMR websocket", "supersedes": null},
    {"id": "D-010", "date": "2026-10-27", "decision": "Anything that defines a permission is literal: file paths, executables, URL scheme+host. Templates only in URL path/query, headers, command args. secrets.* only in http url/headers. env.* rejected until slice 13", "rationale": "Capabilities must be derivable statically from the manifest for consent", "supersedes": null},
    {"id": "D-011", "date": "2026-10-27", "decision": "Capability hash = sha256 over canonical JSON of {widget name -> hosts/executables/paths}", "rationale": "Keyed by widget so slice 9 can diff per widget; BTree ordering keeps it stable", "supersedes": null},
    {"id": "D-012", "date": "2026-10-27", "decision": "File source paths resolve from the widget dir; input `from` and discovery dirs resolve from the repo root; all confined to the repo root (lexical + canonical/symlink check). Dashboard grid is 12 columns", "rationale": "Matches restart-plan examples; confine() is public for the engine to reuse at execution time", "supersedes": null},
    {"id": "D-013", "date": "2026-10-27", "decision": "YAML via serde_yaml_ng + serde_path_to_error; envelope (apiVersion/kind) checked on an untyped Value before the typed parse", "rationale": "serde_yaml is unmaintained; peeking the kind gives 'expected Widget, found Dashboard' instead of field noise", "supersedes": null},
    {"id": "D-014", "date": "2026-10-27", "decision": "The sync plan is held inside the Engine between plan_sync and apply_sync(consent); the frontend only reviews it. apply refuses on lint errors (whole sync, no partial apply), on a moved clone (StalePlan), and on capability drift without consent. discard_sync resets the clone to the last applied commit", "rationale": "Frontend cannot forge what gets stored; files on disk always match the stored snapshot, which the file executor (slice 3) relies on", "supersedes": null},
    {"id": "D-015", "date": "2026-10-27", "decision": "git runs with GIT_TERMINAL_PROMPT=0; clone uses --single-branch --branch=<b> -- <url>; branch names starting with '-' are rejected", "rationale": "No hanging credential prompts inside the app; no option injection via url/branch", "supersedes": null},
    {"id": "D-016", "date": "2026-10-27", "decision": "specta::Type derives sit behind an optional `specta` feature on cockpit-manifest and cockpit-engine; only src-tauri enables it. The engine's Source is exported to TS as RepoSource (serde container rename)", "rationale": "Bindings use the real domain types without pulling specta into the CLI; name clash with the widget Source", "supersedes": null},
    {"id": "D-017", "date": "2026-10-27", "decision": "git runs with stdin closed in its own process group, killed after 120s; GIT_ALLOW_PROTOCOL=file:git:http:https:ssh; clone/fetch set GIT_SSH_COMMAND='ssh -o BatchMode=yes' unless GIT_SSH_COMMAND, GIT_SSH or core.sshCommand is set", "rationale": "Closes the T-003a hang gap (host-key/passphrase prompts); repo URLs are user-typed in onboarding, so ext:: must be refused", "supersedes": "D-015"},
    {"id": "D-018", "date": "2026-10-27", "decision": "tauri-specta export uses disable_serde_phases(); i64/usize fields are exported as u32 via specta(type)", "rationale": "Phased mode left Manifest<S> unresolved for type aliases; unified types are looser (field?: T | null) but a safe superset. Specta forbids bigint-style ints", "supersedes": null},
    {"id": "D-019", "date": "2026-10-27", "decision": "Home syncs on launch and on Sync now. A plan with no lint errors and needs_consent=false is applied without asking (apply_sync(false)) and shown as a changelog; an up-to-date plan is discarded; a first sync, changed capabilities or lint errors go to the review. The widget list stays mounted throughout", "rationale": "restart-plan §3 (silent apply + changelog) and D-004 (non-blocking launch sync). The UI mirrors SyncPlan::is_up_to_date; drift can only produce an empty changelog, never an unconsented apply, because the engine still refuses apply without consent", "supersedes": null},
    {"id": "D-020", "date": "2026-10-27", "decision": "Consent shows the full capability set grouped by widget; Discard lives in the review", "rationale": "Per-widget diff is slice 9 (T-010); a review blocked by lint errors needs a way out", "supersedes": null},
    {"id": "D-021", "date": "2026-10-28", "decision": "Widgets run by name (run_widget(name)) from the applied snapshot until instances exist (T-005/T-009); the frontend sends only the name. Only file sources run; command/http return notRunnable", "rationale": "No instances table yet; the name keys the stored manifest, so nothing executable crosses IPC", "supersedes": null},
    {"id": "D-022", "date": "2026-10-28", "decision": "File source: path must be in the approved capability paths on every run; glob via globset with literal_separator (only ** descends); symlinked dirs not walked, each matched file canonicalised and must stay under the approved path; YAML/JSON via serde_yaml_ng; top-level lists expand to rows; every row gets _source; 500-file cap with truncated flag", "rationale": "Enforce consent at execution time (CLAUDE.md); lint follows symlinks, so the run-time check is the backstop", "supersedes": null},
    {"id": "D-023", "date": "2026-10-28", "decision": "FileData.rows is exported to TS as a JsonValue map via specta(type) and a bindings-only JsonValue enum; new ErrorKinds unknownWidget, notRunnable, permissionDenied, dataError", "rationale": "specta refuses serde_json::Value (i64), same constraint as D-018; the UI branches on kind for Access denied / Could not read the data", "supersedes": null}
  ],
  "completed": [
    {"id": "T-001", "title": "Scaffold: Tauri 2 + Solid + Vite + tauri-specta, strict CSP, CI, ping command", "completed_date": "2026-10-27", "session_ref": "S-001", "notes": "pong verified in real app by user. CI workflow written but not yet run on GitHub (no remote). Icons are placeholders."},
    {"id": "T-002", "title": "Manifest crate + cockpit lint", "completed_date": "2026-10-27", "session_ref": "S-002", "notes": "crates/cockpit-manifest (Tauri-free) + crates/cockpit-cli (`cockpit lint`, text/json). 12 fixtures + insta snapshots, examples/platform-repo lints clean. No line numbers for semantic errors (path only). Manifest types not yet specta-exported."},
    {"id": "T-003a", "title": "Engine: source clone/sync, changelog, consent, snapshot", "completed_date": "2026-10-27", "session_ref": "S-003", "notes": "crates/cockpit-engine (Tauri-free), 9 tests against temp git repos. Known gaps: no git timeout (an SSH host-key prompt could still hang), blocking API (wrap in spawn_blocking in Tauri), engine types not specta-exported yet."},
    {"id": "T-003b", "title": "UI: source onboarding + sync screens", "completed_date": "2026-10-27", "session_ref": "S-004", "notes": "Commits 3fb144a (commands + git hardening), 823be31 (onboarding), f58ec66 (review/consent/widget list), d0749b8 (Sync now, changelog, README). 16 vitest + Rust tests green; demo docs/demos/02-sync.md and README §5 written, repo edits verified with cockpit lint, app boots cleanly; the click-through in the window was not observed by Claude and is still to be confirmed by the user. Gaps: no folder picker (needs dialog plugin), consent shows full set not diff (T-010), docs/demos/00-scaffold.md still mentions pong, README §5 duplicates 02-sync.md. Click-through confirmed by user in S-005."},
    {"id": "T-004", "title": "File executor + table/list/card views", "completed_date": "2026-10-28", "session_ref": "S-005", "notes": "Commits 3a623aa (engine reader + run_widget), efc6b41 (Tauri command), df3164d (views + demo). Demos 03 and 02 confirmed by user in the real app. Runs by widget name, not run_instance(id) (D-021). kv/raw/markdown views not rendered yet. Follow-ups T-015 (lint symlink gap), T-016 (runs during pending sync)."}
  ]
}
```
