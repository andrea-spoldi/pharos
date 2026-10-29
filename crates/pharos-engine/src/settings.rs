//! Platform inputs and secrets the developer provides in Settings.

use crate::db::Source;
use crate::run::read_catalog;
use pharos_manifest::{InputDef, InputType, Platform};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
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

/// What Settings shows. Before the first apply only `source` is filled in.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SettingsView {
    pub source: Option<Source>,
    pub platform: Option<PlatformRef>,
    pub inputs: Vec<InputView>,
    pub secrets: Vec<SecretView>,
    /// Stored rows the applied manifest no longer declares (any platform name).
    pub unused: Vec<StoredRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PlatformRef {
    pub name: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct InputView {
    pub key: String,
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub kind: InputType,
    pub required: bool,
    pub value: Option<String>,
    pub default: Option<String>,
    pub choices: Vec<String>,
    /// A stored enum value that is no longer one of the choices; it counts as missing.
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SecretView {
    pub name: String,
    pub description: Option<String>,
    pub set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct StoredRef {
    pub platform: String,
    pub kind: StoredKind,
    pub key: String,
}

/// `user.*` keys and secret names that count as provided (D-028).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetupState {
    pub inputs: BTreeSet<String>,
    pub secrets: BTreeSet<String>,
}

/// The applied platform's inputs and secrets, with stored values. `stored_inputs` and
/// `stored_secrets` hold every platform's rows, as (platform, key[, value]).
pub(crate) fn view(
    source: Option<Source>,
    platform: Option<&Platform>,
    root: Option<&Path>,
    stored_inputs: &[(String, String, String)],
    stored_secrets: &[(String, String)],
) -> SettingsView {
    let Some(platform) = platform else {
        return SettingsView {
            source,
            platform: None,
            inputs: Vec::new(),
            secrets: Vec::new(),
            unused: Vec::new(),
        };
    };
    let name = &platform.metadata.name;
    let values: BTreeMap<&str, &str> = stored_inputs
        .iter()
        .filter(|(p, _, _)| p == name)
        .map(|(_, k, v)| (k.as_str(), v.as_str()))
        .collect();
    let set_secrets: BTreeSet<&str> = stored_secrets
        .iter()
        .filter(|(p, _)| p == name)
        .map(|(_, n)| n.as_str())
        .collect();
    let inputs = platform
        .spec
        .inputs
        .iter()
        .map(|(key, def)| {
            let choices = match (def.kind, root) {
                (InputType::Enum, Some(root)) => choices(root, def),
                _ => Vec::new(),
            };
            let value = values.get(key.as_str()).map(|v| v.to_string());
            let stale =
                def.kind == InputType::Enum && value.as_ref().is_some_and(|v| !choices.contains(v));
            InputView {
                key: key.clone(),
                label: def.label.clone(),
                kind: def.kind,
                required: def.required,
                value,
                default: def.default.clone(),
                choices,
                stale,
            }
        })
        .collect();
    let secrets = platform
        .spec
        .secrets
        .iter()
        .map(|s| SecretView {
            name: s.name.clone(),
            description: s.description.clone(),
            set: set_secrets.contains(s.name.as_str()),
        })
        .collect();
    let declared_input = |p: &str, k: &str| p == name && platform.spec.inputs.contains_key(k);
    let declared_secret =
        |p: &str, k: &str| p == name && platform.spec.secrets.iter().any(|s| s.name == k);
    let mut unused: Vec<StoredRef> = stored_inputs
        .iter()
        .filter(|(p, k, _)| !declared_input(p, k))
        .map(|(p, k, _)| (p, StoredKind::Input, k))
        .chain(
            stored_secrets
                .iter()
                .filter(|(p, k)| !declared_secret(p, k))
                .map(|(p, k)| (p, StoredKind::Secret, k)),
        )
        .map(|(p, kind, k)| StoredRef {
            platform: p.clone(),
            kind,
            key: k.clone(),
        })
        .collect();
    unused.sort_by(|a, b| {
        (&a.platform, a.kind as u8, &a.key).cmp(&(&b.platform, b.kind as u8, &b.key))
    });
    SettingsView {
        source,
        platform: Some(PlatformRef {
            name: name.clone(),
            title: platform.metadata.title.clone(),
        }),
        inputs,
        secrets,
        unused,
    }
}

impl SettingsView {
    /// Inputs with a valid value or a default, and secrets that are set.
    pub(crate) fn setup_state(&self) -> SetupState {
        SetupState {
            inputs: self
                .inputs
                .iter()
                .filter(|i| (i.value.is_some() && !i.stale) || i.default.is_some())
                .map(|i| i.key.clone())
                .collect(),
            secrets: self
                .secrets
                .iter()
                .filter(|s| s.set)
                .map(|s| s.name.clone())
                .collect(),
        }
    }
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
