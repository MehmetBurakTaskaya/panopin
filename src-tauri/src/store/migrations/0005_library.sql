-- The Library (D-053): nested folders and tags. An item can sit in several folders and carry
-- several tags; deleting a folder or a tag removes only the grouping, never the items.
CREATE TABLE folders (
    id         INTEGER PRIMARY KEY,
    parent_id  INTEGER REFERENCES folders (id) ON DELETE CASCADE,
    name       TEXT    NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE INDEX folders_parent ON folders (parent_id);

-- name_key is the name folded by Rust (the four Turkish i forms are one letter, then lower case),
-- so "İş", "iş" and "IŞ" are one tag.
CREATE TABLE tags (
    id       INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL,
    name_key TEXT    NOT NULL UNIQUE
) STRICT;

CREATE TABLE item_folders (
    item_id   INTEGER NOT NULL REFERENCES items (id) ON DELETE CASCADE,
    folder_id INTEGER NOT NULL REFERENCES folders (id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, folder_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX item_folders_folder ON item_folders (folder_id);

CREATE TABLE item_tags (
    item_id INTEGER NOT NULL REFERENCES items (id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (item_id, tag_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX item_tags_tag ON item_tags (tag_id);
