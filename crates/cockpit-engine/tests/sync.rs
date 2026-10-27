//! End-to-end sync against real git repos in temp dirs (an "origin" seeded from
//! examples/platform-repo, cloned by the engine like any remote).

use cockpit_engine::{Engine, EngineError, GitError};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Fixture {
    _tmp: tempfile::TempDir,
    origin: PathBuf,
    data: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let origin = tmp.path().join("origin");
        copy_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/platform-repo"),
            &origin,
        );
        git(&origin, &["init", "--quiet", "--initial-branch=main"]);
        let fixture = Self {
            data: tmp.path().join("data"),
            origin,
            _tmp: tmp,
        };
        fixture.commit("initial");
        fixture
    }

    fn url(&self) -> String {
        self.origin.to_string_lossy().into_owned()
    }

    fn write(&self, rel: &str, body: &str) {
        let path = self.origin.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn remove(&self, rel: &str) {
        fs::remove_dir_all(self.origin.join(rel)).unwrap();
    }

    fn commit(&self, message: &str) {
        git(&self.origin, &["add", "-A"]);
        git(
            &self.origin,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "--quiet",
                "-m",
                message,
            ],
        );
    }

    /// An engine with the first sync applied.
    fn synced_engine(&self) -> Engine {
        let mut engine = Engine::open(&self.data).unwrap();
        engine.add_source(&self.url(), "main").unwrap();
        engine.plan_sync().unwrap();
        engine.apply_sync(true).unwrap();
        engine
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

const STATUS_WIDGET: &str = "apiVersion: cockpit.dev/v1alpha1
kind: Widget
metadata: { name: status-page }
spec:
  source: { type: http, url: https://status.acme.dev/api/summary }
  view: { type: kv }
";

#[test]
fn first_sync_needs_consent_then_stores_snapshot() {
    let f = Fixture::new();
    let mut engine = Engine::open(&f.data).unwrap();
    assert!(Engine::git_version().unwrap().starts_with("git version"));
    let source = engine.add_source(&f.url(), "main").unwrap();
    assert_eq!(source.synced_commit, None);
    assert!(source.local_path.join("cockpit.yaml").is_file());

    let plan = engine.plan_sync().unwrap();
    assert_eq!(plan.from_commit, None);
    assert_eq!(plan.to_commit, git(&f.origin, &["rev-parse", "HEAD"]));
    assert_eq!(
        plan.widgets.added,
        ["k8s-context", "my-pipelines", "service-catalog", "welcome"]
    );
    assert_eq!(plan.dashboards.added, ["home"]);
    assert!(plan.platform_changed);
    assert!(plan.needs_consent);
    assert_eq!(plan.errors(), 0);
    assert_eq!(plan.capabilities.widgets.len(), 3);

    assert!(matches!(
        engine.apply_sync(false),
        Err(EngineError::ConsentRequired)
    ));
    engine.apply_sync(true).unwrap();
    assert!(engine.pending_sync().is_none());

    let source = engine.source().unwrap().unwrap();
    assert_eq!(
        source.synced_commit,
        Some(git(&f.origin, &["rev-parse", "HEAD"]))
    );
    assert!(source.approved_capability_hash.is_some());
    let names: Vec<_> = engine
        .widgets()
        .unwrap()
        .into_iter()
        .map(|w| w.metadata.name)
        .collect();
    assert_eq!(
        names,
        ["k8s-context", "my-pipelines", "service-catalog", "welcome"]
    );
    assert_eq!(
        engine.dashboard_templates().unwrap()[0].metadata.name,
        "home"
    );

    // Nothing changed upstream: the next plan is a no-op.
    assert!(engine.plan_sync().unwrap().is_up_to_date());
}

#[test]
fn resync_reports_changelog_and_asks_consent_for_new_capabilities() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();

    f.remove("widgets/welcome");
    f.write("widgets/status-page/widget.yaml", STATUS_WIDGET);
    let k8s = fs::read_to_string(f.origin.join("widgets/k8s-context/widget.yaml")).unwrap();
    f.write(
        "widgets/k8s-context/widget.yaml",
        &k8s.replace("refresh: 30s", "refresh: 1m"),
    );
    let home = fs::read_to_string(f.origin.join("dashboards/home.yaml")).unwrap();
    f.write(
        "dashboards/home.yaml",
        &home.replace("welcome", "status-page"),
    );
    f.commit("update");

    let plan = engine.plan_sync().unwrap();
    assert_eq!(plan.widgets.added, ["status-page"]);
    assert_eq!(plan.widgets.removed, ["welcome"]);
    assert_eq!(plan.widgets.changed, ["k8s-context"]);
    assert_eq!(plan.dashboards.changed, ["home"]);
    assert!(!plan.platform_changed);
    assert!(plan.needs_consent, "new host https://status.acme.dev");
    assert!(
        plan.capabilities.widgets["status-page"]
            .hosts
            .contains("https://status.acme.dev")
    );

    engine.apply_sync(true).unwrap();
    assert!(engine.plan_sync().unwrap().is_up_to_date());
}

#[test]
fn unchanged_capabilities_apply_without_consent() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();

    let k8s = fs::read_to_string(f.origin.join("widgets/k8s-context/widget.yaml")).unwrap();
    f.write(
        "widgets/k8s-context/widget.yaml",
        &k8s.replace("refresh: 30s", "refresh: 1m"),
    );
    f.commit("slower refresh");

    let plan = engine.plan_sync().unwrap();
    assert_eq!(plan.widgets.changed, ["k8s-context"]);
    assert!(!plan.needs_consent);
    engine.apply_sync(false).unwrap();
}

