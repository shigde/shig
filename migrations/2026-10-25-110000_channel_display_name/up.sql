ALTER TABLE channels ADD COLUMN display_name VARCHAR(100) NOT NULL DEFAULT '';
UPDATE channels SET display_name = split_part(name, '@', 1);
