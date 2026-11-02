//! Command sources: one process per run, no shell, only an executable the user approved,
//! found in the search path from Settings. Checked on every run, not just at consent time.

use crate::process::{self, RunError};
use crate::run::{WidgetData, rows};
use serde_json::Value;
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Used when the manifest sets no `timeout`.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
/// A manifest `timeout` above this is lowered to it.
pub const MAX_TIMEOUT: Duration = Duration::from_secs(60);
/// Largest stdout read; anything bigger fails the run.
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;
/// Most rows one run yields; the rest are dropped and `truncated` is set.
pub const MAX_ROWS: usize = 500;
/// How much of stderr a message quotes.
const STDERR_EXCERPT: usize = 300;

/// Where bare executable names are looked up until the user changes it in Settings. Apps
/// started from Finder get only `/usr/bin:/bin:/usr/sbin:/sbin`, so Homebrew comes first.
pub const DEFAULT_SEARCH_PATH: &[&str] = &[
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "~/.local/bin",
    "/usr/bin",
    "/bin",
    "/usr/sbin",
    "/sbin",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("`{0}` is not an approved executable")]
    NotApproved(String),
    #[error("`{0}` must be a program name or an absolute path")]
    InvalidExecutable(String),
    #[error("`{executable}` was not found in {}", .searched.join(", "))]
    NotFound {
        executable: String,
        searched: Vec<String>,
    },
    #[error("`{executable}` exited with {}{}", status(.code), excerpt(.stderr))]
    Failed {
        executable: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("`{executable}` did not finish within {}s", .after.as_secs())]
    TimedOut { executable: String, after: Duration },
    #[error("`{0}` printed more than {mb} MB", mb = MAX_BYTES / 1024 / 1024)]
    TooLarge(String),
    #[error("could not run `{executable}`: {message}")]
    Io { executable: String, message: String },
}

fn status(code: &Option<i32>) -> String {
    match code {
        Some(code) => format!("code {code}"),
        None => "a signal".into(),
    }
}

fn excerpt(stderr: &str) -> String {
    if stderr.is_empty() {
        String::new()
    } else {
        format!(": {stderr}")
    }
}

/// A rendered command: args already resolved.
pub(crate) struct Invocation<'a> {
    pub executable: &'a str,
    pub args: &'a [String],
    pub timeout: Duration,
}

/// The manifest's `timeout`, defaulted and capped.
pub(crate) fn timeout(manifest: Option<&str>) -> Duration {
    manifest
        .and_then(pharos_manifest::parse_duration)
        .unwrap_or(DEFAULT_TIMEOUT)
        .min(MAX_TIMEOUT)
}

/// Runs `invocation` if its executable is in `approved` and parses stdout: JSON becomes rows,
/// anything else one `{ value }` row with the trimmed text. `search` is the Settings search
/// path, already expanded; the child gets it as its `PATH` too (e.g. for kubectl auth plugins).
pub(crate) fn execute(
    invocation: &Invocation,
    approved: &BTreeSet<String>,
    search: &[PathBuf],
) -> Result<WidgetData, CommandError> {
    let executable = invocation.executable;
    if !approved.contains(executable) {
        return Err(CommandError::NotApproved(executable.into()));
    }
    let program = resolve(executable, search)?;
    let path = std::env::join_paths(search).map_err(|e| CommandError::Io {
        executable: executable.into(),
        message: e.to_string(),
    })?;
    let mut cmd = std::process::Command::new(&program);
    cmd.args(invocation.args).env("PATH", path);

    let out = process::run(cmd, invocation.timeout, MAX_BYTES).map_err(|e| match e {
        RunError::TimedOut => CommandError::TimedOut {
            executable: executable.into(),
            after: invocation.timeout,
        },
        RunError::NotFound => not_found(executable, search),
        RunError::Io(e) => CommandError::Io {
            executable: executable.into(),
            message: e.to_string(),
        },
    })?;
    if !out.status.success() {
        let stderr = out.stderr.trim();
        let cut = stderr
            .char_indices()
            .nth(STDERR_EXCERPT)
            .map_or(stderr.len(), |(i, _)| i);
        return Err(CommandError::Failed {
            executable: executable.into(),
            code: out.status.code(),
            stderr: stderr[..cut].to_owned(),
        });
    }
    if out.truncated && out.stdout.len() as u64 >= MAX_BYTES {
        return Err(CommandError::TooLarge(executable.into()));
    }
    let text = out.stdout.trim();
    let value = serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.into()));
    Ok(rows(value, MAX_ROWS))
}

/// An absolute path is used as is; a bare name is the first executable file in `search`.
fn resolve(executable: &str, search: &[PathBuf]) -> Result<PathBuf, CommandError> {
    if executable.contains('/') {
        let path = Path::new(executable);
        if !path.is_absolute() {
            return Err(CommandError::InvalidExecutable(executable.into()));
        }
        return Ok(path.to_path_buf());
    }
    search
        .iter()
        .map(|dir| dir.join(executable))
        .find(|p| is_executable(p))
        .ok_or_else(|| not_found(executable, search))
}

