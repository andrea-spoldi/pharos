//! Shared fixture: an "origin" git repo seeded from examples/platform-repo, cloned by the
//! engine like any remote.
#![allow(dead_code)] // each test crate uses a different subset

use cockpit_engine::Engine;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
