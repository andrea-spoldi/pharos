//! Platform dashboards as the UI renders them: each placement with its widget and status.

use cockpit_manifest::{Metadata, Namespace, Placement, Source, ViewType, Widget, template_refs};
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
    /// A part this version cannot run or render yet, e.g. `command sources`.
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
    pub widget: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub manifest: Option<Widget>,
    pub status: PlacementStatus,
    /// For runnable file widgets: what is read, relative to the repo root.
    pub reads: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DashboardView {
    pub metadata: Metadata,
    pub placements: Vec<PlacementView>,
}

/// Precedence: missing, needs setup, not available, static, runnable.
pub(crate) fn placement_status(widget: Option<&Widget>, placement: &Placement) -> PlacementStatus {
    let Some(widget) = widget else {
        return PlacementStatus::Missing;
    };
    let (inputs, secrets) = setup_refs(widget, placement);
    if !inputs.is_empty() || !secrets.is_empty() {
        return PlacementStatus::NeedsSetup {
            inputs: inputs.into_iter().collect(),
            secrets: secrets.into_iter().collect(),
        };
    }
    let view = widget.spec.view.kind;
    let unavailable = match (&widget.spec.source, view) {
        (Some(Source::Command { .. }), _) => Some("command sources".to_string()),
        (Some(Source::Http { .. }), _) => Some("http sources".to_string()),
        (Some(Source::File { .. }), ViewType::Table | ViewType::List | ViewType::Card) => None,
        (Some(Source::File { .. }), ViewType::Markdown) => {
            Some("markdown from a data source".to_string())
        }
        (None, ViewType::Markdown) => return PlacementStatus::Static,
        (_, other) => Some(format!("the {} view", view_name(other))),
    };
    match unavailable {
        Some(feature) => PlacementStatus::NotAvailable { feature },
        None => PlacementStatus::Runnable,
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
            "apiVersion: cockpit.dev/v1alpha1\nkind: Widget\nmetadata: {{ name: w }}\nspec:\n{spec}"
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
            placement_status(None, &placement(&[])),
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
            placement_status(Some(&w), &p),
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
            placement_status(Some(&w), &p),
            PlacementStatus::NeedsSetup {
                inputs: vec!["team".into(), "zed".into()],
                secrets: vec![],
            }
        );
    }

    #[test]
    fn command_and_http_are_not_available() {
        let cmd =
            widget("  source: { type: command, executable: kubectl }\n  view: { type: kv }\n");
        let http = widget(
            "  source: { type: http, url: \"https://g.example/x\" }\n  view: { type: table }\n",
        );
        let p = placement(&[]);
        assert_eq!(
            placement_status(Some(&cmd), &p),
            not_available("command sources")
        );
        assert_eq!(
            placement_status(Some(&http), &p),
            not_available("http sources")
        );
    }

    #[test]
    fn views_without_renderer_are_not_available() {
        let kv = widget("  source: { type: file, path: data/ }\n  view: { type: kv }\n");
        let md = widget("  source: { type: file, path: data/ }\n  view: { type: markdown }\n");
        let p = placement(&[]);
        assert_eq!(
            placement_status(Some(&kv), &p),
            not_available("the kv view")
        );
        assert_eq!(
            placement_status(Some(&md), &p),
            not_available("markdown from a data source")
        );
    }

    #[test]
    fn static_markdown_is_static_and_file_table_runs() {
        let md = widget("  view: { type: markdown, content: \"# Hi\" }\n");
        let table = widget("  view: { type: table }\n");
        let p = placement(&[]);
        assert_eq!(placement_status(Some(&md), &p), PlacementStatus::Static);
        assert_eq!(
            placement_status(Some(&table), &p),
            not_available("the table view")
        );
        assert_eq!(
            placement_status(Some(&widget(FILE_TABLE)), &p),
            PlacementStatus::Runnable
        );
    }
}
