//! File widget execution from the stored snapshot, against a synced clone of
//! examples/platform-repo. Placements are run by dashboard id and path.

mod common;

use common::Fixture;
use pharos_engine::{Engine, EngineError, FileError, HttpError, PlacementStatus};
use serde_json::json;

/// Sets (or, with `None`, removes) the `support` entry of the origin's pharos.yaml.
fn set_support(f: &Fixture, support: Option<&str>) {
    let path = f.origin.join("pharos.yaml");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut out = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("support:") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
        if line == "spec:" {
            if let Some(s) = support {
                out.push_str(&format!("  support: {s}\n"));
            }
        }
    }
    if out != text {
        f.write("pharos.yaml", &out);
        f.commit("support");
    }
}

#[test]
fn service_catalog_reads_catalog_files_with_their_source() {
    let f = Fixture::new();
    let engine = f.synced_engine();

    let data = engine.run_placement("p:home", &[0]).unwrap();

    assert!(!data.truncated);
    let names: Vec<_> = data.rows.iter().map(|r| &r["metadata"]["name"]).collect();
    assert_eq!(
        names,
        [&json!("notification-service"), &json!("payment-service")]
    );
    assert_eq!(
        data.rows[1]["_source"],
        "catalog/services/payment-service.yaml"
    );
    assert_eq!(data.rows[1]["spec"]["tier"], 1);
}

#[test]
fn runs_the_applied_snapshot_not_the_files_on_disk() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    // Widen the glob and plan that sync (the clone moves to it), but never apply it.
    let widget = "widgets/service-catalog/widget.yaml";
    let body = std::fs::read_to_string(f.origin.join(widget)).unwrap();
    f.write(widget, &body.replace("\"*.yaml\"", "\"*\""));
    f.write("catalog/services/notes.txt", "not yaml: [");
    f.commit("widen glob");
    engine.plan_sync().unwrap();

    let data = engine.run_placement("p:home", &[0]).unwrap();

    assert_eq!(data.rows.len(), 2);
}

