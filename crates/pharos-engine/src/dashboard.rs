//! Platform dashboards as the UI renders them: each placement with its widget and status.

use crate::settings::SetupState;
use pharos_manifest::{Metadata, Namespace, Placement, Source, ViewType, Widget, template_refs};
use serde::Serialize;
use std::collections::BTreeSet;

/// What a placement can show, decided here so the frontend never decides what runs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum PlacementStatus {
    /// The dashboard names a widget that is not in the snapshot.
    Missing,
    /// `user.*` inputs and secrets the placement or its source refers to.
    NeedsSetup {
        inputs: Vec<String>,
        secrets: Vec<String>,
    },
    /// A part this version cannot run or render yet, e.g. `the kv view`.
    NotAvailable {
        feature: String,
    },
    /// No source; the view renders the manifest's own content.
    Static,
    Runnable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PlacementView {
    /// Row id of a user dashboard's top-level placement (what `remove_widget` takes);
    /// `None` for platform placements and expanded children.
    pub id: Option<u32>,
    pub widget: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub manifest: Option<Widget>,
    pub status: PlacementStatus,
    /// For runnable file widgets: what is read, relative to the repo root.
    pub reads: Option<String>,
    /// For container widgets: the child placements, expanded recursively. Empty for leaves.
    pub children: Vec<PlacementView>,
}

/// Where a dashboard comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub enum Origin {
    Platform,
    User,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DashboardView {
    /// `p:<name>` for platform dashboards.
    pub id: String,
    pub origin: Origin,
    pub editable: bool,
    pub metadata: Metadata,
    pub placements: Vec<PlacementView>,
}

/// Precedence: missing, needs setup, not available, static, runnable. `needsSetup` lists
/// only what `setup` doesn't provide yet.
pub(crate) fn placement_status(
    widget: Option<&Widget>,
    placement: &Placement,
    setup: &SetupState,
) -> PlacementStatus {
    let Some(widget) = widget else {
        return PlacementStatus::Missing;
    };
    let (mut inputs, mut secrets) = setup_refs(widget, placement);
    inputs.retain(|k| !setup.inputs.contains(k));
    secrets.retain(|k| !setup.secrets.contains(k));
    if !inputs.is_empty() || !secrets.is_empty() {
        return PlacementStatus::NeedsSetup {
            inputs: inputs.into_iter().collect(),
            secrets: secrets.into_iter().collect(),
        };
    }
    // Containers have no view; their status comes from `container_status`.
    let Some(view) = widget.spec.view.as_ref().map(|v| v.kind) else {
        return PlacementStatus::NotAvailable {
            feature: "a widget without a view".to_string(),
        };
    };
    let unavailable = match (&widget.spec.source, view) {
        (Some(_), ViewType::Table | ViewType::List | ViewType::Card | ViewType::Raw) => None,
        (Some(_), ViewType::Markdown) => Some("markdown from a data source".to_string()),
        (None, ViewType::Markdown) => return PlacementStatus::Static,
        (_, other) => Some(format!("the {} view", view_name(other))),
    };
    match unavailable {
        Some(feature) => PlacementStatus::NotAvailable { feature },
        None => PlacementStatus::Runnable,
    }
}

/// Status of a container placement from its (already expanded) children.
///
/// `Runnable` if any descendant leaf is runnable (a child container is itself `Runnable`
/// exactly then); otherwise the child status that wins by `placement_status` precedence
/// (missing, needs setup, not available, static). `Missing` when the widget is absent
/// from the snapshot; a container with no children is `Static`.
pub(crate) fn container_status(widget_exists: bool, children: &[PlacementView]) -> PlacementStatus {
    if !widget_exists {
        return PlacementStatus::Missing;
    }
    if children
        .iter()
        .any(|c| c.status == PlacementStatus::Runnable)
    {
        return PlacementStatus::Runnable;
    }
    children
        .iter()
        .map(|c| &c.status)
        .min_by_key(|s| rank(s))
        .cloned()
        .unwrap_or(PlacementStatus::Static)
}

