//! Platform inputs and secrets, against a synced clone of examples/platform-repo.

mod common;

use common::{Fixture, MemoryStore};
use pharos_engine::{Engine, EngineError};
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
}
