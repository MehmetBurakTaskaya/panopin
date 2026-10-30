-- Item ids are never reused (D-071). Media addresses, the webview's cache and the media folders
-- are keyed by id, so a new item must never take the id of one deleted for good. Inserts take
-- `last + 1`; the trigger also follows inserts that pick their own id.
CREATE TABLE item_ids (last INTEGER NOT NULL);
INSERT INTO item_ids (last) SELECT COALESCE(MAX(id), 0) FROM items;

CREATE TRIGGER items_take_id AFTER INSERT ON items BEGIN
    UPDATE item_ids SET last = MAX(last, NEW.id);
END;