#[test]
fn lint_errors_block_apply_and_discard_restores_the_clone() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    let good = engine.source().unwrap().unwrap().synced_commit.unwrap();

    f.write(
        "widgets/escape/widget.yaml",
        "apiVersion: cockpit.dev/v1alpha1\nkind: Widget\nmetadata: { name: escape }\nspec:\n  source: { type: file, path: ../../../etc }\n  view: { type: raw }\n",
    );
    f.commit("broken");

    let plan = engine.plan_sync().unwrap();
    assert_eq!(plan.errors(), 1);
    assert_eq!(plan.diagnostics[0].code, "path-escape");
    assert!(matches!(
        engine.apply_sync(true),
        Err(EngineError::LintErrors(1))
    ));

    engine.discard_sync().unwrap();
    let source = engine.source().unwrap().unwrap();
    assert_eq!(git(&source.local_path, &["rev-parse", "HEAD"]), good);
    assert!(!source.local_path.join("widgets/escape").exists());
    assert_eq!(engine.widgets().unwrap().len(), 4);
    assert!(matches!(
        engine.apply_sync(true),
        Err(EngineError::NoPendingSync)
    ));
}

#[test]
fn apply_refuses_a_plan_when_the_clone_moved() {
    let f = Fixture::new();
    let mut engine = f.synced_engine();
    f.write("widgets/status-page/widget.yaml", STATUS_WIDGET);
    f.commit("add");
    engine.plan_sync().unwrap();

    let clone = engine.source().unwrap().unwrap().local_path;
    git(&clone, &["reset", "--quiet", "--hard", "HEAD~1"]);
    assert!(matches!(
        engine.apply_sync(true),
        Err(EngineError::StalePlan)
    ));
}

#[test]
fn failed_clone_reports_git_error_and_stores_nothing() {
    let f = Fixture::new();
    let mut engine = Engine::open(&f.data).unwrap();

    let err = engine.add_source("/does/not/exist", "main").unwrap_err();
    let EngineError::Git(GitError::Failed { stderr, .. }) = err else {
        panic!("expected a git failure, got {err:?}");
    };
    assert!(stderr.contains("does not exist"), "{stderr}");
    assert_eq!(engine.source().unwrap(), None);

    let err = engine.add_source(&f.url(), "no-such-branch").unwrap_err();
    assert!(err.to_string().contains("no-such-branch"), "{err}");
    assert!(matches!(
        engine.add_source(&f.url(), "--upload-pack=x"),
        Err(EngineError::InvalidBranch(_))
    ));

    engine.add_source(&f.url(), "main").unwrap();
    assert!(matches!(
        engine.add_source(&f.url(), "main"),
        Err(EngineError::SourceExists)
    ));
}

#[test]
fn state_survives_reopen() {
    let f = Fixture::new();
    let source = f.synced_engine().source().unwrap();

    let engine = Engine::open(&f.data).unwrap();
    assert_eq!(engine.source().unwrap(), source);
    assert_eq!(engine.widgets().unwrap().len(), 4);
    assert!(engine.pending_sync().is_none());
}
