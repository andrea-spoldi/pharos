//! File widget execution from the stored snapshot, against a synced clone of
//! examples/platform-repo. Placements are run by dashboard id and path.

mod common;

use common::Fixture;
use pharos_engine::{EngineError, FileError, HttpError};
use serde_json::json;

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
    // Consent was given at sync; withdraw it, leaving nothing approved.
    let db = rusqlite::Connection::open(f.data.join("pharos.db")).unwrap();
    db.execute("UPDATE sources SET approved_capabilities = '{}'", [])
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
