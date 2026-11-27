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

fn support_diags(support: &str) -> Vec<(&'static str, Option<String>)> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("pharos.yaml"),
        format!(
            "apiVersion: pharos.io/v1alpha1\nkind: Platform\nmetadata: {{ name: t }}\nspec:\n  support: {support}\n"
        ),
    )
    .unwrap();
    lint(dir.path())
        .diagnostics
        .into_iter()
        .map(|d| (d.code, d.path))
        .collect()
}

#[test]
fn support_parses_each_type() {
    use pharos_manifest::{Platform, Support};
    let parse = |s: &str| {
        serde_yaml_ng::from_str::<Platform>(&format!(
            "apiVersion: pharos.io/v1alpha1\nkind: Platform\nmetadata: {{ name: t }}\nspec:\n  support: {s}\n"
        ))
    };
    let support = |s: &str| parse(s).unwrap().spec.support.unwrap();
    assert_eq!(
        support("{ type: github, repo: 'https://github.com/a/b' }"),
        Support::Github {
            repo: "https://github.com/a/b".into()
        }
    );
    assert_eq!(
        support("{ type: gitlab, repo: 'https://gitlab.com/a/b' }"),
        Support::Gitlab {
            repo: "https://gitlab.com/a/b".into()
        }
    );
    assert_eq!(
        support("{ type: email, address: a@b.dev }"),
        Support::Email {
            address: "a@b.dev".into()
        }
    );
    assert!(parse("{ type: jira, repo: x }").is_err());
    assert!(parse("{ type: email, address: a@b.dev, extra: 1 }").is_err());
}

#[test]
fn support_repo_must_be_plain_https() {
    for bad in [
        "http://github.com/a/b",
        "https://h/x?y",
        "https://h/x#y",
        "https://{{ user.x }}/r",
    ] {
        let got = support_diags(&format!("{{ type: github, repo: '{bad}' }}"));
        assert_eq!(
            got,
            vec![("invalid-support-repo", Some("spec.support.repo".into()))],
            "{bad}"
        );
    }
    for ok in [
        "https://github.com/acme/platform",
        "https://github.com/acme/platform/",
    ] {
        assert!(support_diags(&format!("{{ type: gitlab, repo: '{ok}' }}")).is_empty());
    }
}

#[test]
fn support_address_must_be_plain() {
    for bad in ["team", "a@b@c", "a@b?cc=x", "a @b"] {
        let got = support_diags(&format!("{{ type: email, address: '{bad}' }}"));
        assert_eq!(
            got,
            vec![(
                "invalid-support-address",
                Some("spec.support.address".into())
            )],
            "{bad}"
        );
    }
    assert!(support_diags("{ type: email, address: platform-team@acme.dev }").is_empty());
}
