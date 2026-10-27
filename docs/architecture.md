# Architecture

Developer Cockpit is a Tauri 2 desktop application with a Rust backend and React frontend.

## Component Map

```
Developer Cockpit
├─ frontend/              React + Vite + TypeScript
│  ├─ src/app/           Pages and layout (React Router)
│  ├─ src/dashboard/     Dashboard state (Zustand) and components
│  ├─ src/widgets/       Widget library, config form, renderer, runtime renderers
│  ├─ src/settings/      Permission review and secret settings
│  ├─ src/components/    Command palette
│  └─ src/lib/           IPC wrapper (ipc.ts), type definitions, refresh scheduler
│
└─ src-tauri/             Rust Tauri backend
   └─ src/
      ├─ commands/        Tauri IPC command handlers
      │  ├─ dashboard.rs  Dashboard CRUD
      │  ├─ widget.rs     Widget install and list
      │  ├─ placement.rs  Widget placement on dashboard
      │  ├─ secrets.rs    Secret management (names only to frontend)
      │  └─ runtime.rs    run_command, http_fetch, wasm_execute
      ├─ db/              SQLite persistence (rusqlite)
      │  ├─ dashboard_repository.rs
      │  ├─ widget_repository.rs
      │  ├─ dashboard_widget_repository.rs
      │  └─ secrets_repository.rs
      ├─ permissions/     Permission model and evaluator
      ├─ runtimes/        Command, HTTP, and WASM runtimes
      ├─ secrets/         OS keyring adapter
      ├─ widgets/         Widget manifest loader
      └─ events/          Event type definitions

crates/
├─ cockpit-core/          Shared Rust types (Dashboard, Widget, etc.)
├─ cockpit-manifest/      YAML manifest parser, types, validator
└─ cockpit-widget-sdk/    Re-export crate for widget authors
```

## Data Flow

1. Frontend calls IPC via `@tauri-apps/api/core invoke()`
2. Rust command handler validates permissions and calls the relevant repository or runtime
3. Repository reads/writes SQLite via rusqlite
4. Runtime (command/http/wasm) executes the widget logic
5. Secrets are resolved server-side only — never returned to frontend in plaintext
6. Results serialized to JSON and returned to frontend

## Security Model

- Widgets declare permissions in their manifest
- Permissions are reviewed by user on install and stored as approved in SQLite
- Before any command or network call, the evaluator checks `widget_permissions` table
- Secret values never travel to the frontend — only names are listed via `list_secret_names`
- WASM modules are sandboxed by Wasmtime with explicit host capability imports
