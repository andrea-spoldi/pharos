use pharos_manifest::{CapabilitySet, Diagnostic, Severity};
use serde::Serialize;
use std::collections::BTreeMap;

/// Names added, removed or changed between the stored snapshot and the repo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Changes {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// What a sync would do. Held by the engine between `plan_sync` and `apply_sync`,
/// so the frontend reviews it but can never supply or alter it.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SyncPlan {
    pub from_commit: Option<String>,
    pub to_commit: String,
    pub platform_changed: bool,
    pub widgets: Changes,
    pub dashboards: Changes,
    pub diagnostics: Vec<Diagnostic>,
    pub capabilities: CapabilitySet,
    pub capability_hash: String,
    /// The capability set differs from what the user last approved.
    pub needs_consent: bool,
}

impl SyncPlan {
    pub fn errors(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn is_up_to_date(&self) -> bool {
        self.from_commit.as_deref() == Some(self.to_commit.as_str())
            && !self.platform_changed
            && self.widgets.is_empty()
            && self.dashboards.is_empty()
            && !self.needs_consent
    }
}

/// Stored manifests: name → JSON.
#[derive(Debug, Default)]
pub(crate) struct Snapshot {
    pub platform: Option<String>,
    pub widgets: BTreeMap<String, String>,
    pub dashboards: BTreeMap<String, String>,
}

/// A plan plus the manifests it would store: name → (repo-relative file, JSON).
#[derive(Debug)]
pub(crate) struct Pending {
    pub plan: SyncPlan,
    pub platform: Option<String>,
    pub widgets: BTreeMap<String, (String, String)>,
    pub dashboards: BTreeMap<String, (String, String)>,
}

pub(crate) fn diff(
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, (String, String)>,
) -> Changes {
    let mut changes = Changes::default();
    for (name, (_, manifest)) in new {
        match old.get(name) {
            None => changes.added.push(name.clone()),
            Some(prev) if prev != manifest => changes.changed.push(name.clone()),
            Some(_) => {}
        }
    }
    changes.removed = old
        .keys()
        .filter(|n| !new.contains_key(*n))
        .cloned()
        .collect();
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffs_by_name_and_content() {
        let old = BTreeMap::from([
            ("a".into(), "1".into()),
            ("b".into(), "1".into()),
            ("c".into(), "1".into()),
        ]);
        let new = BTreeMap::from([
            ("a".into(), ("f".into(), "1".into())),
            ("b".into(), ("f".into(), "2".into())),
            ("d".into(), ("f".into(), "1".into())),
        ]);
        assert_eq!(
            diff(&old, &new),
            Changes {
                added: vec!["d".into()],
                removed: vec!["c".into()],
                changed: vec!["b".into()],
            }
        );
    }
}