#[test]
fn denies_files_symlinked_out_of_the_repo() {
    let f = Fixture::new();
    // Catalog-shaped, so `pharos lint` (which follows the link) passes and the sync applies.
    let outside = f.origin.parent().unwrap().join("outside.yaml");
    std::fs::write(
        &outside,
        "apiVersion: v1\nkind: Service\nmetadata: { name: secret }\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, f.origin.join("catalog/services/evil.yaml")).unwrap();
    f.commit("symlink out");
    let engine = f.synced_engine();

    let err = engine.run_placement("p:home", &[0]).unwrap_err();

    assert!(
        matches!(&err, EngineError::File(FileError::Denied { path, .. }) if path == "catalog/services/evil.yaml"),
        "{err:?}"
    );
}

#[test]
fn runs_only_runnable_placements() {
    let f = Fixture::new();
    let engine = f.synced_engine();

    assert!(matches!(
        engine.run_placement("p:home", &[9]),
        Err(EngineError::UnknownPlacement { dashboard, index: 9 }) if dashboard == "p:home"
    ));
    assert!(matches!(
        engine.run_placement("p:nope", &[0]),
        Err(EngineError::UnknownDashboard(_))
    ));
    assert!(matches!(
        engine.run_placement("p:home", &[2]),
        Err(EngineError::NoDataSource(name)) if name == "welcome"
    ));
    // My Pipelines: no team and no token yet.
    assert!(matches!(
        engine.run_placement("p:home", &[3]),
        Err(EngineError::NeedsSetup(name)) if name == "my-pipelines"
    ));
}

fn leaf(name: &str, source: &str) -> String {
    format!(
        "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: {{ name: {name} }}\n\
         spec:\n  source: {source}\n  view: {{ type: raw }}\n"
    )
}

/// A repo where `pair` composes an http leaf and a file-less static one.
fn composite_fixture() -> Fixture {
    let f = Fixture::new();
    f.write(
        "widgets/remote/widget.yaml",
        &leaf(
            "remote",
            "{ type: http, url: \"https://elsewhere.example/x\" }",
        ),
    );
    f.write(
        "widgets/pair/widget.yaml",
        "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: { name: pair }\n\
         spec:\n  widgets:\n    - { widget: remote, x: 0, y: 0, w: 6, h: 2 }\n    \
         - { widget: service-catalog, x: 6, y: 0, w: 6, h: 2 }\n",
    );
    f.commit("composite");
    f
}

#[test]
fn run_denies_unapproved_capability_via_path() {
    let f = composite_fixture();
    let mut engine = f.synced_engine();
    engine.create_dashboard("Mine").unwrap();
    engine.add_widget("u:1", "pair", None).unwrap();
    // Consent was given at sync; withdraw it and forget what was seen, so nothing shows
    // as pending and the run-time check is what denies.
    let db = rusqlite::Connection::open(f.data.join("pharos.db")).unwrap();
    db.execute(
        "UPDATE sources SET approved_capabilities = '{}', seen_capabilities = '{}'",
        [],
    )
    .unwrap();

    // The http leaf is denied at the leaf, not just the composite.
    let err = engine.run_placement("u:1", &[0, 0]).unwrap_err();
    assert!(
        matches!(&err, EngineError::Http(HttpError::NotApproved(o)) if o == "https://elsewhere.example"),
        "{err:?}"
    );
    // The file leaf too.
    let err = engine.run_placement("u:1", &[0, 1]).unwrap_err();
    assert!(
        matches!(&err, EngineError::File(FileError::NotApproved(_))),
        "{err:?}"
    );
}

#[test]
fn run_by_path_reaches_an_approved_leaf_in_a_user_composite() {
    let f = composite_fixture();
    let mut engine = f.synced_engine();
    engine.create_dashboard("Mine").unwrap();
    engine.add_widget("u:1", "pair", None).unwrap();

    let data = engine.run_placement("u:1", &[0, 1]).unwrap();

    assert_eq!(data.rows.len(), 2);
}

#[test]
fn bad_paths_are_clean_errors() {
    let f = composite_fixture();
    let mut engine = f.synced_engine();
    engine.create_dashboard("Mine").unwrap();
    engine.add_widget("u:1", "pair", None).unwrap();
    engine.add_widget("u:1", "welcome", None).unwrap();

    // Lands on a container.
    assert!(matches!(
        engine.run_placement("u:1", &[0]),
        Err(EngineError::NotALeaf(n)) if n == "pair"
    ));
    for path in [
        &[][..],
        &[5],
        &[0, 9],
        &[0, 0, 0],
        &[1, 0],
        &[usize::MAX],
        &[0, usize::MAX],
    ] {
        assert!(
            matches!(
                engine.run_placement("u:1", path),
                Err(EngineError::UnknownPlacement { .. })
            ),
            "{path:?}"
        );
    }
    for id in ["u:99", "u:x", "nope", "p:nope", ""] {
        assert!(
            matches!(
                engine.run_placement(id, &[0]),
                Err(EngineError::UnknownDashboard(_))
            ),
            "{id}"
        );
    }
    // A static leaf has no source to run.
    assert!(matches!(
        engine.run_placement("u:1", &[1]),
        Err(EngineError::NoDataSource(_))
    ));
}

#[test]
fn run_reaches_a_leaf_inside_a_user_group() {
    let f = composite_fixture();
    let mut engine = f.synced_engine();
    engine.create_dashboard("Mine").unwrap();
    let view = engine.create_group("u:1", "Infra", None).unwrap();
    let gid = view.placements[0].id.unwrap();
    engine
        .add_widget("u:1", "service-catalog", Some(gid))
        .unwrap();

    let data = engine.run_placement("u:1", &[0, 0]).unwrap();

    assert_eq!(data.rows.len(), 2);
}

#[test]
fn run_on_a_group_node_is_a_clean_error() {
    let f = composite_fixture();
    let mut engine = f.synced_engine();
    engine.create_dashboard("Mine").unwrap();
    engine.create_group("u:1", "Infra", None).unwrap();

    assert!(matches!(
        engine.run_placement("u:1", &[0]),
        Err(EngineError::NotALeaf(_))
    ));
    assert!(matches!(
        engine.run_placement("u:1", &[0, 0]),
        Err(EngineError::UnknownPlacement { .. })
    ));
}

/// An engine whose first sync was applied without approval.
fn unapproved_engine(f: &Fixture) -> Engine {
    let mut engine = Engine::open(&f.data).unwrap();
    engine.add_source(&f.url(), "main").unwrap();
    engine.plan_sync().unwrap();
    engine.apply_sync(false).unwrap();
    engine
}

#[test]
fn first_sync_applied_without_approval_blocks_capable_widgets() {
    let f = Fixture::new();
    let mut engine = unapproved_engine(&f);
    assert!(
        engine.approved_capabilities().unwrap().widgets.is_empty(),
        "nothing is approved yet"
    );
    // What the sync recorded as declared: the engine has no public accessor for it.
    let db = rusqlite::Connection::open(f.data.join("pharos.db")).unwrap();
    let seen_json: String = db
        .query_row("SELECT seen_capabilities FROM sources", [], |r| r.get(0))
        .unwrap();
    let seen: serde_json::Value = serde_json::from_str(&seen_json).unwrap();

    let home = engine.dashboards().unwrap().remove(0);
    for p in &home.placements {
        if p.widget == "welcome" {
            assert_eq!(p.status, PlacementStatus::Static);
            continue;
        }
        let PlacementStatus::NeedsApproval { added } = &p.status else {
            panic!("{}: {:?}", p.widget, p.status);
        };
        assert!(!added.is_empty(), "{}", p.widget);
        assert_eq!(
            serde_json::to_value(added).unwrap(),
            seen[&p.widget],
            "{}",
            p.widget
        );
    }
    let PlacementStatus::NeedsApproval { added } = &home.placements[0].status else {
        panic!("service-catalog should be blocked");
    };
    assert!(added.paths.contains("catalog/services"), "{added:?}");
    assert!(home.placements[0].reads.is_none());
    assert!(matches!(
        engine.run_placement("p:home", &[0]),
        Err(EngineError::NeedsApproval(name)) if name == "service-catalog"
    ));

    engine.approve_widget("service-catalog").unwrap();

    let home = engine.dashboards().unwrap().remove(0);
    assert_eq!(home.placements[0].status, PlacementStatus::Runnable);
    assert_eq!(engine.run_placement("p:home", &[0]).unwrap().rows.len(), 2);
    // Approving one widget leaves the others blocked.
    assert!(matches!(
        home.placements[1].status,
        PlacementStatus::NeedsApproval { .. }
    ));
}

#[test]
fn approve_widget_uses_the_stored_snapshot_not_the_pending_plan() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    let status = |host: &str| {
        format!(
            "apiVersion: pharos.io/v1alpha1\nkind: Widget\nmetadata: {{ name: status-page }}\n\
             spec:\n  source: {{ type: http, url: {host}/api }}\n  view: {{ type: raw }}\n"
        )
    };
    f.write(
        "widgets/status-page/widget.yaml",
        &status("https://h2.acme.dev"),
    );
    f.commit("h2");
    engine.plan_sync().unwrap();
    engine.apply_sync(false).unwrap();
    f.write(
        "widgets/status-page/widget.yaml",
        &status("https://h3.acme.dev"),
    );
    f.commit("h3");
    engine.plan_sync().unwrap();
    assert!(engine.pending_sync().is_some());

    engine.approve_widget("status-page").unwrap();

    let hosts = engine
        .approved_capabilities()
        .unwrap()
        .widgets
        .remove("status-page")
        .unwrap()
        .hosts;
    assert_eq!(hosts, ["https://h2.acme.dev".to_string()].into());
    assert!(engine.pending_sync().is_some(), "the plan is untouched");
}