fn is_executable(path: &Path) -> bool {
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

fn not_found(executable: &str, search: &[PathBuf]) -> CommandError {
    CommandError::NotFound {
        executable: executable.into(),
        searched: search.iter().map(|d| d.display().to_string()).collect(),
    }
}

/// Settings entries as directories: `~/` is the user's home; other entries must be absolute.
pub(crate) fn expand(entries: &[String], home: Option<&Path>) -> Vec<PathBuf> {
    entries
        .iter()
        .filter_map(|e| match e.strip_prefix("~/") {
            Some(rest) => home.map(|h| h.join(rest)),
            None => Some(PathBuf::from(e)),
        })
        .collect()
}

/// Why a Settings search path entry is not acceptable, if it isn't.
pub(crate) fn search_entry_problem(entry: &str) -> Option<&'static str> {
    if !(entry.starts_with('/') || entry.starts_with("~/")) {
        Some("must be an absolute folder or start with ~/")
    } else if entry.contains(':') {
        Some("must not contain `:`")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh() -> (BTreeSet<String>, Vec<PathBuf>) {
        (
            BTreeSet::from(["sh".to_string()]),
            vec![PathBuf::from("/bin"), PathBuf::from("/usr/bin")],
        )
    }

    fn run_sh(script: &str) -> Result<WidgetData, CommandError> {
        let (approved, search) = sh();
        let args = ["-c".to_string(), script.to_string()];
        execute(
            &Invocation {
                executable: "sh",
                args: &args,
                timeout: Duration::from_secs(5),
            },
            &approved,
            &search,
        )
    }

    #[test]
    fn text_output_is_one_value_row() {
        let data = run_sh("echo '  kind-dev  '").unwrap();
        assert_eq!(data.rows, vec![serde_json::json!({ "value": "kind-dev" })]);
    }

    #[test]
    fn json_output_becomes_rows() {
        let data = run_sh(r#"echo '[{"name":"a"},{"name":"b"}]'"#).unwrap();
        assert_eq!(data.rows.len(), 2);
        assert_eq!(data.rows[1]["name"], "b");
    }

    #[test]
    fn args_are_not_interpreted_by_a_shell() {
        let (approved, search) = sh();
        // `printf` gets `$HOME; echo x` verbatim: nothing expands it.
        let args = ["%s".to_string(), "$HOME; echo x".to_string()];
        let approved = approved.into_iter().chain(["printf".to_string()]).collect();
        let data = execute(
            &Invocation {
                executable: "printf",
                args: &args,
                timeout: Duration::from_secs(5),
            },
            &approved,
            &search,
        )
        .unwrap();
        assert_eq!(data.rows[0]["value"], "$HOME; echo x");
    }

    #[test]
    fn refuses_unapproved_executables_before_running() {
        let (_, search) = sh();
        let err = execute(
            &Invocation {
                executable: "sh",
                args: &[],
                timeout: DEFAULT_TIMEOUT,
            },
            &BTreeSet::from(["kubectl".to_string()]),
            &search,
        )
        .unwrap_err();
        assert_eq!(err, CommandError::NotApproved("sh".into()));
    }

    #[test]
    fn reports_exit_code_and_stderr() {
        let err = run_sh("echo 'error: current-context is not set' >&2; exit 1").unwrap_err();
        assert_eq!(
            err,
            CommandError::Failed {
                executable: "sh".into(),
                code: Some(1),
                stderr: "error: current-context is not set".into(),
            }
        );
    }

    #[test]
    fn times_out() {
        let (approved, search) = sh();
        let args = ["-c".to_string(), "sleep 30".to_string()];
        let err = execute(
            &Invocation {
                executable: "sh",
                args: &args,
                timeout: Duration::from_millis(200),
            },
            &approved,
            &search,
        )
        .unwrap_err();
        assert!(matches!(err, CommandError::TimedOut { .. }), "{err:?}");
    }

    #[test]
    fn missing_executables_name_the_folders_searched() {
        let approved = BTreeSet::from(["pharos-no-such-tool".to_string()]);
        let err = execute(
            &Invocation {
                executable: "pharos-no-such-tool",
                args: &[],
                timeout: DEFAULT_TIMEOUT,
            },
            &approved,
            &[PathBuf::from("/bin")],
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "`pharos-no-such-tool` was not found in /bin"
        );
    }

    #[test]
    fn resolves_only_absolute_paths_or_bare_names() {
        let search = [PathBuf::from("/bin")];
        assert_eq!(resolve("/bin/sh", &search), Ok(PathBuf::from("/bin/sh")));
        assert_eq!(resolve("sh", &search), Ok(PathBuf::from("/bin/sh")));
        assert_eq!(
            resolve("bin/sh", &search),
            Err(CommandError::InvalidExecutable("bin/sh".into()))
        );
    }

    #[test]
    fn timeouts_default_and_cap() {
        assert_eq!(timeout(None), DEFAULT_TIMEOUT);
        assert_eq!(timeout(Some("5s")), Duration::from_secs(5));
        assert_eq!(timeout(Some("1h")), MAX_TIMEOUT);
    }

    #[test]
    fn expands_home_entries() {
        let entries = ["~/.local/bin".to_string(), "/usr/bin".to_string()];
        assert_eq!(
            expand(&entries, Some(Path::new("/Users/me"))),
            vec![
                PathBuf::from("/Users/me/.local/bin"),
                PathBuf::from("/usr/bin")
            ]
        );
        assert_eq!(expand(&entries, None), vec![PathBuf::from("/usr/bin")]);
    }

    #[test]
    fn checks_search_entries() {
        assert_eq!(search_entry_problem("/opt/homebrew/bin"), None);
        assert_eq!(search_entry_problem("~/bin"), None);
        assert!(search_entry_problem("bin").is_some());
        assert!(search_entry_problem("/a:/b").is_some());
    }
}
