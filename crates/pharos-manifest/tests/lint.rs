//! Snapshot tests: each fixture is a small platform repo with one class of mistakes.
//! Review changes with `INSTA_UPDATE=always cargo test` + `git diff`, or `cargo insta review`.

use pharos_manifest::lint;
use std::path::PathBuf;

fn path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn render(rel: &str) -> String {
    let report = lint(&path(rel));
    let mut out: String = report
        .diagnostics
        .iter()
        .map(|d| format!("{d}\n"))
        .collect();
    out.push_str(&format!(
        "-- {} errors, {} warnings\n",
        report.errors(),
        report.warnings()
    ));
    out
}

macro_rules! fixture {
    ($($name:ident => $dir:literal),* $(,)?) => {$(
        #[test]
        fn $name() {
            insta::assert_snapshot!(render(concat!("tests/fixtures/", $dir)));
        }
    )*};
}

fixture! {
    missing_platform => "missing-platform",
    syntax_error => "syntax-error",
    wrong_kind => "wrong-kind",
    schema_errors => "schema-errors",
    paths => "paths",
    templates => "templates",
    command => "command",
    dashboard => "dashboard",
    duplicates => "duplicates",
    views => "views",
    catalog => "catalog",
    platform_spec => "platform-spec",
    containers => "containers",
    containers_both => "containers-both",
    containers_empty => "containers-empty",
    containers_unknown_child => "containers-unknown-child",
    containers_cycle => "containers-cycle",
    containers_self => "containers-self",
    containers_cycle3 => "containers-cycle3",
    containers_nested => "containers-nested",
}

#[test]
fn missing_repo() {
    let report = lint(&path("tests/fixtures/does-not-exist"));
    assert_eq!(report.errors(), 1);
    assert_eq!(report.diagnostics[0].code, "repo-not-found");
}

#[test]
fn example_repo_is_clean() {
    let report = lint(&path("../../examples/platform-repo"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    assert_eq!(
        (
            report.summary.widgets,
            report.summary.dashboards,
            report.summary.catalog_files
        ),
        (5, 1, 4)
    );
    insta::assert_json_snapshot!(report.capabilities);
}
