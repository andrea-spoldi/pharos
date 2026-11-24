use crate::sync::{Pending, Snapshot};
use pharos_manifest::CapabilitySet;
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::{M, Migrations};
use serde::Serialize;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATION_LIST: &[M<'_>] = &[
    M::up(include_str!("migrations/001_sources.sql")),
    M::up(include_str!("migrations/002_settings.sql")),
    M::up(include_str!("migrations/003_dashboards.sql")),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATION_LIST);

pub fn open(path: &std::path::Path) -> Result<Connection, crate::EngineError> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}

/// The configured platform repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
// Only renames the TS type (a widget `Source` already exists); JSON is unaffected.
#[serde(rename = "RepoSource")]
pub struct Source {
    // Row ids and unix seconds stay far below 2^53, so `number` is lossless.
    #[cfg_attr(feature = "specta", specta(type = u32))]
    pub id: i64,
    pub url: String,
    pub branch: String,
    /// App-owned clone.
    pub local_path: PathBuf,
    /// `None` until the first sync is applied.
    pub synced_commit: Option<String>,
    #[cfg_attr(feature = "specta", specta(type = Option<u32>))]
    pub synced_at: Option<i64>,
    pub approved_capability_hash: Option<String>,
}

pub fn source(conn: &Connection) -> rusqlite::Result<Option<Source>> {
    conn.query_row(
        "SELECT id, url, branch, local_path, synced_commit, synced_at, approved_capability_hash
         FROM sources ORDER BY id LIMIT 1",
        [],
        |r| {
            Ok(Source {
                id: r.get(0)?,
                url: r.get(1)?,
                branch: r.get(2)?,
                local_path: PathBuf::from(r.get::<_, String>(3)?),
                synced_commit: r.get(4)?,
                synced_at: r.get(5)?,
                approved_capability_hash: r.get(6)?,
            })
        },
    )
    .optional()
}

pub fn insert_source(conn: &Connection, url: &str, branch: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO sources (url, branch, local_path, created_at) VALUES (?1, ?2, '', ?3)",
        params![url, branch, now()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn set_local_path(conn: &Connection, id: i64, path: &std::path::Path) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sources SET local_path = ?2 WHERE id = ?1",
        params![id, path.to_string_lossy()],
    )
    .map(drop)
}

pub fn delete_source(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM sources WHERE id = ?1", [id])
        .map(drop)
}

pub fn snapshot(conn: &Connection, source_id: i64) -> rusqlite::Result<Snapshot> {
    let platform = conn.query_row(
        "SELECT platform FROM sources WHERE id = ?1",
        [source_id],
        |r| r.get(0),
    )?;
    Ok(Snapshot {
        platform,
        widgets: manifests(conn, "widgets", source_id)?,
        dashboards: manifests(conn, "dashboard_templates", source_id)?,
    })
}

