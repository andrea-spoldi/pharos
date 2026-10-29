use crate::diagnostic::{Diagnostic, Sink};
use crate::kinds::{API_VERSION, Manifest, Platform};
use crate::paths::{confine, display_relative};
use serde::de::DeserializeOwned;
use serde_yaml_ng::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const PLATFORM_FILE: &str = "pharos.yaml";
pub const WIDGET_FILE: &str = "widget.yaml";
pub const CATALOG_DIR: &str = "catalog";

/// A parsed manifest and where it came from.
#[derive(Debug, Clone)]
pub struct Loaded<T> {
    /// Repo-relative, `/`-separated.
    pub file: String,
    /// Absolute directory containing the file; relative paths in the manifest resolve here.
    pub dir: PathBuf,
    pub manifest: T,
}

#[derive(Debug, Clone)]
pub struct Repo {
    /// Canonical repo root.
    pub root: PathBuf,
    /// `None` when `pharos.yaml` is missing or invalid; checks against it are then skipped.
    pub platform: Option<Loaded<Platform>>,
    pub widgets: Vec<Loaded<crate::Widget>>,
    pub dashboards: Vec<Loaded<crate::Dashboard>>,
    pub catalog_files: Vec<String>,
}

/// Discovers and parses every manifest in the repo. Files that fail to parse are reported
/// and left out; everything else is returned for validation.
pub fn load(root: &Path) -> (Option<Repo>, Vec<Diagnostic>) {
    let mut out = Vec::new();
    let root = match fs::canonicalize(root) {
        Ok(root) if root.is_dir() => root,
        _ => {
            let file = root.display().to_string();
            Sink {
                file: &file,
                out: &mut out,
            }
            .error("repo-not-found", "", "not a directory");
            return (None, out);
        }
    };

    let platform_path = root.join(PLATFORM_FILE);
    let platform: Option<Loaded<Platform>> = if platform_path.is_file() {
        load_manifest(&root, &platform_path, "Platform", &mut out)
    } else {
        Sink {
            file: PLATFORM_FILE,
            out: &mut out,
        }
        .error(
            "missing-platform",
            "",
            "missing: every platform repo needs one at its root (kind: Platform)",
        );
        None
    };

    let (widgets_dir, dashboards_dir) = match &platform {
        Some(p) => (
            p.manifest.spec.widgets.as_str(),
            p.manifest.spec.dashboards.as_str(),
        ),
        None => ("widgets/", "dashboards/"),
    };

    let mut widgets = Vec::new();
    if let Some(dir) = discovery_root(&root, widgets_dir, "spec.widgets", &mut out) {
        for sub in sorted_entries(&dir).into_iter().filter(|p| p.is_dir()) {
            let file = sub.join(WIDGET_FILE);
            if file.is_file() {
                widgets.extend(load_manifest(&root, &file, "Widget", &mut out));
            } else {
                let rel = display_relative(&root, &sub);
                Sink {
                    file: &rel,
                    out: &mut out,
                }
                .warn(
                    "missing-widget-manifest",
                    "",
                    format!("directory has no {WIDGET_FILE} and is ignored"),
                );
            }
        }
    }

    let mut dashboards = Vec::new();
    if let Some(dir) = discovery_root(&root, dashboards_dir, "spec.dashboards", &mut out) {
        for file in sorted_entries(&dir).into_iter().filter(|p| is_yaml(p)) {
            dashboards.extend(load_manifest(&root, &file, "Dashboard", &mut out));
        }
    }

    let mut catalog = Vec::new();
    walk_yaml(&root.join(CATALOG_DIR), &mut catalog);
    let catalog_files = catalog
        .iter()
        .map(|path| {
            let file = display_relative(&root, path);
            check_catalog_entry(
                path,
                &mut Sink {
                    file: &file,
                    out: &mut out,
                },
            );
            file
        })
        .collect();

    let repo = Repo {
        root,
        platform,
        widgets,
        dashboards,
        catalog_files,
    };
    (Some(repo), out)
}

