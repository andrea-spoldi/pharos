//! IPC surface over `pharos_engine::Engine`. Every command runs on a blocking thread:
//! the engine shells out to git and holds a SQLite connection, and the lock must never
//! be taken on the main thread.

use pharos_engine::{
    DashboardView, Engine, EngineError, FileData, FileError, GitError, Source, SyncPlan,
};
use pharos_manifest::{Platform, Widget};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::State;

pub struct AppState {
    engine: Arc<Mutex<Engine>>,
}

impl AppState {
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
        }
    }
}

/// What the UI branches on; `message` is for display only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    GitMissing,
    Git,
    GitTimedOut,
    SourceExists,
    NoSource,
    InvalidBranch,
    NoPendingSync,
    LintErrors,
    ConsentRequired,
    StalePlan,
    UnknownWidget,
    /// Static widget, or a source type not supported yet.
    NotRunnable,
    /// Outside the repo or not approved.
    PermissionDenied,
    /// The widget's files could not be read or parsed.
    DataError,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct CommandError {
    pub kind: ErrorKind,
    pub message: String,
}

impl From<EngineError> for CommandError {
    fn from(e: EngineError) -> Self {
        let kind = match &e {
            EngineError::Git(GitError::Missing) => ErrorKind::GitMissing,
            EngineError::Git(GitError::Failed { .. }) => ErrorKind::Git,
            EngineError::Git(GitError::TimedOut { .. }) => ErrorKind::GitTimedOut,
            EngineError::SourceExists => ErrorKind::SourceExists,
            EngineError::NoSource => ErrorKind::NoSource,
            EngineError::InvalidBranch(_) => ErrorKind::InvalidBranch,
            EngineError::NoPendingSync => ErrorKind::NoPendingSync,
            EngineError::LintErrors(_) => ErrorKind::LintErrors,
            EngineError::ConsentRequired => ErrorKind::ConsentRequired,
            EngineError::StalePlan => ErrorKind::StalePlan,
            EngineError::UnknownWidget(_) => ErrorKind::UnknownWidget,
            EngineError::NoDataSource(_) | EngineError::SourceNotSupported(_) => {
                ErrorKind::NotRunnable
            }
            EngineError::File(FileError::Denied { .. } | FileError::NotApproved(_)) => {
                ErrorKind::PermissionDenied
            }
            EngineError::File(
                FileError::Glob(_) | FileError::Read { .. } | FileError::Parse { .. },
            ) => ErrorKind::DataError,
            EngineError::Db(_) | EngineError::Migration(_) | EngineError::Io(_) => {
                ErrorKind::Internal
            }
        };
        Self {
            kind,
            message: e.to_string(),
        }
    }
}

type CmdResult<T> = Result<T, CommandError>;

fn internal(message: impl Into<String>) -> CommandError {
    CommandError {
        kind: ErrorKind::Internal,
        message: message.into(),
    }
}

