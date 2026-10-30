//! Platform inputs and secrets, against a synced clone of examples/platform-repo.

mod common;

use common::{Fixture, MemoryStore};
use pharos_engine::{Engine, EngineError, PlacementStatus, StoredKind, StoredRef};
use pharos_manifest::InputType;
use rusqlite::Connection;
use std::fs;

const ACCOUNT: &str = "acme-platform/GITLAB_TOKEN";

fn stored_inputs(f: &Fixture) -> Vec<(String, String)> {
    let conn = Connection::open(f.data.join("pharos.db")).unwrap();
    let mut stmt = conn
        .prepare("SELECT key, value FROM user_inputs ORDER BY key")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn secret_rows(f: &Fixture) -> Vec<String> {
    let conn = Connection::open(f.data.join("pharos.db")).unwrap();
    let mut stmt = conn.prepare("SELECT name FROM secrets_meta").unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn add_string_input(f: &Fixture, engine: &mut Engine) {
    let platform = fs::read_to_string(f.origin.join("pharos.yaml")).unwrap();
    f.write(
        "pharos.yaml",
        &platform.replace("  inputs:\n", "  inputs:\n    nick:\n      type: string\n"),
    );
    f.resync(engine, "nick");
}

#[test]
fn set_and_clear_an_input() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());

    engine.set_input("team", Some("payments-team")).unwrap();
    assert_eq!(stored_inputs(&f), [("team".into(), "payments-team".into())]);

    engine.set_input("team", None).unwrap();
    assert!(stored_inputs(&f).is_empty());
}

#[test]
fn blank_string_input_clears() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());
    add_string_input(&f, &mut engine);

    engine.set_input("nick", Some(" ada ")).unwrap();
    assert_eq!(stored_inputs(&f), [("nick".into(), "ada".into())]);
    engine.set_input("nick", Some("  ")).unwrap();
    assert!(stored_inputs(&f).is_empty());
}

#[test]
fn enum_rejects_values_outside_choices() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());

    let err = engine.set_input("team", Some("nope")).unwrap_err();
    assert!(matches!(err, EngineError::InvalidValue { .. }), "{err:?}");
    engine.set_input("team", Some("platform-team")).unwrap();
}

#[test]
fn undeclared_names_are_refused() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());

    let err = engine.set_input("x", Some("1")).unwrap_err();
    assert!(
        matches!(err, EngineError::Undeclared(ref k) if k == "x"),
        "{err:?}"
    );
    let err = engine.set_secret("X", "v").unwrap_err();
    assert!(
        matches!(err, EngineError::Undeclared(ref k) if k == "X"),
        "{err:?}"
    );
}

#[test]
fn secret_goes_to_the_store_never_to_sqlite() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());

    engine
        .set_secret("GITLAB_TOKEN", "glpat-SECRET123")
        .unwrap();

    assert_eq!(store.get(ACCOUNT).as_deref(), Some("glpat-SECRET123"));
    assert_eq!(secret_rows(&f), ["GITLAB_TOKEN"]);
    drop(engine);
    for entry in fs::read_dir(&f.data).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let bytes = fs::read(&path).unwrap();
            assert!(
                !bytes.windows(15).any(|w| w == b"glpat-SECRET123"),
                "secret found in {}",
                path.display()
            );
        }
    }
}

#[test]
fn secret_values_are_trimmed_and_blank_is_refused() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());

    engine.set_secret("GITLAB_TOKEN", " tok\n").unwrap();
    assert_eq!(store.get(ACCOUNT).as_deref(), Some("tok"));
    let err = engine.set_secret("GITLAB_TOKEN", "  ").unwrap_err();
    assert!(matches!(err, EngineError::InvalidValue { .. }), "{err:?}");
}

#[test]
fn keyring_failure_records_nothing_and_hides_the_value() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());
    store.set_failing(true);

    let err = engine.set_secret("GITLAB_TOKEN", "glpat-X").unwrap_err();

    assert!(matches!(err, EngineError::Keyring { .. }), "{err:?}");
    assert!(!err.to_string().contains("glpat-X"));
    assert!(!format!("{err:?}").contains("glpat-X"));
    assert!(secret_rows(&f).is_empty());
}

#[test]
fn remove_secret_deletes_store_entry_and_row() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());
    engine.set_secret("GITLAB_TOKEN", "tok").unwrap();

    engine.remove_secret("GITLAB_TOKEN").unwrap();

    assert_eq!(store.get(ACCOUNT), None);
    assert!(secret_rows(&f).is_empty());
}

#[test]
fn nothing_before_first_apply() {
    let f = Fixture::new();
    let mut engine = Engine::open_with(&f.data, Box::new(MemoryStore::default())).unwrap();
    engine.add_source(&f.url(), "main").unwrap();

    let err = engine.set_input("team", Some("platform-team")).unwrap_err();
    assert!(matches!(err, EngineError::NoPlatform), "{err:?}");
    let view = engine.settings().unwrap();
    assert!(view.source.is_some());
    assert!(view.platform.is_none());
    assert!(view.inputs.is_empty() && view.secrets.is_empty());
}

