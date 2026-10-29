//! Platform inputs and secrets the developer provides in Settings.

use crate::run::read_catalog;
use pharos_manifest::InputDef;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

/// Caps the files read for one input's choices, like the file executor.
const MAX_CHOICE_FILES: usize = crate::MAX_FILES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum StoredKind {
    Input,
    Secret,
}

/// Choices for an enum input: static `values`, else the `field` of each file under `from`
/// (repo-relative), unique and sorted. Unreadable catalogs give no choices.
pub(crate) fn choices(root: &Path, def: &InputDef) -> Vec<String> {
    if !def.values.is_empty() {
        return def.values.clone();
    }
    let (Some(from), Some(field)) = (&def.from, &def.field) else {
        return Vec::new();
    };
    let rows = read_catalog(root, from, MAX_CHOICE_FILES).unwrap_or_default();
    rows.iter()
        .filter_map(|row| dot_path(row, field)?.as_str().map(str::to_owned))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn dot_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(value, |v, key| v.get(key))
}
