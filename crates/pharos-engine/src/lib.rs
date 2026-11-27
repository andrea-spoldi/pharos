//! Local engine: platform repo sync and app state. Tauri-free so it is unit-testable.
//!
//! Sync is plan → review → apply. `plan_sync` fetches the repo, lints it and diffs it
//! against the stored snapshot; the plan stays inside the engine until `apply_sync`
//! (with consent if capabilities changed) or `discard_sync`.

mod command;
mod dashboard;
mod db;
mod git;
mod http;
mod process;
mod report;
mod run;
mod secrets;
mod settings;
mod sync;

use pharos_manifest::{
    CapabilitySet, Dashboard, InputDef, Metadata, Namespace, Placement, Platform, TemplateError,
    TemplateRef, ViewType, Widget, render_template, template_refs,
};
use rusqlite::Connection;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub use command::{CommandError, DEFAULT_SEARCH_PATH};
pub use dashboard::{DashboardView, Origin, PlacementGeometry, PlacementStatus, PlacementView};
pub use db::Source;
pub use git::GitError;
pub use http::{HttpError, MAX_BYTES, TIMEOUT};
pub use report::Report;
pub use run::{FileError, MAX_FILES, MAX_ROWS, RowsError, WidgetData};
pub use secrets::{KEYRING_SERVICE, KeyringStore, SecretStore};
pub use settings::{
    InputView, PlatformRef, SecretView, SettingsView, SetupState, StoredKind, StoredRef,
};
use sync::Pending;
pub use sync::{CapabilityChange, Changes, SyncPlan};

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
    #[error("the local clone changed after the sync was planned; plan it again")]
    StalePlan,
    #[error("no widget named `{0}`")]
    UnknownWidget(String),
    #[error("widget `{0}` has no data source")]
    NoDataSource(String),
    #[error("{0} are not available yet")]
    NotAvailable(String),
    #[error("dashboard `{dashboard}` has no widget at position {index}")]
    UnknownPlacement { dashboard: String, index: usize },
    #[error("no dashboard `{0}`")]
    UnknownDashboard(String),
    #[error("dashboard `{0}` comes from the platform and cannot be changed")]
    ReadOnlyDashboard(String),
    #[error("`{0}` is a container; only widgets with a data source can run")]
    NotALeaf(String),
    #[error("widget `{0}` has no new permissions to report")]
    NothingToReport(String),
    #[error("widget `{0}` needs approval")]
    NeedsApproval(String),
    #[error("widget `{0}` needs setup in Settings")]
    NeedsSetup(String),
    #[error("`{{{{ {0} }}}}` has no value")]
    MissingValue(String),
    #[error(transparent)]
    Template(#[from] TemplateError),
    #[error(transparent)]
    Http(#[from] HttpError),
    #[error(transparent)]
    File(#[from] FileError),
    #[error(transparent)]
    Command(#[from] CommandError),
    #[error("`{0}` is not declared in pharos.yaml")]
    Undeclared(String),
    #[error("invalid value for `{key}`: {reason}")]
    InvalidValue { key: String, reason: String },
    #[error("nothing is applied yet; settings are available after the first sync")]
    NoPlatform,
    /// `message` comes from the store and never contains the value.
    #[error("could not update secret `{name}` in the OS keyring: {message}")]
    Keyring { name: String, message: String },
}

/// What building a dashboard's placement views needs, computed once per listing.
struct ViewCtx<'a> {
    source: &'a Source,
    root: &'a Path,
    setup: &'a SetupState,
    /// Per widget: seen but not approved.
    pending: &'a CapabilitySet,
}

pub type Result<T> = std::result::Result<T, EngineError>;

pub struct Engine {
    conn: Connection,
    data_dir: PathBuf,
    pending: Option<Pending>,
    store: Box<dyn SecretStore>,
}

/// Deepest container nesting `placement_view` expands; a container at this depth is
/// reported `NotAvailable` instead of being expanded further.
pub const MAX_PLACEMENT_DEPTH: usize = 32;

impl Engine {
    /// Opens (and migrates) the state in `data_dir`, creating it if needed. Secrets go to
    /// the OS keyring.
    pub fn open(data_dir: &Path) -> Result<Self> {
        Self::open_with(data_dir, Box::new(KeyringStore))
    }

    /// Like [`Engine::open`], keeping secret values in `store`.
    pub fn open_with(data_dir: &Path, store: Box<dyn SecretStore>) -> Result<Self> {
        fs::create_dir_all(data_dir)?;
        let conn = db::open(&data_dir.join("pharos.db"))?;
        Ok(Self {
            conn,
            data_dir: data_dir.to_path_buf(),
            pending: None,
            store,
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

        let report = pharos_manifest::lint(&source.local_path);
        let (repo, _) = pharos_manifest::load(&source.local_path);
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
        let containers = repo
            .iter()
            .flat_map(|r| &r.widgets)
            .filter(|w| !w.manifest.spec.widgets.is_empty())
            .map(|w| w.manifest.metadata.name.clone())
            .collect();
        // `derive` omits widgets without capabilities; list every repo widget so one that
        // dropped its last capability still reports the removal.
        let mut declared = report.capabilities.clone();
        for w in repo.iter().flat_map(|r| &r.widgets) {
            declared
                .widgets
                .entry(w.manifest.metadata.name.clone())
                .or_default();
        }
        let capability_changes = sync::capability_changes(
            &db::approved_capabilities(&self.conn, source.id)?,
            &db::seen_capabilities(&self.conn, source.id)?,
            &declared,
            &containers,
        );
        let plan = SyncPlan {
            from_commit: source.synced_commit.clone(),
            to_commit,
            platform_changed: old.platform != platform,
            widgets: sync::diff(&old.widgets, &widgets),
            dashboards: sync::diff(&old.dashboards, &dashboards),
            needs_consent: capability_changes.values().any(|c| !c.added.is_empty()),
            capability_changes,
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

    /// Stores the planned snapshot. `approve` approves exactly the additions listed in the
    /// plan's `capability_changes`; without it they stay unapproved (and are not asked
    /// about again until they change). Removals always take effect.
    pub fn apply_sync(&mut self, approve: bool) -> Result<()> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let pending = self.pending.as_ref().ok_or(EngineError::NoPendingSync)?;
        let plan = &pending.plan;
        if plan.errors() > 0 {
            return Err(EngineError::LintErrors(plan.errors()));
        }
        if git::head(&source.local_path)? != plan.to_commit {
            return Err(EngineError::StalePlan);
        }
        let approved = sync::merge_approval(
            &db::approved_capabilities(&self.conn, source.id)?,
            &plan.capabilities,
            &plan.capability_changes,
            approve,
        );
        db::store_snapshot(&mut self.conn, source.id, pending, &approved)?;
        self.pending = None;
        Ok(())
    }

    /// What the user has approved, per widget; empty before the first applied sync.
    pub fn approved_capabilities(&self) -> Result<CapabilitySet> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        Ok(db::approved_capabilities(&self.conn, source.id)?)
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

    /// Dashboards as the UI renders them: platform `home` first, then platform by name,
    /// then the user's own by position; a valid `home.landing` dashboard moves to the
    /// front (a stale or malformed landing id is ignored). Each placement carries its
    /// widget manifest and status.
    pub fn dashboards(&self) -> Result<Vec<DashboardView>> {
        let Some(source) = self.source()? else {
            return Ok(Vec::new());
        };
        let root = fs::canonicalize(&source.local_path)?;
        let setup = self.settings()?.setup_state();
        let pending = self.pending_capabilities(&source)?;
        let ctx = ViewCtx {
            source: &source,
            root: &root,
            setup: &setup,
            pending: &pending,
        };
        let mut templates: Vec<Dashboard> = self.dashboard_templates()?;
        templates.sort_by_key(|d| d.metadata.name != "home");
        let mut views = Vec::new();
        for d in templates {
            let placements = d
                .spec
                .widgets
                .iter()
                .map(|p| self.placement_view(&ctx, p, 0))
                .collect::<Result<_>>()?;
            views.push(DashboardView {
                id: format!("p:{}", d.metadata.name),
                origin: Origin::Platform,
                editable: false,
                metadata: d.metadata,
                placements,
            });
        }
        for row in db::user_dashboards(&self.conn)? {
            views.push(self.user_view(&ctx, row)?);
        }
        if let Some(landing) = db::app_setting(&self.conn, LANDING_KEY)?
            && let Some(at) = views.iter().position(|v| v.id == landing)
        {
            let view = views.remove(at);
            views.insert(0, view);
        }
        Ok(views)
    }

    fn user_view(&self, ctx: &ViewCtx, row: db::DashboardRow) -> Result<DashboardView> {
        let rows = db::placements_of(&self.conn, row.id)?;
        let placements = rows
            .iter()
            .filter(|r| r.parent_id.is_none())
            .map(|r| self.user_placement_view(ctx, &rows, r, 0))
            .collect::<Result<Vec<_>>>()?;
        Ok(DashboardView {
            id: format!("u:{}", row.id),
            origin: Origin::User,
            editable: true,
            metadata: Metadata {
                name: row.name,
                title: row.title,
                version: None,
                description: None,
            },
            placements,
        })
    }

    /// One user placement: a widget row goes through the manifest path (its expanded
    /// children are the composite's own parts, so they stay id-less); a group row (no
    /// widget) is a titled container of the rows whose `parent_id` is its id.
    fn user_placement_view(
        &self,
        ctx: &ViewCtx,
        rows: &[db::PlacementRow],
        row: &db::PlacementRow,
        depth: usize,
    ) -> Result<PlacementView> {
        if row.widget.is_some() {
            let mut view = self.placement_view(ctx, &placement_of(row), 0)?;
            view.id = Some(row.id as u32);
            return Ok(view);
        }
        let too_deep = depth >= MAX_PLACEMENT_DEPTH;
        let children = if too_deep {
            Vec::new()
        } else {
            rows.iter()
                .filter(|c| c.parent_id == Some(row.id))
                .map(|c| self.user_placement_view(ctx, rows, c, depth + 1))
                .collect::<Result<Vec<_>>>()?
        };
        let status = if too_deep {
            PlacementStatus::NotAvailable {
                feature: "groups nested this deeply".to_string(),
            }
        } else {
            dashboard::container_status(true, &children)
        };
        Ok(PlacementView {
            id: Some(row.id as u32),
            widget: String::new(),
            container: true,
            title: row.title.clone(),
            x: row.x,
            y: row.y,
            w: row.w,
            h: row.h,
            manifest: None,
            status,
            reads: None,
            children,
        })
    }

    fn user_view_by_id(&self, row_id: i64) -> Result<DashboardView> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let root = fs::canonicalize(&source.local_path)?;
        let setup = self.settings()?.setup_state();
        let row = db::user_dashboards(&self.conn)?
            .into_iter()
            .find(|d| d.id == row_id)
            .ok_or_else(|| EngineError::UnknownDashboard(format!("u:{row_id}")))?;
        let pending = self.pending_capabilities(&source)?;
        let ctx = ViewCtx {
            source: &source,
            root: &root,
            setup: &setup,
            pending: &pending,
        };
        self.user_view(&ctx, row)
    }

    /// Per widget, what the last applied sync declared but the user has not approved.
    fn pending_capabilities(&self, source: &Source) -> Result<CapabilitySet> {
        let containers = self
            .widgets()?
            .into_iter()
            .filter(|w| !w.spec.widgets.is_empty())
            .map(|w| w.metadata.name)
            .collect();
        Ok(sync::pending_approvals(
            &db::seen_capabilities(&self.conn, source.id)?,
            &db::approved_capabilities(&self.conn, source.id)?,
            &containers,
        ))
    }

    /// Approves what the stored snapshot declares for leaf widget `name`: its approved set
    /// becomes its seen set. Reads only stored state, never a pending plan, so it approves
    /// exactly what the dashboards showed as blocked.
    pub fn approve_widget(&mut self, name: &str) -> Result<()> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let (_, json) = db::widget(&self.conn, source.id, name)?
            .ok_or_else(|| EngineError::UnknownWidget(name.into()))?;
        let widget: Widget = serde_json::from_str(&json).expect("stored manifests are valid JSON");
        if !widget.spec.widgets.is_empty() {
            return Err(EngineError::NotALeaf(name.into()));
        }
        let seen = db::seen_capabilities(&self.conn, source.id)?;
        let mut approved = db::approved_capabilities(&self.conn, source.id)?;
        match seen.widgets.get(name).filter(|c| !c.is_empty()) {
            Some(caps) => approved.widgets.insert(name.to_string(), caps.clone()),
            None => approved.widgets.remove(name),
        };
        Ok(db::set_approved(&self.conn, source.id, &approved)?)
    }

    /// A pre-filled report of leaf widget `name`'s pending permissions (seen minus approved,
    /// as the card shows). Reads only stored state; changes nothing.
    pub fn report_widget(&self, name: &str) -> Result<Report> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let (_, json) = db::widget(&self.conn, source.id, name)?
            .ok_or_else(|| EngineError::UnknownWidget(name.into()))?;
        let widget: Widget = serde_json::from_str(&json).expect("stored manifests are valid JSON");
        if !widget.spec.widgets.is_empty() {
            return Err(EngineError::NotALeaf(name.into()));
        }
        let pending = self.pending_capabilities(&source)?;
        let caps = pending
            .widgets
            .get(name)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| EngineError::NothingToReport(name.into()))?;
        let commit = source.synced_commit.as_deref().unwrap_or_default();
        let (title, body) = report::compose(name, commit, caps);
        let link = self
            .platform()?
            .and_then(|p| p.spec.support)
            .map(|s| report::link(&s, &title, &body));
        Ok(Report { title, body, link })
    }

    /// Widgets a user can place, by name.
    pub fn catalog(&self) -> Result<Vec<WidgetSummary>> {
        Ok(self
            .widgets()?
            .into_iter()
            .map(|w| WidgetSummary {
                container: !w.spec.widgets.is_empty(),
                title: w.metadata.title,
                name: w.metadata.name,
            })
            .collect())
    }

    /// Creates an empty user dashboard. Its `name` is the slug of `title`, with a numeric
    /// suffix (`-2`, `-3`, ...) when another user dashboard already has it.
    pub fn create_dashboard(&mut self, title: &str) -> Result<DashboardView> {
        let title = title.trim();
        if title.is_empty() {
            return Err(EngineError::InvalidValue {
                key: "title".into(),
                reason: "empty".into(),
            });
        }
        let taken: Vec<String> = db::user_dashboards(&self.conn)?
            .into_iter()
            .map(|d| d.name)
            .collect();
        let base = slugify(title);
        let mut name = base.clone();
        let mut n = 2;
        while taken.contains(&name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        let id = db::insert_dashboard(&self.conn, &name, Some(title))?;
        self.user_view_by_id(id)
    }

    /// Creates a user dashboard titled after the platform dashboard `platform_id` and
    /// seeds it with copies of that dashboard's own placements (widget, geometry,
    /// inputs). A container widget is copied as one row referencing it by name; it
    /// re-expands from the manifest at render time, so no nesting is copied.
    pub fn customize_dashboard(&mut self, platform_id: &str) -> Result<DashboardView> {
        let DashboardId::Platform(name) = parse_id(platform_id)? else {
            return Err(EngineError::UnknownDashboard(platform_id.into()));
        };
        let template = self
            .dashboard_templates()?
            .into_iter()
            .find(|d| d.metadata.name == name)
            .ok_or_else(|| EngineError::UnknownDashboard(platform_id.into()))?;
        // A blank title counts as absent: Customize seeds, it never errors on a degenerate one.
        let title = template
            .metadata
            .title
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or(&name);
        let created = self.create_dashboard(title)?;
        let row_id = self.user_dashboard_id(&created.id)?;
        for (position, p) in template.spec.widgets.iter().enumerate() {
            let inputs = serde_json::to_string(&p.inputs).expect("string maps serialize");
            db::insert_placement(
                &self.conn,
                row_id,
                None,
                Some(&p.widget),
                None,
                p.x,
                p.y,
                p.w,
                p.h,
                position as i64,
                &inputs,
            )?;
        }
        self.user_view_by_id(row_id)
    }

    pub fn rename_dashboard(&mut self, id: &str, title: &str) -> Result<()> {
        let row_id = self.user_dashboard_id(id)?;
        let title = title.trim();
        if title.is_empty() {
            return Err(EngineError::InvalidValue {
                key: "title".into(),
                reason: "empty".into(),
            });
        }
        Ok(db::rename_dashboard(&self.conn, row_id, Some(title))?)
    }

    /// Deletes the dashboard and its placements.
    pub fn delete_dashboard(&mut self, id: &str) -> Result<()> {
        let row_id = self.user_dashboard_id(id)?;
        Ok(db::delete_dashboard(&self.conn, row_id)?)
    }

    /// Appends `widget` below its siblings (the placements sharing `parent_id`; `None` is
    /// the top level), full grid-left, at a default size for its view.
    pub fn add_widget(
        &mut self,
        dashboard_id: &str,
        widget: &str,
        parent_id: Option<u32>,
    ) -> Result<DashboardView> {
        let row_id = self.user_dashboard_id(dashboard_id)?;
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let (_, json) = db::widget(&self.conn, source.id, widget)?
            .ok_or_else(|| EngineError::UnknownWidget(widget.into()))?;
        let manifest: Widget =
            serde_json::from_str(&json).expect("stored manifests are valid JSON");
        let (w, h) = default_size(&manifest);
        let (parent, y, position) = self.slot_in(dashboard_id, row_id, parent_id)?;
        db::insert_placement(
            &self.conn,
            row_id,
            parent,
            Some(widget),
            None,
            0,
            y,
            w,
            h,
            position,
            "{}",
        )?;
        self.user_view_by_id(row_id)
    }

    /// Appends an empty titled group below its siblings (see `add_widget`).
    pub fn create_group(
        &mut self,
        dashboard_id: &str,
        title: &str,
        parent_id: Option<u32>,
    ) -> Result<DashboardView> {
        let row_id = self.user_dashboard_id(dashboard_id)?;
        let title = title.trim();
        if title.is_empty() {
            return Err(EngineError::InvalidValue {
                key: "title".into(),
                reason: "empty".into(),
            });
        }
        let (parent, y, position) = self.slot_in(dashboard_id, row_id, parent_id)?;
        db::insert_placement(
            &self.conn,
            row_id,
            parent,
            None,
            Some(title),
            0,
            y,
            12,
            4,
            position,
            "{}",
        )?;
        self.user_view_by_id(row_id)
    }

    /// Sets the title of the placement (a group's name) whose row id is `placement_id`.
    pub fn rename_placement(
        &mut self,
        dashboard_id: &str,
        placement_id: u32,
        title: &str,
    ) -> Result<DashboardView> {
        let row_id = self.user_dashboard_id(dashboard_id)?;
        let title = title.trim();
        if title.is_empty() {
            return Err(EngineError::InvalidValue {
                key: "title".into(),
                reason: "empty".into(),
            });
        }
        let row = self.placement_row(dashboard_id, row_id, placement_id)?;
        if row.widget.is_some() {
            return Err(EngineError::InvalidValue {
                key: "placement".into(),
                reason: "not a group".into(),
            });
        }
        db::set_placement_title(&self.conn, row.id, title)?;
        self.user_view_by_id(row_id)
    }

    /// Persists new grid geometry for top-level placements of a user dashboard. Touches
    /// only `x, y, w, h`: runs resolve a top-level placement by its `position` index, so
    /// `position` must not follow the visual order. All-or-nothing: every item is
    /// validated before any write.
    pub fn set_layout(
        &mut self,
        dashboard_id: &str,
        items: &[PlacementGeometry],
    ) -> Result<DashboardView> {
        let row_id = self.user_dashboard_id(dashboard_id)?;
        let mut checked = Vec::with_capacity(items.len());
        for item in items {
            let row = self.placement_row(dashboard_id, row_id, item.placement_id)?;
            if row.parent_id.is_some() {
                return Err(EngineError::UnknownPlacement {
                    dashboard: dashboard_id.into(),
                    index: item.placement_id as usize,
                });
            }
            let (min_w, min_h) = if row.widget.is_none() { (3, 2) } else { (1, 1) };
            let reason = if item.w < min_w || item.h < min_h {
                Some(format!("size below {min_w}x{min_h}"))
            } else if item.x.saturating_add(item.w) > 12 {
                Some("exceeds the 12-column grid".to_string())
            } else {
                None
            };
            if let Some(reason) = reason {
                return Err(EngineError::InvalidValue {
                    key: "geometry".into(),
                    reason,
                });
            }
            checked.push((row.id, item));
        }
        for (id, g) in checked {
            db::set_placement_geometry(&self.conn, id, g.x, g.y, g.w, g.h)?;
        }
        self.user_view_by_id(row_id)
    }

    /// Removes the placement (at any depth) whose row id is `placement_id`
    /// (`PlacementView::id`); a group takes its members with it.
    pub fn remove_widget(
        &mut self,
        dashboard_id: &str,
        placement_id: u32,
    ) -> Result<DashboardView> {
        let row_id = self.user_dashboard_id(dashboard_id)?;
        let row = self.placement_row(dashboard_id, row_id, placement_id)?;
        db::delete_placement(&self.conn, row.id)?;
        self.user_view_by_id(row_id)
    }

    /// The row `placement_id` of this dashboard, at any depth.
    fn placement_row(
        &self,
        dashboard_id: &str,
        row_id: i64,
        placement_id: u32,
    ) -> Result<db::PlacementRow> {
        db::placements_of(&self.conn, row_id)?
            .into_iter()
            .find(|r| r.id == i64::from(placement_id))
            .ok_or_else(|| EngineError::UnknownPlacement {
                dashboard: dashboard_id.into(),
                index: placement_id as usize,
            })
    }

    /// Where a new placement under `parent_id` goes: the parent row id, then `y` and
    /// `position` below the siblings that share that parent. A parent must be a row of
    /// this dashboard.
    fn slot_in(
        &self,
        dashboard_id: &str,
        row_id: i64,
        parent_id: Option<u32>,
    ) -> Result<(Option<i64>, u32, i64)> {
        let rows = db::placements_of(&self.conn, row_id)?;
        let parent = match parent_id {
            Some(id) => {
                let row = self.placement_row(dashboard_id, row_id, id)?;
                if row.widget.is_some() {
                    return Err(EngineError::InvalidValue {
                        key: "parent".into(),
                        reason: "not a group".into(),
                    });
                }
                Some(row.id)
            }
            None => None,
        };
        let siblings: Vec<_> = rows.iter().filter(|r| r.parent_id == parent).collect();
        let y = siblings.iter().map(|r| r.y + r.h).max().unwrap_or(0);
        let position = siblings.iter().map(|r| r.position + 1).max().unwrap_or(0);
        Ok((parent, y, position))
    }

    /// Chooses the dashboard shown first; `None` clears it. A dashboard that later
    /// disappears just stops mattering (`dashboards` ignores a stale id).
    pub fn set_landing(&mut self, dashboard_id: Option<&str>) -> Result<()> {
        match dashboard_id {
            None => Ok(db::delete_app_setting(&self.conn, LANDING_KEY)?),
            Some(id) => {
                parse_id(id)?;
                Ok(db::set_app_setting(&self.conn, LANDING_KEY, id)?)
            }
        }
    }

    /// The row id of an existing user dashboard; platform ids are read-only.
    fn user_dashboard_id(&self, id: &str) -> Result<i64> {
        let DashboardId::User(row_id) = parse_id(id)? else {
            return Err(EngineError::ReadOnlyDashboard(id.into()));
        };
        db::user_dashboards(&self.conn)?
            .iter()
            .any(|d| d.id == row_id)
            .then_some(row_id)
            .ok_or_else(|| EngineError::UnknownDashboard(id.into()))
    }

    /// One placement with its widget and status; a container's children are expanded
    /// recursively (the manifest lint refuses cycles; `MAX_PLACEMENT_DEPTH` bounds a
    /// pathologically deep acyclic chain so one widget cannot break the whole list).
    fn placement_view(&self, ctx: &ViewCtx, p: &Placement, depth: usize) -> Result<PlacementView> {
        let stored = db::widget(&self.conn, ctx.source.id, &p.widget)?;
        let manifest = stored.as_ref().map(|(_, json)| {
            serde_json::from_str::<Widget>(json).expect("stored manifests are valid JSON")
        });
        let is_container = manifest
            .as_ref()
            .is_some_and(|w| !w.spec.widgets.is_empty());
        if depth >= MAX_PLACEMENT_DEPTH && is_container {
            return Ok(PlacementView {
                id: None,
                widget: p.widget.clone(),
                container: is_container,
                title: None,
                x: p.x,
                y: p.y,
                w: p.w,
                h: p.h,
                manifest,
                status: PlacementStatus::NotAvailable {
                    feature: "widgets nested this deeply".to_string(),
                },
                reads: None,
                children: Vec::new(),
            });
        }
        let children = match &manifest {
            Some(w) => w
                .spec
                .widgets
                .iter()
                .map(|c| self.placement_view(ctx, c, depth + 1))
                .collect::<Result<Vec<_>>>()?,
            None => Vec::new(),
        };
        let status = if is_container || manifest.is_none() {
            dashboard::container_status(manifest.is_some(), &children)
        } else {
            dashboard::placement_status(manifest.as_ref(), p, ctx.setup, ctx.pending)
        };
        let reads = match (&status, &stored, &manifest) {
            (PlacementStatus::Runnable, Some((file, _)), Some(w)) if !is_container => {
                file_reads(ctx.root, file, w)
            }
            _ => None,
        };
        Ok(PlacementView {
            id: None,
            widget: p.widget.clone(),
            container: is_container,
            title: None,
            x: p.x,
            y: p.y,
            w: p.w,
            h: p.h,
            manifest,
            status,
            reads,
            children,
        })
    }

    /// The platform manifest (`pharos.yaml`) of the last applied sync.
    pub fn platform(&self) -> Result<Option<Platform>> {
        let Some(source) = self.source()? else {
            return Ok(None);
        };
        Ok(db::snapshot(&self.conn, source.id)?
            .platform
            .map(|json| serde_json::from_str(&json).expect("stored manifests are valid JSON")))
    }

    /// Inputs and secrets of the applied platform, with what is stored, for Settings.
    pub fn settings(&self) -> Result<SettingsView> {
        let source = self.source()?;
        let platform = self.platform()?;
        let root = match (&source, &platform) {
            (Some(s), Some(_)) => fs::canonicalize(&s.local_path).ok(),
            _ => None,
        };
        Ok(settings::view(
            source,
            platform.as_ref(),
            root.as_deref(),
            &db::all_inputs(&self.conn)?,
            &db::all_secrets(&self.conn)?,
        ))
    }

    /// Sets (or, for `None` or blank, clears) a Platform input declared in `spec.inputs`.
    /// Enum values must be one of the current choices.
    pub fn set_input(&mut self, key: &str, value: Option<&str>) -> Result<()> {
        let (platform, root) = self.applied_platform()?;
        let def = platform
            .spec
            .inputs
            .get(key)
            .ok_or_else(|| EngineError::Undeclared(key.into()))?;
        let name = &platform.metadata.name;
        let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(db::delete_input(&self.conn, name, key)?);
        };
        if def.kind == pharos_manifest::InputType::Enum
            && !settings::choices(&root, def).iter().any(|c| c == value)
        {
            return Err(EngineError::InvalidValue {
                key: key.into(),
                reason: "not one of the choices".into(),
            });
        }
        Ok(db::set_input(&self.conn, name, key, value)?)
    }

    /// Stores a declared secret's value in the secret store (trimmed) and records its name.
    /// A store failure records nothing.
    pub fn set_secret(&mut self, name: &str, value: &str) -> Result<()> {
        let (platform, _) = self.applied_platform()?;
        if !platform.spec.secrets.iter().any(|s| s.name == name) {
            return Err(EngineError::Undeclared(name.into()));
        }
        let value = value.trim();
        if value.is_empty() {
            return Err(EngineError::InvalidValue {
                key: name.into(),
                reason: "empty".into(),
            });
        }
        let platform = &platform.metadata.name;
        self.store
            .set(&secrets::account(platform, name), value)
            .map_err(|message| EngineError::Keyring {
                name: name.into(),
                message,
            })?;
        Ok(db::set_secret_meta(&self.conn, platform, name)?)
    }

    /// Removes a declared secret from the store and its record.
    pub fn remove_secret(&mut self, name: &str) -> Result<()> {
        let (platform, _) = self.applied_platform()?;
        if !platform.spec.secrets.iter().any(|s| s.name == name) {
            return Err(EngineError::Undeclared(name.into()));
        }
        self.remove_stored(&platform.metadata.name, StoredKind::Secret, name)
    }

    /// Removes any stored input or secret, declared or not (removal only ever deletes).
    pub fn remove_stored(&mut self, platform: &str, kind: StoredKind, key: &str) -> Result<()> {
        match kind {
            StoredKind::Input => Ok(db::delete_input(&self.conn, platform, key)?),
            StoredKind::Secret => {
                self.store
                    .delete(&secrets::account(platform, key))
                    .map_err(|message| EngineError::Keyring {
                        name: key.into(),
                        message,
                    })?;
                Ok(db::delete_secret_meta(&self.conn, platform, key)?)
            }
        }
    }

    /// Forgets the platform repo: its row, snapshot, approval, pending plan and clone. With
    /// `forget`, also the applied platform's inputs and secrets, secrets first. A store
    /// failure stops with the repo still connected; secrets already deleted lose their
    /// record too, so Settings never shows "Set" for a value that is gone (D-028).
    pub fn disconnect(&mut self, forget: bool) -> Result<()> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        if forget && let Some(platform) = self.platform()? {
            let name = &platform.metadata.name;
            let secrets: Vec<String> = db::all_secrets(&self.conn)?
                .into_iter()
                .filter(|(p, _)| p == name)
                .map(|(_, n)| n)
                .collect();
            for secret in &secrets {
                self.store
                    .delete(&secrets::account(name, secret))
                    .map_err(|message| EngineError::Keyring {
                        name: secret.clone(),
                        message,
                    })?;
                db::delete_secret_meta(&self.conn, name, secret)?;
            }
            db::forget_platform(&self.conn, name)?;
        }
        self.pending = None;
        db::delete_source(&self.conn, source.id)?;
        // App-owned; `add_source` replaces a leftover destination anyway.
        let _ = fs::remove_dir_all(&source.local_path);
        Ok(())
    }

    /// The applied platform manifest and the canonical clone root.
    fn applied_platform(&self) -> Result<(Platform, PathBuf)> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let platform = self.platform()?.ok_or(EngineError::NoPlatform)?;
        Ok((platform, fs::canonicalize(&source.local_path)?))
    }

    /// Runs the leaf widget addressed by `path` on `dashboard_id` (`p:<name>` or `u:<id>`) as
    /// of the last applied sync, within the capabilities the user approved. `path` lists
    /// child indices: the first picks a top-level placement, each further one a child of
    /// the previous placement's container widget. The frontend names the slot only (D-031,
    /// superseded by this addressing); the leaf, its source, inputs and secrets are all
    /// re-derived here from the stored manifests, rows and settings. A path that does not
    /// resolve, or that ends on a container, is an error, never a panic.
    ///
    /// Consent is enforced at the addressed leaf: the approved capabilities looked up
    /// below are the leaf widget's own, so a leaf inside a composite is denied unless its
    /// own host/executable/path was approved.
    ///
    /// Not propagated, by design (in-app instance inputs are a non-goal of slice 8a): a
    /// container placement's own `inputs` do not reach its children, and a `user.*`
    /// reference on a container's own placement is not checked for `NeedsSetup`.
    pub fn run_placement(&self, dashboard_id: &str, path: &[usize]) -> Result<WidgetData> {
        let source = self.source()?.ok_or(EngineError::NoSource)?;
        let placement = self.resolve_path(&source, dashboard_id, path)?;
        let name = &placement.widget;
        if name.is_empty() {
            // A user group has no widget of its own to run.
            return Err(EngineError::NotALeaf("group".into()));
        }
        let (file, manifest) = db::widget(&self.conn, source.id, name)?
            .ok_or_else(|| EngineError::UnknownWidget(name.clone()))?;
        let widget: Widget =
            serde_json::from_str(&manifest).expect("stored manifests are valid JSON");
        if !widget.spec.widgets.is_empty() {
            return Err(EngineError::NotALeaf(name.clone()));
        }
        let settings = self.settings()?;
        // The same decision the dashboard showed: only runnable placements run.
        let pending = self.pending_capabilities(&source)?;
        match dashboard::placement_status(
            Some(&widget),
            &placement,
            &settings.setup_state(),
            &pending,
        ) {
            PlacementStatus::Runnable => {}
            PlacementStatus::NeedsApproval { .. } => {
                return Err(EngineError::NeedsApproval(name.clone()));
            }
            PlacementStatus::NeedsSetup { .. } => {
                return Err(EngineError::NeedsSetup(name.clone()));
            }
            PlacementStatus::NotAvailable { feature } => {
                return Err(EngineError::NotAvailable(feature));
            }
            PlacementStatus::Static | PlacementStatus::Missing => {
                return Err(EngineError::NoDataSource(name.clone()));
            }
        }
        let approved = db::approved_capabilities(&self.conn, source.id)?
            .widgets
            .remove(name)
            .unwrap_or_default();
        match &widget.spec.source {
            Some(src @ pharos_manifest::Source::File { path, glob, .. }) => {
                let root = fs::canonicalize(&source.local_path)?;
                let dir = root.join(&file);
                let dir = dir.parent().expect("widget file has a parent");
                Ok(run::read_files(
                    &root,
                    dir,
                    path,
                    glob.as_deref(),
                    src.rows(),
                    &approved.paths,
                    MAX_FILES,
                    MAX_ROWS,
                )?)
            }
            Some(
                src @ pharos_manifest::Source::Http {
                    url,
                    method,
                    headers,
                    ..
                },
            ) => {
                let values = Values {
                    user: settings.usable_inputs(),
                    placement: &placement,
                    widget_inputs: &widget.spec.inputs,
                    secrets: self.secret_values(std::iter::once(url).chain(headers.values()))?,
                };
                let url = render_template(url, |r| values.get(r, true))?;
                let headers = headers
                    .iter()
                    .map(|(n, v)| Ok((n.clone(), render_template(v, |r| values.get(r, false))?)))
                    .collect::<Result<Vec<_>>>()?;
                let request = http::Request {
                    method: *method,
                    url: &url,
                    headers: &headers,
                    rows: src.rows(),
                };
                let secrets: Vec<String> = values.secrets.into_values().collect();
                Ok(http::fetch(&request, &approved.hosts, &secrets)?)
            }
            Some(
                src @ pharos_manifest::Source::Command {
                    executable,
                    args,
                    timeout,
                    ..
                },
            ) => {
                // No secrets in args (lint): they would show in `ps`.
                let values = Values {
                    user: settings.usable_inputs(),
                    placement: &placement,
                    widget_inputs: &widget.spec.inputs,
                    secrets: BTreeMap::new(),
                };
                let args = args
                    .iter()
                    .map(|a| render_template(a, |r| values.get(r, false)))
                    .collect::<Result<Vec<_>>>()?;
                let invocation = command::Invocation {
                    executable,
                    args: &args,
                    timeout: command::timeout(timeout.as_deref()),
                    rows: src.rows(),
                };
                let search = command::expand(&self.search_path()?, home_dir().as_deref());
                Ok(command::execute(
                    &invocation,
                    &approved.executables,
                    &search,
                )?)
            }
            None => unreachable!("placement_status only lets sources run"),
        }
    }

    /// Walks `path` from the addressed dashboard's top-level placements down through
    /// container widgets (read fresh from the stored snapshot) and returns the placement
    /// it ends on. Every step is bounds-checked.
    fn resolve_path(
        &self,
        source: &Source,
        dashboard_id: &str,
        path: &[usize],
    ) -> Result<Placement> {
        let unknown = |index| EngineError::UnknownPlacement {
            dashboard: dashboard_id.into(),
            index,
        };
        let (&first, rest) = path.split_first().ok_or_else(|| unknown(0))?;
        let (mut current, rest) = match parse_id(dashboard_id)? {
            DashboardId::Platform(name) => {
                let top = self
                    .dashboard_templates()?
                    .into_iter()
                    .find(|d| d.metadata.name == name)
                    .ok_or_else(|| EngineError::UnknownDashboard(dashboard_id.into()))?
                    .spec
                    .widgets;
                (top.get(first).cloned().ok_or_else(|| unknown(first))?, rest)
            }
            DashboardId::User(row_id) => {
                if !db::user_dashboards(&self.conn)?
                    .iter()
                    .any(|d| d.id == row_id)
                {
                    return Err(EngineError::UnknownDashboard(dashboard_id.into()));
                }
                let rows = db::placements_of(&self.conn, row_id)?;
                let mut row = rows
                    .iter()
                    .filter(|r| r.parent_id.is_none())
                    .nth(first)
                    .ok_or_else(|| unknown(first))?;
                let mut rest = rest;
                // A user group's members are rows; descend through them until a widget.
                while row.widget.is_none() {
                    let Some((&index, tail)) = rest.split_first() else {
                        break;
                    };
                    row = rows
                        .iter()
                        .filter(|c| c.parent_id == Some(row.id))
                        .nth(index)
                        .ok_or_else(|| unknown(index))?;
                    rest = tail;
                }
                (placement_of(row), rest)
            }
        };
        for &index in rest {
            let (_, json) = db::widget(&self.conn, source.id, &current.widget)?
                .ok_or_else(|| EngineError::UnknownWidget(current.widget.clone()))?;
            let widget: Widget =
                serde_json::from_str(&json).expect("stored manifests are valid JSON");
            current = widget
                .spec
                .widgets
                .into_iter()
                .nth(index)
                .ok_or_else(|| unknown(index))?;
        }
        Ok(current)
    }

    /// Folders where command sources look up bare executable names, in order (D-033).
    pub fn search_path(&self) -> Result<Vec<String>> {
        Ok(match db::app_setting(&self.conn, SEARCH_PATH_KEY)? {
            Some(json) => serde_json::from_str(&json).expect("stored search path is JSON"),
            None => DEFAULT_SEARCH_PATH.iter().map(|d| d.to_string()).collect(),
        })
    }

    /// Replaces the search path; blank entries are dropped, and an empty list restores the
    /// default. Each entry must be absolute or start with `~/`.
    pub fn set_search_path(&mut self, entries: &[String]) -> Result<()> {
        let entries: Vec<&str> = entries
            .iter()
            .map(|e| e.trim())
            .filter(|e| !e.is_empty())
            .collect();
        if entries.is_empty() {
            return Ok(db::delete_app_setting(&self.conn, SEARCH_PATH_KEY)?);
        }
        for entry in &entries {
            if let Some(reason) = command::search_entry_problem(entry) {
                return Err(EngineError::InvalidValue {
                    key: entry.to_string(),
                    reason: reason.into(),
                });
            }
        }
        let json = serde_json::to_string(&entries).expect("strings serialise");
        Ok(db::set_app_setting(&self.conn, SEARCH_PATH_KEY, &json)?)
    }

    /// Values of the secrets `templates` reference: each must be declared by the applied
    /// platform, and only those are read from the store.
    fn secret_values<'a>(
        &self,
        templates: impl Iterator<Item = &'a String>,
    ) -> Result<BTreeMap<String, String>> {
        let (platform, _) = self.applied_platform()?;
        let mut values = BTreeMap::new();
        for t in templates {
            for r in template_refs(t)? {
                if r.namespace != Namespace::Secrets || values.contains_key(&r.key) {
                    continue;
                }
                if !platform.spec.secrets.iter().any(|s| s.name == r.key) {
                    return Err(EngineError::Undeclared(r.key));
                }
                let account = secrets::account(&platform.metadata.name, &r.key);
                let value = self
                    .store
                    .get(&account)
                    .map_err(|message| EngineError::Keyring {
                        name: r.key.clone(),
                        message,
                    })?
                    .ok_or_else(|| EngineError::MissingValue(format!("secrets.{}", r.key)))?;
                values.insert(r.key, value);
            }
        }
        Ok(values)
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

/// What `{{ … }}` in an http source resolves to for one placement.
struct Values<'a> {
    /// Platform inputs with a usable value (stored and not stale, else the default).
    user: BTreeMap<String, String>,
    placement: &'a Placement,
    widget_inputs: &'a BTreeMap<String, InputDef>,
    secrets: BTreeMap<String, String>,
}

