//! Thin wrapper over the system `git` (D-001): it inherits the user's SSH agent and
//! credential helpers, and its errors are reported verbatim.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Upper bound for one git invocation; network calls are the slow ones.
const TIMEOUT: Duration = Duration::from_secs(120);

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
    let out = run(cmd, TIMEOUT).map_err(|e| match e {
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
    if !out.success {
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

#[derive(Debug)]
struct Output {
    success: bool,
    stdout: String,
    stderr: String,
}

#[derive(Debug)]
enum RunError {
    NotFound,
    Io(std::io::Error),
    TimedOut,
}

/// Runs `cmd` in its own process group with no stdin, killing the whole group
/// (git and any ssh it spawned) after `timeout`.
fn run(mut cmd: Command, timeout: Duration) -> Result<Output, RunError> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => RunError::NotFound,
        _ => RunError::Io(e),
    })?;
    let stdout = drain(&mut child, |c| c.stdout.take().map(|p| Box::new(p) as _));
    let stderr = drain(&mut child, |c| c.stderr.take().map(|p| Box::new(p) as _));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(RunError::Io)? {
            break status;
        }
        if Instant::now() >= deadline {
            kill_group(&mut child);
            return Err(RunError::TimedOut);
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ok(Output {
        success: status.success(),
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

type Pipe = Box<dyn Read + Send>;

/// Reads a pipe on a thread so a full pipe buffer cannot stall the child.
fn drain(
    child: &mut Child,
    take: impl FnOnce(&mut Child) -> Option<Pipe>,
) -> thread::JoinHandle<String> {
    let pipe = take(child);
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn kill_group(child: &mut Child) {
    // The group id equals the child's pid (`process_group(0)`).
    if let Ok(pgid) = i32::try_from(child.id()) {
        // SAFETY: kill(2) has no memory-safety preconditions.
        unsafe {
            libc::kill(-pgid, libc::SIGKILL);
        }
    }
    let _ = child.wait();
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
    fn run_kills_the_process_group_on_timeout() {
        let mut cmd = Command::new("sh");
        // The grandchild keeps stdout open; it must die with the group.
        cmd.args(["-c", "sleep 30 & sleep 30"]);
        let started = Instant::now();
        let err = run(cmd, Duration::from_millis(200)).unwrap_err();
        assert!(matches!(err, RunError::TimedOut), "{err:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn run_captures_output_and_status() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo out; echo err >&2; exit 3"]);
        let out = run(cmd, TIMEOUT).unwrap();
        assert!(!out.success);
        assert_eq!(out.stdout, "out\n");
        assert_eq!(out.stderr, "err\n");
    }

    #[test]
    fn ext_transport_is_refused() {
        let dest = std::env::temp_dir().join("cockpit-ext-transport-test");
        let err = clone("ext::sh -c true", "main", &dest).unwrap_err();
        assert!(matches!(err, GitError::Failed { .. }), "{err:?}");
        assert!(!dest.exists());
    }
}
