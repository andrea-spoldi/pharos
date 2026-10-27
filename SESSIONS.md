# Sessions — Local IDP (developer-cockpit-v2)

> Session-planner state. Named `SESSIONS.md` (not `TASKS.md`) to avoid a case clash
> with `tasks.md` on macOS — see `docs/restart-plan.md` §7.
> Source of the backlog: `docs/restart-plan.md` §5 (slices). JSON below is canonical.

## Current session
**S-002** — Slice 1 manifest crate + `cockpit lint` (T-002) — **done**. Next: T-003 source onboarding + sync (M)

## Backlog (milestone: slices 0–5 = "works for the IDP story")
| ID | Slice | Title | Size | Status |
|---|---|---|---|---|
| T-001 | 0 | Scaffold + typed IPC + strict CSP + CI | S | done |
| T-002 | 1 | Manifest crate + `cockpit lint` | M | done |
| T-003 | 2 | Source onboarding + sync | M | pending |
| T-004 | 3 | File executor + table/list/card views | M | pending |
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
  "updated": "2026-10-27",
  "current_session": {
    "id": "S-002",
    "goal": "Slice 1: cockpit-manifest crate (Platform/Widget/Dashboard kinds, structured validation, permission derivation) + cockpit lint CLI + examples/platform-repo + snapshot tests",
    "task_ref": "T-002",
    "started": "2026-10-27",
    "status": "done",
    "blocker": null
  },
  "backlog": [
    {"id": "T-001", "title": "Scaffold: Tauri 2 + Solid + Vite + tauri-specta, strict CSP, CI, ping command", "description": "Done when app window shows 'pong' from Rust via generated bindings and CI is green. Also: git init, lean CLAUDE.md commands section.", "size": "S", "priority": 1, "status": "done", "tags": ["slice-0", "scaffold"]},
    {"id": "T-002", "title": "Manifest crate + cockpit lint", "description": "Platform/Widget/Dashboard kinds, structured validation errors, permission derivation, examples/platform-repo, snapshot tests for broken fixtures", "size": "M", "priority": 2, "status": "done", "tags": ["slice-1", "manifest", "cli"]},
    {"id": "T-003", "title": "Source onboarding + sync", "description": "First-run screen, clone via system git, lint, capability consent, snapshot, Sync now + changelog", "size": "M", "priority": 3, "status": "pending", "tags": ["slice-2", "git", "consent"]},
    {"id": "T-004", "title": "File executor + table/list/card views", "description": "run_instance, repo-confined paths, glob, _source, 500-file cap, traversal test", "size": "M", "priority": 4, "status": "pending", "tags": ["slice-3", "engine", "views"]},
    {"id": "T-005", "title": "Platform dashboards", "description": "Render kind: Dashboard read-only, switcher, shared empty/loading/error/needs-setup states", "size": "M", "priority": 5, "status": "pending", "tags": ["slice-4", "ui"]},
    {"id": "T-006", "title": "User inputs + secrets", "description": "Platform spec.inputs form, keyring secrets screen, setup-required widget state", "size": "M", "priority": 6, "status": "pending", "tags": ["slice-5", "secrets"]},
    {"id": "T-007", "title": "HTTP executor", "description": "Declared secrets only, host enforcement", "size": "S", "priority": 7, "status": "pending", "tags": ["slice-6", "engine"]},
    {"id": "T-008", "title": "Command executor", "description": "Executable + args, no shell, timeout, stderr/exit code", "size": "S", "priority": 8, "status": "pending", "tags": ["slice-7", "engine"]},
    {"id": "T-009", "title": "User dashboards", "description": "Create/rename/delete via in-app dialogs, Customize fork, add from Catalog, auto-placement", "size": "M", "priority": 9, "status": "pending", "tags": ["slice-8", "ui"]},
    {"id": "T-010", "title": "Capability diff on re-sync", "description": null, "size": "S", "priority": 10, "status": "pending", "tags": ["slice-9", "consent"]},
    {"id": "T-011", "title": "Layout editing: drag/resize", "description": "Evaluate gridstack", "size": "M", "priority": 11, "status": "pending", "tags": ["slice-10", "ui"]},
    {"id": "T-012", "title": "Command palette", "description": null, "size": "S", "priority": 12, "status": "pending", "tags": ["slice-11", "ui"]},
    {"id": "T-013", "title": "Pre-configured build", "description": "Embedded platform.json, app name/icon override", "size": "S", "priority": 13, "status": "pending", "tags": ["slice-12", "distribution"]},
    {"id": "T-014", "title": "Environments + team scoping", "description": "Env switcher, {{ env.X }}, team picker from catalog, row filter (restart-plan §6.1)", "size": "M", "priority": 14, "status": "pending", "tags": ["slice-13"]}
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
    {"id": "D-013", "date": "2026-10-27", "decision": "YAML via serde_yaml_ng + serde_path_to_error; envelope (apiVersion/kind) checked on an untyped Value before the typed parse", "rationale": "serde_yaml is unmaintained; peeking the kind gives 'expected Widget, found Dashboard' instead of field noise", "supersedes": null}
  ],
  "completed": [
    {"id": "T-001", "title": "Scaffold: Tauri 2 + Solid + Vite + tauri-specta, strict CSP, CI, ping command", "completed_date": "2026-10-27", "session_ref": "S-001", "notes": "pong verified in real app by user. CI workflow written but not yet run on GitHub (no remote). Icons are placeholders."},
    {"id": "T-002", "title": "Manifest crate + cockpit lint", "completed_date": "2026-10-27", "session_ref": "S-002", "notes": "crates/cockpit-manifest (Tauri-free) + crates/cockpit-cli (`cockpit lint`, text/json). 12 fixtures + insta snapshots, examples/platform-repo lints clean. No line numbers for semantic errors (path only). Manifest types not yet specta-exported."}
  ]
}
```
