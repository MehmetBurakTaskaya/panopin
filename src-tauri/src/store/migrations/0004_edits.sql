-- Edited images (D-050): the size after cropping, NULL while the image has no edit layer, and
-- when the layer or the thumbnail last changed, which versions the thumbnail URL.
ALTER TABLE items ADD COLUMN edit_width INTEGER;
ALTER TABLE items ADD COLUMN edit_height INTEGER;
ALTER TABLE items ADD COLUMN edited_at INTEGER;
