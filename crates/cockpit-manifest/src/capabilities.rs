use crate::Widget;
use crate::kinds::Source;
use crate::load::{Loaded, Repo};
use crate::paths::{confine, display_relative};
use crate::template;
use crate::validate::executable_problem;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

/// What a widget may touch, derived from its `source` (plus `permissions` widening).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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
        let caps = for_widget(repo, widget);
        if !caps.is_empty() {
            set.widgets.insert(name.clone(), caps);
        }
    }
    set
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
pub(crate) enum OriginError {
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
pub(crate) fn http_origin(url: &str) -> Result<String, OriginError> {
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
}