fn pipelines_status(engine: &Engine) -> PlacementStatus {
    engine.dashboards().unwrap()[0]
        .placements
        .iter()
        .find(|p| p.widget == "my-pipelines")
        .unwrap()
        .status
        .clone()
}

#[test]
fn settings_lists_inputs_secrets_and_choices() {
    let f = Fixture::new();
    let engine = f.synced_engine_with(MemoryStore::default());

    let view = engine.settings().unwrap();

    let platform = view.platform.unwrap();
    assert_eq!(platform.name, "acme-platform");
    assert_eq!(platform.title.as_deref(), Some("ACME Developer Portal"));
    let team = &view.inputs[0];
    assert_eq!(team.key, "team");
    assert_eq!(team.kind, InputType::Enum);
    assert!(team.required);
    assert_eq!(team.label.as_deref(), Some("Your team"));
    assert_eq!(team.choices, ["payments-team", "platform-team"]);
    assert_eq!(team.value, None);
    assert!(!team.stale);
    assert_eq!(view.secrets.len(), 1);
    assert_eq!(view.secrets[0].name, "GITLAB_TOKEN");
    assert_eq!(
        view.secrets[0].description.as_deref(),
        Some("Personal access token (read_api)")
    );
    assert!(!view.secrets[0].set);
    assert!(view.unused.is_empty());
}

#[test]
fn setting_everything_clears_needs_setup() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());

    engine.set_input("team", Some("payments-team")).unwrap();
    engine.set_secret("GITLAB_TOKEN", "tok").unwrap();
    assert_eq!(pipelines_status(&engine), PlacementStatus::Runnable);
    assert!(engine.settings().unwrap().secrets[0].set);

    engine.remove_secret("GITLAB_TOKEN").unwrap();
    assert_eq!(
        pipelines_status(&engine),
        PlacementStatus::NeedsSetup {
            inputs: vec![],
            secrets: vec!["GITLAB_TOKEN".into()],
        }
    );
}

#[test]
fn a_default_counts_as_set() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());
    let platform = fs::read_to_string(f.origin.join("pharos.yaml")).unwrap();
    f.write(
        "pharos.yaml",
        &platform.replace(
            "      required: true\n",
            "      required: true\n      default: platform-team\n",
        ),
    );
    f.resync(&mut engine, "default team");

    assert_eq!(
        pipelines_status(&engine),
        PlacementStatus::NeedsSetup {
            inputs: vec![],
            secrets: vec!["GITLAB_TOKEN".into()],
        }
    );
    assert_eq!(
        engine.settings().unwrap().inputs[0].default.as_deref(),
        Some("platform-team")
    );
}

#[test]
fn a_removed_choice_goes_stale() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());
    engine.set_input("team", Some("payments-team")).unwrap();

    f.delete("catalog/teams/payments-team.yaml");
    f.resync(&mut engine, "drop payments");

    let team = &engine.settings().unwrap().inputs[0];
    assert_eq!(team.value.as_deref(), Some("payments-team"));
    assert!(team.stale);
    assert_eq!(
        pipelines_status(&engine),
        PlacementStatus::NeedsSetup {
            inputs: vec!["team".into()],
            secrets: vec!["GITLAB_TOKEN".into()],
        }
    );
}

#[test]
fn enum_with_missing_catalog_has_no_choices() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());
    let platform = fs::read_to_string(f.origin.join("pharos.yaml")).unwrap();
    f.write(
        "pharos.yaml",
        &platform.replace("from: catalog/teams/", "from: catalog/nowhere/"),
    );
    f.commit("nowhere");
    let plan = engine.plan_sync().unwrap();
    if plan
        .diagnostics
        .iter()
        .any(|d| d.severity == pharos_manifest::Severity::Error)
    {
        // Lint refuses a missing catalog directory, so an empty one stands in for it.
        engine.discard_sync().unwrap();
        f.write(
            "pharos.yaml",
            &platform.replace("from: catalog/teams/", "from: catalog/empty/"),
        );
        f.write("catalog/empty/README.md", "no teams yet\n");
        f.commit("empty");
        engine.plan_sync().unwrap();
    }
    engine.apply_sync(true).unwrap();

    assert!(engine.settings().unwrap().inputs[0].choices.is_empty());
    let err = engine.set_input("team", Some("platform-team")).unwrap_err();
    assert!(matches!(err, EngineError::InvalidValue { .. }), "{err:?}");
}

#[test]
fn undeclared_rows_are_listed_as_unused() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());
    engine.set_secret("GITLAB_TOKEN", "tok").unwrap();
    let platform = fs::read_to_string(f.origin.join("pharos.yaml")).unwrap();
    let without = &platform[..platform.find("  secrets:").unwrap()];
    f.write("pharos.yaml", without);
    // my-pipelines still references the secret, so drop that widget from the repo too.
    f.remove("widgets/my-pipelines");
    let home = fs::read_to_string(f.origin.join("dashboards/home.yaml")).unwrap();
    f.write(
        "dashboards/home.yaml",
        &home[..home.find("    - widget: my-pipelines").unwrap()],
    );
    f.resync(&mut engine, "no secrets");

    let unused = engine.settings().unwrap().unused;
    assert_eq!(
        unused,
        [StoredRef {
            platform: "acme-platform".into(),
            kind: StoredKind::Secret,
            key: "GITLAB_TOKEN".into(),
        }]
    );
    engine
        .remove_stored("acme-platform", StoredKind::Secret, "GITLAB_TOKEN")
        .unwrap();
    assert!(engine.settings().unwrap().unused.is_empty());
    assert_eq!(store.get(ACCOUNT), None);
}

