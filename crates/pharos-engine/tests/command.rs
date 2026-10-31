//! Command sources against the example repo: templated args, the Settings search path, and
//! the errors a widget shows (exit code + stderr, executable not found).

mod common;

use common::Fixture;
use pharos_engine::{CommandError, DEFAULT_SEARCH_PATH, Engine, EngineError};
use serde_json::json;

/// The example repo plus a `local` dashboard whose widgets run `echo` and `sh`.
fn engine() -> Engine {
    let f = Fixture::new();
    f.write(
        "widgets/echo/widget.yaml",
        r#"apiVersion: pharos.io/v1alpha1
kind: Widget
metadata: { name: echo, version: 0.1.0 }
spec:
  inputs:
    word: { type: string, default: hello }
  source:
    type: command
    executable: echo
    args: ["{{ inputs.word }}", "{{ user.team }}"]
  view: { type: raw }
"#,
    );
    f.write(
        "widgets/fails/widget.yaml",
        r#"apiVersion: pharos.io/v1alpha1
kind: Widget
metadata: { name: fails, version: 0.1.0 }
spec:
  source:
    type: command
    executable: sh
    args: [-c, "echo 'no context' >&2; exit 2"]
  view: { type: raw }
"#,
    );
    f.write(
        "dashboards/local.yaml",
        r#"apiVersion: pharos.io/v1alpha1
kind: Dashboard
metadata: { name: local }
spec:
  widgets:
    - { widget: echo, x: 0, y: 0, w: 6, h: 2, inputs: { word: "hi-{{ user.team }}" } }
    - { widget: echo, x: 6, y: 0, w: 6, h: 2 }
    - { widget: fails, x: 0, y: 2, w: 6, h: 2 }
"#,
    );
    f.commit("command widgets");
    let mut engine = f.synced_engine();
    engine.set_input("team", Some("payments-team")).unwrap();
    std::mem::forget(f);
    engine
}

#[test]
fn runs_with_args_rendered_per_placement() {
    let engine = engine();

    let data = engine.run_placement("local", 0).unwrap();
    assert_eq!(data.rows, vec![json!({ "value": "hi-payments-team payments-team" })]);

    let data = engine.run_placement("local", 1).unwrap();
    assert_eq!(data.rows, vec![json!({ "value": "hello payments-team" })]);
}

#[test]
fn reports_exit_code_and_stderr() {
    let engine = engine();

    let err = engine.run_placement("local", 2).unwrap_err();
    assert_eq!(
        err.to_string(),
        "`sh` exited with code 2: no context",
        "{err:?}"
    );
}

#[test]
fn looks_up_executables_in_the_search_path() {
    let mut engine = engine();
    let empty = tempfile::tempdir().unwrap();
    let dir = empty.path().to_string_lossy().into_owned();
    engine.set_search_path(std::slice::from_ref(&dir)).unwrap();

    let err = engine.run_placement("local", 1).unwrap_err();
    assert!(
        matches!(&err, EngineError::Command(CommandError::NotFound { executable, searched })
            if executable == "echo" && searched == std::slice::from_ref(&dir)),
        "{err:?}"
    );
}

#[test]
fn search_path_defaults_validates_and_resets() {
    let mut engine = engine();
    let default: Vec<String> = DEFAULT_SEARCH_PATH.iter().map(|d| d.to_string()).collect();
    assert_eq!(engine.search_path().unwrap(), default);

    engine
        .set_search_path(&[" /opt/tools/bin ".into(), String::new(), "~/bin".into()])
        .unwrap();
    assert_eq!(engine.search_path().unwrap(), ["/opt/tools/bin", "~/bin"]);

    assert!(matches!(
        engine.set_search_path(&["bin".into()]),
        Err(EngineError::InvalidValue { key, .. }) if key == "bin"
    ));
    assert_eq!(engine.search_path().unwrap(), ["/opt/tools/bin", "~/bin"]);

    engine.set_search_path(&[]).unwrap();
    assert_eq!(engine.search_path().unwrap(), default);
}