fn rank(status: &PlacementStatus) -> u8 {
    match status {
        PlacementStatus::Missing => 0,
        PlacementStatus::NeedsSetup { .. } => 1,
        PlacementStatus::NotAvailable { .. } => 2,
        PlacementStatus::Static => 3,
        PlacementStatus::Runnable => 4,
    }
}

/// `user.*` keys and `secrets.*` names in the placement inputs and the templated source
/// fields. File paths are literal (D-010); a template that does not parse adds nothing,
/// because lint already refuses it.
fn setup_refs(widget: &Widget, placement: &Placement) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut templates: Vec<&str> = placement.inputs.values().map(String::as_str).collect();
    match &widget.spec.source {
        Some(Source::Http { url, headers, .. }) => {
            templates.push(url);
            templates.extend(headers.values().map(String::as_str));
        }
        Some(Source::Command { args, .. }) => templates.extend(args.iter().map(String::as_str)),
        Some(Source::File { .. }) | None => {}
    }
    let (mut inputs, mut secrets) = (BTreeSet::new(), BTreeSet::new());
    for r in templates
        .into_iter()
        .filter_map(|t| template_refs(t).ok())
        .flatten()
    {
        match r.namespace {
            Namespace::User => inputs.insert(r.key),
            Namespace::Secrets => secrets.insert(r.key),
            Namespace::Inputs | Namespace::Env => false,
        };
    }
    (inputs, secrets)
}

