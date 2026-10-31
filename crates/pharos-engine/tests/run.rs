//! File widget execution from the stored snapshot, against a synced clone of
//! examples/platform-repo. Placements are run by dashboard and position.

mod common;

use common::Fixture;
use pharos_engine::{EngineError, FileError};
use serde_json::json;

#[test]
fn service_catalog_reads_catalog_files_with_their_source() {
    let f = Fixture::new();
    let engine = f.synced_engine();

    let data = engine.run_placement("home", 0).unwrap();

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

    let data = engine.run_placement("home", 0).unwrap();

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

    let err = engine.run_placement("home", 0).unwrap_err();

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
        engine.run_placement("home", 9),
        Err(EngineError::UnknownPlacement { dashboard, index: 9 }) if dashboard == "home"
    ));
    assert!(matches!(
        engine.run_placement("nope", 0),
        Err(EngineError::UnknownPlacement { .. })
    ));
    assert!(matches!(
        engine.run_placement("home", 1),
        Err(EngineError::NotAvailable(feature)) if feature == "command sources"
    ));
    assert!(matches!(
        engine.run_placement("home", 2),
        Err(EngineError::NoDataSource(name)) if name == "welcome"
    ));
    // My Pipelines: no team and no token yet.
    assert!(matches!(
        engine.run_placement("home", 3),
        Err(EngineError::NeedsSetup(name)) if name == "my-pipelines"
    ));
}
