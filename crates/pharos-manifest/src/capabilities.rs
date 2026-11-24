use crate::Widget;
use crate::kinds::Source;
use crate::load::{Loaded, Repo};
use crate::paths::{confine, display_relative};
use crate::template;
use crate::validate::executable_problem;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

/// What a widget may touch, derived from its `source` (plus `permissions` widening).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Capabilities {
    /// Origins, e.g. `https://gitlab.com`.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub hosts: BTreeSet<String>,
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub executables: BTreeSet<String>,
    /// Repo-relative paths.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub paths: BTreeSet<String>,
}

impl Capabilities {
    pub fn is_empty(&self) -> bool {
        self.hosts.is_empty() && self.executables.is_empty() && self.paths.is_empty()
    }
}

/// Capabilities of every widget in a repo, keyed by widget name. This is what users consent to.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CapabilitySet {
    pub widgets: BTreeMap<String, Capabilities>,
}

impl CapabilitySet {
    /// Stable digest of the set; consent is stored against it.
    pub fn hash(&self) -> String {
        let json = serde_json::to_vec(self).expect("capability set serialises");
        format!("sha256:{:x}", Sha256::digest(json))
    }
}

pub(crate) fn derive(repo: &Repo) -> CapabilitySet {
    let mut set = CapabilitySet::default();
    for widget in &repo.widgets {
        let name = &widget.manifest.metadata.name;
        // Duplicates are reported by validation; the first definition wins.
        if set.widgets.contains_key(name) {
            continue;
        }
        let caps = if widget.manifest.spec.widgets.is_empty() {
            for_widget(repo, widget)
        } else {
            for_container(repo, widget)
        };
        if !caps.is_empty() {
            set.widgets.insert(name.clone(), caps);
        }
    }
    set
}

/// A container may do whatever any transitive leaf descendant can, so consent covers the union.
/// Children resolve by name, first definition winning (as in `derive`); unknown children are
/// lint errors and contribute nothing here.
fn for_container(repo: &Repo, root: &Loaded<Widget>) -> Capabilities {
    let mut by_name: BTreeMap<&str, &Loaded<Widget>> = BTreeMap::new();
    for w in &repo.widgets {
        by_name
            .entry(w.manifest.metadata.name.as_str())
            .or_insert(w);
    }
    let mut caps = Capabilities::default();
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(widget) = stack.pop() {
        if !visited.insert(widget.manifest.metadata.name.as_str()) {
            continue;
        }
        let spec = &widget.manifest.spec;
        let own = for_widget(repo, widget);
        caps.hosts.extend(own.hosts);
        caps.executables.extend(own.executables);
        caps.paths.extend(own.paths);
        for placement in &spec.widgets {
            if let Some(child) = by_name.get(placement.widget.as_str()) {
                stack.push(child);
            }
        }
    }
    caps
}

fn for_widget(repo: &Repo, widget: &Loaded<Widget>) -> Capabilities {
    let spec = &widget.manifest.spec;
    let mut caps = Capabilities::default();
    match &spec.source {
        Some(Source::File { path, .. }) if !path.contains("{{") => {
            if let Ok(p) = confine(&repo.root, &widget.dir, path) {
                caps.paths.insert(display_relative(&repo.root, &p));
            }
        }
        Some(Source::Command { executable, .. }) if executable_problem(executable).is_none() => {
            caps.executables.insert(executable.clone());
        }
        Some(Source::Http { url, .. }) => {
            if let Ok(origin) = http_origin(url) {
                caps.hosts.insert(origin);
            }
        }
        _ => {}
    }
    caps.hosts.extend(
        spec.permissions
            .hosts
            .iter()
            .filter_map(|h| bare_origin(h).ok()),
    );
    caps
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("scheme and host must be literal, not templated")]
    TemplatedHost,
    #[error("not a valid URL: {0}")]
    Invalid(String),
    #[error("scheme must be http or https")]
    Scheme,
    #[error("expected a bare origin like `https://gitlab.acme`")]
    NotBare,
}

