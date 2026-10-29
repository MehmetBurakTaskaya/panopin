-- Whether an item shows on the shelf. Taking down a saved item keeps it in the Library but
-- off the shelf; copying it again puts it back.
ALTER TABLE items ADD COLUMN shelved INTEGER NOT NULL DEFAULT 1 CHECK (shelved IN (0, 1));