fn load_manifest<S: DeserializeOwned>(
    root: &Path,
    path: &Path,
    kind: &str,
    out: &mut Vec<Diagnostic>,
) -> Option<Loaded<Manifest<S>>> {
    let file = display_relative(root, path);
    let mut sink = Sink { file: &file, out };
    let text = read(path, &mut sink)?;

    // Check the envelope on an untyped value first, so a file of the wrong kind gets
    // "expected kind Widget" rather than a confusing field error.
    let header: Value = parse_value(&text, &mut sink)?;
    if !check_header(&header, kind, &mut sink) {
        return None;
    }

    let de = serde_yaml_ng::Deserializer::from_str(&text);
    match serde_path_to_error::deserialize::<_, Manifest<S>>(de) {
        Ok(manifest) => Some(Loaded {
            dir: path.parent().unwrap_or(root).to_path_buf(),
            file,
            manifest,
        }),
        Err(e) => {
            let at = e.path().to_string();
            report(&mut sink, "schema", &at, e.into_inner());
            None
        }
    }
}

fn check_header(header: &Value, kind: &str, sink: &mut Sink) -> bool {
    if !header.is_mapping() {
        sink.error(
            "not-a-mapping",
            "",
            "expected a mapping with apiVersion, kind, metadata and spec",
        );
        return false;
    }
    let mut ok = true;
    match header.get("apiVersion").and_then(Value::as_str) {
        Some(API_VERSION) => {}
        Some(other) => {
            sink.error(
                "api-version",
                "apiVersion",
                format!("expected `{API_VERSION}`, found `{other}`"),
            );
            ok = false;
        }
        None => {
            sink.error("missing-field", "apiVersion", "missing or not a string");
            ok = false;
        }
    }
    match header.get("kind").and_then(Value::as_str) {
        Some(k) if k == kind => {}
        Some(other) => {
            sink.error(
                "wrong-kind",
                "kind",
                format!("expected `{kind}` here, found `{other}`"),
            );
            ok = false;
        }
        None => {
            sink.error("missing-field", "kind", "missing or not a string");
            ok = false;
        }
    }
    ok
}

/// Catalog entries are free-form but must be Kubernetes-style.
fn check_catalog_entry(path: &Path, sink: &mut Sink) {
    let Some(text) = read(path, sink) else { return };
    let Some(value) = parse_value(&text, sink) else {
        return;
    };
    if !value.is_mapping() {
        sink.error(
            "catalog-shape",
            "",
            "catalog entries must be a mapping with apiVersion, kind and metadata.name",
        );
        return;
    }
    for key in ["apiVersion", "kind"] {
        if value.get(key).and_then(Value::as_str).is_none() {
            sink.error("catalog-shape", key, "missing or not a string");
        }
    }
    let name = value
        .get("metadata")
        .and_then(|m| m.get("name"))
        .and_then(Value::as_str);
    if name.is_none() {
        sink.error("catalog-shape", "metadata.name", "missing or not a string");
    }
}

fn discovery_root(
    root: &Path,
    rel: &str,
    path: &str,
    out: &mut Vec<Diagnostic>,
) -> Option<PathBuf> {
    match confine(root, root, rel) {
        Ok(dir) => dir.is_dir().then_some(dir),
        Err(e) => {
            Sink {
                file: PLATFORM_FILE,
                out,
            }
            .error("path-escape", path, format!("`{rel}`: {e}"));
            None
        }
    }
}

fn read(path: &Path, sink: &mut Sink) -> Option<String> {
    fs::read_to_string(path)
        .map_err(|e| sink.error("read-failed", "", e.to_string()))
        .ok()
}

fn parse_value(text: &str, sink: &mut Sink) -> Option<Value> {
    serde_yaml_ng::from_str(text)
        .map_err(|e| report(sink, "invalid-yaml", "", e))
        .ok()
}

fn report(sink: &mut Sink, code: &'static str, path: &str, e: serde_yaml_ng::Error) {
    let line = e.location().map(|l| l.line());
    let mut message = e.to_string();
    // The line and path are reported separately.
    if let Some(i) = message.find(" at line ") {
        message.truncate(i);
    }
    if let Some((head, rest)) = message.split_once(": ")
        && !head.is_empty()
        && head
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-[]".contains(c))
    {
        message = rest.to_owned();
    }
    sink.error_at_line(code, path, line, message);
}

fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    entries
}

/// Recursively collects YAML files. Symlinked directories are not followed.
fn walk_yaml(dir: &Path, out: &mut Vec<PathBuf>) {
    for path in sorted_entries(dir) {
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            walk_yaml(&path, out);
        } else if is_yaml(&path) {
            out.push(path);
        }
    }
}

fn is_yaml(path: &Path) -> bool {
    path.is_file()
        && matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yaml" | "yml")
        )
}
