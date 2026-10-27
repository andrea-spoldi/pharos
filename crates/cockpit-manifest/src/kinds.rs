use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const API_VERSION: &str = "cockpit.dev/v1alpha1";

/// Kubernetes-style envelope shared by every kind.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Manifest<S> {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: S,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Metadata {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

pub type Platform = Manifest<PlatformSpec>;
pub type Widget = Manifest<WidgetSpec>;
pub type Dashboard = Manifest<DashboardSpec>;

// ── Platform ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PlatformSpec {
    /// Asked once per user; available to widgets as `{{ user.<key> }}`.
    #[serde(default)]
    pub inputs: BTreeMap<String, InputDef>,
    #[serde(default)]
    pub secrets: Vec<SecretDef>,
    /// Widget discovery root, relative to the repo root.
    #[serde(default = "default_widgets_dir")]
    pub widgets: String,
    /// Dashboard discovery root, relative to the repo root.
    #[serde(default = "default_dashboards_dir")]
    pub dashboards: String,
}

fn default_widgets_dir() -> String {
    "widgets/".into()
}

fn default_dashboards_dir() -> String {
    "dashboards/".into()
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct InputDef {
    #[serde(rename = "type")]
    pub kind: InputType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
    /// Static choices for `type: enum`.
    #[serde(default)]
    pub values: Vec<String>,
    /// Catalog directory feeding the choices for `type: enum` (with `field`).
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum InputType {
    String,
    Enum,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SecretDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

// ── Widget ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WidgetSpec {
    /// Absent for static (view-only) widgets.
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub refresh: Option<String>,
    /// Per-placement inputs; available as `{{ inputs.<key> }}`.
    #[serde(default)]
    pub inputs: BTreeMap<String, InputDef>,
    pub view: View,
    /// Only widens what is derived from `source`.
    #[serde(default)]
    pub permissions: Permissions,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Source {
    File {
        /// Relative to the widget directory; must stay inside the repo.
        path: String,
        #[serde(default)]
        glob: Option<String>,
    },
    Command {
        executable: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        timeout: Option<String>,
    },
    Http {
        url: String,
        #[serde(default)]
        method: HttpMethod,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct View {
    #[serde(rename = "type")]
    pub kind: ViewType,
    #[serde(default)]
    pub columns: Vec<Column>,
    /// Static markdown body for `type: markdown` widgets without a source.
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum ViewType {
    Table,
    List,
    Card,
    Kv,
    Raw,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Column {
    /// Dot path into each row, e.g. `metadata.name`.
    pub field: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Permissions {
    /// Extra origins, e.g. `https://gitlab.acme`.
    #[serde(default)]
    pub hosts: Vec<String>,
}

// ── Dashboard ───────────────────────────────────────────────────────────────

/// Dashboards lay widgets out on a grid this many columns wide.
pub const GRID_COLUMNS: u32 = 12;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DashboardSpec {
    #[serde(default)]
    pub widgets: Vec<Placement>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Placement {
    pub widget: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
}
