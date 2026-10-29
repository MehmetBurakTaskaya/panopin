-- Captured items. Times are Unix milliseconds. Media files live in media/<id>/ (D-006).
CREATE TABLE items (
    id             INTEGER PRIMARY KEY,
    kind           TEXT    NOT NULL CHECK (kind IN ('image', 'text', 'link', 'video')),
    identity       BLOB    NOT NULL,
    created_at     INTEGER NOT NULL,
    last_copied_at INTEGER NOT NULL,
    title          TEXT,
    note           TEXT,
    pinned         INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    trashed_at     INTEGER,
    source         TEXT    NOT NULL CHECK (source IN ('clipboard', 'folder', 'import', 'manual')),
    source_app     TEXT,
    source_path    TEXT,
    text_body      TEXT,
    url            TEXT,
    width          INTEGER,
    height         INTEGER,
    byte_size      INTEGER,
    media_ext      TEXT,
    thumb_ext      TEXT,
    UNIQUE (kind, identity)
) STRICT;

CREATE INDEX items_shelf ON items (kind, trashed_at, last_copied_at DESC);