impl Values<'_> {
    /// With `encode`, each value that comes from the user (an input or a secret) is
    /// percent-encoded for a URL; text written in the manifests is kept as is, so a
    /// placement input like `acme%2F{{ user.team }}` stays a single path segment.
    fn get(&self, r: &TemplateRef, encode: bool) -> Result<String> {
        let escape = |v: &String| {
            if encode {
                http::encode_component(v)
            } else {
                v.clone()
            }
        };
        let missing = || EngineError::MissingValue(ref_name(r));
        match r.namespace {
            Namespace::User => self.user.get(&r.key).map(escape).ok_or_else(missing),
            Namespace::Secrets => self.secrets.get(&r.key).map(escape).ok_or_else(missing),
            // A placement's inputs may only refer to `user.*` (lint); their defaults are text.
            Namespace::Inputs => match self.placement.inputs.get(&r.key) {
                Some(template) => render_template(template, |inner| match inner.namespace {
                    Namespace::User => self.get(inner, encode),
                    _ => Err(EngineError::MissingValue(ref_name(inner))),
                }),
                None => self
                    .widget_inputs
                    .get(&r.key)
                    .and_then(|def| def.default.clone())
                    .ok_or_else(missing),
            },
            Namespace::Env => Err(missing()),
        }
    }
}

