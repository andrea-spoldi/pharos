# Widget Manifest

Widgets are defined by a `widget.yaml` file in their folder.

## Minimal Example

```yaml
apiVersion: cockpit.dev/v1alpha1
kind: Widget
metadata:
  name: my-widget
  title: My Widget
  version: 0.1.0
spec:
  runtime: static
```

## Full Schema Reference

```yaml
apiVersion: cockpit.dev/v1alpha1
kind: Widget

metadata:
  name: string          # machine-readable identifier (kebab-case)
  title: string         # display name
  description: string   # optional description
  version: string       # semver

spec:
  # Runtime: static | command | http | wasm-experimental
  runtime: command
  refreshInterval: 30s  # optional: s | m | h

  inputs:
    myParam:
      type: string
      required: true
      default: "value"
      description: "Shown in config form"

  secrets:
    - name: MY_TOKEN    # referenced in headers/templates as {{ secrets.MY_TOKEN }}
      required: true

  permissions:
    network:
      allow:
        - api.example.com
    commands:
      allow:
        - docker
    filesystem:
      read:
        - ~/.kube/config
      write: []

  # For command runtime:
  command:
    executable: docker
    args: [ps, --format, json]

  # For http runtime:
  source:
    method: GET
    url: "https://api.example.com/data?id={{ inputs.myParam }}"
    headers:
      Authorization: "Bearer {{ secrets.MY_TOKEN }}"

  view:
    type: table   # table | card | list | raw
    columns:
      - field: id
        label: ID
      - field: name
        label: Name

  actions:
    - name: openLink
      label: Open
      type: open_url
      url: "https://example.com"
```

## Template Syntax

URLs and headers support `{{ inputs.KEY }}` and `{{ secrets.NAME }}` placeholders.
Secrets are resolved server-side — values never reach the frontend.
