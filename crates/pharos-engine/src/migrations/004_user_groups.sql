-- User composites (slice 8b). A group is a placement with no widget and a title; its
-- members reference it by parent_id. SQLite cannot relax NOT NULL in place, so the
-- table is rebuilt (widget becomes nullable, title is added).
CREATE TABLE placements_new (
    id           INTEGER PRIMARY KEY,
    dashboard_id INTEGER NOT NULL REFERENCES dashboards (id) ON DELETE CASCADE,
    parent_id    INTEGER REFERENCES placements (id) ON DELETE CASCADE,
    widget       TEXT,
    title        TEXT,
    x            INTEGER NOT NULL,
    y            INTEGER NOT NULL,
    w            INTEGER NOT NULL,
    h            INTEGER NOT NULL,
    position     INTEGER NOT NULL DEFAULT 0,
    inputs       TEXT NOT NULL DEFAULT '{}'
);

INSERT INTO placements_new (id, dashboard_id, parent_id, widget, x, y, w, h, position, inputs)
SELECT id, dashboard_id, parent_id, widget, x, y, w, h, position, inputs FROM placements;

DROP TABLE placements;
ALTER TABLE placements_new RENAME TO placements;

CREATE INDEX placements_by_dashboard ON placements (dashboard_id, position);