fn ref_name(r: &TemplateRef) -> String {
    let ns = match r.namespace {
        Namespace::User => "user",
        Namespace::Inputs => "inputs",
        Namespace::Secrets => "secrets",
        Namespace::Env => "env",
    };
    format!("{ns}.{}", r.key)
}

/// A file source's path and glob as seen from the repo root, e.g. `catalog/services/*.yaml`.
fn file_reads(root: &Path, widget_file: &str, widget: &Widget) -> Option<String> {
    let Some(pharos_manifest::Source::File { path, glob, .. }) = &widget.spec.source else {
        return None;
    };
    let joined = root.join(widget_file);
    let dir = joined.parent()?;
    let resolved = pharos_manifest::confine(root, dir, path).ok()?;
    let shown = pharos_manifest::display_relative(root, &resolved);
    Some(match glob {
        Some(glob) => format!("{}/{glob}", shown.trim_end_matches('/')),
        None => shown,
    })
}

const SEARCH_PATH_KEY: &str = "command.search_path";
const LANDING_KEY: &str = "home.landing";

/// A widget a user can place.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WidgetSummary {
    pub name: String,
    pub title: Option<String>,
    /// Composed of other widgets rather than showing data itself.
    pub container: bool,
}

enum DashboardId {
    Platform(String),
    User(i64),
}

