# Demo — Slice 1: Manifest crate + `pharos lint`

1. `cargo run -q -p pharos-cli -- lint examples/platform-repo`
   → `4 widgets, 1 dashboard, 4 catalog files: 0 errors, 0 warnings`, then a capability
   summary: `kubectl` (k8s-context), `https://gitlab.com` (my-pipelines), `catalog/services` (service-catalog).
2. Copy the repo and break it:
   `cp -r examples/platform-repo /tmp/broken && sed -i '' 's#../../catalog/services/#../../../../etc#' /tmp/broken/widgets/service-catalog/widget.yaml`
3. `cargo run -q -p pharos-cli -- lint /tmp/broken` → exit code 1 and
   `error: widgets/service-catalog/widget.yaml: spec.source.path: `../../../../etc`: path escapes the repository root [path-escape]`
4. `cargo run -q -p pharos-cli -- lint --format json /tmp/broken` → the same finding as JSON (`code`, `file`, `path`).
5. Every other class of mistake has a fixture in `crates/pharos-manifest/tests/fixtures/`;
   the expected output lives in `crates/pharos-manifest/tests/snapshots/`.