/// name → manifest JSON, ordered by name.
pub fn manifests(
    conn: &Connection,
    table: &str,
    source_id: i64,
) -> rusqlite::Result<std::collections::BTreeMap<String, String>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT name, manifest FROM {table} WHERE source_id = ?1"
    ))?;
    stmt.query_map([source_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect()
}

/// A widget's repo-relative file and manifest JSON from the stored snapshot.
pub fn widget(
    conn: &Connection,
    source_id: i64,
    name: &str,
) -> rusqlite::Result<Option<(String, String)>> {
    conn.query_row(
        "SELECT file, manifest FROM widgets WHERE source_id = ?1 AND name = ?2",
        params![source_id, name],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()
}

/// What the user consented to; empty until they have.
pub fn approved_capabilities(conn: &Connection, source_id: i64) -> rusqlite::Result<CapabilitySet> {
    let json: Option<String> = conn.query_row(
        "SELECT approved_capabilities FROM sources WHERE id = ?1",
        [source_id],
        |r| r.get(0),
    )?;
    Ok(json
        .map(|j| serde_json::from_str(&j).expect("stored capabilities are valid JSON"))
        .unwrap_or_default())
}

pub fn store_snapshot(
    conn: &mut Connection,
    source_id: i64,
    pending: &Pending,
    consent: bool,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let plan = &pending.plan;
    tx.execute(
        "UPDATE sources SET synced_commit = ?2, synced_at = ?3, platform = ?4 WHERE id = ?1",
        params![source_id, plan.to_commit, now(), pending.platform],
    )?;
    if consent {
        let capabilities =
            serde_json::to_string(&plan.capabilities).expect("capabilities serialise");
        tx.execute(
            "UPDATE sources SET approved_capability_hash = ?2, approved_capabilities = ?3 WHERE id = ?1",
            params![source_id, plan.capability_hash, capabilities],
        )?;
    }
    for (table, rows) in [
        ("widgets", &pending.widgets),
        ("dashboard_templates", &pending.dashboards),
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE source_id = ?1"),
            [source_id],
        )?;
        let mut insert = tx.prepare(&format!(
            "INSERT INTO {table} (source_id, name, file, manifest) VALUES (?1, ?2, ?3, ?4)"
        ))?;
        for (name, (file, manifest)) in rows {
            insert.execute(params![source_id, name, file, manifest])?;
        }
    }
    tx.commit()
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

// ── App settings (not tied to a platform) ───────────────────────────────────

pub fn app_setting(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
        r.get(0)
    })
    .optional()
}

pub fn set_app_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map(drop)
}

pub fn delete_app_setting(conn: &Connection, key: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", [key])
        .map(drop)
}

// ── Settings (keyed by platform name) ───────────────────────────────────────

pub fn set_input(
    conn: &Connection,
    platform: &str,
    key: &str,
    value: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO user_inputs (platform, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (platform, key) DO UPDATE SET value = excluded.value",
        params![platform, key, value],
    )
    .map(drop)
}

pub fn delete_input(conn: &Connection, platform: &str, key: &str) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM user_inputs WHERE platform = ?1 AND key = ?2",
        params![platform, key],
    )
    .map(drop)
}

pub fn set_secret_meta(conn: &Connection, platform: &str, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO secrets_meta (platform, name, set_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (platform, name) DO UPDATE SET set_at = excluded.set_at",
        params![platform, name, now()],
    )
    .map(drop)
}

pub fn delete_secret_meta(conn: &Connection, platform: &str, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM secrets_meta WHERE platform = ?1 AND name = ?2",
        params![platform, name],
    )
    .map(drop)
}

/// Deletes a platform's stored inputs and secret names.
pub fn forget_platform(conn: &Connection, platform: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM user_inputs WHERE platform = ?1", [platform])?;
    conn.execute("DELETE FROM secrets_meta WHERE platform = ?1", [platform])
        .map(drop)
}

/// Every stored input as (platform, key, value).
pub fn all_inputs(conn: &Connection) -> rusqlite::Result<Vec<(String, String, String)>> {
    let mut stmt =
        conn.prepare("SELECT platform, key, value FROM user_inputs ORDER BY platform, key")?;
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect()
}

/// Every stored secret name as (platform, name).
pub fn all_secrets(conn: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt =
        conn.prepare("SELECT platform, name FROM secrets_meta ORDER BY platform, name")?;
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect()
}

// ── User dashboards ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardRow {
    pub id: i64,
    pub name: String,
    pub title: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRow {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub widget: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub position: i64,
    /// JSON object text.
    pub inputs: String,
}

/// Appends after the existing dashboards.
pub fn insert_dashboard(
    conn: &Connection,
    name: &str,
    title: Option<&str>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO dashboards (name, title, position)
         VALUES (?1, ?2, (SELECT COALESCE(MAX(position) + 1, 0) FROM dashboards))",
        params![name, title],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn rename_dashboard(conn: &Connection, id: i64, title: Option<&str>) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE dashboards SET title = ?2 WHERE id = ?1",
        params![id, title],
    )
    .map(drop)
}