/// Runs `f` against the engine on a blocking thread.
async fn with_engine<T, F>(state: State<'_, AppState>, f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce(&mut Engine) -> pharos_engine::Result<T> + Send + 'static,
{
    let engine = Arc::clone(&state.engine);
    tauri::async_runtime::spawn_blocking(move || {
        let mut engine = engine
            .lock()
            .map_err(|_| internal("engine lock poisoned"))?;
        f(&mut engine).map_err(CommandError::from)
    })
    .await
    .map_err(|e| internal(e.to_string()))?
}

#[tauri::command]
#[specta::specta]
pub async fn git_version() -> CmdResult<String> {
    tauri::async_runtime::spawn_blocking(Engine::git_version)
        .await
        .map_err(|e| internal(e.to_string()))?
        .map_err(CommandError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn get_source(state: State<'_, AppState>) -> CmdResult<Option<Source>> {
    with_engine(state, |e| e.source()).await
}

/// Onboarding: the user types the repo URL or local path. It reaches git only after `--`
/// and git is limited to `file`/`git`/`http(s)`/`ssh` transports (see engine `git.rs`).
#[tauri::command]
#[specta::specta]
pub async fn add_source(
    state: State<'_, AppState>,
    url: String,
    branch: String,
) -> CmdResult<Source> {
    let url = expand_home(url.trim(), std::env::var("HOME").ok().as_deref());
    with_engine(state, move |e| e.add_source(&url, branch.trim())).await
}

/// git does not expand `~`, but it is how people type local paths.
fn expand_home(input: &str, home: Option<&str>) -> String {
    match (input.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => format!("{}/{rest}", home.trim_end_matches('/')),
        _ => input.to_owned(),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn plan_sync(state: State<'_, AppState>) -> CmdResult<SyncPlan> {
    with_engine(state, |e| e.plan_sync().cloned()).await
}

#[tauri::command]
#[specta::specta]
pub async fn pending_sync(state: State<'_, AppState>) -> CmdResult<Option<SyncPlan>> {
    with_engine(state, |e| Ok(e.pending_sync().cloned())).await
}

/// `consent` approves the capabilities of the plan held by the engine, never a set
/// supplied by the frontend (D-014).
#[tauri::command]
#[specta::specta]
pub async fn apply_sync(state: State<'_, AppState>, consent: bool) -> CmdResult<()> {
    with_engine(state, move |e| e.apply_sync(consent)).await
}

#[tauri::command]
#[specta::specta]
pub async fn discard_sync(state: State<'_, AppState>) -> CmdResult<()> {
    with_engine(state, |e| e.discard_sync()).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_widgets(state: State<'_, AppState>) -> CmdResult<Vec<Widget>> {
    with_engine(state, |e| e.widgets()).await
}

/// Platform dashboards of the last applied sync, with each placement's status.
#[tauri::command]
#[specta::specta]
pub async fn list_dashboards(state: State<'_, AppState>) -> CmdResult<Vec<DashboardView>> {
    with_engine(state, |e| e.dashboards()).await
}

/// `pharos.yaml` of the last applied sync; `None` before the first apply.
#[tauri::command]
#[specta::specta]
pub async fn get_platform(state: State<'_, AppState>) -> CmdResult<Option<Platform>> {
    with_engine(state, |e| e.platform()).await
}

/// The frontend names the widget only; what is read, and from where, comes from the
/// stored manifest and the approved capabilities.
#[tauri::command]
#[specta::specta]
pub async fn run_widget(state: State<'_, AppState>, name: String) -> CmdResult<FileData> {
    with_engine(state, move |e| e.run_widget(&name)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_a_leading_tilde_only() {
        let home = Some("/Users/dev/");
        assert_eq!(expand_home("~/src/repo", home), "/Users/dev/src/repo");
        assert_eq!(
            expand_home("git@host:~/repo.git", home),
            "git@host:~/repo.git"
        );
        assert_eq!(expand_home("~other/repo", home), "~other/repo");
        assert_eq!(expand_home("~/src/repo", None), "~/src/repo");
    }

    #[test]
    fn engine_errors_map_to_kinds() {
        let cases = [
            (EngineError::Git(GitError::Missing), ErrorKind::GitMissing),
            (EngineError::ConsentRequired, ErrorKind::ConsentRequired),
            (EngineError::StalePlan, ErrorKind::StalePlan),
            (EngineError::LintErrors(2), ErrorKind::LintErrors),
            (
                EngineError::UnknownWidget("w".into()),
                ErrorKind::UnknownWidget,
            ),
            (
                EngineError::NoDataSource("w".into()),
                ErrorKind::NotRunnable,
            ),
            (
                EngineError::SourceNotSupported("http"),
                ErrorKind::NotRunnable,
            ),
            (
                EngineError::File(FileError::NotApproved("data".into())),
                ErrorKind::PermissionDenied,
            ),
            (
                EngineError::File(FileError::Denied {
                    path: "../../etc".into(),
                    reason: pharos_manifest::ConfineError::Escapes,
                }),
                ErrorKind::PermissionDenied,
            ),
            (
                EngineError::File(FileError::Glob("bad".into())),
                ErrorKind::DataError,
            ),
        ];
        for (err, kind) in cases {
            let message = err.to_string();
            assert_eq!(CommandError::from(err), CommandError { kind, message });
        }
    }
}
