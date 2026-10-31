//! Shared fixture: an "origin" git repo seeded from examples/platform-repo, cloned by the
//! engine like any remote.
#![allow(dead_code)] // each test crate uses a different subset

use pharos_engine::{Engine, SecretStore};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// In-memory stand-in for the OS keyring; clones share state, `fail` makes every call fail.
#[derive(Clone, Default)]
pub struct MemoryStore {
    pub entries: Arc<Mutex<BTreeMap<String, String>>>,
    pub fail: Arc<AtomicBool>,
    /// Fail only calls for this account.
    pub fail_on: Arc<Mutex<Option<String>>>,
}

impl MemoryStore {
    pub fn get(&self, account: &str) -> Option<String> {
        self.entries.lock().unwrap().get(account).cloned()
    }

    pub fn set_failing(&self, fail: bool) {
        self.fail.store(fail, Ordering::SeqCst);
    }

    pub fn fail_on(&self, account: &str) {
        *self.fail_on.lock().unwrap() = Some(account.into());
    }

    fn fails(&self, account: &str) -> bool {
        self.fail.load(Ordering::SeqCst) || self.fail_on.lock().unwrap().as_deref() == Some(account)
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        if self.fails(account) {
            return Err("store unavailable".into());
        }
        Ok(self.entries.lock().unwrap().get(account).cloned())
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        if self.fails(account) {
            return Err("store unavailable".into());
        }
        self.entries
            .lock()
            .unwrap()
            .insert(account.into(), value.into());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        if self.fails(account) {
            return Err("store unavailable".into());
        }
        self.entries.lock().unwrap().remove(account);
        Ok(())
    }
}

pub struct Fixture {
    _tmp: tempfile::TempDir,
    pub origin: PathBuf,
    pub data: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
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

    pub fn url(&self) -> String {
        self.origin.to_string_lossy().into_owned()
    }

    pub fn write(&self, rel: &str, body: &str) {
        let path = self.origin.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    pub fn delete(&self, rel: &str) {
        fs::remove_file(self.origin.join(rel)).unwrap();
    }

    pub fn remove(&self, rel: &str) {
        fs::remove_dir_all(self.origin.join(rel)).unwrap();
    }

    pub fn commit(&self, message: &str) {
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

    /// An engine with the first sync applied, keeping secrets in `store`.
    pub fn synced_engine_with(&self, store: MemoryStore) -> Engine {
        let mut engine = Engine::open_with(&self.data, Box::new(store)).unwrap();
        engine.add_source(&self.url(), "main").unwrap();
        engine.plan_sync().unwrap();
        engine.apply_sync(true).unwrap();
        engine
    }

    /// Commits the origin and applies the resulting sync.
    pub fn resync(&self, engine: &mut Engine, message: &str) {
        self.commit(message);
        engine.plan_sync().unwrap();
        engine.apply_sync(true).unwrap();
    }

    /// An engine with the first sync applied.
    pub fn synced_engine(&self) -> Engine {
        let mut engine = Engine::open(&self.data).unwrap();
        engine.add_source(&self.url(), "main").unwrap();
        engine.plan_sync().unwrap();
        engine.apply_sync(true).unwrap();
        engine
    }
}

pub fn git(dir: &Path, args: &[&str]) -> String {
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
