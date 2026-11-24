//! Platform dashboards from the stored snapshot, against a synced clone of examples/platform-repo.

mod common;

use common::Fixture;
use pharos_engine::{Engine, Origin, PlacementStatus};

#[test]
fn home_placements_have_status_and_reads() {
    let f = Fixture::new();
    let engine = f.synced_engine();

    let dashboards = engine.dashboards().unwrap();

    let home = &dashboards[0];
    assert_eq!(home.metadata.name, "home");
    let names: Vec<_> = home.placements.iter().map(|p| p.widget.as_str()).collect();
    assert_eq!(
        names,
        ["service-catalog", "k8s-context", "welcome", "my-pipelines"]
    );
    let statuses: Vec<_> = home.placements.iter().map(|p| p.status.clone()).collect();
    assert_eq!(
        statuses,
        [
            PlacementStatus::Runnable,
            PlacementStatus::Runnable,
            PlacementStatus::Static,
            PlacementStatus::NeedsSetup {
                inputs: vec!["team".into()],
                secrets: vec!["GITLAB_TOKEN".into()],
            },
        ]
    );
    let reads: Vec<_> = home.placements.iter().map(|p| p.reads.as_deref()).collect();
    assert_eq!(reads, [Some("catalog/services/*.yaml"), None, None, None]);
    assert!(home.placements.iter().all(|p| p.manifest.is_some()));
}

#[test]
fn home_comes_first_then_by_name() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    for name in ["zulu", "alpha"] {
        f.write(
            &format!("dashboards/{name}.yaml"),
            &format!(
                "apiVersion: pharos.io/v1alpha1\nkind: Dashboard\nmetadata: {{ name: {name} }}\n\
                 spec:\n  widgets:\n    - {{ widget: welcome, x: 0, y: 0, w: 4, h: 2 }}\n"
            ),
        );
    }
    f.commit("more dashboards");
    engine.plan_sync().unwrap();
    engine.apply_sync(false).unwrap();

    let names: Vec<_> = engine
        .dashboards()
        .unwrap()
        .into_iter()
        .map(|d| d.metadata.name)
        .collect();
    assert_eq!(names, ["home", "alpha", "zulu"]);
}

#[test]
fn nothing_before_first_apply() {
    let f = Fixture::new();
    let mut engine = Engine::open(&f.data).unwrap();
    engine.add_source(&f.url(), "main").unwrap();

    assert!(engine.dashboards().unwrap().is_empty());
    assert!(engine.platform().unwrap().is_none());
}

#[test]
fn platform_after_apply() {
    let f = Fixture::new();
    let engine = f.synced_engine();

    let platform = engine.platform().unwrap().unwrap();
    assert_eq!(
        platform.metadata.title.as_deref(),
        Some("ACME Developer Portal")
    );
}

fn leaf(name: &str) -> String {
    format!(
        "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: {{ name: {name} }}\n\
         spec:\n  source: {{ type: command, executable: kubectl }}\n  view: {{ type: raw }}\n"
    )
}

#[test]
fn platform_composite_expands_children() {
    let f = Fixture::new();
    f.write("widgets/leaf-a/widget.yaml", &leaf("leaf-a"));
    f.write("widgets/leaf-b/widget.yaml", &leaf("leaf-b"));
    f.write(
        "widgets/pair/widget.yaml",
        "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: { name: pair }\n\
         spec:\n  widgets:\n    - { widget: leaf-a, x: 0, y: 0, w: 6, h: 2 }\n    \
         - { widget: leaf-b, x: 6, y: 0, w: 6, h: 2 }\n",
    );
    let home = std::fs::read_to_string(f.origin.join("dashboards/home.yaml")).unwrap();
    f.write(
        "dashboards/home.yaml",
        &format!("{home}    - {{ widget: pair, x: 0, y: 8, w: 12, h: 2 }}\n"),
    );
    let engine = {
        f.commit("composite");
        f.synced_engine()
    };

    let dashboards = engine.dashboards().unwrap();

    let home = &dashboards[0];
    assert_eq!(home.id, "p:home");
    assert_eq!(home.origin, Origin::Platform);
    assert!(!home.editable);
    let pair = home.placements.iter().find(|p| p.widget == "pair").unwrap();
    assert_eq!(pair.status, PlacementStatus::Runnable);
    assert_eq!(pair.children.len(), 2);
    for (child, name) in pair.children.iter().zip(["leaf-a", "leaf-b"]) {
        assert_eq!(child.widget, name);
        assert_eq!(child.status, PlacementStatus::Runnable);
        assert!(child.children.is_empty());
        assert!(child.manifest.is_some());
    }
}