/// Origin of a possibly templated URL. Only the path and query may be templated.
pub fn http_origin(url: &str) -> Result<String, OriginError> {
    let authority_end = match url.find("://") {
        Some(i) => {
            let rest = &url[i + 3..];
            i + 3 + rest.find(['/', '?', '#']).unwrap_or(rest.len())
        }
        None => url.len(),
    };
    if url[..authority_end].contains("{{") {
        return Err(OriginError::TemplatedHost);
    }
    let parsed = parse_http(&template::substitute(url, "x"))?;
    Ok(parsed.origin().ascii_serialization())
}

/// Origin for `permissions.hosts`: no path, query or templates.
pub(crate) fn bare_origin(s: &str) -> Result<String, OriginError> {
    if s.contains("{{") {
        return Err(OriginError::TemplatedHost);
    }
    let parsed = parse_http(s)?;
    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(OriginError::NotBare);
    }
    Ok(parsed.origin().ascii_serialization())
}

fn parse_http(s: &str) -> Result<Url, OriginError> {
    let parsed = Url::parse(s).map_err(|e| OriginError::Invalid(e.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(OriginError::Scheme);
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_of_templated_url() {
        assert_eq!(
            http_origin("https://gitlab.com/api/v4/projects/{{ inputs.p }}/pipelines"),
            Ok("https://gitlab.com".into())
        );
        assert_eq!(
            http_origin("https://h.acme:8443?x={{ user.a }}"),
            Ok("https://h.acme:8443".into())
        );
        assert_eq!(
            http_origin("https://{{ user.team }}.acme/x"),
            Err(OriginError::TemplatedHost)
        );
        assert_eq!(http_origin("file:///etc/passwd"), Err(OriginError::Scheme));
    }

    #[test]
    fn bare_origins_only() {
        assert_eq!(
            bare_origin("https://gitlab.acme"),
            Ok("https://gitlab.acme".into())
        );
        assert_eq!(
            bare_origin("https://gitlab.acme/api"),
            Err(OriginError::NotBare)
        );
        assert!(matches!(
            bare_origin("gitlab.acme"),
            Err(OriginError::Invalid(_))
        ));
    }

    #[test]
    fn hash_is_stable_and_content_sensitive() {
        let mut a = CapabilitySet::default();
        assert_eq!(a.hash(), CapabilitySet::default().hash());
        let before = a.hash();
        a.widgets
            .entry("w".into())
            .or_default()
            .hosts
            .insert("https://x".into());
        assert_ne!(a.hash(), before);
    }

    fn write_widget(root: &std::path::Path, name: &str, spec: &str) {
        let dir = root.join("widgets").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("widget.yaml"),
            format!(
                "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: {{ name: {name} }}\nspec:\n{spec}\n"
            ),
        )
        .unwrap();
    }

    fn container_repo(root: &std::path::Path) -> Repo {
        std::fs::write(
            root.join("pharos.yaml"),
            "apiVersion: pharos.io/v1alpha1\nkind: Platform\nmetadata: { name: t }\nspec: {}\n",
        )
        .unwrap();
        write_widget(
            root,
            "g",
            "  source: { type: http, url: \"https://g.example/api\" }\n  view: { type: raw }",
        );
        write_widget(
            root,
            "k",
            "  source: { type: command, executable: kubectl }\n  view: { type: raw }",
        );
        write_widget(
            root,
            "mid",
            "  widgets:\n    - { widget: k, x: 0, y: 0, w: 6, h: 2 }",
        );
        write_widget(
            root,
            "top",
            "  widgets:\n    - { widget: g, x: 0, y: 0, w: 6, h: 2 }\n    - { widget: mid, x: 0, y: 2, w: 6, h: 2 }\n    - { widget: g, x: 6, y: 0, w: 6, h: 2 }",
        );
        crate::load::load(root).0.expect("repo loads")
    }

    #[test]
    fn container_capability_is_union_of_children() {
        let dir = tempfile::tempdir().unwrap();
        let repo = container_repo(dir.path());
        let set = derive(&repo);
        let top = set.widgets.get("top").expect("container has an entry");
        assert!(top.hosts.contains("https://g.example"));
        assert!(top.executables.contains("kubectl"));
        // Nested container unions its own subtree under its own name.
        let mid = set.widgets.get("mid").expect("nested container entry");
        assert!(mid.executables.contains("kubectl"));
        assert!(mid.hosts.is_empty());
        // Leaves are unchanged.
        assert_eq!(set.widgets["g"].hosts.len(), 1);
        assert!(set.widgets["g"].executables.is_empty());
    }
}
