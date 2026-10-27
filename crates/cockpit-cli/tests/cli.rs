use std::process::Command;

fn cockpit(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cockpit"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cockpit runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn lint_passes_on_example_repo() {
    let (code, stdout) = cockpit(&["lint", "../../examples/platform-repo"]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("0 errors, 0 warnings"), "{stdout}");
    assert!(
        stdout.contains("my-pipelines             host  https://gitlab.com"),
        "{stdout}"
    );
}

#[test]
fn lint_fails_on_broken_repo() {
    let (code, stdout) = cockpit(&["lint", "../cockpit-manifest/tests/fixtures/paths"]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("[path-escape]"), "{stdout}");
    assert!(!stdout.contains("capabilities"), "{stdout}");
}

#[test]
fn lint_json_output() {
    let (code, stdout) = cockpit(&[
        "lint",
        "--format",
        "json",
        "../cockpit-manifest/tests/fixtures/dashboard",
    ]);
    assert_eq!(code, 1);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert_eq!(report["diagnostics"][0]["code"], "overlap");
    assert_eq!(report["diagnostics"][0]["path"], "spec.widgets[1]");
}
