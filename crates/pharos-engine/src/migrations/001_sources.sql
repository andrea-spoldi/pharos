-- One row per platform repo. v1 allows a single source; source_id columns keep multi-repo a migration away.
CREATE TABLE sources (
    id                       INTEGER PRIMARY KEY,
    url                      TEXT NOT NULL,
    branch                   TEXT NOT NULL,
    local_path               TEXT NOT NULL,
    synced_commit            TEXT,
    synced_at                INTEGER,
    platform                 TEXT,  -- Platform manifest JSON at synced_commit
    approved_capability_hash TEXT,
    approved_capabilities    TEXT,  -- CapabilitySet JSON the user consented to
    created_at               INTEGER NOT NULL
);

-- Manifest snapshots at sources.synced_commit.
CREATE TABLE widgets (
    source_id INTEGER NOT NULL REFERENCES sources (id) ON DELETE CASCADE,
    name      TEXT NOT NULL,
    file      TEXT NOT NULL,
    manifest  TEXT NOT NULL,
    PRIMARY KEY (source_id, name)
);

-- Platform dashboards are read-only templates; user dashboards (slice 8) fork them.
CREATE TABLE dashboard_templates (
    source_id INTEGER NOT NULL REFERENCES sources (id) ON DELETE CASCADE,
    name      TEXT NOT NULL,
    file      TEXT NOT NULL,
    manifest  TEXT NOT NULL,
    PRIMARY KEY (source_id, name)
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
