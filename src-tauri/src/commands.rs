//! IPC surface over `cockpit_engine::Engine`. Every command runs on a blocking thread:
//! the engine shells out to git and holds a SQLite connection, and the lock must never
//! be taken on the main thread.

use cockpit_engine::{Engine, EngineError, GitError, Source, SyncPlan};
use cockpit_manifest::Widget;
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
    F: FnOnce(&mut Engine) -> cockpit_engine::Result<T> + Send + 'static,
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
    with_engine(state, move |e| e.add_source(url.trim(), branch.trim())).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_errors_map_to_kinds() {
        let cases = [
            (EngineError::Git(GitError::Missing), ErrorKind::GitMissing),
            (EngineError::ConsentRequired, ErrorKind::ConsentRequired),
            (EngineError::StalePlan, ErrorKind::StalePlan),
            (EngineError::LintErrors(2), ErrorKind::LintErrors),
        ];
        for (err, kind) in cases {
            let message = err.to_string();
            assert_eq!(CommandError::from(err), CommandError { kind, message });
        }
    }
}
