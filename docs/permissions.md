# Permission Model

Permissions are declared in the widget manifest and reviewed by the user on install.

## Permission Types

| Type | Field | Example |
|------|-------|---------|
| network | `permissions.network.allow` | `gitlab.com` |
| command | `permissions.commands.allow` | `docker` |
| filesystem | `permissions.filesystem.read/write` | `~/.kube/config` |

## Lifecycle

1. Widget is loaded from a local folder
2. Manifest is parsed and validated
3. Permissions are extracted as `WidgetPermission` records with `approved = false`
4. User is shown the Permission Review screen
5. On approval, all permissions are stored with `approved = true` in `widget_permissions` table
6. Before each runtime call, the evaluator queries `widget_permissions` to check approval

## Enforcement

- `run_command`: checks `permission_type = 'command'` for the exact executable
- `http_fetch`: extracts the URL host and checks `permission_type = 'network'`
- WASM host capabilities also go through the evaluator

## Denied Behavior

If a widget attempts an undeclared capability, the command returns a `PermissionDenied` error
which is displayed in the widget's error state.