#[test]
fn approve_widget_rejects_containers_and_unknown_names() {
    let f = Fixture::new();
    let mut engine = unapproved_engine(&f);

    assert!(matches!(
        engine.approve_widget("on-call-overview"),
        Err(EngineError::NotALeaf(n)) if n == "on-call-overview"
    ));
    assert!(matches!(
        engine.approve_widget("nope"),
        Err(EngineError::UnknownWidget(n)) if n == "nope"
    ));
    assert!(engine.approved_capabilities().unwrap().widgets.is_empty());
}

#[test]
fn report_widget_uses_pending_capabilities_and_support() {
    let f = Fixture::new();
    set_support(
        &f,
        Some("{ type: github, repo: https://github.com/acme/platform }"),
    );
    let engine = unapproved_engine(&f);

    let report = engine.report_widget("service-catalog").unwrap();
    assert!(
        report.body.contains("Read catalog/services"),
        "{}",
        report.body
    );
    assert!(
        report
            .link
            .unwrap()
            .starts_with("https://github.com/acme/platform/issues/new?title=")
    );
}

#[test]
fn report_widget_without_support_has_no_link() {
    let f = Fixture::new();
    set_support(&f, None);
    let engine = unapproved_engine(&f);

    let report = engine.report_widget("service-catalog").unwrap();
    assert!(report.link.is_none());
}

#[test]
fn report_widget_refuses_unblocked_containers_and_unknown() {
    let f = Fixture::new();
    let mut engine = unapproved_engine(&f);

    assert!(matches!(
        engine.report_widget("on-call-overview"),
        Err(EngineError::NotALeaf(n)) if n == "on-call-overview"
    ));
    assert!(matches!(
        engine.report_widget("nope"),
        Err(EngineError::UnknownWidget(n)) if n == "nope"
    ));
    engine.approve_widget("service-catalog").unwrap();
    assert!(matches!(
        engine.report_widget("service-catalog"),
        Err(EngineError::NothingToReport(n)) if n == "service-catalog"
    ));
}
