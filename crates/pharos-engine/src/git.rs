//! Thin wrapper over the system `git` (D-001): it inherits the user's SSH agent and
//! credential helpers, and its errors are reported verbatim.

use crate::process::{RunError, run};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Upper bound for one git invocation; network calls are the slow ones.
const TIMEOUT: Duration = Duration::from_secs(120);
/// Largest output kept from one git invocation; ours are short (hashes, versions, errors).
const MAX_OUTPUT: u64 = 1024 * 1024;

/// Transports git may use. Excludes `ext::`, which runs arbitrary commands.
const ALLOWED_PROTOCOLS: &str = "file:git:http:https:ssh";

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git is not installed or not on PATH")]
    Missing,
    #[error("`git {command}` failed: {stderr}")]
    Failed { command: String, stderr: String },
    #[error("`git {command}` did not finish within {}s", .after.as_secs())]
    TimedOut { command: String, after: Duration },
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<String, GitError> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    // Never block on an interactive credential prompt; fail with git's message instead.
    cmd.args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ALLOW_PROTOCOL", ALLOWED_PROTOCOLS);
    if is_network(args) && !custom_ssh(dir) {
        // Unknown host keys and key passphrases fail instead of prompting on a tty.
        cmd.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }
    let command = args.join(" ");
    let out = run(cmd, TIMEOUT, MAX_OUTPUT).map_err(|e| match e {
        RunError::NotFound => GitError::Missing,
        RunError::Io(e) => GitError::Failed {
            command: command.clone(),
            stderr: e.to_string(),
        },
        RunError::TimedOut => GitError::TimedOut {
            command: command.clone(),
            after: TIMEOUT,
        },
    })?;
    if !out.status.success() {
        return Err(GitError::Failed {
            command,
            stderr: out.stderr.trim().to_owned(),
        });
    }
    Ok(out.stdout.trim().to_owned())
}

fn is_network(args: &[&str]) -> bool {
    matches!(args.first(), Some(&"clone" | &"fetch"))
}

/// The user chose their own ssh wrapper (env or `core.sshCommand`); leave it alone.
fn custom_ssh(dir: Option<&Path>) -> bool {
    if std::env::var_os("GIT_SSH_COMMAND").is_some() || std::env::var_os("GIT_SSH").is_some() {
        return true;
    }
    git(dir, &["config", "--get", "core.sshCommand"]).is_ok_and(|v| !v.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ext_transport_is_refused() {
        let dest = std::env::temp_dir().join("pharos-ext-transport-test");
        let err = clone("ext::sh -c true", "main", &dest).unwrap_err();
        assert!(matches!(err, GitError::Failed { .. }), "{err:?}");
        assert!(!dest.exists());
    }
}
