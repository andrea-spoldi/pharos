//! Runs a child process without a shell or stdin, in its own process group, with a deadline.
//! Shared by git and command sources.

use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(crate) struct Output {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
    /// stdout or stderr had more than the byte limit; the rest was discarded.
    pub truncated: bool,
}

#[derive(Debug)]
pub(crate) enum RunError {
    NotFound,
    Io(io::Error),
    TimedOut,
}

/// Runs `cmd` in its own process group with no stdin, killing the whole group (the child and
/// anything it spawned) after `timeout`. Keeps at most `max_bytes` of each output stream.
pub(crate) fn run(mut cmd: Command, timeout: Duration, max_bytes: u64) -> Result<Output, RunError> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => RunError::NotFound,
        _ => RunError::Io(e),
    })?;
    let stdout = drain(&mut child, max_bytes, |c| {
        c.stdout.take().map(|p| Box::new(p) as _)
    });
    let stderr = drain(&mut child, max_bytes, |c| {
        c.stderr.take().map(|p| Box::new(p) as _)
    });
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
    let (stdout, out_cut) = stdout.join().unwrap_or_default();
    let (stderr, err_cut) = stderr.join().unwrap_or_default();
    Ok(Output {
        status,
        stdout,
        stderr,
        truncated: out_cut || err_cut,
    })
}

type Pipe = Box<dyn Read + Send>;

/// Reads a pipe on a thread so a full pipe buffer cannot stall the child. Past `max_bytes`
/// it keeps reading but discards, and reports the cut.
fn drain(
    child: &mut Child,
    max_bytes: u64,
    take: impl FnOnce(&mut Child) -> Option<Pipe>,
) -> thread::JoinHandle<(String, bool)> {
    let pipe = take(child);
    thread::spawn(move || {
        let mut buf = Vec::new();
        let mut cut = false;
        if let Some(mut pipe) = pipe {
            let _ = (&mut pipe).take(max_bytes).read_to_end(&mut buf);
            cut = io::copy(&mut pipe, &mut io::sink()).is_ok_and(|n| n > 0);
        }
        (String::from_utf8_lossy(&buf).into_owned(), cut)
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

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: u64 = 1024 * 1024;

    #[test]
    fn kills_the_process_group_on_timeout() {
        let mut cmd = Command::new("sh");
        // The grandchild keeps stdout open; it must die with the group.
        cmd.args(["-c", "sleep 30 & sleep 30"]);
        let started = Instant::now();
        let err = run(cmd, Duration::from_millis(200), MAX).unwrap_err();
        assert!(matches!(err, RunError::TimedOut), "{err:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn captures_output_and_status() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo out; echo err >&2; exit 3"]);
        let out = run(cmd, Duration::from_secs(5), MAX).unwrap();
        assert_eq!(out.status.code(), Some(3));
        assert_eq!(out.stdout, "out\n");
        assert_eq!(out.stderr, "err\n");
        assert!(!out.truncated);
    }

    #[test]
    fn caps_output_without_stalling_the_child() {
        let mut cmd = Command::new("sh");
        // Far more than a pipe buffer; the child must still finish.
        cmd.args(["-c", "head -c 1000000 /dev/zero"]);
        let out = run(cmd, Duration::from_secs(5), 10).unwrap();
        assert!(out.status.success());
        assert_eq!(out.stdout.len(), 10);
        assert!(out.truncated);
    }
}