fn view_name(view: ViewType) -> &'static str {
    match view {
        ViewType::Table => "table",
        ViewType::List => "list",
        ViewType::Card => "card",
        ViewType::Kv => "kv",
        ViewType::Raw => "raw",
        ViewType::Markdown => "markdown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn widget(spec: &str) -> Widget {
        serde_yaml_ng::from_str(&format!(
            "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: {{ name: w }}\nspec:\n{spec}"
        ))
        .unwrap()
    }

    fn placement(inputs: &[(&str, &str)]) -> Placement {
        Placement {
            widget: "w".into(),
            x: 0,
            y: 0,
            w: 4,
            h: 2,
            inputs: inputs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
        }
    }

    fn not_available(feature: &str) -> PlacementStatus {
        PlacementStatus::NotAvailable {
            feature: feature.into(),
        }
    }

    const FILE_TABLE: &str = "  source: { type: file, path: data/ }\n  view: { type: table }\n";

    #[test]
    fn missing_widget_is_missing() {
        assert_eq!(
            placement_status(None, &placement(&[]), &SetupState::default()),
            PlacementStatus::Missing
        );
    }

    #[test]
    fn needs_setup_from_placement_and_source() {
        let w = widget(concat!(
            "  source:\n",
            "    type: http\n",
            "    url: \"https://g.example/{{ inputs.project }}\"\n",
            "    headers: { PRIVATE-TOKEN: \"{{ secrets.GITLAB_TOKEN }}\" }\n",
            "  view: { type: table }\n",
        ));
        let p = placement(&[("project", "acme%2F{{ user.team }}")]);
        assert_eq!(
            placement_status(Some(&w), &p, &SetupState::default()),
            PlacementStatus::NeedsSetup {
                inputs: vec!["team".into()],
                secrets: vec!["GITLAB_TOKEN".into()],
            }
        );
    }

    #[test]
    fn dedupes_and_sorts_setup_refs() {
        let w = widget(FILE_TABLE);
        let p = placement(&[
            ("a", "{{ user.zed }}{{ user.team }}"),
            ("b", "{{ user.team }}"),
        ]);
        assert_eq!(
            placement_status(Some(&w), &p, &SetupState::default()),
            PlacementStatus::NeedsSetup {
                inputs: vec!["team".into(), "zed".into()],
                secrets: vec![],
            }
        );
    }

    #[test]
    fn command_and_http_run() {
        let cmd =
            widget("  source: { type: command, executable: kubectl }\n  view: { type: raw }\n");
        let http = widget(
            "  source: { type: http, url: \"https://g.example/x\" }\n  view: { type: table }\n",
        );
        let p = placement(&[]);
        assert_eq!(
            placement_status(Some(&cmd), &p, &SetupState::default()),
            PlacementStatus::Runnable
        );
        assert_eq!(
            placement_status(Some(&http), &p, &SetupState::default()),
            PlacementStatus::Runnable
        );
    }

    #[test]
    fn views_without_renderer_are_not_available() {
        let kv = widget("  source: { type: file, path: data/ }\n  view: { type: kv }\n");
        let md = widget("  source: { type: file, path: data/ }\n  view: { type: markdown }\n");
        let p = placement(&[]);
        assert_eq!(
            placement_status(Some(&kv), &p, &SetupState::default()),
            not_available("the kv view")
        );
        assert_eq!(
            placement_status(Some(&md), &p, &SetupState::default()),
            not_available("markdown from a data source")
        );
    }

    #[test]
    fn static_markdown_is_static_and_file_table_runs() {
        let md = widget("  view: { type: markdown, content: \"# Hi\" }\n");
        let table = widget("  view: { type: table }\n");
        let p = placement(&[]);
        assert_eq!(
            placement_status(Some(&md), &p, &SetupState::default()),
            PlacementStatus::Static
        );
        assert_eq!(
            placement_status(Some(&table), &p, &SetupState::default()),
            not_available("the table view")
        );
        assert_eq!(
            placement_status(Some(&widget(FILE_TABLE)), &p, &SetupState::default()),
            PlacementStatus::Runnable
        );
    }

    #[test]
    fn set_refs_are_not_needed() {
        let w = widget(concat!(
            "  source:\n",
            "    type: http\n",
            "    url: \"https://g.example/{{ inputs.project }}\"\n",
            "    headers: { PRIVATE-TOKEN: \"{{ secrets.GITLAB_TOKEN }}\" }\n",
            "  view: { type: table }\n",
        ));
        let p = placement(&[("project", "acme%2F{{ user.team }}")]);
        let all = SetupState {
            inputs: ["team".to_string()].into(),
            secrets: ["GITLAB_TOKEN".to_string()].into(),
        };
        assert_eq!(
            placement_status(Some(&w), &p, &all),
            PlacementStatus::Runnable
        );
        let team_only = SetupState {
            inputs: ["team".to_string()].into(),
            ..SetupState::default()
        };
        assert_eq!(
            placement_status(Some(&w), &p, &team_only),
            PlacementStatus::NeedsSetup {
                inputs: vec![],
                secrets: vec!["GITLAB_TOKEN".into()],
            }
        );
    }

    fn child(status: PlacementStatus) -> PlacementView {
        PlacementView {
            id: None,
            widget: "c".into(),
            x: 0,
            y: 0,
            w: 1,
            h: 1,
            manifest: None,
            status,
            reads: None,
            children: vec![],
        }
    }

    #[test]
    fn container_status_precedence() {
        let setup = PlacementStatus::NeedsSetup {
            inputs: vec!["team".into()],
            secrets: vec![],
        };
        let n_a = not_available("the kv view");
        let status = |exists, kids: Vec<PlacementStatus>| {
            let kids: Vec<_> = kids.into_iter().map(child).collect();
            container_status(exists, &kids)
        };
        let runnable = PlacementStatus::Runnable;
        assert_eq!(
            status(false, vec![runnable.clone()]),
            PlacementStatus::Missing
        );
        assert_eq!(
            status(true, vec![setup.clone(), runnable.clone()]),
            PlacementStatus::Runnable
        );
        assert_eq!(
            status(
                true,
                vec![PlacementStatus::Static, n_a.clone(), setup.clone()]
            ),
            setup
        );
        assert_eq!(
            status(true, vec![PlacementStatus::Static, n_a.clone()]),
            n_a
        );
        assert_eq!(
            status(true, vec![PlacementStatus::Missing, n_a]),
            PlacementStatus::Missing
        );
        assert_eq!(status(true, vec![]), PlacementStatus::Static);
    }
}
