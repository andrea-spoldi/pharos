-- User dashboards (slice 8). Platform dashboards stay manifest-based; these are the
-- user's own, app-local. Placements form a tree (parent_id) so containers can nest.
CREATE TABLE dashboards (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    title      TEXT,
    position   INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE placements (
    id           INTEGER PRIMARY KEY,
    dashboard_id INTEGER NOT NULL REFERENCES dashboards (id) ON DELETE CASCADE,
    parent_id    INTEGER REFERENCES placements (id) ON DELETE CASCADE,
    widget       TEXT NOT NULL,
    x            INTEGER NOT NULL,
    y            INTEGER NOT NULL,
    w            INTEGER NOT NULL,
    h            INTEGER NOT NULL,
    position     INTEGER NOT NULL DEFAULT 0,
    inputs       TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX placements_by_dashboard ON placements (dashboard_id, position);
