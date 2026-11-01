ALTER TABLE recorded_videos DROP CONSTRAINT IF EXISTS videos_status_check;
ALTER TABLE recorded_videos ADD CONSTRAINT recorded_videos_status_check
    CHECK (status IN ('pending', 'recording', 'processing', 'ready', 'failed', 'deleting'));