/// Placements go with it (`ON DELETE CASCADE`; needs `foreign_keys = ON`, which `open` sets).
pub fn delete_dashboard(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM dashboards WHERE id = ?1", [id])
        .map(drop)
}

pub fn user_dashboards(conn: &Connection) -> rusqlite::Result<Vec<DashboardRow>> {
    let mut stmt =
        conn.prepare("SELECT id, name, title, position FROM dashboards ORDER BY position, id")?;
    stmt.query_map([], |r| {
        Ok(DashboardRow {
            id: r.get(0)?,
            name: r.get(1)?,
            title: r.get(2)?,
            position: r.get(3)?,
        })
    })?
    .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn insert_placement(
    conn: &Connection,
    dashboard_id: i64,
    parent_id: Option<i64>,
    widget: &str,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    position: i64,
    inputs_json: &str,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO placements (dashboard_id, parent_id, widget, x, y, w, h, position, inputs)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            dashboard_id,
            parent_id,
            widget,
            x,
            y,
            w,
            h,
            position,
            inputs_json
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Child placements go with it (`ON DELETE CASCADE`).
pub fn delete_placement(conn: &Connection, id: i64) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM placements WHERE id = ?1", [id])
        .map(drop)
}

pub fn placements_of(conn: &Connection, dashboard_id: i64) -> rusqlite::Result<Vec<PlacementRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, widget, x, y, w, h, position, inputs
         FROM placements WHERE dashboard_id = ?1 ORDER BY position, id",
    )?;
    stmt.query_map([dashboard_id], |r| {
        Ok(PlacementRow {
            id: r.get(0)?,
            parent_id: r.get(1)?,
            widget: r.get(2)?,
            x: r.get(3)?,
            y: r.get(4)?,
            w: r.get(5)?,
            h: r.get(6)?,
            position: r.get(7)?,
            inputs: r.get(8)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        MIGRATIONS.validate().unwrap();
    }

    #[test]
    fn user_dashboard_roundtrip() {
        MIGRATIONS.validate().unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        MIGRATIONS.to_latest(&mut conn).unwrap();

        let d = insert_dashboard(&conn, "mine", Some("Mine")).unwrap();
        // Inserted out of order to prove `position` drives the ordering.
        let b = insert_placement(&conn, d, None, "b-widget", 6, 0, 6, 4, 1, "{\"k\":1}").unwrap();
        let a = insert_placement(&conn, d, None, "a-widget", 0, 0, 6, 4, 0, "{}").unwrap();

        let ds = user_dashboards(&conn).unwrap();
        assert_eq!(ds.len(), 1);
        assert_eq!(ds[0].id, d);
        assert_eq!(ds[0].name, "mine");
        assert_eq!(ds[0].title.as_deref(), Some("Mine"));

        let ps = placements_of(&conn, d).unwrap();
        assert_eq!(ps.iter().map(|p| p.id).collect::<Vec<_>>(), vec![a, b]);
        assert_eq!(ps[1].widget, "b-widget");
        assert_eq!((ps[1].x, ps[1].y, ps[1].w, ps[1].h), (6, 0, 6, 4));
        assert_eq!(ps[1].inputs, "{\"k\":1}");
        assert_eq!(ps[0].parent_id, None);

        rename_dashboard(&conn, d, None).unwrap();
        assert_eq!(user_dashboards(&conn).unwrap()[0].title, None);

        delete_placement(&conn, a).unwrap();
        assert_eq!(placements_of(&conn, d).unwrap().len(), 1);

        delete_dashboard(&conn, d).unwrap();
        assert!(user_dashboards(&conn).unwrap().is_empty());
        assert!(placements_of(&conn, d).unwrap().is_empty());
    }
}