fn reconnect(f: &Fixture, engine: &mut Engine) {
    engine.add_source(&f.url(), "main").unwrap();
    engine.plan_sync().unwrap();
    engine.apply_sync(true).unwrap();
}

#[test]
fn disconnect_keeps_details_for_the_same_platform() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());
    engine.set_input("team", Some("payments-team")).unwrap();
    engine.set_secret("GITLAB_TOKEN", "tok").unwrap();
    let clone = engine.source().unwrap().unwrap().local_path;

    engine.disconnect(false).unwrap();

    assert!(engine.source().unwrap().is_none());
    assert!(!clone.exists());
    reconnect(&f, &mut engine);
    let view = engine.settings().unwrap();
    assert_eq!(view.inputs[0].value.as_deref(), Some("payments-team"));
    assert!(view.secrets[0].set);
    assert_eq!(store.get(ACCOUNT).as_deref(), Some("tok"));
}

#[test]
fn disconnect_with_forget_clears_everything() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());
    engine.set_input("team", Some("payments-team")).unwrap();
    engine.set_secret("GITLAB_TOKEN", "tok").unwrap();

    engine.disconnect(true).unwrap();

    assert!(stored_inputs(&f).is_empty());
    assert!(secret_rows(&f).is_empty());
    assert_eq!(store.get(ACCOUNT), None);
}

#[test]
fn disconnect_with_forget_stops_on_keyring_failure() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());
    engine.set_input("team", Some("payments-team")).unwrap();
    engine.set_secret("GITLAB_TOKEN", "tok").unwrap();
    store.set_failing(true);

    let err = engine.disconnect(true).unwrap_err();

    assert!(matches!(err, EngineError::Keyring { .. }), "{err:?}");
    assert!(engine.source().unwrap().is_some());
    assert_eq!(stored_inputs(&f).len(), 1);
    assert_eq!(secret_rows(&f), ["GITLAB_TOKEN"]);
    assert_eq!(store.get(ACCOUNT).as_deref(), Some("tok"));
}

#[test]
fn disconnect_drops_a_pending_plan() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());
    f.write("dashboards/extra.yaml", "apiVersion: pharos.io/v1alpha1\nkind: Dashboard\nmetadata: { name: extra }\nspec: { widgets: [] }\n");
    f.commit("extra");
    engine.plan_sync().unwrap();

    engine.disconnect(false).unwrap();

    engine.add_source(&f.url(), "main").unwrap();
    assert!(engine.pending_sync().is_none());
}

#[test]
fn a_stale_value_counts_as_missing_even_with_a_default() {
    let f = Fixture::new();
    let mut engine = f.synced_engine_with(MemoryStore::default());
    let platform = fs::read_to_string(f.origin.join("pharos.yaml")).unwrap();
    f.write(
        "pharos.yaml",
        &platform.replace(
            "      required: true\n",
            "      required: true\n      default: platform-team\n",
        ),
    );
    f.resync(&mut engine, "default team");
    engine.set_input("team", Some("payments-team")).unwrap();

    f.delete("catalog/teams/payments-team.yaml");
    f.resync(&mut engine, "drop payments");

    assert!(engine.settings().unwrap().inputs[0].stale);
    assert_eq!(
        pipelines_status(&engine),
        PlacementStatus::NeedsSetup {
            inputs: vec!["team".into()],
            secrets: vec!["GITLAB_TOKEN".into()],
        }
    );
}

#[test]
fn a_partly_failed_forget_keeps_records_in_step_with_the_store() {
    let f = Fixture::new();
    let store = MemoryStore::default();
    let mut engine = f.synced_engine_with(store.clone());
    let platform = fs::read_to_string(f.origin.join("pharos.yaml")).unwrap();
    f.write("pharos.yaml", &format!("{platform}    - name: OTHER\n"));
    f.resync(&mut engine, "second secret");
    engine.set_secret("GITLAB_TOKEN", "tok").unwrap();
    engine.set_secret("OTHER", "other").unwrap();
    store.fail_on("acme-platform/OTHER");

    let err = engine.disconnect(true).unwrap_err();

    assert!(
        matches!(err, EngineError::Keyring { ref name, .. } if name == "OTHER"),
        "{err:?}"
    );
    assert!(engine.source().unwrap().is_some());
    // GITLAB_TOKEN is gone from the store, so it must not still be recorded as set.
    assert_eq!(store.get(ACCOUNT), None);
    assert_eq!(secret_rows(&f), ["OTHER"]);
    let secrets = engine.settings().unwrap().secrets;
    assert!(
        !secrets
            .iter()
            .find(|s| s.name == "GITLAB_TOKEN")
            .unwrap()
            .set
    );
}
