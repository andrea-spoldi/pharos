//! Local engine: platform repo sync and app state. Tauri-free so it is unit-testable.
//!
//! Sync is plan → review → apply. `plan_sync` fetches the repo, lints it and diffs it
//! against the stored snapshot; the plan stays inside the engine until `apply_sync`
//! (with consent if capabilities changed) or `discard_sync`.

mod dashboard;
mod db;
mod git;
mod run;
mod sync;

use cockpit_manifest::{Dashboard, Platform, Widget};
use rusqlite::Connection;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub use dashboard::{DashboardView, PlacementStatus, PlacementView};
pub use db::Source;
pub use git::GitError;
pub use run::{FileData, FileError, MAX_FILES};
use sync::Pending;
pub use sync::{Changes, SyncPlan};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Git(#[from] GitError),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("a platform repo is already configured")]
    SourceExists,
    #[error("no platform repo is configured")]
    NoSource,
    #[error("invalid branch name `{0}`")]
    InvalidBranch(String),
    #[error("no sync is waiting for review")]
    NoPendingSync,
    #[error("the platform repo has {0} lint error(s); it cannot be applied")]
    LintErrors(usize),
    #[error("the platform repo's capabilities changed and need approval")]
    ConsentRequired,
    #[error("the local clone changed after the sync was planned; plan it again")]
    StalePlan,
    #[error("no widget named `{0}`")]
    UnknownWidget(String),
    #[error("widget `{0}` has no data source")]
    NoDataSource(String),
    #[error("{0} sources are not supported yet")]
    SourceNotSupported(&'static str),
    #[error(transparent)]
    File(#[from] FileError),
}

pub type Result<T> = std::result::Result<T, EngineError>;

pub struct Engine {
    conn: Connection,
    data_dir: PathBuf,
    pending: Option<Pending>,
}

impl Engine {
    /// Opens (and migrates) the state in `data_dir`, creating it if needed.
    pub fn open(data_dir: &Path) -> Result<Self> {
        fs::create_dir_all(data_dir)?;
        let conn = db::open(&data_dir.join("cockpit.db"))?;
        Ok(Self {
            conn,
            data_dir: data_dir.to_path_buf(),
            pending: None,
        })
    }

    /// `git --version`; onboarding checks this before asking for a repo (D-001).
    pub fn git_version() -> Result<String> {
        Ok(git::version()?)
    }

    pub fn source(&self) -> Result<Option<Source>> {
        Ok(db::source(&self.conn)?)
    }

    /// Clones the platform repo. Nothing is stored if the clone fails.
    pub fn add_source(&mut self, url: &str, branch: &str) -> Result<Source> {
        if self.source()?.is_some() {
            return Err(EngineError::SourceExists);
        }
        if branch.is_empty() || branch.starts_with('-') || branch.contains(char::is_whitespace) {
            return Err(EngineError::InvalidBranch(branch.into()));
        }
        let id = db::insert_source(&self.conn, url, branch)?;
        let dest = self.data_dir.join("sources").join(id.to_string());
        let cloned = (|| {
            if dest.exists() {
                fs::remove_dir_all(&dest)?;
            }
            fs::create_dir_all(dest.parent().expect("dest has a parent"))?;
            git::clone(url, branch, &dest)?;
            db::set_local_path(&self.conn, id, &dest)?;
            Ok(())
        })();
        if let Err(e) = cloned {
            let _ = fs::remove_dir_all(&dest);
            db::delete_source(&self.conn, id)?;
            return Err(e);
        }
        self.source()?.ok_or(EngineError::NoSource)
    }

    /// Fetches the branch, lints it and diffs it against the stored snapshot.
    pub fn plan_sync(&mut self) -> Result<&SyncPlan> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let to_commit = git::fetch_and_reset(&source.local_path, &source.branch)?;

        let report = cockpit_manifest::lint(&source.local_path);
        let (repo, _) = cockpit_manifest::load(&source.local_path);
        let mut widgets = BTreeMap::new();
        let mut dashboards = BTreeMap::new();
        let mut platform = None;
        if let Some(repo) = &repo {
            platform = repo.platform.as_ref().map(|p| to_json(&p.manifest));
            for w in &repo.widgets {
                let name = w.manifest.metadata.name.clone();
                widgets
                    .entry(name)
                    .or_insert_with(|| (w.file.clone(), to_json(&w.manifest)));
            }
            for d in &repo.dashboards {
                let name = d.manifest.metadata.name.clone();
                dashboards
                    .entry(name)
                    .or_insert_with(|| (d.file.clone(), to_json(&d.manifest)));
            }
        }

        let old = db::snapshot(&self.conn, source.id)?;
        let plan = SyncPlan {
            from_commit: source.synced_commit.clone(),
            to_commit,
            platform_changed: old.platform != platform,
            widgets: sync::diff(&old.widgets, &widgets),
            dashboards: sync::diff(&old.dashboards, &dashboards),
            needs_consent: source.approved_capability_hash.as_deref()
                != Some(report.capability_hash.as_str()),
            diagnostics: report.diagnostics,
            capabilities: report.capabilities,
            capability_hash: report.capability_hash,
        };
        let pending = self.pending.insert(Pending {
            plan,
            platform,
            widgets,
            dashboards,
        });
        Ok(&pending.plan)
    }

    pub fn pending_sync(&self) -> Option<&SyncPlan> {
        self.pending.as_ref().map(|p| &p.plan)
    }

    /// Stores the planned snapshot. `consent` approves the plan's capability set;
    /// it is required when the plan says `needs_consent`.
    pub fn apply_sync(&mut self, consent: bool) -> Result<()> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let pending = self.pending.as_ref().ok_or(EngineError::NoPendingSync)?;
        let plan = &pending.plan;
        if plan.errors() > 0 {
            return Err(EngineError::LintErrors(plan.errors()));
        }
        if git::head(&source.local_path)? != plan.to_commit {
            return Err(EngineError::StalePlan);
        }
        if plan.needs_consent && !consent {
            return Err(EngineError::ConsentRequired);
        }
        db::store_snapshot(&mut self.conn, source.id, pending, consent)?;
        self.pending = None;
        Ok(())
    }

    /// Drops the plan and moves the clone back to the last applied commit, so files on
    /// disk always match the stored snapshot.
    pub fn discard_sync(&mut self) -> Result<()> {
        self.pending = None;
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        if let Some(commit) = &source.synced_commit {
            git::reset_to(&source.local_path, commit)?;
        }
        Ok(())
    }

    /// Widgets of the last applied sync, by name.
    pub fn widgets(&self) -> Result<Vec<Widget>> {
        self.load_manifests("widgets")
    }

    /// Platform dashboards of the last applied sync, by name.
    pub fn dashboard_templates(&self) -> Result<Vec<Dashboard>> {
        self.load_manifests("dashboard_templates")
    }

    /// Platform dashboards as the UI renders them: `home` first, then by name, each
    /// placement with its widget manifest and status.
    pub fn dashboards(&self) -> Result<Vec<DashboardView>> {
        let Some(source) = self.source()? else {
            return Ok(Vec::new());
        };
        let root = fs::canonicalize(&source.local_path)?;
        let mut dashboards: Vec<Dashboard> = self.dashboard_templates()?;
        dashboards.sort_by_key(|d| d.metadata.name != "home");
        dashboards
            .into_iter()
            .map(|d| {
                let placements = d
                    .spec
                    .widgets
                    .iter()
                    .map(|p| {
                        let stored = db::widget(&self.conn, source.id, &p.widget)?;
                        let manifest = stored.as_ref().map(|(_, json)| {
                            serde_json::from_str::<Widget>(json)
                                .expect("stored manifests are valid JSON")
                        });
                        let status = dashboard::placement_status(manifest.as_ref(), p);
                        let reads = match (&status, &stored, &manifest) {
                            (PlacementStatus::Runnable, Some((file, _)), Some(w)) => {
                                file_reads(&root, file, w)
                            }
                            _ => None,
                        };
                        Ok(PlacementView {
                            widget: p.widget.clone(),
                            x: p.x,
                            y: p.y,
                            w: p.w,
                            h: p.h,
                            manifest,
                            status,
                            reads,
                        })
                    })
                    .collect::<Result<_>>()?;
                Ok(DashboardView {
                    metadata: d.metadata,
                    placements,
                })
            })
            .collect()
    }

    /// The platform manifest (`cockpit.yaml`) of the last applied sync.
    pub fn platform(&self) -> Result<Option<Platform>> {
        let Some(source) = self.source()? else {
            return Ok(None);
        };
        Ok(db::snapshot(&self.conn, source.id)?
            .platform
            .map(|json| serde_json::from_str(&json).expect("stored manifests are valid JSON")))
    }

    /// Runs a widget as of the last applied sync, within the capabilities the user approved.
    pub fn run_widget(&self, name: &str) -> Result<FileData> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let (file, manifest) = db::widget(&self.conn, source.id, name)?
            .ok_or_else(|| EngineError::UnknownWidget(name.into()))?;
        let widget: Widget =
            serde_json::from_str(&manifest).expect("stored manifests are valid JSON");
        match widget.spec.source {
            None => Err(EngineError::NoDataSource(name.into())),
            Some(cockpit_manifest::Source::File { path, glob }) => {
                let root = fs::canonicalize(&source.local_path)?;
                let dir = root.join(&file);
                let dir = dir.parent().expect("widget file has a parent");
                let approved = db::approved_capabilities(&self.conn, source.id)?
                    .widgets
                    .remove(name)
                    .map(|caps| caps.paths)
                    .unwrap_or_default();
                Ok(run::read_files(
                    &root,
                    dir,
                    &path,
                    glob.as_deref(),
                    &approved,
                    MAX_FILES,
                )?)
            }
            Some(cockpit_manifest::Source::Command { .. }) => {
                Err(EngineError::SourceNotSupported("command"))
            }
            Some(cockpit_manifest::Source::Http { .. }) => {
                Err(EngineError::SourceNotSupported("http"))
            }
        }
    }

    fn load_manifests<T: DeserializeOwned>(&self, table: &str) -> Result<Vec<T>> {
        let Some(source) = self.source()? else {
            return Ok(Vec::new());
        };
        Ok(db::manifests(&self.conn, table, source.id)?
            .values()
            .map(|json| serde_json::from_str(json).expect("stored manifests are valid JSON"))
            .collect())
    }
}

/// A file source's path and glob as seen from the repo root, e.g. `catalog/services/*.yaml`.
fn file_reads(root: &Path, widget_file: &str, widget: &Widget) -> Option<String> {
    let Some(cockpit_manifest::Source::File { path, glob }) = &widget.spec.source else {
        return None;
    };
    let joined = root.join(widget_file);
    let dir = joined.parent()?;
    let resolved = cockpit_manifest::confine(root, dir, path).ok()?;
    let shown = cockpit_manifest::display_relative(root, &resolved);
    Some(match glob {
        Some(glob) => format!("{}/{glob}", shown.trim_end_matches('/')),
        None => shown,
    })
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("manifests serialise")
}
