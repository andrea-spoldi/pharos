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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        MIGRATIONS.validate().unwrap();
    }
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
