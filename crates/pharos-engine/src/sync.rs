use pharos_manifest::{Capabilities, CapabilitySet, Diagnostic, Severity};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

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
    /// Per leaf widget: what it newly declares (neither approved nor already seen) and
    /// what it no longer declares.
    pub capability_changes: BTreeMap<String, CapabilityChange>,
    /// Some widget declares a capability that is neither approved nor already seen.
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

/// What one leaf widget gains and loses against what the user approved.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CapabilityChange {
    /// New to this widget: not approved and not already seen.
    pub added: Capabilities,
    /// Approved earlier, no longer declared.
    pub removed: Capabilities,
}

/// Per-leaf capability changes. Containers are never diffed; widgets absent from `new`
/// are omitted (they show as removed widgets).
pub(crate) fn capability_changes(
    approved: &CapabilitySet,
    seen: &CapabilitySet,
    new: &CapabilitySet,
    containers: &BTreeSet<String>,
) -> BTreeMap<String, CapabilityChange> {
    let none = Capabilities::default();
    let mut out = BTreeMap::new();
    for (name, now) in &new.widgets {
        if containers.contains(name) {
            continue;
        }
        let approved = approved.widgets.get(name).unwrap_or(&none);
        let seen = seen.widgets.get(name).unwrap_or(&none);
        let mut known = approved.clone();
        known.extend(seen.clone());
        let change = CapabilityChange {
            added: now.minus(&known),
            removed: approved.minus(now),
        };
        if !change.added.is_empty() || !change.removed.is_empty() {
            out.insert(name.clone(), change);
        }
    }
    out
}

/// The approved set after a sync: each widget keeps only what `new` still declares, plus
/// its listed additions when `approve`. Widgets absent from `new` drop out.
pub(crate) fn merge_approval(
    approved: &CapabilitySet,
    new: &CapabilitySet,
    changes: &BTreeMap<String, CapabilityChange>,
    approve: bool,
) -> CapabilitySet {
    let none = Capabilities::default();
    let mut out = CapabilitySet::default();
    for (name, now) in &new.widgets {
        let mut caps = approved.widgets.get(name).unwrap_or(&none).intersect(now);
        if approve && let Some(change) = changes.get(name) {
            // Intersected with `new`: a stale change list never approves what is not declared.
            caps.extend(change.added.intersect(now));
        }
        if !caps.is_empty() {
            out.widgets.insert(name.clone(), caps);
        }
    }
    out
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
    use pharos_manifest::Capabilities;
    use std::collections::BTreeSet;

    fn set(entries: &[(&str, &[&str], &[&str])]) -> CapabilitySet {
        let strs = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
        CapabilitySet {
            widgets: entries
                .iter()
                .map(|(n, hosts, paths)| {
                    (
                        n.to_string(),
                        Capabilities {
                            hosts: strs(hosts),
                            paths: strs(paths),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
        }
    }

    fn hosts(of: &[&str]) -> BTreeSet<String> {
        of.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn changes_list_only_unseen_additions() {
        let approved = set(&[("a", &["h1"], &[])]);
        let seen = set(&[("a", &["h1", "h2"], &[])]);
        let new = set(&[("a", &["h1", "h2", "h3"], &[])]);
        let changes = capability_changes(&approved, &seen, &new, &BTreeSet::new());
        assert_eq!(changes["a"].added.hosts, hosts(&["h3"]));
        assert!(changes["a"].removed.is_empty());
    }

    #[test]
    fn removals_are_reported_without_additions() {
        let approved = set(&[("a", &["h1", "h2"], &[])]);
        let new = set(&[("a", &["h1"], &[])]);
        let changes =
            capability_changes(&approved, &CapabilitySet::default(), &new, &BTreeSet::new());
        assert!(changes["a"].added.is_empty());
        assert_eq!(changes["a"].removed.hosts, hosts(&["h2"]));
    }

    #[test]
    fn new_widget_is_all_added_and_containers_are_skipped() {
        let new = set(&[("leaf", &[], &["p"]), ("box", &[], &["p"])]);
        let containers = BTreeSet::from(["box".to_string()]);
        let empty = CapabilitySet::default();
        let changes = capability_changes(&empty, &empty, &new, &containers);
        assert_eq!(changes.keys().collect::<Vec<_>>(), ["leaf"]);
        assert_eq!(changes["leaf"].added.paths, hosts(&["p"]));
    }

    #[test]
    fn merge_without_approval_shrinks_but_never_grows() {
        let approved = set(&[("a", &["h1", "h2"], &[])]);
        let new = set(&[("a", &["h1", "h3"], &[])]);
        let changes =
            capability_changes(&approved, &CapabilitySet::default(), &new, &BTreeSet::new());
        assert_eq!(
            merge_approval(&approved, &new, &changes, false),
            set(&[("a", &["h1"], &[])])
        );
    }

    #[test]
    fn merge_with_approval_adds_only_the_listed_additions() {
        let approved = set(&[("a", &["h1"], &[])]);
        let seen = set(&[("a", &["h1", "h2"], &[])]);
        let new = set(&[("a", &["h1", "h2", "h3"], &[])]);
        let changes = capability_changes(&approved, &seen, &new, &BTreeSet::new());
        assert_eq!(
            merge_approval(&approved, &new, &changes, true),
            set(&[("a", &["h1", "h3"], &[])])
        );
    }

    #[test]
    fn merge_never_approves_what_new_does_not_declare() {
        let approved = CapabilitySet::default();
        let stale = set(&[("a", &["h1", "h2"], &[])]);
        let new = set(&[("a", &["h1"], &[])]);
        let changes = capability_changes(
            &approved,
            &CapabilitySet::default(),
            &stale,
            &BTreeSet::new(),
        );
        assert_eq!(
            merge_approval(&approved, &new, &changes, true),
            set(&[("a", &["h1"], &[])])
        );
    }

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
