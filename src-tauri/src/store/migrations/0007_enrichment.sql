-- Text found in images (D-056). ocr_state: 0 waiting, 1 done (the text may be empty), 2 failed.
ALTER TABLE items ADD COLUMN ocr_state INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN ocr_text TEXT;

CREATE INDEX items_ocr_waiting ON items (last_copied_at) WHERE kind = 'image' AND ocr_state = 0;

-- Link previews (D-057). link_state: 0 waiting, 1 done, 2 failed, 3 refused by the network rules.
-- The picture and the site icon are cached files; only their extensions are stored.
ALTER TABLE items ADD COLUMN link_state INTEGER NOT NULL DEFAULT 0;
ALTER TABLE items ADD COLUMN link_title TEXT;
ALTER TABLE items ADD COLUMN link_description TEXT;
ALTER TABLE items ADD COLUMN link_site TEXT;
ALTER TABLE items ADD COLUMN link_image_ext TEXT;
ALTER TABLE items ADD COLUMN link_icon_ext TEXT;
ALTER TABLE items ADD COLUMN link_checked_at INTEGER;

-- Screen recordings (D-058).
ALTER TABLE items ADD COLUMN duration_ms INTEGER;