/// `p:<name>` or `u:<row id>`.
fn parse_id(id: &str) -> Result<DashboardId> {
    if let Some(name) = id.strip_prefix("p:") {
        return Ok(DashboardId::Platform(name.into()));
    }
    id.strip_prefix("u:")
        .and_then(|n| n.parse().ok())
        .map(DashboardId::User)
        .ok_or_else(|| EngineError::UnknownDashboard(id.into()))
}

fn placement_of(row: &db::PlacementRow) -> Placement {
    Placement {
        widget: row.widget.clone().unwrap_or_default(),
        x: row.x,
        y: row.y,
        w: row.w,
        h: row.h,
        inputs: serde_json::from_str(&row.inputs).unwrap_or_default(),
    }
}

/// Lowercase ASCII letters and digits joined by single dashes; `dashboard` if nothing is left.
fn slugify(title: &str) -> String {
    let slug = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "dashboard".into()
    } else {
        slug
    }
}

/// Default grid size for a newly placed widget, by view kind; containers get a wide band.
fn default_size(widget: &Widget) -> (u32, u32) {
    if !widget.spec.widgets.is_empty() {
        return (12, 4);
    }
    match widget.spec.view.as_ref().map(|v| v.kind) {
        Some(ViewType::Table | ViewType::List) => (12, 4),
        Some(ViewType::Markdown) => (6, 2),
        _ => (6, 3),
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("manifests serialise")
}
