//! Thin wrapper over the system `git` (D-001): it inherits the user's SSH agent and
//! credential helpers, and its errors are reported verbatim.

use std::path::Path;
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git is not installed or not on PATH")]
    Missing,
    #[error("`git {command}` failed: {stderr}")]
    Failed { command: String, stderr: String },
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<String, GitError> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    // Never block on an interactive credential prompt; fail with git's message instead.
    cmd.args(args).env("GIT_TERMINAL_PROMPT", "0");
    let failed = |stderr: String| GitError::Failed {
        command: args.join(" "),
        stderr,
    };
    let out = cmd.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => GitError::Missing,
        _ => failed(e.to_string()),
    })?;
    if !out.status.success() {
        return Err(failed(
            String::from_utf8_lossy(&out.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

pub fn version() -> Result<String, GitError> {
    git(None, &["--version"])
}

pub fn clone(url: &str, branch: &str, dest: &Path) -> Result<(), GitError> {
    let dest = dest.to_string_lossy();
    git(
        None,
        &[
            "clone",
            "--quiet",
            "--single-branch",
            &format!("--branch={branch}"),
            "--",
            url,
            &dest,
        ],
    )
    .map(drop)
}

/// Fetches `branch` and moves the app-owned clone to it. Returns the new HEAD.
pub fn fetch_and_reset(dir: &Path, branch: &str) -> Result<String, GitError> {
    git(
        Some(dir),
        &["fetch", "--quiet", "--prune", "origin", branch],
    )?;
    reset_to(dir, "FETCH_HEAD")?;
    head(dir)
}

/// The clone is never edited by hand, so local changes are always discarded.
pub fn reset_to(dir: &Path, rev: &str) -> Result<(), GitError> {
    git(Some(dir), &["reset", "--quiet", "--hard", rev])?;
    git(Some(dir), &["clean", "-ffdxq"]).map(drop)
}

pub fn head(dir: &Path) -> Result<String, GitError> {
    git(Some(dir), &["rev-parse", "HEAD"])
}
