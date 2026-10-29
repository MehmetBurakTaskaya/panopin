-- Search (D-017): a contentless FTS5 index that Rust fills with folded copies of the searchable
-- fields. The plain columns treat the four Turkish i forms as one letter and ignore case; the
-- bare columns also drop the Turkish accents, so "cicek" finds "çiçek". The rowid is the item id.
CREATE VIRTUAL TABLE search USING fts5 (
    title, body, extra,
    title_bare, body_bare, extra_bare,
    content = '',
    contentless_delete = 1,
    tokenize = 'unicode61 remove_diacritics 0',
    prefix = '2 3'
);

-- A deleted item leaves the index with it.
CREATE TRIGGER items_leave_search AFTER DELETE ON items BEGIN
    DELETE FROM search WHERE rowid = old.id;
END;
